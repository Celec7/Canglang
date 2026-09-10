//! 核心规则 IPC 命令
//!
//! 把 `core` 模块（棋盘、记法、规则、对局状态）桥接到前端。局面类命令是对
//! FEN 字符串的纯函数；对局会话命令通过 `tauri::State` 持有受管的
//! [`GameState`]，使悔棋/重做/认输等历史驱动操作在多次调用间保持一致
//!
//! 契约说明：
//! - 局面以中国象棋 FEN 字符串收发（2 段式 `<board> <w/b>`）
//! - 走法以 4 字符 ICCS 字符串收发（如 `h2e2`）
//! - 中文记法在 Rust 侧生成（繁体，如 `炮二平五` / `馬８進７`）

use crate::AppError;
use crate::core::board::{BoardState, INITIAL_FEN};
use crate::core::game::{
    ActiveGame, GameResult, GameState, PlyRecord, SessionMutation, XiangqiGame,
};
use crate::core::notation::NotationConverter;
use crate::core::piece::Color;
use crate::core::position::{Move, Position};
use crate::core::rules::{MoveEffect, MoveValidator, RuleExplanation, RuleProfile, RuleStatus};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

/// 用于验证 IPC 桥接已连通的握手命令
#[tauri::command]
#[specta::specta]
pub fn ping() -> String {
    "pong".to_string()
}

/// 对单个局面执行无状态 `make_move` 的结果
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct MoveResult {
    /// 合法走后得到的新局面 FEN（非法时返回输入 FEN）
    pub fen: String,
    /// 实际应用的 ICCS 走法
    pub iccs: String,
    /// 该走法的繁体中文记法（非法时为空串）
    pub chinese_notation: String,
    /// 该走法在当前局面上是否合法
    pub legal: bool,
    /// 走子后轮到的一方是否被将军
    pub check: bool,
    /// 该走法是否结束对局（绝杀或困毙）
    pub game_over: bool,
}

/// 只读分析变例预览请求，不修改受管 GameState
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PreviewRequest {
    pub start_fen: String,
    pub history: Vec<String>,
    pub pv: Vec<String>,
    pub rule_profile: RuleProfile,
}

/// 分析变例在 core 规则服务中的只读快照
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PreviewSnapshot {
    pub fen: String,
    pub red_to_move: bool,
    pub in_check: bool,
    pub last_move_iccs: Option<String>,
    pub repetition_count: u32,
    pub rule_status: RuleStatus,
    pub rule_explanation: Option<RuleExplanation>,
    pub applied_pv_len: u32,
}

/// 将预览前缀原子应用到正式对局
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ApplyMoveLineRequest {
    pub expected_fen: String,
    pub moves: Vec<String>,
}

/// 受管 [`GameState`] 会话的轻量序列化视图
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameSnapshot {
    /// 当前局面 FEN（保持向后兼容，等同于 current_fen）
    pub fen: String,
    /// 对局起始局面的 FEN
    pub start_fen: String,
    /// 当前呈现局面的 FEN
    pub current_fen: String,
    /// 当前所处的步数游标深度 (0 表示处于起始局面，等于 history.len() 表示处于最新局面)
    pub current_ply: u32,
    /// 完整主线历史（包含已走步与可重做步）
    pub history: Vec<PlyRecord>,
    /// 对局结果：`ongoing` | `redwin` | `blackwin` | `draw`
    pub result: String,
    /// 当前是否轮到红方走棋
    pub red_to_move: bool,
    /// 当前轮到的一方是否被将军
    pub in_check: bool,
    /// 当前是否可悔棋
    pub can_undo: bool,
    /// 当前是否可重做
    pub can_redo: bool,
    /// 当前对局使用的规则档案
    pub rule_profile: RuleProfile,
    /// 当前局面的完整重复次数
    pub repetition_count: u32,
    /// 达到重复阈值时给裁判的规则解释
    pub repetition_explanation: Option<String>,
    /// 机器可读的规则裁判状态
    pub rule_status: RuleStatus,
    /// 机器可读的规则解释
    pub rule_explanation: Option<RuleExplanation>,
}

/// 返回标准初始局面 FEN
#[tauri::command]
#[specta::specta]
pub fn get_initial_board() -> String {
    INITIAL_FEN.to_string()
}

/// 校验 FEN 并返回其规范化形式
#[tauri::command]
#[specta::specta]
pub fn parse_fen(fen: String) -> Result<String, AppError> {
    let board = BoardState::from_fen(&fen)?;
    Ok(board.to_fen())
}

/// 严格校验可用于开始正式对局的自定义局面，并返回规范化 FEN
#[tauri::command]
#[specta::specta]
pub fn validate_position(fen: String) -> Result<String, AppError> {
    let board = BoardState::from_fen(&fen)?;
    MoveValidator::validate_board(&board)?;
    Ok(board.to_fen())
}

/// 导出受管会话的当前局面 FEN
#[tauri::command]
#[specta::specta]
pub fn to_fen(state: State<'_, Mutex<GameState>>) -> Result<String, AppError> {
    let game = state.lock().unwrap();
    Ok(game
        .xiangqi()
        .map_err(legacy_session_error)?
        .current_board()
        .to_fen())
}

/// 对局面执行一步走法并返回结果状态
///
/// 前端视角是纯函数（局面 FEN + ICCS 输入，`MoveResult` 输出）。副作用：当传入
/// FEN 与受管会话的当前局面一致且对局未终局时，该走法也会作用于会话，使历史
/// （悔棋/重做/认输）与当前对局保持同步
#[tauri::command]
#[specta::specta]
pub fn make_move(
    fen: String,
    iccs: String,
    state: State<'_, Mutex<GameState>>,
) -> Result<MoveResult, AppError> {
    let board = BoardState::from_fen(&fen)?;
    let mv = Move::from_iccs(&iccs)?;
    let legal = MoveValidator::can_move(&board, mv);

    if !legal {
        let check = MoveValidator::is_in_check(&board, board.turn);
        return Ok(MoveResult {
            fen,
            iccs,
            chinese_notation: String::new(),
            legal: false,
            check,
            game_over: false,
        });
    }

    let chinese_notation = NotationConverter::to_chinese_notation(&board, &mv)?;
    let (next_board, _) = board.apply_move(mv);
    let check = MoveValidator::is_in_check(&next_board, next_board.turn);
    let game_over = MoveValidator::is_checkmate(&next_board, next_board.turn)
        || MoveValidator::is_stalemate(&next_board, next_board.turn);

    // 让受管会话与当前对局保持一致
    let mut state = state.lock().unwrap();
    if let Ok(game) = state.xiangqi()
        && game.current_board() == board
        && game.result() == GameResult::Ongoing
    {
        let token = state.token();
        state
            .mutate(&token, SessionMutation::Content, |active| match active {
                ActiveGame::Xiangqi(game) => {
                    game.make_move(mv);
                    Ok(())
                }
                ActiveGame::Jieqi(_) => unreachable!("已在同一锁内确认普通象棋分支"),
            })
            .map_err(legacy_session_error)?;
    }

    Ok(MoveResult {
        fen: next_board.to_fen(),
        iccs,
        chinese_notation,
        legal: true,
        check,
        game_over,
    })
}

/// 返回给定局面上当前走方的全部合法 ICCS 走法
#[tauri::command]
#[specta::specta]
pub fn get_legal_moves(fen: String) -> Result<Vec<String>, AppError> {
    let board = BoardState::from_fen(&fen)?;
    Ok(MoveValidator::get_all_legal_moves(&board)
        .iter()
        .map(|m| m.to_iccs())
        .collect())
}

/// 返回给定局面上指定起始格（row: 0..9, col: 0..8）棋子的全部候选目标格（包含会送将/未应将的点位，供 UI 展示落点与反馈）
#[tauri::command]
#[specta::specta]
pub fn get_candidate_moves(fen: String, row: u8, col: u8) -> Result<Vec<String>, AppError> {
    let board = BoardState::from_fen(&fen)?;
    let pos = Position::new(row, col);
    Ok(MoveValidator::get_candidate_moves(&board, pos)
        .iter()
        .map(|m| m.to_iccs())
        .collect())
}

/// 返回局面上一步走法的繁体中文记法
#[tauri::command]
#[specta::specta]
pub fn to_chinese_notation(fen: String, iccs: String) -> Result<String, AppError> {
    let board = BoardState::from_fen(&fen)?;
    let mv = Move::from_iccs(&iccs)?;
    Ok(NotationConverter::to_chinese_notation(&board, &mv)?)
}

/// 报告局面上当前走方是否被将军
#[tauri::command]
#[specta::specta]
pub fn is_in_check(fen: String) -> Result<bool, AppError> {
    let board = BoardState::from_fen(&fen)?;
    Ok(MoveValidator::is_in_check(&board, board.turn))
}

/// 纯函数式重放历史和 PV 前缀，返回规则快照；不会写入受管 GameState
#[tauri::command]
#[specta::specta]
pub fn preview_line(request: PreviewRequest) -> Result<PreviewSnapshot, AppError> {
    let start = BoardState::from_fen(&request.start_fen)?;
    let mut game = XiangqiGame::new_with_profile(start, request.rule_profile);
    for (index, iccs) in request.history.iter().chain(request.pv.iter()).enumerate() {
        if !game.make_move_iccs(iccs) {
            return Err(AppError::InvalidArgument(format!(
                "preview move at index {} is illegal: {iccs}",
                index + 1
            )));
        }
    }
    let assessment = game.rule_assessment();
    Ok(PreviewSnapshot {
        fen: game.current_board().to_fen(),
        red_to_move: game.is_red_to_move(),
        in_check: game.is_in_check(),
        last_move_iccs: request
            .history
            .iter()
            .chain(request.pv.iter())
            .last()
            .cloned(),
        repetition_count: game.repetition_count(),
        rule_status: assessment.status,
        rule_explanation: assessment.explanation,
        applied_pv_len: request.pv.len() as u32,
    })
}

/// 校验整条 PV 后一次性提交，任一步非法都不改变正式对局
#[tauri::command]
#[specta::specta]
pub fn apply_move_line(
    request: ApplyMoveLineRequest,
    state: State<'_, Mutex<GameState>>,
) -> Result<GameSnapshot, AppError> {
    let mut state = state.lock().unwrap();
    let game = state.xiangqi().map_err(legacy_session_error)?;
    if game.current_board().to_fen() != request.expected_fen {
        return Err(AppError::InvalidArgument(
            "正式棋局已变化，请重新选择变例".to_string(),
        ));
    }
    let mut candidate = game.clone();
    for (index, iccs) in request.moves.iter().enumerate() {
        if !candidate.make_move_iccs(iccs) {
            return Err(AppError::InvalidArgument(format!(
                "PV 第 {} 步不合法: {iccs}",
                index + 1
            )));
        }
    }
    let token = state.token();
    state
        .mutate(&token, SessionMutation::Content, |active| {
            *active = ActiveGame::Xiangqi(candidate);
            Ok(())
        })
        .map_err(legacy_session_error)?;
    game_snapshot(&state)
}

/// 开始新对局会话，可选从给定 FEN 开始（默认初始局面）
#[tauri::command]
#[specta::specta]
pub fn new_game(
    fen: Option<String>,
    state: State<'_, Mutex<GameState>>,
) -> Result<GameSnapshot, AppError> {
    let (rule_profile, token) = {
        let state = state.lock().unwrap();
        (
            state
                .xiangqi()
                .map_err(legacy_session_error)?
                .rule_profile(),
            state.token(),
        )
    };
    let candidate = match fen {
        Some(fen_str) => {
            let board = BoardState::from_fen(&fen_str)?;
            MoveValidator::validate_board(&board)?;
            ActiveGame::Xiangqi(XiangqiGame::new_with_profile(board, rule_profile))
        }
        None => ActiveGame::Xiangqi(XiangqiGame::new_with_profile(
            BoardState::default(),
            rule_profile,
        )),
    };
    let mut state = state.lock().unwrap();
    state
        .replace(&token, candidate)
        .map_err(legacy_session_error)?;
    game_snapshot(&state)
}

/// 切换当前对局使用的规则档案
#[tauri::command]
#[specta::specta]
pub fn set_rule_profile(
    profile: RuleProfile,
    state: State<'_, Mutex<GameState>>,
) -> Result<GameSnapshot, AppError> {
    let mut state = state.lock().unwrap();
    let token = state.token();
    state
        .mutate(
            &token,
            SessionMutation::Presentation,
            |active| match active {
                ActiveGame::Xiangqi(game) => {
                    game.set_rule_profile(profile);
                    Ok(())
                }
                ActiveGame::Jieqi(_) => Err(crate::core::game::SessionError::new(
                    crate::core::game::SessionErrorCode::OperationUnavailable,
                    "揭棋不能切换普通象棋规则档案",
                )),
            },
        )
        .map_err(legacy_session_error)?;
    game_snapshot(&state)
}

/// 返回当前对局会话的快照
#[tauri::command]
#[specta::specta]
pub fn game_result(state: State<'_, Mutex<GameState>>) -> Result<GameSnapshot, AppError> {
    let game = state.lock().unwrap();
    game_snapshot(&game)
}

/// 悔掉会话中的上一步
#[tauri::command]
#[specta::specta]
pub fn undo_move(state: State<'_, Mutex<GameState>>) -> Result<GameSnapshot, AppError> {
    legacy_mutation(state, SessionMutation::Presentation, |game| {
        game.undo_move();
    })
}

/// 重新应用刚悔掉的这一步
#[tauri::command]
#[specta::specta]
pub fn redo_move(state: State<'_, Mutex<GameState>>) -> Result<GameSnapshot, AppError> {
    legacy_mutation(state, SessionMutation::Presentation, |game| {
        game.redo_move();
    })
}

/// 将当前对局会话定位到指定步数游标（0 为起始局面）
#[tauri::command]
#[specta::specta]
pub fn jump_to(ply: u32, state: State<'_, Mutex<GameState>>) -> Result<GameSnapshot, AppError> {
    legacy_mutation(state, SessionMutation::Presentation, |game| {
        game.jump_to(ply as usize);
    })
}

/// 标记指定一方（`red` / `black`）认输
#[tauri::command]
#[specta::specta]
pub fn resign(color: String, state: State<'_, Mutex<GameState>>) -> Result<GameSnapshot, AppError> {
    let side = parse_resign_color(&color)?;
    legacy_mutation(state, SessionMutation::Content, |game| game.resign(side))
}

fn parse_resign_color(color: &str) -> Result<Color, AppError> {
    match color.to_ascii_lowercase().as_str() {
        "red" => Ok(Color::Red),
        "black" => Ok(Color::Black),
        _ => Err(AppError::InvalidArgument(
            "color must be 'red' or 'black'".to_string(),
        )),
    }
}

fn game_snapshot(state: &GameState) -> Result<GameSnapshot, AppError> {
    let game = state.xiangqi().map_err(legacy_session_error)?;
    let rule_assessment = game.rule_assessment();
    let current_fen = game.current_board().to_fen();
    let full_history = game.full_history();
    let history: Vec<PlyRecord> = full_history
        .into_iter()
        .enumerate()
        .map(|(idx, r)| PlyRecord {
            ply: (idx + 1) as u32,
            iccs: r.mv.to_iccs(),
            notation: r.chinese_notation,
            mover: if r.mover.is_red() {
                "red".to_string()
            } else {
                "black".to_string()
            },
            is_capture: r.captured.is_some(),
            is_check: matches!(r.effect, MoveEffect::Check | MoveEffect::Checkmate),
            fen: r.position_fen,
        })
        .collect();

    Ok(GameSnapshot {
        fen: current_fen.clone(),
        start_fen: game.start_fen(),
        current_fen,
        current_ply: game.current_ply() as u32,
        history,
        result: game_result_str(game.result()),
        red_to_move: game.is_red_to_move(),
        in_check: game.is_in_check(),
        can_undo: game.can_undo(),
        can_redo: game.can_redo(),
        rule_profile: game.rule_profile(),
        repetition_count: game.repetition_count(),
        repetition_explanation: game.repetition_assessment().explanation,
        rule_status: rule_assessment.status,
        rule_explanation: rule_assessment.explanation,
    })
}

fn legacy_mutation(
    state: State<'_, Mutex<GameState>>,
    mutation: SessionMutation,
    apply: impl FnOnce(&mut XiangqiGame),
) -> Result<GameSnapshot, AppError> {
    let mut state = state.lock().unwrap();
    let token = state.token();
    state
        .mutate(&token, mutation, |active| match active {
            ActiveGame::Xiangqi(game) => {
                apply(game);
                Ok(())
            }
            ActiveGame::Jieqi(_) => Err(crate::core::game::SessionError::new(
                crate::core::game::SessionErrorCode::OperationUnavailable,
                "当前揭棋会话不支持旧象棋命令",
            )),
        })
        .map_err(legacy_session_error)?;
    game_snapshot(&state)
}

fn legacy_session_error(error: crate::core::game::SessionError) -> AppError {
    AppError::InvalidArgument(error.message)
}

fn game_result_str(result: GameResult) -> String {
    match result {
        GameResult::Ongoing => "ongoing",
        GameResult::RedWin => "redwin",
        GameResult::BlackWin => "blackwin",
        GameResult::Draw => "draw",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_resign_color_is_rejected() {
        assert!(parse_resign_color("green").is_err());
        assert_eq!(parse_resign_color("RED").unwrap(), Color::Red);
    }

    #[test]
    fn validate_position_rejects_facing_kings() {
        let fen = "4k4/9/9/9/9/9/9/9/9/4K4 w";
        assert!(validate_position(fen.to_string()).is_err());
    }

    #[test]
    fn validate_position_accepts_initial_position() {
        assert!(validate_position(INITIAL_FEN.to_string()).is_ok());
    }
}
