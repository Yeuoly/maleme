mod claude;
mod codex;
mod cursor;
mod error;
mod normalize;
mod opencode;

use std::{
    path::{Path, PathBuf},
    pin::Pin,
};

use futures::{Stream, TryStreamExt, stream};

pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;
pub use cursor::CursorAdapter;
pub use error::AdapterError;
pub use opencode::OpenCodeAdapter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdapterKind {
    Codex,
    Claude,
    OpenCode,
    Cursor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMessage {
    pub adapter: AdapterKind,
    pub text: String,
    pub time: i64,
}

pub type UserMessageStream = Pin<Box<dyn Stream<Item = Result<UserMessage, AdapterError>> + Send>>;

#[allow(async_fn_in_trait)]
pub trait AgentAdapter {
    async fn check(&self) -> bool;
    async fn poll(&self) -> Result<UserMessageStream, AdapterError>;
    async fn tokens(&self) -> Result<i64, AdapterError>;
}

#[derive(Debug, Clone)]
pub struct UnifiedAgentAdapter {
    codex: CodexAdapter,
    claude: ClaudeAdapter,
    opencode: OpenCodeAdapter,
    cursor: CursorAdapter,
}

impl UnifiedAgentAdapter {
    pub fn new() -> Result<Self, AdapterError> {
        let home = std::env::var("HOME").map_err(AdapterError::MissingHome)?;
        Ok(Self::from_home(home))
    }

    pub fn from_home(home: impl AsRef<Path>) -> Self {
        Self {
            codex: CodexAdapter::new(home.as_ref()),
            claude: ClaudeAdapter::new(home.as_ref()),
            opencode: OpenCodeAdapter::new(home.as_ref()),
            cursor: CursorAdapter::new(home.as_ref()),
        }
    }

    pub fn from_paths(
        codex_path: impl Into<PathBuf>,
        claude_path: impl Into<PathBuf>,
        opencode_path: impl Into<PathBuf>,
        cursor_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            codex: CodexAdapter::from_path(codex_path),
            claude: ClaudeAdapter::from_path(claude_path),
            opencode: OpenCodeAdapter::from_path(opencode_path),
            cursor: CursorAdapter::from_path(cursor_path),
        }
    }
}

impl AgentAdapter for UnifiedAgentAdapter {
    async fn check(&self) -> bool {
        self.codex.check().await
            || self.claude.check().await
            || self.opencode.check().await
            || self.cursor.check().await
    }

    async fn poll(&self) -> Result<UserMessageStream, AdapterError> {
        let mut messages = Vec::new();

        if self.codex.check().await {
            messages.extend(self.codex.poll().await?.try_collect::<Vec<_>>().await?);
        }

        if self.claude.check().await {
            messages.extend(self.claude.poll().await?.try_collect::<Vec<_>>().await?);
        }

        if self.opencode.check().await {
            messages.extend(self.opencode.poll().await?.try_collect::<Vec<_>>().await?);
        }

        if self.cursor.check().await {
            messages.extend(self.cursor.poll().await?.try_collect::<Vec<_>>().await?);
        }

        messages.sort_by_key(|message| message.time);
        Ok(stream_messages(messages))
    }

    async fn tokens(&self) -> Result<i64, AdapterError> {
        let mut total = 0_i64;

        if self.codex.check().await {
            total += self.codex.tokens().await?;
        }

        if self.claude.check().await {
            total += self.claude.tokens().await?;
        }

        if self.opencode.check().await {
            total += self.opencode.tokens().await?;
        }

        if self.cursor.check().await {
            total += self.cursor.tokens().await?;
        }

        Ok(total)
    }
}

pub(crate) fn stream_messages(messages: Vec<UserMessage>) -> UserMessageStream {
    Box::pin(stream::iter(messages.into_iter().map(Ok)))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use futures::TryStreamExt;
    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::{AgentAdapter, UnifiedAgentAdapter};

    #[tokio::test]
    async fn merges_all_adapters_in_time_order() {
        let temp = tempdir().unwrap();

        let codex_dir = temp.path().join(".codex/sessions/1970/01/01");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("rollout-1.jsonl"),
            "{\"timestamp\":\"1970-01-01T00:00:02.000Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"codex\"}]}}\n",
        )
        .unwrap();

        let claude_dir = temp.path().join(".claude/transcripts");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("ses_1.jsonl"),
            "{\"type\":\"user\",\"timestamp\":\"1970-01-01T00:00:01.500Z\",\"content\":\"claude\"}\n",
        )
        .unwrap();

        let opencode_dir = temp.path().join(".local/share/opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let db_path = opencode_dir.join("opencode.db");
        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE message (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    time_created INTEGER NOT NULL,
                    time_updated INTEGER NOT NULL,
                    data TEXT NOT NULL
                );
                CREATE TABLE part (
                    id TEXT PRIMARY KEY,
                    message_id TEXT NOT NULL,
                    session_id TEXT NOT NULL,
                    time_created INTEGER NOT NULL,
                    time_updated INTEGER NOT NULL,
                    data TEXT NOT NULL
                );
                INSERT INTO message (id, session_id, time_created, time_updated, data)
                VALUES ('msg1', 'ses1', 1000, 1000, '{"role":"user"}');
                INSERT INTO part (id, message_id, session_id, time_created, time_updated, data)
                VALUES ('prt1', 'msg1', 'ses1', 1001, 1001, '{"type":"text","text":"opencode"}');
                "#,
            )
            .unwrap();

        let adapter = UnifiedAgentAdapter::from_home(temp.path());
        let messages = adapter
            .poll()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].text, "opencode");
        assert_eq!(messages[1].text, "claude");
        assert_eq!(messages[2].text, "codex");
    }

    #[tokio::test]
    async fn sums_tokens_across_adapters() {
        let temp = tempdir().unwrap();

        let codex_dir = temp.path().join(".codex");
        fs::create_dir_all(codex_dir.join("sessions/1970/01/01")).unwrap();
        let codex_db = codex_dir.join("state_5.sqlite");
        let codex_conn = Connection::open(&codex_db).unwrap();
        codex_conn
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
                VALUES ('t1', 'r', 0, 0, 's', 'm', '/', 't', 'p', 'a', 10);
                "#,
            )
            .unwrap();

        let claude_dir = temp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("transcripts")).unwrap();
        fs::write(
            claude_dir.join("stats-cache.json"),
            r#"{"version":1,"lastComputedDate":"2026-02-11","dailyActivity":[],"dailyModelTokens":[],"modelUsage":{"claude-opus":{"inputTokens":1,"outputTokens":2,"cacheReadInputTokens":3,"cacheCreationInputTokens":4,"webSearchRequests":0,"costUSD":0,"contextWindow":0}},"totalSessions":1,"totalMessages":1,"longestSession":{"sessionId":"s","duration":1,"messageCount":1,"timestamp":"2025-11-20T06:26:38.724Z"},"firstSessionDate":"2025-11-20T06:26:38.724Z","hourCounts":{"14":1}}"#,
        )
        .unwrap();

        let opencode_dir = temp.path().join(".local/share/opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let opencode_db = opencode_dir.join("opencode.db");
        let opencode_conn = Connection::open(&opencode_db).unwrap();
        opencode_conn
            .execute_batch(
                r#"
                CREATE TABLE message (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    time_created INTEGER NOT NULL,
                    time_updated INTEGER NOT NULL,
                    data TEXT NOT NULL
                );
                CREATE TABLE part (
                    id TEXT PRIMARY KEY,
                    message_id TEXT NOT NULL,
                    session_id TEXT NOT NULL,
                    time_created INTEGER NOT NULL,
                    time_updated INTEGER NOT NULL,
                    data TEXT NOT NULL
                );
                INSERT INTO part (id, message_id, session_id, time_created, time_updated, data)
                VALUES ('p1', 'm1', 's1', 0, 0, '{"type":"step-finish","tokens":{"input":5,"output":6,"reasoning":7,"cache":{"read":8,"write":9}}}');
                "#,
            )
            .unwrap();

        let adapter = UnifiedAgentAdapter::from_home(temp.path());

        assert_eq!(adapter.tokens().await.unwrap(), 55);
    }
}
