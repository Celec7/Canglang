use std::io;

/// 棋谱加载、解析或保存时的统一错误类型
#[derive(Debug, thiserror::Error)]
pub enum ManualError {
    #[error("unsupported chess manual format: {ext}")]
    UnsupportedFormat { ext: String },

    #[error("invalid chess manual data: {msg}")]
    InvalidData { msg: String },

    #[error("io error: {source}")]
    Io { source: io::Error },

    #[error("manual parse error: {msg}")]
    Parse { msg: String },

    #[error("chess manual file does not exist: {path}")]
    NotExist { path: String },
}
