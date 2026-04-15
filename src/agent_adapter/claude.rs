use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use indicatif::ProgressBar;
use serde::Deserialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::fs;

use super::{
    AdapterError, AdapterKind, AgentAdapter, ModelTokenCounts, UserMessage, UserMessageStream,
    normalize::{normalize_claude_text, normalize_model_id},
    stream_messages,
};

#[derive(Debug, Clone)]
pub struct ClaudeAdapter {
    root_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaudeLogKind {
    Transcript,
    Project,
}

#[derive(Debug, Clone)]
struct ClaudeLogFile {
    kind: ClaudeLogKind,
    path: PathBuf,
}

#[derive(Debug, Default)]
struct ClaudeLogIndex {
    log_files: Vec<ClaudeLogFile>,
}

impl ClaudeAdapter {
    pub fn new(home: impl AsRef<Path>) -> Self {
        Self {
            root_dir: home.as_ref().join(".claude"),
        }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: path.into(),
        }
    }

    fn transcripts_dir(&self) -> PathBuf {
        self.root_dir.join("transcripts")
    }

    fn projects_dir(&self) -> PathBuf {
        self.root_dir.join("projects")
    }

    fn stats_cache_path(&self) -> PathBuf {
        self.root_dir.join("stats-cache.json")
    }

    pub async fn transcript_file_count(&self) -> Result<usize, AdapterError> {
        Ok(self.collect_log_index().await?.log_files.len())
    }

    pub async fn collect_messages_with_progress(
        &self,
        progress: ProgressBar,
    ) -> Result<Vec<UserMessage>, AdapterError> {
        let index = self.collect_log_index().await?;
        let total_files = index.log_files.len();
        let mut messages = Vec::new();

        for (index, log_file) in index.log_files.into_iter().enumerate() {
            progress.set_message(format!(
                "Claude {}/{} · {}",
                index + 1,
                total_files,
                log_file.path.file_name().unwrap().to_string_lossy()
            ));
            let contents =
                fs::read_to_string(&log_file.path)
                    .await
                    .map_err(|source| AdapterError::Io {
                        path: log_file.path.clone(),
                        source,
                    })?;
            messages.extend(parse_claude_log_file(&log_file, &contents)?);

            progress.inc(1);
        }

        dedupe_messages(&mut messages);
        Ok(messages)
    }

    async fn collect_log_index(&self) -> Result<ClaudeLogIndex, AdapterError> {
        let mut log_files = Vec::new();
        let mut transcript_paths = Vec::new();

        if fs::metadata(self.transcripts_dir()).await.is_ok() {
            collect_jsonl_paths(&self.transcripts_dir(), false, &mut transcript_paths).await?;
        }

        let spawned_transcript_ids = collect_spawned_transcript_ids(&transcript_paths).await?;

        for path in transcript_paths {
            let session_id = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            if spawned_transcript_ids.contains(&session_id) {
                continue;
            }

            log_files.push(ClaudeLogFile {
                kind: ClaudeLogKind::Transcript,
                path,
            });
        }

        if fs::metadata(self.projects_dir()).await.is_ok() {
            let mut project_paths = Vec::new();
            collect_jsonl_paths(&self.projects_dir(), true, &mut project_paths).await?;
            log_files.extend(project_paths.into_iter().map(|path| ClaudeLogFile {
                kind: ClaudeLogKind::Project,
                path,
            }));
        }

        log_files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(ClaudeLogIndex { log_files })
    }
}

impl AgentAdapter for ClaudeAdapter {
    async fn check(&self) -> bool {
        fs::metadata(self.transcripts_dir()).await.is_ok()
            || fs::metadata(self.projects_dir()).await.is_ok()
    }

    async fn poll(&self) -> Result<UserMessageStream, AdapterError> {
        let paths = self.collect_log_index().await?.log_files;
        let mut messages = Vec::new();

        for log_file in paths {
            let contents =
                fs::read_to_string(&log_file.path)
                    .await
                    .map_err(|source| AdapterError::Io {
                        path: log_file.path.clone(),
                        source,
                    })?;
            messages.extend(parse_claude_log_file(&log_file, &contents)?);
        }

        dedupe_messages(&mut messages);
        Ok(stream_messages(messages))
    }

    async fn tokens(&self) -> Result<i64, AdapterError> {
        let stats_cache_path = self.stats_cache_path();
        let contents = fs::read_to_string(&stats_cache_path)
            .await
            .map_err(|source| AdapterError::Io {
                path: stats_cache_path.clone(),
                source,
            })?;
        let stats: ClaudeStatsCache =
            serde_json::from_str(&contents).map_err(|source| AdapterError::InvalidJsonLine {
                path: stats_cache_path,
                line: 1,
                source,
            })?;
        let mut total = 0_i64;

        for usage in stats.model_usage.into_values() {
            total += usage.input_tokens;
            total += usage.output_tokens;
            total += usage.cache_read_input_tokens;
            total += usage.cache_creation_input_tokens;
        }

        Ok(total)
    }

    async fn tokens_by_model(&self) -> Result<ModelTokenCounts, AdapterError> {
        let stats_cache_path = self.stats_cache_path();
        let contents = fs::read_to_string(&stats_cache_path)
            .await
            .map_err(|source| AdapterError::Io {
                path: stats_cache_path.clone(),
                source,
            })?;
        let stats: ClaudeStatsCache =
            serde_json::from_str(&contents).map_err(|source| AdapterError::InvalidJsonLine {
                path: stats_cache_path,
                line: 1,
                source,
            })?;
        let mut totals = BTreeMap::new();

        for (model, usage) in stats.model_usage {
            let Some(model) = normalize_model_id(&model) else {
                continue;
            };
            let tokens = usage.input_tokens
                + usage.output_tokens
                + usage.cache_read_input_tokens
                + usage.cache_creation_input_tokens;
            *totals.entry(model).or_insert(0) += tokens;
        }

        Ok(totals)
    }
}

#[derive(Debug, Deserialize)]
struct ClaudeEventKind {
    #[serde(rename = "type")]
    event_type: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeUserEvent {
    timestamp: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeProjectUserEvent {
    #[serde(default, rename = "isSidechain")]
    is_sidechain: bool,
    #[serde(default, rename = "isMeta")]
    is_meta: bool,
    timestamp: String,
    uuid: Option<String>,
    message: ClaudeProjectMessage,
}

#[derive(Debug, Deserialize)]
struct ClaudeProjectMessage {
    content: serde_json::Value,
}

fn extract_user_text(content: &serde_json::Value) -> Option<String> {
    match content {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                        parts.push(text.to_string());
                    }
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
            }
        }
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct ClaudeProjectAssistantEvent {
    #[serde(default, rename = "isSidechain")]
    is_sidechain: bool,
    #[serde(rename = "parentUuid")]
    parent_uuid: Option<String>,
    message: ClaudeProjectAssistantMessage,
}

#[derive(Debug, Deserialize)]
struct ClaudeProjectAssistantMessage {
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeTaskToolResultEvent {
    #[serde(rename = "tool_name")]
    tool_name: String,
    #[serde(rename = "tool_output")]
    tool_output: Option<ClaudeTaskToolOutput>,
}

#[derive(Debug, Deserialize)]
struct ClaudeTaskToolOutput {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeStatsCache {
    #[serde(rename = "version")]
    _version: serde_json::Value,
    #[serde(rename = "modelUsage")]
    model_usage: std::collections::BTreeMap<String, ClaudeModelUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeModelUsage {
    #[serde(default, rename = "inputTokens")]
    input_tokens: i64,
    #[serde(default, rename = "outputTokens")]
    output_tokens: i64,
    #[serde(default, rename = "cacheReadInputTokens")]
    cache_read_input_tokens: i64,
    #[serde(default, rename = "cacheCreationInputTokens")]
    cache_creation_input_tokens: i64,
    #[serde(default, rename = "webSearchRequests")]
    _web_search_requests: serde_json::Value,
    #[serde(default, rename = "costUSD")]
    _cost_usd: serde_json::Value,
    #[serde(default, rename = "contextWindow")]
    _context_window: serde_json::Value,
}

async fn collect_jsonl_paths(
    root: &Path,
    recursive: bool,
    output: &mut Vec<PathBuf>,
) -> Result<(), AdapterError> {
    let mut entries = fs::read_dir(root)
        .await
        .map_err(|source| AdapterError::Io {
            path: root.to_path_buf(),
            source,
        })?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|source| AdapterError::Io {
            path: root.to_path_buf(),
            source,
        })?
    {
        let path = entry.path();
        let file_type = entry.file_type().await.map_err(|source| AdapterError::Io {
            path: path.clone(),
            source,
        })?;

        if file_type.is_dir() && recursive {
            Box::pin(collect_jsonl_paths(&path, true, output)).await?;
            continue;
        }

        if file_type.is_file() && path.to_string_lossy().ends_with(".jsonl") {
            output.push(path);
        }
    }

    Ok(())
}

async fn collect_spawned_transcript_ids(
    transcript_paths: &[PathBuf],
) -> Result<std::collections::BTreeSet<String>, AdapterError> {
    let mut spawned = std::collections::BTreeSet::new();

    for path in transcript_paths {
        let contents = fs::read_to_string(path)
            .await
            .map_err(|source| AdapterError::Io {
                path: path.clone(),
                source,
            })?;

        for (line_index, raw_line) in contents.lines().enumerate() {
            let line_number = line_index + 1;
            let kind: ClaudeEventKind =
                serde_json::from_str(raw_line).map_err(|source| AdapterError::InvalidJsonLine {
                    path: path.clone(),
                    line: line_number,
                    source,
                })?;

            if kind.event_type != "tool_result" {
                continue;
            }

            let event: ClaudeTaskToolResultEvent =
                serde_json::from_str(raw_line).map_err(|source| AdapterError::InvalidJsonLine {
                    path: path.clone(),
                    line: line_number,
                    source,
                })?;

            if event.tool_name != "task" {
                continue;
            }

            if let Some(session_id) = event.tool_output.and_then(|output| output.session_id) {
                spawned.insert(session_id);
            }
        }
    }

    Ok(spawned)
}

fn parse_claude_user_message(
    log_file: &ClaudeLogFile,
    raw_line: &str,
    line_number: usize,
    project_models: Option<&BTreeMap<String, String>>,
) -> Result<Option<UserMessage>, AdapterError> {
    let kind: ClaudeEventKind =
        serde_json::from_str(raw_line).map_err(|source| AdapterError::InvalidJsonLine {
            path: log_file.path.clone(),
            line: line_number,
            source,
        })?;

    if kind.event_type != "user" {
        return Ok(None);
    }

    let (timestamp, content) = match log_file.kind {
        ClaudeLogKind::Transcript => {
            let event: ClaudeUserEvent =
                serde_json::from_str(raw_line).map_err(|source| AdapterError::InvalidJsonLine {
                    path: log_file.path.clone(),
                    line: line_number,
                    source,
                })?;
            (event.timestamp, event.content)
        }
        ClaudeLogKind::Project => {
            let event: ClaudeProjectUserEvent =
                serde_json::from_str(raw_line).map_err(|source| AdapterError::InvalidJsonLine {
                    path: log_file.path.clone(),
                    line: line_number,
                    source,
                })?;
            if event.is_sidechain || event.is_meta {
                return Ok(None);
            }
            let Some(content) = extract_user_text(&event.message.content) else {
                return Ok(None);
            };
            let model = event
                .uuid
                .as_ref()
                .and_then(|uuid| project_models.and_then(|models| models.get(uuid)))
                .cloned();
            return build_claude_user_message(
                log_file,
                line_number,
                event.timestamp,
                content,
                model,
            );
        }
    };
    build_claude_user_message(log_file, line_number, timestamp, content, None)
}

fn build_claude_user_message(
    log_file: &ClaudeLogFile,
    line_number: usize,
    timestamp: String,
    content: String,
    model: Option<String>,
) -> Result<Option<UserMessage>, AdapterError> {
    let datetime = OffsetDateTime::parse(&timestamp, &Rfc3339).map_err(|source| {
        AdapterError::InvalidTimestamp {
            path: log_file.path.clone(),
            line: line_number,
            value: timestamp.clone(),
            source,
        }
    })?;
    let text = normalize_claude_text(&content);

    if text.is_empty() {
        return Ok(None);
    }

    Ok(Some(UserMessage {
        adapter: AdapterKind::Claude,
        model: model.as_deref().and_then(normalize_model_id),
        text,
        time: (datetime.unix_timestamp_nanos() / 1_000_000) as i64,
    }))
}

fn dedupe_messages(messages: &mut Vec<UserMessage>) {
    messages.sort_by(|left, right| {
        left.time
            .cmp(&right.time)
            .then(left.text.cmp(&right.text))
            .then(right.model.is_some().cmp(&left.model.is_some()))
    });
    messages.dedup_by(|left, right| {
        if left.time == right.time && left.text == right.text {
            if left.model.is_none() {
                left.model = right.model.clone();
            }
            true
        } else {
            false
        }
    });
}

fn parse_claude_log_file(
    log_file: &ClaudeLogFile,
    contents: &str,
) -> Result<Vec<UserMessage>, AdapterError> {
    let project_models = if log_file.kind == ClaudeLogKind::Project {
        Some(collect_project_models(log_file, contents)?)
    } else {
        None
    };
    let mut messages = Vec::new();

    for (index, raw_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        if let Some(message) =
            parse_claude_user_message(log_file, raw_line, line_number, project_models.as_ref())?
        {
            messages.push(message);
        }
    }

    Ok(messages)
}

fn collect_project_models(
    log_file: &ClaudeLogFile,
    contents: &str,
) -> Result<BTreeMap<String, String>, AdapterError> {
    let mut models = BTreeMap::new();

    for (index, raw_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let kind: ClaudeEventKind =
            serde_json::from_str(raw_line).map_err(|source| AdapterError::InvalidJsonLine {
                path: log_file.path.clone(),
                line: line_number,
                source,
            })?;

        if kind.event_type != "assistant" {
            continue;
        }

        let event: ClaudeProjectAssistantEvent =
            serde_json::from_str(raw_line).map_err(|source| AdapterError::InvalidJsonLine {
                path: log_file.path.clone(),
                line: line_number,
                source,
            })?;

        if event.is_sidechain {
            continue;
        }

        let Some(parent_uuid) = event.parent_uuid else {
            continue;
        };
        let Some(model) = event.message.model else {
            continue;
        };
        let Some(model) = normalize_model_id(&model) else {
            continue;
        };

        models.entry(parent_uuid).or_insert(model);
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use futures::TryStreamExt;
    use tempfile::tempdir;

    use super::{AgentAdapter, ClaudeAdapter};

    #[tokio::test]
    async fn parses_and_cleans_claude_messages() {
        let temp = tempdir().unwrap();
        let transcripts = temp.path().join(".claude/transcripts");
        fs::create_dir_all(&transcripts).unwrap();
        fs::write(
            transcripts.join("ses_1.jsonl"),
            concat!(
                "{\"type\":\"assistant\",\"timestamp\":\"2026-03-04T07:01:57.000Z\",\"content\":\"ignore\"}\n",
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:56.809Z\",\"content\":\"\\n\\n---\\n\\n[SYSTEM DIRECTIVE: TEST]\\nignore\\n\\n---\\n\\nactual user text\\n<!-- OMO_INTERNAL_INITIATOR -->\"}\n",
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:57.500Z\",\"content\":\"[analyze-mode]\\nGather context first.\\n\\n---\\n\\n\\n\\n---\\n\\n[SYSTEM DIRECTIVE: TEST]\\nignore\\n\\n---\\n\\nContinue with the full answer.\"}\n",
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:58.000Z\",\"content\":\"[>0;276;0c]10;rgb:e2e2/e8e8/f0f0\\u001b\\\\]11;rgb:0202/0606/1717\\u001b\\n\"}\n",
            ),
        )
        .unwrap();

        let messages = ClaudeAdapter::new(temp.path())
            .poll()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(format!("{:?}", messages[0].adapter), "Claude");
        assert_eq!(messages[0].model, None);
        assert_eq!(messages[0].text, "actual user text");
        assert_eq!(messages[0].time, 1_772_607_716_809);
        assert_eq!(
            messages[1].text,
            "[analyze-mode]\nGather context first.\n\nContinue with the full answer."
        );
    }

    #[tokio::test]
    async fn reads_project_logs_and_dedupes_messages() {
        let temp = tempdir().unwrap();
        let claude_dir = temp.path().join(".claude");
        let transcripts = claude_dir.join("transcripts");
        let projects = claude_dir.join("projects/workspace");
        fs::create_dir_all(&transcripts).unwrap();
        fs::create_dir_all(&projects).unwrap();
        fs::write(
            transcripts.join("ses_1.jsonl"),
            "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:56.809Z\",\"content\":\"shared message\",\"extra\":\"ok\"}\n",
        )
        .unwrap();
        fs::write(
            projects.join("session.jsonl"),
            concat!(
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:55.000Z\",\"isSidechain\":true,\"uuid\":\"warmup\",\"message\":{\"role\":\"user\",\"content\":\"Warmup\"}}\n",
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:56.809Z\",\"uuid\":\"u1\",\"message\":{\"role\":\"user\",\"content\":\"shared message\"}}\n",
                "{\"type\":\"assistant\",\"parentUuid\":\"u1\",\"message\":{\"model\":\"claude-3-7-sonnet\"}}\n",
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:02:00.000Z\",\"uuid\":\"u2\",\"message\":{\"role\":\"user\",\"content\":\"project only\"}}\n",
                "{\"type\":\"assistant\",\"parentUuid\":\"u2\",\"message\":{\"model\":\"claude-3-5-haiku\"}}\n",
            ),
        )
        .unwrap();

        let messages = ClaudeAdapter::new(temp.path())
            .poll()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "shared message");
        assert_eq!(messages[0].model.as_deref(), Some("claude-3-7-sonnet"));
        assert_eq!(messages[1].text, "project only");
        assert_eq!(messages[1].model.as_deref(), Some("claude-3-5-haiku"));
    }

    #[tokio::test]
    async fn handles_project_polymorphic_content_and_meta() {
        let temp = tempdir().unwrap();
        let projects = temp.path().join(".claude/projects/workspace");
        fs::create_dir_all(&projects).unwrap();
        fs::write(
            projects.join("session.jsonl"),
            concat!(
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:55.000Z\",\"uuid\":\"u1\",\"message\":{\"role\":\"user\",\"content\":\"plain string\"}}\n",
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:56.000Z\",\"uuid\":\"u2\",\"message\":{\"role\":\"user\",\"content\":[{\"tool_use_id\":\"t1\",\"type\":\"tool_result\",\"content\":\"tool output\"}]}}\n",
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:57.000Z\",\"uuid\":\"u3\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"hello from blocks\"}]}}\n",
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:58.000Z\",\"isMeta\":true,\"uuid\":\"u4\",\"message\":{\"role\":\"user\",\"content\":\"<command-name>/clear</command-name>\"}}\n",
            ),
        )
        .unwrap();

        let messages = ClaudeAdapter::new(temp.path())
            .poll()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "plain string");
        assert_eq!(messages[1].text, "hello from blocks");
    }

    #[tokio::test]
    async fn excludes_spawned_subagent_transcripts() {
        let temp = tempdir().unwrap();
        let transcripts = temp.path().join(".claude/transcripts");
        fs::create_dir_all(&transcripts).unwrap();
        fs::write(
            transcripts.join("ses_main.jsonl"),
            concat!(
                "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:01:56.809Z\",\"content\":\"real user message\"}\n",
                "{\"type\":\"tool_result\",\"timestamp\":\"2026-03-04T07:01:57.000Z\",\"tool_name\":\"task\",\"tool_output\":{\"sessionId\":\"ses_child\"}}\n",
            ),
        )
        .unwrap();
        fs::write(
            transcripts.join("ses_child.jsonl"),
            "{\"type\":\"user\",\"timestamp\":\"2026-03-04T07:02:00.000Z\",\"content\":\"generated subagent prompt\"}\n",
        )
        .unwrap();

        let messages = ClaudeAdapter::new(temp.path())
            .poll()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].model, None);
        assert_eq!(messages[0].text, "real user message");
    }

    #[tokio::test]
    async fn fails_on_invalid_user_shape() {
        let temp = tempdir().unwrap();
        let transcripts = temp.path().join(".claude/transcripts");
        fs::create_dir_all(&transcripts).unwrap();
        fs::write(
            transcripts.join("ses_1.jsonl"),
            "{\"type\":\"user\",\"timestamp\":\"bad\",\"content\":\"hello\"}\n",
        )
        .unwrap();

        let error = match ClaudeAdapter::new(temp.path()).poll().await {
            Ok(_) => panic!("expected claude poll to fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("invalid timestamp"));
    }

    #[tokio::test]
    async fn sums_claude_tokens_from_stats_cache() {
        let temp = tempdir().unwrap();
        let claude_dir = temp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("transcripts")).unwrap();
        fs::write(
            claude_dir.join("stats-cache.json"),
            r#"{"version":2,"lastComputedDate":"2026-02-11","dailyActivity":[],"dailyModelTokens":[],"modelUsage":{"claude-opus":{"inputTokens":1,"outputTokens":2,"cacheReadInputTokens":3,"cacheCreationInputTokens":4,"webSearchRequests":0,"costUSD":0,"contextWindow":0,"maxOutputTokens":0},"claude-sonnet":{"inputTokens":5,"outputTokens":6,"cacheReadInputTokens":7,"cacheCreationInputTokens":8,"webSearchRequests":0,"costUSD":0,"contextWindow":0,"maxOutputTokens":0}},"totalSessions":1,"totalMessages":1,"longestSession":{},"firstSessionDate":"2025-11-20T06:26:38.724Z","hourCounts":{"14":1},"totalSpeculationTimeSavedMs":0}"#,
        )
        .unwrap();

        let adapter = ClaudeAdapter::new(temp.path());

        assert_eq!(adapter.tokens().await.unwrap(), 36);
    }

    #[tokio::test]
    async fn tolerates_missing_and_new_claude_stats_fields() {
        let temp = tempdir().unwrap();
        let claude_dir = temp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("transcripts")).unwrap();
        fs::write(
            claude_dir.join("stats-cache.json"),
            r#"{"version":{"major":2},"modelUsage":{"claude-opus":{"inputTokens":1,"outputTokens":2,"cacheReadInputTokens":3,"brandNewField":999},"claude-sonnet":{"outputTokens":6,"cacheCreationInputTokens":8,"anotherNewField":"x"}},"newTopLevelField":{"anything":true}}"#,
        )
        .unwrap();

        let adapter = ClaudeAdapter::new(temp.path());

        assert_eq!(adapter.tokens().await.unwrap(), 20);
    }
}
