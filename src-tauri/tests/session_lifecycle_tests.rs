use canglang_app::core::game::{ActiveGame, GameState, SessionErrorCode};
use canglang_app::core::jieqi::{JieqiGame, JieqiPlayMode};
use canglang_app::ipc::engine::EngineState;
use canglang_app::services::jieqi_setup::create_random_jieqi_position;
use std::sync::Mutex;

#[tokio::test]
async fn coordinated_replacement_rechecks_token_and_changes_mode_atomically() {
    let engine = EngineState::default();
    let game = Mutex::new(GameState::default());
    let token = game.lock().unwrap().token();
    let candidate = ActiveGame::Jieqi(JieqiGame::new(
        create_random_jieqi_position().unwrap(),
        JieqiPlayMode::Duel,
    ));
    let snapshot = engine
        .coordinate_replacement(&game, &token, candidate)
        .await
        .unwrap();

    assert!(!snapshot.capabilities.analyze.enabled);
    assert!(!snapshot.capabilities.query_book.enabled);
    assert_ne!(snapshot.game_id, token.game_id);

    let second_candidate = ActiveGame::Jieqi(JieqiGame::new(
        create_random_jieqi_position().unwrap(),
        JieqiPlayMode::Training,
    ));
    let error = engine
        .coordinate_replacement(&game, &token, second_candidate)
        .await
        .unwrap_err();
    assert_eq!(error.code, SessionErrorCode::StaleSession);
    assert_eq!(game.lock().unwrap().snapshot(), snapshot);
}
