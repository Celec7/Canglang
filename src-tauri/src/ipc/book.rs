//! 开局库 IPC 命令
//!
//! 把 `engine::book` 模块（`OpeningBookService`）桥接到前端。服务存于受管的
//! [`BookState`]；命令为 async，避免磁盘上的开局库加载或网络云库查询阻塞 UI 线程

use crate::AppError;
use crate::core::board::BoardState;
use crate::core::game::GameState;
use crate::core::notation::NotationConverter;
use crate::core::position::Move;
use crate::core::rules::MoveValidator;
use crate::engine::book::{CloudBookMode, OpeningBookService};
use crate::engine::models::BookMove;
use serde::{Deserialize, Serialize};
use std::sync::Mutex as StdMutex;
use tauri::State;
use tokio::sync::Mutex;

/// 象棋云库当前状态报告
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CloudBookStatus {
    pub enabled: bool,
    pub mode: CloudBookMode,
}

/// 受管的开局库服务
#[derive(Default)]
pub struct BookState(pub Mutex<OpeningBookService>);

/// 从磁盘加载一个 `.bh` 开局库（按路径幂等）
#[tauri::command]
#[specta::specta]
pub async fn book_load(path: String, state: State<'_, BookState>) -> Result<(), AppError> {
    let mut service = state.0.lock().await;
    service.load_book(&path)?;
    Ok(())
}

/// 按路径卸载一个已加载的开局库
#[tauri::command]
#[specta::specta]
pub async fn book_unload(path: String, state: State<'_, BookState>) -> Result<(), AppError> {
    let mut service = state.0.lock().await;
    service.unload_book(&path);
    Ok(())
}

/// 卸载所有已加载的开局库
#[tauri::command]
#[specta::specta]
pub async fn book_clear(state: State<'_, BookState>) -> Result<(), AppError> {
    let mut service = state.0.lock().await;
    service.clear_books();
    Ok(())
}

/// 返回当前已加载开局库的路径列表
#[tauri::command]
#[specta::specta]
pub async fn book_loaded(state: State<'_, BookState>) -> Result<Vec<String>, AppError> {
    let service = state.0.lock().await;
    Ok(service.loaded_books())
}

/// 设置象棋云库 (chessdb.cn) 是否启用
#[tauri::command]
#[specta::specta]
pub async fn book_set_cloud_enabled(
    enabled: bool,
    state: State<'_, BookState>,
) -> Result<(), AppError> {
    let mut service = state.0.lock().await;
    service.set_cloud_enabled(enabled);
    Ok(())
}

/// 设置象棋云库协同查询模式（协同/仅云库/仅本地/合并）
#[tauri::command]
#[specta::specta]
pub async fn book_set_cloud_mode(
    mode: CloudBookMode,
    state: State<'_, BookState>,
) -> Result<(), AppError> {
    let mut service = state.0.lock().await;
    service.set_cloud_mode(mode);
    Ok(())
}

/// 获取当前象棋云库状态（启用开关与协同模式）
#[tauri::command]
#[specta::specta]
pub async fn book_get_cloud_status(
    state: State<'_, BookState>,
) -> Result<CloudBookStatus, AppError> {
    let service = state.0.lock().await;
    Ok(CloudBookStatus {
        enabled: service.is_cloud_enabled(),
        mode: service.cloud_mode(),
    })
}

/// 查询开局库在指定局面下的候选招法（根据配置综合本地 .bh 库与在线象棋云库）
#[tauri::command]
#[specta::specta]
pub async fn book_query(
    fen: String,
    state: State<'_, BookState>,
    game_state: State<'_, StdMutex<GameState>>,
) -> Result<Vec<BookMove>, AppError> {
    if !game_state
        .lock()
        .unwrap()
        .snapshot()
        .capabilities
        .query_book
        .enabled
    {
        return Err(AppError::InvalidArgument(
            "当前会话模式不支持普通象棋开局库查询".to_string(),
        ));
    }
    let board = BoardState::from_fen(&fen)?;
    let (mode, local_moves, cloud_client) = {
        let service = state.0.lock().await;
        let mode = service.cloud_mode();
        let local_moves = match mode {
            CloudBookMode::CloudOnly => Vec::new(),
            _ => service.query(&board),
        };
        (mode, local_moves, service.cloud().clone())
    };

    let mut moves = match mode {
        CloudBookMode::LocalOnly => local_moves,
        CloudBookMode::CloudOnly => cloud_client.query(&fen).await.unwrap_or_default(),
        CloudBookMode::Hybrid => {
            if !local_moves.is_empty() {
                local_moves
            } else {
                cloud_client.query(&fen).await.unwrap_or_default()
            }
        }
        CloudBookMode::Merge => {
            let cloud_moves = cloud_client.query(&fen).await.unwrap_or_default();
            let mut merged = local_moves;
            for cm in cloud_moves {
                if !merged.iter().any(|lm| lm.iccs == cm.iccs) {
                    merged.push(cm);
                }
            }
            merged.sort_by(|a, b| {
                b.score.cmp(&a.score).then(
                    b.win_rate
                        .partial_cmp(&a.win_rate)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
            });
            merged
        }
    };

    // 自动为所有候选招法解析并填充标准中国象棋传统记谱（如「炮二平五」、「相七进五」）
    for m in &mut moves {
        if m.notation.is_none()
            && let Ok(mv) = Move::from_iccs(&m.iccs)
        {
            m.notation = NotationConverter::to_chinese_notation(&board, &mv).ok();
        }
    }

    retain_legal_book_moves(&board, &mut moves);

    Ok(moves)
}

fn retain_legal_book_moves(board: &BoardState, moves: &mut Vec<BookMove>) {
    moves.retain(|m| Move::from_iccs(&m.iccs).is_ok_and(|mv| MoveValidator::can_move(board, mv)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book_move(iccs: &str) -> BookMove {
        BookMove {
            iccs: iccs.to_string(),
            notation: None,
            score: 1,
            win_count: 0,
            draw_count: 0,
            lose_count: 0,
            win_rate: 50.0,
            note: None,
            source: "test".to_string(),
        }
    }

    #[test]
    fn filter_book_moves_rejects_invalid_and_illegal_candidates() {
        let board = BoardState::initial();
        let mut moves = vec![
            book_move("h2e2"),
            book_move("h2h2"),
            book_move("not-a-move"),
        ];

        retain_legal_book_moves(&board, &mut moves);

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].iccs, "h2e2");
    }
}
