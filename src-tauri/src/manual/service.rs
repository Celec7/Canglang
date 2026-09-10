use crate::manual::encoding::decode_pgn_text;
use crate::manual::error::ManualError;
use crate::manual::models::ChessManual;
use crate::manual::pgn_exporter::PgnExporter;
use crate::manual::pgn_parser::PgnParser;
use crate::manual::xqf_exporter::XqfExporter;
use crate::manual::xqf_parser::XqfParser;
use std::path::Path;

/// 统一棋谱服务门面：按扩展名分派 `.xqf`/`.pgn` 解析，并提供独立的 PGN/XQF 写出路径
pub struct ManualService;

impl ManualService {
    /// 根据路径加载并解析棋谱
    pub fn load(path: &str) -> Result<ChessManual, ManualError> {
        if path.trim().is_empty() || !Path::new(path).exists() {
            return Err(ManualError::NotExist {
                path: path.to_string(),
            });
        }

        let bytes = std::fs::read(path).map_err(|source| ManualError::Io { source })?;
        Self::load_bytes(&bytes, path)
    }

    /// 按路径扩展名分派解析字节数据
    pub fn load_bytes(bytes: &[u8], path: &str) -> Result<ChessManual, ManualError> {
        let ext = extension_lower(path);
        match ext.as_str() {
            "xqf" => XqfParser::parse(bytes),
            "pgn" => {
                let text = decode_pgn_text(bytes);
                PgnParser::parse(&text)
            }
            _ => Err(ManualError::UnsupportedFormat { ext }),
        }
    }

    /// 将棋谱导出并保存为 `.pgn`（UTF-8 无 BOM）
    pub fn save(manual: &ChessManual, path: &str) -> Result<(), ManualError> {
        let ext = extension_lower(path);
        if ext != "pgn" {
            return Err(ManualError::UnsupportedFormat { ext });
        }

        let text = PgnExporter::export(manual, true);
        if let Some(parent) = Path::new(path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|source| ManualError::Io { source })?;
        }
        std::fs::write(path, text.as_bytes()).map_err(|source| ManualError::Io { source })?;
        Ok(())
    }

    /// 将棋谱规范化导出并保存为未加密的 XQF v10
    pub fn save_xqf(manual: &ChessManual, path: &str, version: u8) -> Result<(), ManualError> {
        let ext = extension_lower(path);
        if ext != "xqf" {
            return Err(ManualError::UnsupportedFormat { ext });
        }

        let bytes = XqfExporter::export(manual, version)?;
        if let Some(parent) = Path::new(path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|source| ManualError::Io { source })?;
        }
        std::fs::write(path, bytes).map_err(|source| ManualError::Io { source })?;
        Ok(())
    }

    /// 导出为 PGN 字符串（供测试断言内容）
    pub fn save_pgn_string(manual: &ChessManual) -> String {
        PgnExporter::export(manual, true)
    }
}

fn extension_lower(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}
