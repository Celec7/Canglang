use crate::core::board::BoardState;
use crate::core::game::{
    ActiveGame, GameResult, GameState, NewGameOptions, SessionError, SessionErrorCode,
    SessionMutation, SessionSnapshot, SessionToken, XiangqiGame,
};
use crate::core::jieqi::{JieqiGame, JieqiGameError, JieqiRules};
use crate::core::piece::Color;
use crate::core::position::{Move, Position};
use crate::core::rules::{MoveValidator, RuleProfile};
use crate::ipc::engine::EngineState;
use crate::services::jieqi_setup::create_random_jieqi_position;
use std::sync::Mutex;
use tauri::State;

const SELF_CHECK_MESSAGE: &str = "不能送将，请选择其他位置";

#[tauri::command]
#[specta::specta]
pub fn session_get(state: State<'_, Mutex<GameState>>) -> SessionSnapshot {
    state.lock().unwrap().snapshot()
}

#[tauri::command]
#[specta::specta]
pub async fn session_new(
    token: SessionToken,
    options: NewGameOptions,
    state: State<'_, Mutex<GameState>>,
    engine_state: State<'_, EngineState>,
) -> Result<SessionSnapshot, SessionError> {
    let candidate = match options {
        NewGameOptions::Xiangqi { fen, rule_profile } => {
            let board = match fen {
                Some(value) => {
                    let board = BoardState::from_fen(&value).map_err(invalid_input)?;
                    MoveValidator::validate_board(&board).map_err(invalid_input)?;
                    board
                }
                None => BoardState::initial(),
            };
            ActiveGame::Xiangqi(XiangqiGame::new_with_profile(board, rule_profile))
        }
        NewGameOptions::Jieqi { play_mode } => {
            let position = create_random_jieqi_position().map_err(invalid_input)?;
            ActiveGame::Jieqi(JieqiGame::new(position, play_mode))
        }
    };
    engine_state
        .coordinate_replacement(&state, &token, candidate)
        .await
}

#[tauri::command]
#[specta::specta]
pub fn session_targets(
    token: SessionToken,
    from: Position,
    state: State<'_, Mutex<GameState>>,
) -> Result<Vec<String>, SessionError> {
    let game = state.lock().unwrap();
    game.check_token(&token)?;
    let moves = match game.active() {
        ActiveGame::Xiangqi(xiangqi) => {
            MoveValidator::get_candidate_moves(&xiangqi.current_board(), from)
                .into_iter()
                .map(|mv| mv.to_iccs())
                .collect()
        }
        ActiveGame::Jieqi(jieqi) => JieqiRules::candidate_targets(jieqi.position(), from)
            .into_iter()
            .map(|to| Move::new(from, to).to_iccs())
            .collect(),
    };
    Ok(moves)
}

#[tauri::command]
#[specta::specta]
pub fn session_move(
    token: SessionToken,
    iccs: String,
    state: State<'_, Mutex<GameState>>,
) -> Result<SessionSnapshot, SessionError> {
    let mv = Move::from_iccs(&iccs).map_err(invalid_input)?;
    state
        .lock()
        .unwrap()
        .mutate(&token, SessionMutation::Content, |active| match active {
            ActiveGame::Xiangqi(game) => apply_xiangqi_move(game, mv),
            ActiveGame::Jieqi(game) => game.make_move(mv).map(|_| ()).map_err(map_jieqi),
        })
}

fn apply_xiangqi_move(game: &mut XiangqiGame, mv: Move) -> Result<(), SessionError> {
    if game.result() != GameResult::Ongoing {
        return Err(SessionError::new(
            SessionErrorCode::GameFinished,
            "对局已经结束",
        ));
    }
    let is_candidate = MoveValidator::get_candidate_moves(&game.current_board(), mv.from)
        .into_iter()
        .any(|candidate| candidate == mv);
    if game.make_move(mv) {
        return Ok(());
    }
    Err(SessionError::new(
        SessionErrorCode::IllegalMove,
        if is_candidate {
            SELF_CHECK_MESSAGE
        } else {
            "走法不合法"
        },
    ))
}

#[tauri::command]
#[specta::specta]
pub fn session_undo(
    token: SessionToken,
    state: State<'_, Mutex<GameState>>,
) -> Result<SessionSnapshot, SessionError> {
    mutate_history(token, state, |active| match active {
        ActiveGame::Xiangqi(game) => {
            if game.undo_move() {
                Ok(())
            } else {
                Err(unavailable("没有可撤销的走法"))
            }
        }
        ActiveGame::Jieqi(game) => game.undo().map_err(map_jieqi),
    })
}

#[tauri::command]
#[specta::specta]
pub fn session_redo(
    token: SessionToken,
    state: State<'_, Mutex<GameState>>,
) -> Result<SessionSnapshot, SessionError> {
    mutate_history(token, state, |active| match active {
        ActiveGame::Xiangqi(game) => {
            if game.redo_move() {
                Ok(())
            } else {
                Err(unavailable("没有可重做的走法"))
            }
        }
        ActiveGame::Jieqi(game) => game.redo().map_err(map_jieqi),
    })
}

#[tauri::command]
#[specta::specta]
pub fn session_jump(
    token: SessionToken,
    ply: u32,
    state: State<'_, Mutex<GameState>>,
) -> Result<SessionSnapshot, SessionError> {
    mutate_history(token, state, |active| match active {
        ActiveGame::Xiangqi(game) => {
            if ply as usize > game.full_history().len() {
                return Err(invalid("历史位置越界"));
            }
            game.jump_to(ply as usize);
            Ok(())
        }
        ActiveGame::Jieqi(game) => game.jump_to(ply as usize).map_err(map_jieqi),
    })
}

#[tauri::command]
#[specta::specta]
pub fn session_resign(
    token: SessionToken,
    side: Color,
    state: State<'_, Mutex<GameState>>,
) -> Result<SessionSnapshot, SessionError> {
    state
        .lock()
        .unwrap()
        .mutate(&token, SessionMutation::Content, |active| match active {
            ActiveGame::Xiangqi(game) => {
                if side != game.current_board().turn {
                    return Err(invalid("认输方必须是当前走方"));
                }
                if game.result() != GameResult::Ongoing {
                    return Err(SessionError::new(
                        SessionErrorCode::GameFinished,
                        "对局已经结束",
                    ));
                }
                game.resign(side);
                Ok(())
            }
            ActiveGame::Jieqi(game) => game.resign(side).map_err(map_jieqi),
        })
}

#[tauri::command]
#[specta::specta]
pub fn session_offer_draw(
    token: SessionToken,
    side: Color,
    state: State<'_, Mutex<GameState>>,
) -> Result<SessionSnapshot, SessionError> {
    mutate_jieqi(token, state, SessionMutation::Presentation, |game| {
        game.offer_draw(side).map(|_| ())
    })
}

#[tauri::command]
#[specta::specta]
pub fn session_respond_draw(
    token: SessionToken,
    offer_id: String,
    side: Color,
    accept: bool,
    state: State<'_, Mutex<GameState>>,
) -> Result<SessionSnapshot, SessionError> {
    let mutation = if accept {
        SessionMutation::Content
    } else {
        SessionMutation::Presentation
    };
    mutate_jieqi(token, state, mutation, |game| {
        game.respond_draw(&offer_id, side, accept)
    })
}

#[tauri::command]
#[specta::specta]
pub fn session_cancel_draw(
    token: SessionToken,
    offer_id: String,
    side: Color,
    state: State<'_, Mutex<GameState>>,
) -> Result<SessionSnapshot, SessionError> {
    mutate_jieqi(token, state, SessionMutation::Presentation, |game| {
        game.cancel_draw(&offer_id, side)
    })
}

fn mutate_history(
    token: SessionToken,
    state: State<'_, Mutex<GameState>>,
    apply: impl FnOnce(&mut ActiveGame) -> Result<(), SessionError>,
) -> Result<SessionSnapshot, SessionError> {
    state
        .lock()
        .unwrap()
        .mutate(&token, SessionMutation::Presentation, apply)
}

fn mutate_jieqi(
    token: SessionToken,
    state: State<'_, Mutex<GameState>>,
    mutation: SessionMutation,
    apply: impl FnOnce(&mut JieqiGame) -> Result<(), JieqiGameError>,
) -> Result<SessionSnapshot, SessionError> {
    state
        .lock()
        .unwrap()
        .mutate(&token, mutation, |active| match active {
            ActiveGame::Jieqi(game) => apply(game).map_err(map_jieqi),
            ActiveGame::Xiangqi(_) => Err(unavailable("普通象棋不支持该操作")),
        })
}

fn map_jieqi(error: JieqiGameError) -> SessionError {
    let code = match error {
        JieqiGameError::IllegalMove | JieqiGameError::ExposesKing => SessionErrorCode::IllegalMove,
        JieqiGameError::OperationUnavailable(_) => SessionErrorCode::OperationUnavailable,
        JieqiGameError::InvalidPly(_)
        | JieqiGameError::WrongSide
        | JieqiGameError::StaleDrawOffer
        | JieqiGameError::WrongDrawResponder
        | JieqiGameError::WrongDrawCanceller
        | JieqiGameError::RecordedPlyMismatch => SessionErrorCode::InvalidInput,
    };
    SessionError::new(code, error.to_string())
}

fn invalid_input(error: impl std::fmt::Display) -> SessionError {
    invalid(error.to_string())
}

fn invalid(message: impl Into<String>) -> SessionError {
    SessionError::new(SessionErrorCode::InvalidInput, message)
}

fn unavailable(message: impl Into<String>) -> SessionError {
    SessionError::new(SessionErrorCode::OperationUnavailable, message)
}

pub fn default_xiangqi_options(profile: RuleProfile) -> NewGameOptions {
    NewGameOptions::Xiangqi {
        fen: None,
        rule_profile: profile,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_king_maps_to_specific_illegal_move_feedback() {
        let error = map_jieqi(JieqiGameError::ExposesKing);

        assert_eq!(error.code, SessionErrorCode::IllegalMove);
        assert_eq!(error.message, SELF_CHECK_MESSAGE);
    }

    #[test]
    fn xiangqi_self_check_uses_the_same_feedback() {
        let board = BoardState::from_fen("3kr4/9/9/9/9/4R4/9/9/9/4K4 w").unwrap();
        let mut game = XiangqiGame::new(board);
        let before = game.current_board();

        let error = apply_xiangqi_move(
            &mut game,
            Move::new(Position::new(5, 4), Position::new(5, 5)),
        )
        .unwrap_err();

        assert_eq!(error.code, SessionErrorCode::IllegalMove);
        assert_eq!(error.message, SELF_CHECK_MESSAGE);
        assert_eq!(game.current_board(), before);
    }
}
