use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use indicatif::ProgressBar;
use serde::Deserialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::task;

use super::{
    AdapterError, AdapterKind, AgentAdapter, ModelTokenCounts, UserMessage, UserMessageStream,
    normalize::{normalize_codex_text, normalize_model_id},
    stream_messages,
};

#[derive(Debug, Clone)]
pub struct CodexAdapter {
    root_dir: PathBuf,
}

impl CodexAdapter {
    pub fn new(home: impl AsRef<Path>) -> Self {
        Self {
            root_dir: home.as_ref().join(".codex"),
        }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: path.into(),
        }
    }

    fn sessions_dir(&self) -> PathBuf {
        self.root_dir.join("sessions")
    }

    fn archived_sessions_dir(&self) -> PathBuf {
        self.root_dir.join("archived_sessions")
    }

    fn state_db_path(&self) -> PathBuf {
        self.root_dir.join("state_5.sqlite")
    }

    pub async fn session_file_count(&self) -> Result<usize, AdapterError> {
        let sessions_dir = self.sessions_dir();
        let archived_sessions_dir = self.archived_sessions_dir();
        task::spawn_blocking(move || {
            let mut session_paths = Vec::new();

            if sessions_dir.exists() {
                collect_jsonl_files(&sessions_dir, &mut session_paths)?;
            }

            if archived_sessions_dir.exists() {
                collect_jsonl_files(&archived_sessions_dir, &mut session_paths)?;
            }

            Ok(session_paths.len())
        })
        .await
        .map_err(AdapterError::Join)?
    }

    pub async fn collect_messages_with_progress(
        &self,
        progress: ProgressBar,
    ) -> Result<Vec<UserMessage>, AdapterError> {
        let sessions_dir = self.sessions_dir();
        let archived_sessions_dir = self.archived_sessions_dir();
        let state_db_path = self.state_db_path();
        task::spawn_blocking(move || {
            let mut session_paths = Vec::new();

            if sessions_dir.exists() {
                collect_jsonl_files(&sessions_dir, &mut session_paths)?;
            }

            if archived_sessions_dir.exists() {
                collect_jsonl_files(&archived_sessions_dir, &mut session_paths)?;
            }

            session_paths.sort();

            let total_files = session_paths.len();
            let mut messages: Vec<UserMessage> = Vec::new();
            let session_models = read_session_models_from_state_db(&state_db_path)?;

            for (index, path) in session_paths.into_iter().enumerate() {
                progress.set_message(format!(
                    "Codex {}/{} · {}",
                    index + 1,
                    total_files,
                    path.file_name().unwrap().to_string_lossy()
                ));
                let session_model = session_models
                    .get(&path)
                    .cloned()
                    .or_else(|| session_models.get(&canonicalize_lossy(&path)).cloned());
                if let Ok(msgs) = parse_session_file(&path, session_model) {
                    messages.extend(msgs);
                }

                progress.inc(1);
            }

            Ok(messages)
        })
        .await
        .map_err(AdapterError::Join)?
    }
}

impl AgentAdapter for CodexAdapter {
    async fn check(&self) -> bool {
        self.sessions_dir().exists() || self.archived_sessions_dir().exists()
    }

    async fn poll(&self) -> Result<UserMessageStream, AdapterError> {
        let sessions_dir = self.sessions_dir();
        let archived_sessions_dir = self.archived_sessions_dir();
        let state_db_path = self.state_db_path();
        let messages = task::spawn_blocking(move || {
            let mut session_paths = Vec::new();

            if sessions_dir.exists() {
                collect_jsonl_files(&sessions_dir, &mut session_paths)?;
            }

            if archived_sessions_dir.exists() {
                collect_jsonl_files(&archived_sessions_dir, &mut session_paths)?;
            }

            session_paths.sort();

            let mut messages = Vec::new();
            let session_models = read_session_models_from_state_db(&state_db_path)?;

            for path in session_paths {
                let session_model = session_models
                    .get(&path)
                    .cloned()
                    .or_else(|| session_models.get(&canonicalize_lossy(&path)).cloned());
                if let Ok(msgs) = parse_session_file(&path, session_model) {
                    messages.extend(msgs);
                }
            }

            Ok(messages)
        })
        .await
        .map_err(AdapterError::Join)??;

        Ok(stream_messages(messages))
    }

    async fn tokens(&self) -> Result<i64, AdapterError> {
        let db_path = self.state_db_path();
        let total = task::spawn_blocking(move || {
            let connection = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .map_err(|source| AdapterError::SqliteOpen {
                path: db_path.clone(),
                source,
            })?;
            let total = connection
                .query_row(
                    "SELECT COALESCE(SUM(tokens_used), 0) FROM threads",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|source| AdapterError::SqliteQuery {
                    path: db_path.clone(),
                    source,
                })?;

            Ok(total)
        })
        .await
        .map_err(AdapterError::Join)??;

        Ok(total)
    }

    async fn tokens_by_model(&self) -> Result<ModelTokenCounts, AdapterError> {
        let db_path = self.state_db_path();
        task::spawn_blocking(move || read_model_tokens_from_state_db(&db_path))
            .await
            .map_err(AdapterError::Join)?
    }
}

fn parse_session_file(
    path: &Path,
    session_model: Option<String>,
) -> Result<Vec<UserMessage>, AdapterError> {
    let contents = fs::read_to_string(path).map_err(|source| AdapterError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut messages: Vec<UserMessage> = Vec::new();
    let mut legacy_timestamp_ms = 0_i64;
    let mut current_model = session_model.and_then(|model| normalize_model_id(&model));
    let mut pending_message_indexes: Vec<usize> = Vec::new();

    for (_index, raw_line) in contents.lines().enumerate() {
        let Ok(raw) = serde_json::from_str::<serde_json::Value>(raw_line) else {
            continue; // skip bad JSON lines
        };
        let line_type = raw.get("type").and_then(serde_json::Value::as_str);

        if line_type == Some("turn_context") {
            let normalized_model = raw
                .get("payload")
                .and_then(|payload| payload.get("model"))
                .and_then(serde_json::Value::as_str)
                .and_then(normalize_model_id);
            current_model = normalized_model.clone();
            for message_index in pending_message_indexes.drain(..) {
                messages[message_index].model = normalized_model.clone();
            }
            continue;
        }

        if line_type == Some("response_item")
            && raw
                .get("payload")
                .and_then(|payload| payload.get("type"))
                .and_then(serde_json::Value::as_str)
                == Some("message")
            && raw
                .get("payload")
                .and_then(|payload| payload.get("role"))
                .and_then(serde_json::Value::as_str)
                == Some("user")
        {
            // Skip lines with missing/invalid timestamp instead of failing
            let Some(timestamp) = raw.get("timestamp").and_then(|v| v.as_str()) else {
                continue;
            };
            let Ok(datetime) = OffsetDateTime::parse(timestamp, &Rfc3339) else {
                continue; // skip invalid timestamps
            };
            let content = raw
                .get("payload")
                .and_then(|payload| payload.get("content"))
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();

            for item in content {
                let Some(text) = item.get("text").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let text = normalize_codex_text(text);

                if !text.is_empty() {
                    messages.push(UserMessage {
                        adapter: AdapterKind::Codex,
                        model: current_model.clone(),
                        text,
                        time: (datetime.unix_timestamp_nanos() / 1_000_000) as i64,
                    });
                    pending_message_indexes.push(messages.len() - 1);
                }
            }
            continue;
        }

        if line_type == Some("message")
            && raw.get("role").and_then(serde_json::Value::as_str) == Some("user")
        {
            let content = raw
                .get("content")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();

            for item in content {
                let Some(text) = item.get("text").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let text = normalize_codex_text(text);

                if !text.is_empty() {
                    messages.push(UserMessage {
                        adapter: AdapterKind::Codex,
                        model: current_model.clone(),
                        text,
                        time: legacy_timestamp_ms,
                    });
                }
            }
            continue;
        }

        if raw.get("git").is_some() && raw.get("timestamp").is_some() {
            // Skip lines with missing/invalid timestamp instead of failing
            let Some(timestamp) = raw.get("timestamp").and_then(|v| v.as_str()) else {
                continue;
            };
            let Ok(datetime) = OffsetDateTime::parse(timestamp, &Rfc3339) else {
                continue; // skip invalid timestamps
            };
            legacy_timestamp_ms = (datetime.unix_timestamp_nanos() / 1_000_000) as i64;
        }
    }

    Ok(messages)
}

fn read_session_models_from_state_db(
    db_path: &Path,
) -> Result<BTreeMap<PathBuf, String>, AdapterError> {
    if !db_path.exists() {
        return Ok(BTreeMap::new());
    }

    let connection =
        rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|source| AdapterError::SqliteOpen {
                path: db_path.to_path_buf(),
                source,
            })?;
    let mut statement = connection
        .prepare("SELECT rollout_path, model FROM threads WHERE model IS NOT NULL AND model != ''")
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut models = BTreeMap::new();

    for row in rows {
        let (rollout_path, model) = row.map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
        let Some(model) = normalize_model_id(&model) else {
            continue;
        };
        models.insert(PathBuf::from(&rollout_path), model.clone());
        models.insert(canonicalize_lossy(Path::new(&rollout_path)), model);
    }

    Ok(models)
}

fn read_model_tokens_from_state_db(db_path: &Path) -> Result<ModelTokenCounts, AdapterError> {
    if !db_path.exists() {
        return Ok(BTreeMap::new());
    }

    let connection =
        rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|source| AdapterError::SqliteOpen {
                path: db_path.to_path_buf(),
                source,
            })?;
    let mut statement = connection
        .prepare(
            "SELECT model, COALESCE(SUM(tokens_used), 0) FROM threads WHERE model IS NOT NULL AND model != '' GROUP BY model",
        )
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut totals = BTreeMap::new();

    for row in rows {
        let (model, tokens) = row.map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
        let Some(model) = normalize_model_id(&model) else {
            continue;
        };
        *totals.entry(model).or_insert(0) += tokens;
    }

    Ok(totals)
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn collect_jsonl_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), AdapterError> {
    for entry in fs::read_dir(dir).map_err(|source| AdapterError::Io {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| AdapterError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();

        if path.is_dir() {
            collect_jsonl_files(&path, files)?;
        } else if path.to_string_lossy().ends_with(".jsonl") {
            files.push(path);
        }
    }

    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CodexLine {
    LegacySessionMeta(CodexLegacySessionMeta),
    NewUserMessage(CodexNewUserMessageLine),
    LegacyUserMessage(CodexLegacyUserMessageLine),
    LegacyStateRecord(CodexLegacyStateRecord),
    Ignored(CodexIgnoredLine),
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexLegacySessionMeta {
    id: String,
    timestamp: String,
    instructions: serde_json::Value,
    git: CodexLegacyGit,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexLegacyGit {
    commit_hash: String,
    branch: String,
    repository_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexNewUserMessageLine {
    _timestamp: String,
    #[serde(rename = "type")]
    _line_type: String,
    _payload: CodexNewUserMessagePayload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexNewUserMessagePayload {
    #[serde(rename = "type")]
    _item_type: String,
    _role: String,
    _content: Vec<CodexMessageContent>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexTurnContextLine {
    #[serde(rename = "timestamp")]
    _timestamp: String,
    #[serde(rename = "type")]
    _line_type: String,
    payload: CodexTurnContextPayload,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexTurnContextPayload {
    model: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexLegacyUserMessageLine {
    #[serde(rename = "type")]
    _line_type: String,
    #[serde(rename = "id")]
    _id: serde_json::Value,
    _role: String,
    _content: Vec<CodexMessageContent>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexLegacyStateRecord {
    #[serde(rename = "record_type")]
    _record_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexMessageContent {
    #[serde(rename = "type")]
    _item_type: String,
    _text: String,
}

#[derive(Debug, Deserialize)]
struct CodexIgnoredLine {
    #[serde(rename = "type")]
    _line_type: String,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use futures::TryStreamExt;
    use tempfile::tempdir;

    use super::{AgentAdapter, CodexAdapter};

    #[tokio::test]
    async fn parses_codex_sessions() {
        let temp = tempdir().unwrap();
        let sessions_dir = temp.path().join(".codex/sessions/2026/04/13");
        fs::create_dir_all(&sessions_dir).unwrap();
        fs::write(
            sessions_dir.join("rollout-1.jsonl"),
            concat!(
                "{\"timestamp\":\"2026-04-13T12:20:08.985Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"# AGENTS.md instructions for /tmp/project\"},{\"type\":\"input_text\",\"text\":\"<environment_context>\\n  <cwd>/tmp</cwd>\\n</environment_context>\"},{\"type\":\"input_text\",\"text\":\" hello \"}]}}\n",
                "{\"timestamp\":\"2026-04-13T12:20:08.985Z\",\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-5.4\"}}\n",
                "{\"timestamp\":\"2026-04-13T12:20:10.000Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"input_text\",\"text\":\"ignore\"}],\"phase\":\"final\"}}\n",
                "{\"id\":\"legacy-session\",\"timestamp\":\"2025-09-01T17:41:44.550Z\",\"instructions\":null,\"git\":{\"commit_hash\":\"abc\",\"branch\":\"main\",\"repository_url\":\"git@example.com:repo.git\"}}\n",
                "{\"type\":\"message\",\"id\":null,\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\" legacy \"}]}\n",
            ),
        )
        .unwrap();

        let messages = CodexAdapter::new(temp.path())
            .poll()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(format!("{:?}", messages[0].adapter), "Codex");
        assert_eq!(messages[0].model.as_deref(), Some("gpt-5.4"));
        assert_eq!(messages[0].text, "hello");
        assert_eq!(messages[0].time, 1_776_082_808_985);
        assert_eq!(messages[1].model.as_deref(), Some("gpt-5.4"));
        assert_eq!(messages[1].text, "legacy");
        assert_eq!(messages[1].time, 1_756_748_504_550);
    }

    #[tokio::test]
    async fn skips_invalid_lines_preserves_valid_lines() {
        use futures::TryStreamExt;
        let temp = tempdir().unwrap();
        let sessions_dir = temp.path().join(".codex/sessions/2026/04/13");
        fs::create_dir_all(&sessions_dir).unwrap();
        // Invalid timestamp line (skipped), then valid user message line
        fs::write(
            sessions_dir.join("rollout-1.jsonl"),
            "{\"timestamp\":\"bad\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"should be skipped\"}]}}\n{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"should be kept\"}]}\n",
        )
        .unwrap();

        let messages: Vec<_> = CodexAdapter::new(temp.path())
            .poll()
            .await
            .expect("poll should succeed despite bad lines")
            .try_collect()
            .await
            .expect("collect should succeed");
        // Only valid line should be parsed
        assert_eq!(messages.len(), 1);
        assert!(messages[0].text.contains("should be kept"));
    }

    #[tokio::test]
    async fn sums_codex_tokens_from_state_db() {
        let temp = tempdir().unwrap();
        let codex_dir = temp.path().join(".codex");
        fs::create_dir_all(codex_dir.join("sessions/2026/04/13")).unwrap();
        let db_path = codex_dir.join("state_5.sqlite");
        let connection = rusqlite::Connection::open(&db_path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    rollout_path TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    source TEXT NOT NULL,
                    model_provider TEXT NOT NULL,
                    cwd TEXT NOT NULL,
                    title TEXT NOT NULL,
                    sandbox_policy TEXT NOT NULL,
                    approval_mode TEXT NOT NULL,
                    tokens_used INTEGER NOT NULL DEFAULT 0,
                    has_user_event INTEGER NOT NULL DEFAULT 0,
                    archived INTEGER NOT NULL DEFAULT 0,
                    archived_at INTEGER,
                    git_sha TEXT,
                    git_branch TEXT,
                    git_origin_url TEXT,
                    cli_version TEXT NOT NULL DEFAULT '',
                    first_user_message TEXT NOT NULL DEFAULT '',
                    agent_nickname TEXT,
                    agent_role TEXT,
                    memory_mode TEXT NOT NULL DEFAULT 'enabled',
                    model TEXT,
                    reasoning_effort TEXT,
                    agent_path TEXT
                );
                INSERT INTO threads (id, rollout_path, created_at, updated_at, source, model_provider, cwd, title, sandbox_policy, approval_mode, tokens_used)
                VALUES ('t1', 'r1', 0, 0, 's', 'm', '/', 't', 'p', 'a', 10);
                INSERT INTO threads (id, rollout_path, created_at, updated_at, source, model_provider, cwd, title, sandbox_policy, approval_mode, tokens_used)
                VALUES ('t2', 'r2', 0, 0, 's', 'm', '/', 't', 'p', 'a', 20);
                "#,
            )
            .unwrap();

        let adapter = CodexAdapter::new(temp.path());

        assert_eq!(adapter.tokens().await.unwrap(), 30);
    }
}
