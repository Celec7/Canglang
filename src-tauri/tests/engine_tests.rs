use canglang_app::core::board::BoardState;
use canglang_app::core::position::Move;
use canglang_app::core::rules::MoveValidator;
use canglang_app::engine::EngineSession;
use canglang_app::engine::Protocol;
use canglang_app::engine::compute_search_moves;
use canglang_app::engine::models::{
    AnalysisConfig, AnalysisEvent, AnalysisMode, AnalysisRequest, EngineConfig,
    EnginePositionContext, RootMoveConstraint,
};
use canglang_app::engine::protocols::UciProtocol;
use std::time::Duration;

#[test]
fn change_tactic_excludes_specified_moves_from_search_moves() {
    let board = BoardState::initial();
    let all_legal = MoveValidator::get_all_legal_moves(&board);
    let first_move = all_legal[0].to_iccs();

    let remaining = compute_search_moves(&all_legal, std::slice::from_ref(&first_move));

    assert!(!remaining.contains(&first_move));
    assert_eq!(remaining.len(), all_legal.len() - 1);

    let uci = UciProtocol::new();
    let go_cmd = uci.format_go(
        &AnalysisConfig::new(AnalysisMode::FixedTime, 1000),
        Some(&remaining),
    );

    assert!(go_cmd.starts_with("go movetime 1000 searchmoves"));
    assert!(!go_cmd.contains(&format!(" {first_move} ")));
}

#[test]
fn protocol_parses_stream_data_accurately() {
    let protocol = UciProtocol::new();
    let mut think_list = Vec::new();
    let mut move_list = Vec::new();

    let simulated_lines = [
        "info depth 10 score cp 25 time 100 pv h2e2",
        "info depth 12 score cp 40 time 200 pv h2e2 h9g7",
        "bestmove h2e2 ponder h9g7",
    ];

    for line in simulated_lines {
        if let Some(td) = protocol.parse_info(line) {
            think_list.push(td);
        }
        if let Some(iccs) = protocol.parse_best_move(line)
            && let Ok(mv) = Move::from_iccs(&iccs)
        {
            move_list.push(mv);
        }
    }

    assert_eq!(think_list.len(), 2);
    assert_eq!(think_list[1].score, 40);
    assert_eq!(move_list.len(), 1);
    assert_eq!(move_list[0], Move::from_iccs("h2e2").unwrap());
}

#[cfg(unix)]
#[tokio::test]
async fn session_lifecycle_handles_handshake_analysis_stop_and_restart() {
    use std::os::unix::fs::PermissionsExt;

    let path = std::env::temp_dir().join(format!(
        "canglang_fake_engine_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let script = r##"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    uci) printf 'uciok\r\n'; printf 'fake diagnostic\n' >&2 ;;
    isready) printf '\r\n'; echo readyok ;;
    stop) echo "bestmove h2e2"; sleep 0.05; echo "info string tail-after-bestmove" ;;
    quit) exit 0 ;;
  esac
done
"##;
    std::fs::write(&path, script).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();

    let config = EngineConfig::new(path.to_string_lossy(), "uci");
    let mut session = EngineSession::new();
    session.start(&config).await.unwrap();
    assert!(session.is_running());
    assert!(!session.is_analyzing());

    let mut best_rx = session.best_move_stream();
    let initial_fen = BoardState::initial().to_fen();
    session
        .analyze(AnalysisRequest {
            analysis_session_id: "lifecycle".to_string(),
            position: EnginePositionContext {
                start_fen: initial_fen.clone(),
                history: Vec::new(),
                current_fen: initial_fen.clone(),
            },
            constraint: RootMoveConstraint::default(),
            config: AnalysisConfig::default(),
        })
        .await
        .unwrap();
    assert!(session.is_analyzing());

    session.move_now().await.unwrap();
    assert!(!session.is_analyzing());
    let best = tokio::time::timeout(Duration::from_secs(1), best_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        best,
        AnalysisEvent::BestMove {
            analysis_session_id,
            move_iccs
        } if analysis_session_id == "lifecycle" && move_iccs == "h2e2"
    ));

    tokio::time::sleep(Duration::from_millis(30)).await;
    let session_log = session.protocol_log_for_session("lifecycle").await;
    assert!(!session_log.is_empty());
    assert!(session_log.iter().any(|line| {
        line.analysis_session_id.as_deref() == Some("lifecycle") && line.raw.contains("position")
    }));
    assert!(session_log.iter().all(|line| {
        line.analysis_session_id.is_none()
            || line.analysis_session_id.as_deref() == Some("lifecycle")
    }));
    assert!(session_log.iter().all(|line| !line.run_id.is_empty()));
    assert!(session_log.iter().any(|line| {
        line.stream == canglang_app::engine::EngineOutputStream::Stderr
            && line.raw.contains("fake diagnostic")
    }));
    assert!(session_log.iter().any(|line| {
        line.analysis_session_id.as_deref() == Some("lifecycle")
            && line.raw.contains("tail-after-bestmove")
    }));
    assert!(session_log.iter().any(|line| {
        line.stream == canglang_app::engine::EngineOutputStream::Stdout && line.raw == "uciok\r\n"
    }));

    // 第二次分析必须获得新的诊断边界
    // 第一次停止后迟到的输出只能留在第一次会话或无标签的生命周期前置流中，不能被重新标记为第二次会话
    session
        .analyze(AnalysisRequest {
            analysis_session_id: "lifecycle-2".to_string(),
            position: EnginePositionContext {
                start_fen: initial_fen.clone(),
                history: Vec::new(),
                current_fen: initial_fen.clone(),
            },
            constraint: RootMoveConstraint::default(),
            config: AnalysisConfig::default(),
        })
        .await
        .unwrap();
    session.move_now().await.unwrap();
    let second_best = tokio::time::timeout(Duration::from_secs(1), best_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        second_best,
        AnalysisEvent::BestMove {
            analysis_session_id,
            ..
        } if analysis_session_id == "lifecycle-2"
    ));
    tokio::time::sleep(Duration::from_millis(30)).await;
    let second_log = session.protocol_log_for_session("lifecycle-2").await;
    assert!(second_log.iter().any(|line| {
        line.analysis_session_id.as_deref() == Some("lifecycle-2") && line.raw.contains("position")
    }));
    assert!(second_log.iter().all(|line| {
        line.analysis_session_id.is_none()
            || line.analysis_session_id.as_deref() == Some("lifecycle-2")
    }));

    session.stop().await;
    assert!(!session.is_running());
    assert!(!session.is_analyzing());

    session.start(&config).await.unwrap();
    assert!(session.is_running());
    session.stop().await;

    std::fs::remove_file(path).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn natural_bestmove_is_synchronized_before_next_analysis() {
    use std::os::unix::fs::PermissionsExt;

    let path = std::env::temp_dir().join(format!(
        "canglang_fake_engine_natural_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let script = r##"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    uci) echo "option name MultiPV type spin default 1 min 1 max 4"; echo uciok ;;
    isready) echo readyok ;;
    go*) echo "bestmove h2e2" ;;
    quit) exit 0 ;;
  esac
done
"##;
    std::fs::write(&path, script).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();

    let config = EngineConfig::new(path.to_string_lossy(), "uci");
    let mut session = EngineSession::new();
    let mut best_rx = session.best_move_stream();
    session.start(&config).await.unwrap();
    let initial_fen = BoardState::initial().to_fen();
    let first_move = Move::from_iccs("h2e2").unwrap();
    let after_first_fen = BoardState::initial().apply_move(first_move).0.to_fen();
    let request =
        |analysis_session_id: &str, history: Vec<String>, current_fen: String| AnalysisRequest {
            analysis_session_id: analysis_session_id.to_string(),
            position: EnginePositionContext {
                start_fen: initial_fen.clone(),
                history,
                current_fen,
            },
            constraint: RootMoveConstraint::default(),
            config: AnalysisConfig::default(),
        };

    session
        .analyze(request("natural-1", Vec::new(), initial_fen.clone()))
        .await
        .unwrap();
    let first_best = tokio::time::timeout(Duration::from_secs(1), best_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        first_best,
        AnalysisEvent::BestMove { analysis_session_id, .. }
            if analysis_session_id == "natural-1"
    ));

    session
        .analyze(request(
            "natural-2",
            vec![first_move.to_iccs()],
            after_first_fen,
        ))
        .await
        .unwrap();
    let first_log = session.protocol_log_for_session("natural-1").await;
    assert!(first_log.iter().any(|line| {
        line.analysis_session_id.as_deref() == Some("natural-1") && line.raw == "isready\n"
    }));
    let second_log = session.protocol_log_for_session("natural-2").await;
    assert!(second_log.iter().any(|line| {
        line.analysis_session_id.as_deref() == Some("natural-2")
            && line.raw.contains("position")
            && line.raw.contains("moves h2e2")
    }));
    assert!(second_log.iter().all(|line| {
        line.analysis_session_id.is_none()
            || line.analysis_session_id.as_deref() == Some("natural-2")
    }));

    session.stop().await;
    std::fs::remove_file(path).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn session_reports_unexpected_engine_exit() {
    use std::os::unix::fs::PermissionsExt;

    let path = std::env::temp_dir().join(format!(
        "canglang_fake_engine_exit_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let script = r##"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    uci) echo uciok ;;
    isready) echo readyok; exit 0 ;;
  esac
done
"##;
    std::fs::write(&path, script).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();

    let config = EngineConfig::new(path.to_string_lossy(), "uci");
    let mut session = EngineSession::new();
    let mut stopped_rx = session.stopped_stream();
    session.start(&config).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), stopped_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(!session.is_running());

    session.stop().await;
    std::fs::remove_file(path).unwrap();
}
