use canglang_app::core::board::INITIAL_FEN;
use canglang_app::core::position::Move;
use canglang_app::manual::{ChessManual, ManualNode, ManualService, XqfExporter, XqfParser};

fn sample_manual() -> ChessManual {
    let mut root = ManualNode::new(0, None, "开始局面").with_comment("开局总评\n先观察中炮布局");
    root.children.push(
        ManualNode::new(1, Some(Move::from_iccs("h2e2").unwrap()), "炮二平五")
            .with_comment("第一手说明"),
    );

    ChessManual {
        title: "导出测试棋谱".to_string(),
        date: Some("2026.09.09".to_string()),
        red_player: Some("红方选手".to_string()),
        black_player: Some("黑方选手".to_string()),
        event_name: Some("测试赛事".to_string()),
        start_fen: INITIAL_FEN.to_string(),
        root,
    }
}

#[test]
fn export_v10_roundtrips_metadata_comments_and_mainline() {
    let bytes = XqfExporter::export(&sample_manual(), 10).unwrap();
    assert_eq!(&bytes[..2], b"XQ");
    assert_eq!(bytes[2], 10);
    assert!(bytes.len() > 1024);

    let loaded = XqfParser::parse(&bytes).unwrap();
    assert_eq!(loaded.title, "导出测试棋谱");
    assert_eq!(loaded.event_name.as_deref(), Some("测试赛事"));
    assert_eq!(loaded.date.as_deref(), Some("2026.09.09"));
    assert_eq!(loaded.red_player.as_deref(), Some("红方选手"));
    assert_eq!(loaded.black_player.as_deref(), Some("黑方选手"));
    assert_eq!(loaded.start_fen, INITIAL_FEN);
    assert_eq!(
        loaded.root.comment.as_deref(),
        Some("开局总评\n先观察中炮布局")
    );
    assert_eq!(loaded.root.children.len(), 1);
    assert_eq!(loaded.root.children[0].mv.unwrap().to_iccs(), "h2e2");
    assert_eq!(
        loaded.root.children[0].comment.as_deref(),
        Some("第一手说明")
    );
}

#[test]
fn export_v10_rejects_variations_instead_of_flattening() {
    let mut manual = sample_manual();
    manual.root.children.push(ManualNode::new(
        2,
        Some(Move::from_iccs("b0c2").unwrap()),
        "马八进七",
    ));

    let error = XqfExporter::export(&manual, 10).unwrap_err().to_string();
    assert!(error.contains("does not support variations"));
}

#[test]
fn export_v10_rejects_unsupported_version_and_gbk_text() {
    let error = XqfExporter::export(&sample_manual(), 11)
        .unwrap_err()
        .to_string();
    assert!(error.contains("version 10"));

    let mut manual = sample_manual();
    manual.root.comment = Some("不可编码 😀".to_string());
    let error = XqfExporter::export(&manual, 10).unwrap_err().to_string();
    assert!(error.contains("cannot be represented in GBK"));
}

#[test]
fn export_v10_rejects_oversized_metadata() {
    let mut manual = sample_manual();
    manual.title = "字".repeat(64);

    let error = XqfExporter::export(&manual, 10).unwrap_err().to_string();
    assert!(error.contains("title is too long"));
}

#[test]
fn service_saves_xqf_as_binary_and_rejects_other_extensions() {
    let path = std::env::temp_dir().join(format!("canglang_xqf_export_{}.xqf", std::process::id()));
    ManualService::save_xqf(&sample_manual(), path.to_str().unwrap(), 10).unwrap();
    let loaded = ManualService::load(path.to_str().unwrap()).unwrap();
    assert_eq!(loaded.root.children[0].mv.unwrap().to_iccs(), "h2e2");
    assert!(ManualService::save_xqf(&sample_manual(), "/tmp/game.pgn", 10).is_err());
    let _ = std::fs::remove_file(path);
}
