use std::{
    fs,
    path::{Path, PathBuf},
};

use indicatif::ProgressBar;
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use tokio::task;

use super::{
    AdapterError, AdapterKind, AgentAdapter, UserMessage, UserMessageStream,
    normalize::trim_to_owned, stream_messages,
};

const CHATDATA_QUERY: &str = r#"
SELECT CAST(value AS TEXT)
FROM ItemTable
WHERE key = 'workbench.panel.aichat.view.aichat.chatdata'
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
        self.root_dir.join("User").join("globalStorage").join("state.vscdb")
    }

    pub async fn workspace_file_count(&self) -> Result<usize, AdapterError> {
        let workspace_storage_dir = self.workspace_storage_dir();
        task::spawn_blocking(move || {
            collect_workspace_db_paths(&workspace_storage_dir).map(|paths| paths.len())
        })
        .await
        .map_err(AdapterError::Join)?
    }

    pub async fn collect_messages_with_progress(
        &self,
        progress: ProgressBar,
    ) -> Result<Vec<UserMessage>, AdapterError> {
        let workspace_storage_dir = self.workspace_storage_dir();
        task::spawn_blocking(move || {
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
                messages.extend(read_messages_from_db(&db_path)?);
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
        self.workspace_storage_dir().exists()
    }

    async fn poll(&self) -> Result<UserMessageStream, AdapterError> {
        let workspace_storage_dir = self.workspace_storage_dir();
        let messages = task::spawn_blocking(move || {
            let db_paths = collect_workspace_db_paths(&workspace_storage_dir)?;
            let mut messages = Vec::new();

            for db_path in db_paths {
                messages.extend(read_messages_from_db(&db_path)?);
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
        let value = row.map_err(|source| AdapterError::SqliteQuery {
            path: db_path.to_path_buf(),
            source,
        })?;
        let chat_data: CursorChatData =
            serde_json::from_str(&value).map_err(|source| AdapterError::InvalidJsonLine {
                path: db_path.to_path_buf(),
                line: 1,
                source,
            })?;

        for tab in chat_data.tabs {
            for bubble in tab.bubbles {
                if bubble.bubble_type == "user" {
                    let text = trim_to_owned(&bubble.text.into_string());

                    if !text.is_empty() {
                        messages.push(UserMessage {
                            adapter: AdapterKind::Cursor,
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
    async fn fails_when_chatdata_json_is_invalid() {
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
                VALUES ('workbench.panel.aichat.view.aichat.chatdata', '{');
                "#,
            )
            .unwrap();

        let error = match CursorAdapter::from_path(temp.path()).poll().await {
            Ok(_) => panic!("expected invalid json error"),
            Err(error) => error,
        };

        assert!(matches!(error, super::AdapterError::InvalidJsonLine { .. }));
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

        let tokens = CursorAdapter::from_path(temp.path()).tokens().await.unwrap();

        assert_eq!(tokens, 200);
    }
}
