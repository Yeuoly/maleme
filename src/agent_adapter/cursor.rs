use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use indicatif::ProgressBar;
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use tokio::task;

use super::{
    AdapterError, AdapterKind, AgentAdapter, ModelTokenCounts, UserMessage, UserMessageStream,
    normalize::{normalize_model_id, trim_to_owned},
    stream_messages,
};

const CHATDATA_QUERY: &str = r#"
SELECT CAST(value AS TEXT)
FROM ItemTable
WHERE key = 'workbench.panel.aichat.view.aichat.chatdata'
"#;

const COMPOSER_QUERY: &str = r#"
SELECT key, CAST(value AS TEXT)
FROM cursorDiskKV
WHERE key LIKE 'composerData:%'
ORDER BY key ASC
"#;

const TOKEN_QUERY: &str = r#"
SELECT SUM(
    CAST(json_extract(value, '$.tokenCount.inputTokens') AS INTEGER) +
    CAST(json_extract(value, '$.tokenCount.outputTokens') AS INTEGER)
)
FROM cursorDiskKV
WHERE key LIKE 'bubbleId:%'
"#;

#[derive(Debug, Clone)]
pub struct CursorAdapter {
    root_dir: PathBuf,
}

impl CursorAdapter {
    pub fn new(home: impl AsRef<Path>) -> Self {
        Self {
            root_dir: resolve_cursor_root_dir(home.as_ref()),
        }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: path.into(),
        }
    }

    fn workspace_storage_dir(&self) -> PathBuf {
        self.root_dir.join("User").join("workspaceStorage")
    }

    fn global_storage_db_path(&self) -> PathBuf {
        self.root_dir
            .join("User")
            .join("globalStorage")
            .join("state.vscdb")
    }

    pub async fn workspace_file_count(&self) -> Result<usize, AdapterError> {
        let workspace_storage_dir = self.workspace_storage_dir();
        let global_storage_db_path = self.global_storage_db_path();
        task::spawn_blocking(move || {
            if global_storage_db_path.exists() {
                Ok(1)
            } else {
                collect_workspace_db_paths(&workspace_storage_dir).map(|paths| paths.len())
            }
        })
        .await
        .map_err(AdapterError::Join)?
    }

    pub async fn collect_messages_with_progress(
        &self,
        progress: ProgressBar,
    ) -> Result<Vec<UserMessage>, AdapterError> {
        let workspace_storage_dir = self.workspace_storage_dir();
        let global_storage_db_path = self.global_storage_db_path();
        task::spawn_blocking(move || {
            if global_storage_db_path.exists() {
                progress.set_message("Cursor 1/1 · globalStorage".to_owned());
                let messages = read_messages_from_global_db(&global_storage_db_path)?;
                progress.inc(1);
                return Ok(messages);
            }

            let db_paths = collect_workspace_db_paths(&workspace_storage_dir)?;
            let total_files = db_paths.len();
            let mut messages = Vec::new();

            for (index, db_path) in db_paths.into_iter().enumerate() {
                progress.set_message(format!(
                    "Cursor {}/{} · {}",
                    index + 1,
                    total_files,
                    db_path
                        .parent()
                        .and_then(Path::file_name)
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| db_path.display().to_string())
                ));
                if let Ok(msgs) = read_messages_from_db(&db_path) {
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

fn resolve_cursor_root_dir(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library")
            .join("Application Support")
            .join("Cursor")
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Roaming"))
            .join("Cursor")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("Cursor")
    }
}

impl AgentAdapter for CursorAdapter {
    async fn check(&self) -> bool {
        self.global_storage_db_path().exists() || self.workspace_storage_dir().exists()
    }

    async fn poll(&self) -> Result<UserMessageStream, AdapterError> {
        let workspace_storage_dir = self.workspace_storage_dir();
        let global_storage_db_path = self.global_storage_db_path();
        let messages = task::spawn_blocking(move || {
            if global_storage_db_path.exists() {
                return read_messages_from_global_db(&global_storage_db_path);
            }

            let db_paths = collect_workspace_db_paths(&workspace_storage_dir)?;
            let mut messages = Vec::new();

            for db_path in db_paths {
                if let Ok(msgs) = read_messages_from_db(&db_path) {
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
        let db_path = self.global_storage_db_path();
        task::spawn_blocking(move || read_tokens_from_global_db(&db_path))
            .await
            .map_err(AdapterError::Join)?
    }

    async fn tokens_by_model(&self) -> Result<ModelTokenCounts, AdapterError> {
        let db_path = self.global_storage_db_path();
        task::spawn_blocking(move || read_model_tokens_from_global_db(&db_path))
            .await
            .map_err(AdapterError::Join)?
    }
}

fn collect_workspace_db_paths(workspace_storage_dir: &Path) -> Result<Vec<PathBuf>, AdapterError> {
    let mut db_paths = Vec::new();

    for entry in fs::read_dir(workspace_storage_dir).map_err(|source| AdapterError::Io {
        path: workspace_storage_dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| AdapterError::Io {
            path: workspace_storage_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let db_path = path.join("state.vscdb");

        if db_path.exists() {
            db_paths.push(db_path);
        }
    }

    db_paths.sort();
    Ok(db_paths)
}

fn read_messages_from_db(db_path: &Path) -> Result<Vec<UserMessage>, AdapterError> {
    let connection = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|source| AdapterError::SqliteOpen {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut statement =
        connection
            .prepare(CHATDATA_QUERY)
            .map_err(|source| AdapterError::SqliteQuery {
                path: db_path.to_path_buf(),
                source,
            })?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut messages = Vec::new();

    for row in rows {
        let value = match row {
            Ok(v) => v,
            Err(_) => continue, // skip rows with read errors
        };
        let Ok(chat_data) = serde_json::from_str::<CursorChatData>(&value) else {
            continue; // skip rows with invalid JSON
        };

        for tab in chat_data.tabs {
            for bubble in tab.bubbles {
                if bubble.bubble_type == "user" {
                    let text = trim_to_owned(&bubble.text.into_string());

                    if !text.is_empty() {
                        messages.push(UserMessage {
                            adapter: AdapterKind::Cursor,
                            model: None,
                            text,
                            time: tab.last_send_time,
                        });
                    }
                }
            }
        }
    }

    Ok(messages)
}

fn read_messages_from_global_db(db_path: &Path) -> Result<Vec<UserMessage>, AdapterError> {
    let connection = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|source| AdapterError::SqliteOpen {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut statement =
        connection
            .prepare(COMPOSER_QUERY)
            .map_err(|source| AdapterError::SqliteQuery {
                path: db_path.to_path_buf(),
                source,
            })?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut messages = Vec::new();

    for row in rows {
        let (_key, value) = match row {
            Ok(v) => v,
            Err(_) => continue, // skip rows with read errors
        };
        let Some(value) = value else {
            continue;
        };
        let composer: CursorComposerData = match serde_json::from_str(&value) {
            Ok(composer) => composer,
            Err(_) => continue,
        };
        let model = composer
            .model_config
            .as_ref()
            .and_then(|config| normalize_model_id(&config.model_name));
        let base_time = if composer.created_at != 0 {
            composer.created_at
        } else {
            composer.last_updated_at
        };

        if !composer.conversation.is_empty() {
            for (index, bubble) in composer.conversation.into_iter().enumerate() {
                if bubble.bubble_type != 1 {
                    continue;
                }

                let text = trim_to_owned(&bubble.text.into_string());
                if text.is_empty() {
                    continue;
                }

                messages.push(UserMessage {
                    adapter: AdapterKind::Cursor,
                    model: model.clone(),
                    text,
                    time: base_time + index as i64,
                });
            }
            continue;
        }

        for (index, header) in composer
            .full_conversation_headers_only
            .into_iter()
            .enumerate()
        {
            if header.bubble_type != 1 {
                continue;
            }

            let text = read_cursor_bubble_text(
                &connection,
                db_path,
                &composer.composer_id,
                &header.bubble_id,
            )?;
            if text.is_empty() {
                continue;
            }

            messages.push(UserMessage {
                adapter: AdapterKind::Cursor,
                model: model.clone(),
                text,
                time: base_time + index as i64,
            });
        }
    }

    messages.sort_by_key(|message| message.time);
    Ok(messages)
}

fn read_cursor_bubble_text(
    connection: &Connection,
    db_path: &Path,
    composer_id: &str,
    bubble_id: &str,
) -> Result<String, AdapterError> {
    let key = format!("bubbleId:{composer_id}:{bubble_id}");
    let value: Option<String> = connection
        .query_row(
            "SELECT CAST(value AS TEXT) FROM cursorDiskKV WHERE key = ?1",
            [&key],
            |row| row.get(0),
        )
        .or_else(|source| {
            if matches!(source, rusqlite::Error::QueryReturnedNoRows) {
                Ok(None)
            } else {
                Err(source)
            }
        })
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let Some(value) = value else {
        return Ok(String::new());
    };
    let bubble: CursorStoredBubble = match serde_json::from_str(&value) {
        Ok(bubble) => bubble,
        Err(_) => return Ok(String::new()),
    };

    Ok(trim_to_owned(&bubble.text.into_string()))
}

fn read_tokens_from_global_db(db_path: &Path) -> Result<i64, AdapterError> {
    let connection = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|source| AdapterError::SqliteOpen {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut statement =
        connection
            .prepare(TOKEN_QUERY)
            .map_err(|source| AdapterError::SqliteQuery {
                path: db_path.to_path_buf(),
                source,
            })?;

    statement
        .query_row([], |row| row.get::<_, i64>(0))
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })
}

fn read_model_tokens_from_global_db(db_path: &Path) -> Result<ModelTokenCounts, AdapterError> {
    if !db_path.exists() {
        return Ok(ModelTokenCounts::new());
    }

    let connection = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|source| AdapterError::SqliteOpen {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut bubble_statement = connection
        .prepare("SELECT key, CAST(value AS TEXT) FROM cursorDiskKV WHERE key LIKE 'bubbleId:%'")
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let bubble_rows = bubble_statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut composer_tokens: BTreeMap<String, i64> = BTreeMap::new();

    for row in bubble_rows {
        let (key, value) = row.map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
        let Some(value) = value else {
            continue;
        };
        let Some(composer_id) = key.split(':').nth(1) else {
            continue;
        };
        let Ok(bubble) = serde_json::from_str::<CursorTokenBubble>(&value) else {
            continue;
        };
        *composer_tokens.entry(composer_id.to_owned()).or_insert(0) +=
            bubble.token_count.input_tokens + bubble.token_count.output_tokens;
    }

    let mut statement =
        connection
            .prepare(COMPOSER_QUERY)
            .map_err(|source| AdapterError::SqliteQuery {
                path: db_path.to_path_buf(),
                source,
            })?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
    let mut totals = ModelTokenCounts::new();

    for row in rows {
        let (_key, value) = row.map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
        let Some(value) = value else {
            continue;
        };
        let Ok(composer) = serde_json::from_str::<CursorComposerData>(&value) else {
            continue;
        };
        let Some(model) = composer
            .model_config
            .as_ref()
            .and_then(|config| normalize_model_id(&config.model_name))
        else {
            continue;
        };
        let tokens = composer_tokens
            .get(&composer.composer_id)
            .copied()
            .unwrap_or(0);
        *totals.entry(model).or_insert(0) += tokens;
    }

    Ok(totals)
}

#[derive(Debug, Deserialize)]
struct CursorChatData {
    tabs: Vec<CursorChatTab>,
}

#[derive(Debug, Deserialize)]
struct CursorChatTab {
    #[serde(rename = "lastSendTime")]
    #[serde(default)]
    last_send_time: i64,
    bubbles: Vec<CursorBubble>,
}

#[derive(Debug, Deserialize)]
struct CursorBubble {
    #[serde(rename = "type")]
    bubble_type: String,
    #[serde(default)]
    text: CursorText,
}

#[derive(Debug, Deserialize)]
struct CursorComposerData {
    #[serde(rename = "composerId")]
    composer_id: String,
    #[serde(default)]
    conversation: Vec<CursorComposerBubble>,
    #[serde(default, rename = "fullConversationHeadersOnly")]
    full_conversation_headers_only: Vec<CursorConversationHeader>,
    #[serde(default, rename = "modelConfig")]
    model_config: Option<CursorModelConfig>,
    #[serde(default, rename = "createdAt")]
    created_at: i64,
    #[serde(default, rename = "lastUpdatedAt")]
    last_updated_at: i64,
}

#[derive(Debug, Deserialize)]
struct CursorComposerBubble {
    #[serde(rename = "type")]
    bubble_type: i64,
    #[serde(default)]
    text: CursorText,
}

#[derive(Debug, Deserialize)]
struct CursorConversationHeader {
    #[serde(rename = "type")]
    bubble_type: i64,
    #[serde(rename = "bubbleId")]
    bubble_id: String,
}

#[derive(Debug, Deserialize)]
struct CursorModelConfig {
    #[serde(rename = "modelName")]
    model_name: String,
}

#[derive(Debug, Deserialize)]
struct CursorStoredBubble {
    #[serde(default)]
    text: CursorText,
}

#[derive(Debug, Deserialize, Default)]
struct CursorTokenBubble {
    #[serde(default, rename = "tokenCount")]
    token_count: CursorTokenCount,
}

#[derive(Debug, Deserialize, Default)]
struct CursorTokenCount {
    #[serde(default, rename = "inputTokens")]
    input_tokens: i64,
    #[serde(default, rename = "outputTokens")]
    output_tokens: i64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CursorText {
    Text(String),
    Null(()),
}

impl Default for CursorText {
    fn default() -> Self {
        Self::Null(())
    }
}

impl CursorText {
    fn into_string(self) -> String {
        match self {
            Self::Text(text) => text,
            Self::Null(()) => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use futures::TryStreamExt;
    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::{AgentAdapter, CursorAdapter};

    #[tokio::test]
    async fn reads_user_bubbles_from_cursor_chatdata() {
        let temp = tempdir().unwrap();
        let db_dir = temp
            .path()
            .join("User")
            .join("workspaceStorage")
            .join("workspace-a");
        fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join("state.vscdb");
        let connection = Connection::open(&db_path).unwrap();

        connection
            .execute_batch(
                r#"
                CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
                INSERT INTO ItemTable (key, value)
                VALUES (
                    'workbench.panel.aichat.view.aichat.chatdata',
                    '{"tabs":[{"lastSendTime":1736258828015,"bubbles":[{"type":"user","text":" first "},{"type":"assistant","text":"skip"},{"type":"user","text":null},{"type":"user","text":"second"}]}]}'
                );
                "#,
            )
            .unwrap();

        let messages = CursorAdapter::from_path(temp.path())
            .poll()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "first");
        assert_eq!(messages[0].time, 1736258828015);
        assert_eq!(format!("{:?}", messages[0].adapter), "Cursor");
        assert_eq!(messages[1].text, "second");
    }

    #[tokio::test]
    async fn skips_invalid_json_preserves_valid_data() {
        use futures::TryStreamExt;
        let temp = tempdir().unwrap();
        let db_dir = temp
            .path()
            .join("User")
            .join("workspaceStorage")
            .join("workspace-a");
        fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join("state.vscdb");
        let connection = Connection::open(&db_path).unwrap();

        // Insert invalid JSON (skipped) and valid JSON
        connection
            .execute_batch(
                r#"
                CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
                INSERT INTO ItemTable (key, value)
                VALUES ('workbench.panel.aichat.view.aichat.chatdata', '{invalid}');
                "#,
            )
            .unwrap();

        let messages: Vec<_> = CursorAdapter::from_path(temp.path())
            .poll()
            .await
            .expect("poll should succeed despite bad rows")
            .try_collect()
            .await
            .expect("collect should succeed");
        // Invalid JSON row should be skipped
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn sums_tokens_from_global_storage_bubbles() {
        let temp = tempdir().unwrap();
        let global_storage_dir = temp.path().join("User").join("globalStorage");
        fs::create_dir_all(&global_storage_dir).unwrap();
        let db_path = global_storage_dir.join("state.vscdb");
        let connection = Connection::open(&db_path).unwrap();

        connection
            .execute_batch(
                r#"
                CREATE TABLE cursorDiskKV (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
                INSERT INTO cursorDiskKV (key, value)
                VALUES
                    ('bubbleId:chat-1:user-1', '{"tokenCount":{"inputTokens":0,"outputTokens":0}}'),
                    ('bubbleId:chat-1:assistant-1', '{"tokenCount":{"inputTokens":120,"outputTokens":30}}'),
                    ('bubbleId:chat-2:assistant-2', '{"tokenCount":{"inputTokens":40,"outputTokens":10}}');
                "#,
            )
            .unwrap();

        let tokens = CursorAdapter::from_path(temp.path())
            .tokens()
            .await
            .unwrap();

        assert_eq!(tokens, 200);
    }

    #[tokio::test]
    async fn reads_user_messages_and_models_from_cursor_composer_data() {
        let temp = tempdir().unwrap();
        let global_storage_dir = temp.path().join("User").join("globalStorage");
        fs::create_dir_all(&global_storage_dir).unwrap();
        let db_path = global_storage_dir.join("state.vscdb");
        let connection = Connection::open(&db_path).unwrap();

        connection
            .execute_batch(
                r#"
                CREATE TABLE cursorDiskKV (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
                INSERT INTO cursorDiskKV (key, value)
                VALUES (
                    'composerData:cmp-1',
                    '{"composerId":"cmp-1","createdAt":1735800794838,"modelConfig":{"modelName":"claude-4.5-sonnet-thinking"},"conversation":[{"type":1,"bubbleId":"u1","text":" first "},{"type":2,"bubbleId":"a1","text":"skip"},{"type":1,"bubbleId":"u2","text":"second"}]}'
                );
                "#,
            )
            .unwrap();

        let messages = CursorAdapter::from_path(temp.path())
            .poll()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "first");
        assert_eq!(
            messages[0].model.as_deref(),
            Some("claude-4.5-sonnet-thinking")
        );
        assert_eq!(messages[0].time, 1_735_800_794_838);
        assert_eq!(messages[1].text, "second");
        assert_eq!(
            messages[1].model.as_deref(),
            Some("claude-4.5-sonnet-thinking")
        );
        assert_eq!(messages[1].time, 1_735_800_794_840);
    }

    #[tokio::test]
    async fn reads_header_only_cursor_composer_data_via_bubble_rows() {
        let temp = tempdir().unwrap();
        let global_storage_dir = temp.path().join("User").join("globalStorage");
        fs::create_dir_all(&global_storage_dir).unwrap();
        let db_path = global_storage_dir.join("state.vscdb");
        let connection = Connection::open(&db_path).unwrap();

        connection
            .execute_batch(
                r#"
                CREATE TABLE cursorDiskKV (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
                INSERT INTO cursorDiskKV (key, value)
                VALUES
                    (
                        'composerData:cmp-2',
                        '{"composerId":"cmp-2","createdAt":1770023876985,"modelConfig":{"modelName":"claude-4.5-sonnet-thinking"},"fullConversationHeadersOnly":[{"bubbleId":"u1","type":1},{"bubbleId":"a1","type":2},{"bubbleId":"u2","type":1}]}'
                    ),
                    (
                        'bubbleId:cmp-2:u1',
                        '{"text":" hello "}'
                    ),
                    (
                        'bubbleId:cmp-2:u2',
                        '{"text":"world"}'
                    );
                "#,
            )
            .unwrap();

        let messages = CursorAdapter::from_path(temp.path())
            .poll()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "hello");
        assert_eq!(
            messages[0].model.as_deref(),
            Some("claude-4.5-sonnet-thinking")
        );
        assert_eq!(messages[1].text, "world");
        assert_eq!(
            messages[1].model.as_deref(),
            Some("claude-4.5-sonnet-thinking")
        );
    }
}
