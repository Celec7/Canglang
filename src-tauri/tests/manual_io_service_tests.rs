use canglang_app::core::board::INITIAL_FEN;
use canglang_app::core::position::Move;
use canglang_app::manual::{ChessManual, ManualNode, ManualService, PgnParser};

fn sample_manual() -> ChessManual {
    let mut root = ManualNode::new(0, None, "开始局面");
    root.children.push(ManualNode::new(
        1,
        Some(Move::from_iccs("h2e2").unwrap()),
        "炮二平五",
    ));

    ChessManual {
        title: "服务测试棋谱".to_string(),
        date: Some("2025.06.01".to_string()),
        red_player: Some("选手A".to_string()),
        black_player: Some("选手B".to_string()),
        event_name: Some("锦标赛".to_string()),
        start_fen: INITIAL_FEN.to_string(),
        root,
    }
}

#[test]
fn load_file_not_found_throws() {
    assert!(ManualService::load("/non/existent/file.pgn").is_err());
}

#[test]
fn load_unsupported_extension_throws() {
    let path =
        std::env::temp_dir().join(format!("canglang_unsupported_{}.xyz", std::process::id()));
    std::fs::write(&path, b"dummy").unwrap();
    assert!(ManualService::load(path.to_str().unwrap()).is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_and_load_pgn_file_roundtrip_succeeds() {
    let path = std::env::temp_dir().join(format!("canglang_service_{}.pgn", std::process::id()));

    let manual = sample_manual();
    ManualService::save(&manual, path.to_str().unwrap()).unwrap();
    assert!(path.exists());

    let loaded = ManualService::load(path.to_str().unwrap()).unwrap();
    assert_eq!(loaded.title, "服务测试棋谱");
    assert_eq!(loaded.red_player.as_deref(), Some("选手A"));
    assert_eq!(loaded.root.children.len(), 1);
    assert_eq!(loaded.root.children[0].mv.unwrap().to_iccs(), "h2e2");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_pgn_string_then_parse_preserves_move() {
    let text = ManualService::save_pgn_string(&sample_manual());
    let loaded = PgnParser::parse(&text).unwrap();
    assert_eq!(loaded.root.children.len(), 1);
    assert_eq!(loaded.root.children[0].mv.unwrap().to_iccs(), "h2e2");
}
