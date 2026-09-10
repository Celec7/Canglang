use serde::{Deserialize, Serialize};
use specta_typescript::Number;

/// 引擎原始协议流的方向，保留 stdout/stderr 差异供诊断界面筛选
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EngineOutputStream {
    Stdin,
    Stdout,
    Stderr,
    Lifecycle,
}

/// 在协议边界采集的原始行；`raw` 保留 CRLF、空行和长行内容
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EngineRawLine {
    pub run_id: String,
    pub analysis_session_id: Option<String>,
    #[specta(type = Number)]
    pub timestamp_ms: i64,
    pub sequence: u32,
    pub stream: EngineOutputStream,
    pub raw: String,
}
