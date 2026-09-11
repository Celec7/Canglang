//! 棋谱（对局记录）IPC 命令
//!
//! 把 `manual` 模块桥接到前端，棋谱以 [`ChessManual`] serde 结构收发；磁盘上的
//! `.xqf`/`.pgn` 读写由 `ManualService` 按扩展名分派

use crate::AppError;
use crate::core::game::{
    ActiveGame, GameState, SessionError, SessionErrorCode, SessionSnapshot, SessionToken,
};
use crate::core::jieqi::JieqiOperation;
use crate::ipc::engine::EngineState;
use crate::manual::jieqi::{
    JieqiDocumentKind, JieqiDocumentMetadata, JieqiDocumentOpenResult, JieqiDocumentPublic,
    JieqiDocumentReceipt, JieqiDocumentService,
};
use crate::manual::{ChessManual, ManualError, ManualService, PgnParser};
use std::collections::BTreeMap;
use std::sync::Mutex;
use tauri::State;

/// 从磁盘加载并解析棋谱（`.xqf`/`.pgn`）
#[tauri::command]
#[specta::specta]
pub fn manual_load(path: String) -> Result<ChessManual, AppError> {
    Ok(ManualService::load(&path)?)
}

/// 弹出棋谱文件选择器；普通棋谱与揭棋文档使用互不混淆的 action
#[tauri::command]
#[specta::specta]
pub async fn manual_pick_file(action: String) -> Result<Option<String>, AppError> {
    let file = match action.as_str() {
        "open" => {
            rfd::AsyncFileDialog::new()
                .add_filter("中国象棋棋谱", &["pgn", "xqf"])
                .set_title("打开棋谱文件")
                .pick_file()
                .await
        }
        "save" => {
            rfd::AsyncFileDialog::new()
                .add_filter("PGN 棋谱", &["pgn"])
                .set_title("保存棋谱文件")
                .save_file()
                .await
        }
        "open_jieqi" => {
            rfd::AsyncFileDialog::new()
                .add_filter("揭棋文档", &["cjq"])
                .set_title("打开揭棋文档")
                .pick_file()
                .await
        }
        "save_jieqi_private" => {
            rfd::AsyncFileDialog::new()
                .add_filter("揭棋私有续局", &["cjq"])
                .set_title("保存揭棋私有续局")
                .save_file()
                .await
        }
        "save_jieqi_public" => {
            rfd::AsyncFileDialog::new()
                .add_filter("揭棋公开回放", &["cjq"])
                .set_title("保存揭棋公开回放")
                .save_file()
                .await
        }
        other => {
            return Err(AppError::InvalidArgument(format!(
                "unsupported manual file picker action: {other}"
            )));
        }
    };

    Ok(file.map(|selected| selected.path().to_string_lossy().into_owned()))
}

/// 保存揭棋文档；完整身份从锁内克隆的 Rust 对局直接进入文件服务
#[tauri::command]
#[specta::specta]
pub async fn jieqi_document_save(
    token: SessionToken,
    path: String,
    kind: JieqiDocumentKind,
    metadata: JieqiDocumentMetadata,
    annotations: BTreeMap<u32, String>,
    edit_revision: String,
    state: State<'_, Mutex<GameState>>,
) -> Result<JieqiDocumentReceipt, SessionError> {
    jieqi_document_save_with_state(
        token,
        path,
        kind,
        metadata,
        annotations,
        edit_revision,
        &state,
    )
    .await
}

pub async fn jieqi_document_save_with_state(
    token: SessionToken,
    path: String,
    kind: JieqiDocumentKind,
    metadata: JieqiDocumentMetadata,
    annotations: BTreeMap<u32, String>,
    edit_revision: String,
    state: &Mutex<GameState>,
) -> Result<JieqiDocumentReceipt, SessionError> {
    let (game, game_id, content_revision) = {
        let state = state.lock().unwrap();
        state.check_token(&token)?;
        let game = state.jieqi()?;
        let operation = match kind {
            JieqiDocumentKind::PrivateGame => JieqiOperation::SavePrivate,
            JieqiDocumentKind::PublicReplay => JieqiOperation::SavePublic,
        };
        let capability = game.capability(operation);
        if !capability.enabled {
            return Err(SessionError::new(
                SessionErrorCode::OperationUnavailable,
                format!("当前不能保存此类揭棋文档: {:?}", capability.reason),
            ));
        }
        (
            game.clone(),
            state.game_id().to_string(),
            state.content_revision().to_string(),
        )
    };

    tokio::task::spawn_blocking(move || {
        JieqiDocumentService::save(&game, &path, kind, metadata, annotations)
    })
    .await
    .map_err(task_failure)?
    .map_err(map_manual)?;

    Ok(JieqiDocumentReceipt {
        game_id,
        content_revision,
        edit_revision,
        kind,
    })
}

/// 打开揭棋文档；文件在锁外完整解析，提交候选时统一停止引擎并复核 token
#[tauri::command]
#[specta::specta]
pub async fn jieqi_document_open(
    token: SessionToken,
    path: String,
    state: State<'_, Mutex<GameState>>,
    engine_state: State<'_, EngineState>,
) -> Result<JieqiDocumentOpenResult, SessionError> {
    jieqi_document_open_with_states(token, path, &state, &engine_state).await
}

pub async fn jieqi_document_open_with_states(
    token: SessionToken,
    path: String,
    state: &Mutex<GameState>,
    engine_state: &EngineState,
) -> Result<JieqiDocumentOpenResult, SessionError> {
    // 提前拒绝已经过期的请求，但不在解析和磁盘 I/O 期间持锁
    state.lock().unwrap().check_token(&token)?;
    let (game, document): (_, JieqiDocumentPublic) =
        tokio::task::spawn_blocking(move || JieqiDocumentService::load(&path))
            .await
            .map_err(task_failure)?
            .map_err(map_manual)?;
    let snapshot: SessionSnapshot = engine_state
        .coordinate_replacement(state, &token, ActiveGame::Jieqi(game))
        .await?;
    Ok(JieqiDocumentOpenResult { snapshot, document })
}

fn task_failure(error: tokio::task::JoinError) -> SessionError {
    SessionError::new(
        SessionErrorCode::IoFailure,
        format!("揭棋文件任务失败: {error}"),
    )
}

fn map_manual(error: ManualError) -> SessionError {
    let code = match error {
        ManualError::Io { .. } | ManualError::NotExist { .. } => SessionErrorCode::IoFailure,
        ManualError::UnsupportedFormat { .. }
        | ManualError::InvalidData { .. }
        | ManualError::Parse { .. } => SessionErrorCode::InvalidInput,
    };
    SessionError::new(code, error.to_string())
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
