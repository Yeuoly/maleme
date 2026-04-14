pub mod agent_adapter;
pub mod fuck_detector;
pub mod report;

pub use agent_adapter::{
    AdapterError, AdapterKind, AgentAdapter, ClaudeAdapter, CodexAdapter, CursorAdapter,
    OpenCodeAdapter, UnifiedAgentAdapter, UserMessage, UserMessageStream,
};
pub use fuck_detector::{FuckDetector, FuckDetectorError, ProfanityEntry};
pub use report::{ReportError, render_report, write_report_and_open};
