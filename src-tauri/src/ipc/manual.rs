//! 棋谱（对局记录）IPC 命令
//!
//! 把 `manual` 模块桥接到前端，棋谱以 [`ChessManual`] serde 结构收发；磁盘上的
//! `.xqf`/`.pgn` 读写由 `ManualService` 按扩展名分派

use crate::AppError;
use crate::manual::{ChessManual, ManualService, PgnParser};

/// 从磁盘加载并解析棋谱（`.xqf`/`.pgn`）
#[tauri::command]
#[specta::specta]
pub fn manual_load(path: String) -> Result<ChessManual, AppError> {
    Ok(ManualService::load(&path)?)
}

/// 弹出棋谱文件选择器；`open` 选择已有 PGN/XQF，`save` 选择输出文件路径
#[tauri::command]
#[specta::specta]
pub async fn manual_pick_file(action: String) -> Result<Option<String>, AppError> {
    let dialog = rfd::AsyncFileDialog::new()
        .add_filter("棋谱文件", &["pgn", "xqf"])
        .set_title(if action == "save" {
            "保存棋谱文件"
        } else {
            "打开棋谱文件"
        });

    let file = match action.as_str() {
        "open" => dialog.pick_file().await,
        "save" => dialog.save_file().await,
        other => {
            return Err(AppError::InvalidArgument(format!(
                "unsupported manual file picker action: {other}"
            )));
        }
    };

    Ok(file.map(|selected| selected.path().to_string_lossy().into_owned()))
}

/// 将棋谱保存为 `.pgn` 到磁盘
#[tauri::command]
#[specta::specta]
pub fn manual_save(path: String, manual: ChessManual) -> Result<(), AppError> {
    ManualService::save(&manual, &path)?;
    Ok(())
}

/// 将棋谱规范化导出为 XQF v10 到磁盘
#[tauri::command]
#[specta::specta]
pub fn manual_save_xqf(path: String, manual: ChessManual, version: u8) -> Result<(), AppError> {
    ManualService::save_xqf(&manual, &path, version)?;
    Ok(())
}

/// 将棋谱导出为 PGN 字符串（不写入文件）
#[tauri::command]
#[specta::specta]
pub fn manual_export_pgn(manual: ChessManual) -> String {
    ManualService::save_pgn_string(&manual)
}

/// 解析文本棋谱；文件格式和字节编码仍由 `ManualService` 负责
#[tauri::command]
#[specta::specta]
pub fn manual_parse_text(format: String, text: String) -> Result<ChessManual, AppError> {
    match format.trim().to_ascii_lowercase().as_str() {
        "pgn" => Ok(PgnParser::parse(&text)?),
        other => Err(AppError::InvalidArgument(format!(
            "unsupported text manual format: {other}"
        ))),
    }
}
