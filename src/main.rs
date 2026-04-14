use indicatif::{ProgressBar, ProgressStyle};
use maleme::{
    AgentAdapter, ClaudeAdapter, CodexAdapter, CursorAdapter, FuckDetector, OpenCodeAdapter,
    write_report_and_open,
};

#[tokio::main]
async fn main() {
    let home = match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        Ok(home) => home,
        Err(error) => {
            eprintln!("failed to resolve home directory: {error}");
            return;
        }
    };
    let codex = CodexAdapter::new(&home);
    let claude = ClaudeAdapter::new(&home);
    let opencode = OpenCodeAdapter::new(&home);
    let cursor = CursorAdapter::new(&home);
    let codex_enabled = codex.check().await;
    let claude_enabled = claude.check().await;
    let opencode_enabled = opencode.check().await;
    let cursor_enabled = cursor.check().await;
    let codex_units = if codex_enabled {
        match codex.session_file_count().await {
            Ok(count) => count,
            Err(error) => {
                eprintln!("failed to count Codex sessions: {error}");
                0
            }
        }
    } else {
        0
    };
    let claude_units = if claude_enabled {
        match claude.transcript_file_count().await {
            Ok(count) => count,
            Err(error) => {
                eprintln!("failed to count Claude transcripts: {error}");
                0
            }
        }
    } else {
        0
    };
    let opencode_units = if opencode_enabled { 1 } else { 0 };
    let cursor_units = if cursor_enabled {
        match cursor.workspace_file_count().await {
            Ok(count) => count,
            Err(error) => {
                eprintln!("failed to count Cursor workspaces: {error}");
                0
            }
        }
    } else {
        0
    };
    let token_units = codex_enabled as u64
        + claude_enabled as u64
        + opencode_enabled as u64
        + cursor_enabled as u64;
    let total_units = codex_units as u64
        + claude_units as u64
        + opencode_units
        + cursor_units as u64
        + token_units
        + 1;
    let progress = ProgressBar::new(total_units.max(1));
    match ProgressStyle::with_template("{spinner:.green} {bar:40.cyan/blue} {pos}/{len} {msg}") {
        Ok(style) => progress.set_style(style),
        Err(error) => eprintln!("failed to configure progress bar style: {error}"),
    }

    let mut messages = Vec::new();

    if codex_enabled {
        match codex.collect_messages_with_progress(progress.clone()).await {
            Ok(found) => messages.extend(found),
            Err(error) => eprintln!("failed to collect Codex messages: {error}"),
        }
    }

    if claude_enabled {
        match claude.collect_messages_with_progress(progress.clone()).await {
            Ok(found) => messages.extend(found),
            Err(error) => eprintln!("failed to collect Claude messages: {error}"),
        }
    }

    if opencode_enabled {
        match opencode.collect_messages_with_progress(progress.clone()).await {
            Ok(found) => messages.extend(found),
            Err(error) => eprintln!("failed to collect OpenCode messages: {error}"),
        }
    }

    if cursor_enabled {
        match cursor.collect_messages_with_progress(progress.clone()).await {
            Ok(found) => messages.extend(found),
            Err(error) => eprintln!("failed to collect Cursor messages: {error}"),
        }
    }

    messages.sort_by_key(|message| message.time);

    let mut tokens = 0_i64;

    if codex_enabled {
        progress.set_message("Codex tokens".to_owned());
        match codex.tokens().await {
            Ok(count) => tokens += count,
            Err(error) => eprintln!("failed to collect Codex tokens: {error}"),
        }
        progress.inc(1);
    }

    if claude_enabled {
        progress.set_message("Claude tokens".to_owned());
        match claude.tokens().await {
            Ok(count) => tokens += count,
            Err(error) => eprintln!("failed to collect Claude tokens: {error}"),
        }
        progress.inc(1);
    }

    if opencode_enabled {
        progress.set_message("OpenCode tokens".to_owned());
        match opencode.tokens().await {
            Ok(count) => tokens += count,
            Err(error) => eprintln!("failed to collect OpenCode tokens: {error}"),
        }
        progress.inc(1);
    }

    if cursor_enabled {
        progress.set_message("Cursor tokens".to_owned());
        match cursor.tokens().await {
            Ok(count) => tokens += count,
            Err(error) => eprintln!("failed to collect Cursor tokens: {error}"),
        }
        progress.inc(1);
    }

    let detector = match FuckDetector::new() {
        Ok(detector) => detector,
        Err(error) => {
            eprintln!("failed to initialize detector: {error}");
            return;
        }
    };
    progress.set_message("生成 HTML 报告".to_owned());
    let report_path = match write_report_and_open(&messages, tokens, &detector) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("failed to write report: {error}");
            return;
        }
    };
    progress.inc(1);
    progress.finish_with_message("报告已生成并打开");

    println!("report: {}", report_path.display());
    println!("messages: {}", messages.len());
    println!("tokens: {}", tokens);
}
