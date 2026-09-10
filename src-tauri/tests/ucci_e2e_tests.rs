//! 可选真实 UCCI 引擎验收。未提供 `UCCI_ENGINE` 时只报告跳过原因

use canglang_app::core::board::BoardState;
use canglang_app::engine::EngineSession;
use canglang_app::engine::models::{
    AnalysisConfig, AnalysisEvent, AnalysisMode, AnalysisRequest, EngineConfig,
    EnginePositionContext, RootMoveConstraint,
};
use std::path::PathBuf;
use std::time::Duration;

/// 使用代表性 UCCI 引擎验证握手、UCCI 时间格式、info 和 bestmove 生命周期
///
/// `UCCI_ENGINE=/absolute/path/to/engine cargo test --test ucci_e2e_tests -- --ignored`
#[tokio::test]
#[ignore]
async fn ucci_engine_handshake_and_analysis() {
    let path = match std::env::var("UCCI_ENGINE") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            eprintln!("skip: UCCI_ENGINE not set");
            return;
        }
    };
    if !path.is_file() {
        eprintln!("skip: UCCI engine not found at {path:?}");
        return;
    }

    let board = BoardState::initial();
    let mut session = EngineSession::new();
    session
        .start(&EngineConfig::new(path.to_string_lossy(), "ucci"))
        .await
        .expect("UCCI handshake failed");
    let mut best_rx = session.best_move_stream();
    session
        .analyze(AnalysisRequest {
            analysis_session_id: "ucci-e2e".to_string(),
            position: EnginePositionContext {
                start_fen: board.to_fen(),
                history: Vec::new(),
                current_fen: board.to_fen(),
            },
            constraint: RootMoveConstraint::default(),
            config: AnalysisConfig::new(AnalysisMode::FixedTime, 1000),
        })
        .await
        .expect("UCCI analysis failed");

    let best = tokio::time::timeout(Duration::from_secs(15), best_rx.recv())
        .await
        .expect("UCCI bestmove timed out")
        .expect("UCCI bestmove channel closed");
    assert!(matches!(best, AnalysisEvent::BestMove { move_iccs, .. } if move_iccs.len() == 4));
    session.stop().await;
}
