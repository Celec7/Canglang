use std::io;

/// 引擎子模块统一错误类型
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("cannot start engine process '{path}': {source}")]
    ProcessStart { path: String, source: io::Error },

    #[error("cannot send command to engine: {source}")]
    CommandSend { source: io::Error },

    #[error("engine protocol timeout during {stage}")]
    ProtocolTimeout { stage: String },

    #[error("invalid engine configuration: {0}")]
    InvalidConfig(String),

    #[error("sqlite error: {source}")]
    Sqlite { source: rusqlite::Error },

    #[error("engine line stream closed")]
    StreamClosed,

    #[error("engine protocol probe failed: {details}")]
    ProbeFailed { details: String },

    #[error("engine is not running")]
    NotRunning,

    #[error("no legal moves remain after exclusions")]
    NoSearchMoves,

    #[error("engine does not support the requested root-move constraint")]
    ConstraintUnsupported,

    #[error("io error: {0}")]
    Io(#[from] io::Error),
}
