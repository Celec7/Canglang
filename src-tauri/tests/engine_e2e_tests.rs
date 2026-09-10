use canglang_app::core::board::BoardState;
use canglang_app::engine::EngineSession;
use canglang_app::engine::models::{
    AnalysisConfig, AnalysisEvent, AnalysisMode, AnalysisRequest, EngineConfig,
    EnginePositionContext, RootMoveConstraint,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 端到端验证：以真实外部引擎（默认 Pikafish）完成握手、分析并收到 bestmove
///
/// 依赖环境变量（含本地路径的机器须通过环境变量提供，测试文件不含任何本地路径）：
/// - `PIKAFISH_ENGINE`：引擎可执行文件绝对路径（必需）
/// - `PIKAFISH_NNUE`：NNUE 权值文件绝对路径（可选；若未设，则在引擎同目录查找 *.nnue）
///
/// 默认 `#[ignore]`，需显式运行：
///   `cargo test --test engine_e2e_tests -- --ignored`
#[tokio::test]
#[ignore]
async fn enginesession_handshake_and_analysis_with_external_engine() {
    let engine = match std::env::var("PIKAFISH_ENGINE") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            eprintln!("skip: PIKAFISH_ENGINE not set");
            return;
        }
    };
    if !engine.exists() {
        eprintln!("skip: engine not found at {engine:?}");
        return;
    }

    let nnue = std::env::var("PIKAFISH_NNUE")
        .ok()
        .map(PathBuf::from)
        .or_else(|| find_nnue_sibling(&engine));

    // 引擎二进制可能源自只读目录且无执行位，复制到可写临时目录并加执行权限
    let dir = std::env::temp_dir().join(format!("pika_e2e_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join(engine.file_name().unwrap_or_default());
    std::fs::copy(&engine, &bin).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();
    }

    let mut options = HashMap::new();
    options.insert("Threads".to_string(), "1".to_string());

    // 权重经 EngineConfig::nnue_path 下发（会话在握手后发送
    // `setoption name EvalFile value ...`），与内置引擎自动装配的路径一致
    let nnue = match nnue {
        Some(nnue) => nnue.to_string_lossy().into_owned(),
        None => {
            eprintln!("skip: no NNUE found (set PIKAFISH_NNUE or place *.nnue next to engine)");
            return;
        }
    };

    let config = EngineConfig::new(bin.to_str().unwrap().to_string(), "uci")
        .with_options(options)
        .with_nnue(nnue);

    let mut session = EngineSession::new();
    session
        .start(&config)
        .await
        .expect("engine start/handshake failed");

    // 订阅须先于 analyze，才能捕获思考期间的实时 info/bestmove 行
    let mut best_rx = session.best_move_stream();
    let mut think_rx = session.think_stream();

    let board = BoardState::initial();
    let cfg = AnalysisConfig::new(AnalysisMode::FixedTime, 1000).with_multi_pv(2);
    session
        .analyze(AnalysisRequest {
            analysis_session_id: "pikafish-e2e".to_string(),
            position: EnginePositionContext {
                start_fen: board.to_fen(),
                history: Vec::new(),
                current_fen: board.to_fen(),
            },
            constraint: RootMoveConstraint::default(),
            config: cfg,
        })
        .await
        .expect("analyze failed");

    let best =
        tokio::time::timeout(Duration::from_secs(15), async { best_rx.recv().await.ok() }).await;

    let think = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match think_rx.recv().await {
                Ok(AnalysisEvent::Think { data, .. }) if data.depth > 0 => return Some(data),
                Ok(_) => continue, // 跳过 info string 等非搜索行
                Err(_) => return None,
            }
        }
    })
    .await;

    session.stop().await;

    let best = best
        .expect("bestmove timed out")
        .expect("engine produced no bestmove");
    assert!(matches!(best, AnalysisEvent::BestMove { move_iccs, .. } if move_iccs.len() == 4));
    assert!(think.is_ok(), "did not observe an engine search-info line");

    let _ = std::fs::remove_dir_all(&dir);
}

fn find_nnue_sibling(engine: &Path) -> Option<PathBuf> {
    let parent = engine.parent()?;
    let mut entries = std::fs::read_dir(parent).ok()?;
    entries.find_map(|e| {
        let p = e.ok()?.path();
        if p.extension()
            .map(|x| x.eq_ignore_ascii_case("nnue"))
            .unwrap_or(false)
        {
            Some(p)
        } else {
            None
        }
    })
}
