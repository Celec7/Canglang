use canglang_app::core::board::BoardState;
use canglang_app::core::game::{
    ActiveGame, GameState, PositionView, SessionError, SessionErrorCode, SessionMutation,
    SessionPly, XiangqiGame,
};
use canglang_app::core::jieqi::{JieqiGame, JieqiPlayMode};
use canglang_app::core::position::Move;
use canglang_app::services::jieqi_setup::create_random_jieqi_position;

#[test]
fn same_layout_replacement_gets_a_new_game_id() {
    let mut state = GameState::default();
    let before = state.snapshot();
    let token = state.token();
    let after = state
        .replace(
            &token,
            ActiveGame::Xiangqi(XiangqiGame::new(BoardState::initial())),
        )
        .unwrap();

    assert_ne!(before.game_id, after.game_id);
    assert_eq!(after.revision, "0");
    assert_eq!(after.content_revision, "0");
}

#[test]
fn stale_and_duplicate_mutations_are_rejected_atomically() {
    let mut state = GameState::default();
    let token = state.token();
    let mv = Move::from_iccs("h2e2").unwrap();
    state
        .mutate(&token, SessionMutation::Content, |active| match active {
            ActiveGame::Xiangqi(game) => {
                if game.make_move(mv) {
                    Ok(())
                } else {
                    Err(SessionError::new(
                        SessionErrorCode::IllegalMove,
                        "走法不合法",
                    ))
                }
            }
            ActiveGame::Jieqi(_) => unreachable!(),
        })
        .unwrap();
    let after_first = state.snapshot();

    let error = state
        .mutate(&token, SessionMutation::Content, |_| Ok(()))
        .unwrap_err();
    assert_eq!(error.code, SessionErrorCode::StaleSession);
    assert_eq!(state.snapshot(), after_first);
}

#[test]
fn presentation_revision_does_not_change_document_revision() {
    let mut state = GameState::default();
    let first = state.token();
    state
        .mutate(&first, SessionMutation::Content, |active| match active {
            ActiveGame::Xiangqi(game) => {
                assert!(game.make_move_iccs("h2e2"));
                Ok(())
            }
            ActiveGame::Jieqi(_) => unreachable!(),
        })
        .unwrap();
    let second = state.token();
    let after_move = state.snapshot();
    state
        .mutate(
            &second,
            SessionMutation::Presentation,
            |active| match active {
                ActiveGame::Xiangqi(game) => {
                    game.jump_to(0);
                    Ok(())
                }
                _ => unreachable!(),
            },
        )
        .unwrap();
    let after_jump = state.snapshot();

    assert_ne!(after_move.revision, after_jump.revision);
    assert_eq!(after_move.content_revision, after_jump.content_revision);
    assert_eq!(after_jump.current_ply, 0);
}

#[test]
fn jieqi_snapshot_contains_only_public_position_and_events() {
    let position = create_random_jieqi_position().unwrap();
    let game = JieqiGame::new(position, JieqiPlayMode::Duel);
    let state = GameState::new_jieqi(game);
    let snapshot = state.snapshot();
    let json = serde_json::to_string(&snapshot).unwrap();

    assert!(matches!(snapshot.position, PositionView::Jieqi { .. }));
    assert!(
        snapshot
            .history
            .iter()
            .all(|ply| matches!(ply, SessionPly::Jieqi(_)))
    );
    assert!(!json.contains("Assigned"));
    assert!(!json.contains("assigned"));
    assert!(!json.contains("piece_id"));
    assert!(!snapshot.capabilities.analyze.enabled);
    assert!(!snapshot.capabilities.query_book.enabled);
}

#[test]
fn variant_accessors_return_typed_errors() {
    let state = GameState::new_jieqi(JieqiGame::new(
        create_random_jieqi_position().unwrap(),
        JieqiPlayMode::Training,
    ));
    let error = state.xiangqi().unwrap_err();
    assert_eq!(error.code, SessionErrorCode::OperationUnavailable);
}
