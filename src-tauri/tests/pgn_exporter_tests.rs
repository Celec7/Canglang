use canglang_app::manual::{ChessManual, ManualNode, PgnExporter, PgnParser};

#[test]
fn export_empty_manual_still_returns_a_visible_pgn_document() {
    let manual = ChessManual::new(
        "未命名棋谱",
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w",
        ManualNode::new(0, None, "开始局面"),
    );

    let exported = PgnExporter::export(&manual, true);

    assert!(exported.contains("[Event \"未命名棋谱\"]"));
    assert!(exported.ends_with("*\n"));
}

#[test]
fn export_and_roundtrip_parse_preserves_tree_structure() {
    let pgn = r#"
[Game "Chinese Chess"]
[Event "棋王争霸赛"]
[Date "2025.05.01"]
[Red "棋手甲"]
[Black "棋手乙"]
[Result "*"]

1. 炮二平五 马８进７ 2. 马八进七 {正马防守} *
"#;

    let manual1 = PgnParser::parse(pgn).unwrap();
    let exported = PgnExporter::export(&manual1, true);

    assert!(exported.contains("[Event \"棋王争霸赛\"]"));
    assert!(exported.contains("炮二平五"));
    assert!(exported.contains("{正马防守}"));

    // 往返解析
    let manual2 = PgnParser::parse(&exported).unwrap();

    assert_eq!(manual1.title, manual2.title);
    assert_eq!(manual1.red_player, manual2.red_player);
    assert_eq!(manual1.black_player, manual2.black_player);

    assert_eq!(manual2.root.children.len(), 1);
    let s1 = &manual2.root.children[0];
    assert_eq!(s1.mv.unwrap().to_iccs(), "h2e2");

    assert_eq!(s1.children.len(), 1);
    let s2 = &s1.children[0];
    assert_eq!(s2.mv.unwrap().to_iccs(), "h9g7");

    assert_eq!(s2.children.len(), 1);
    let s3 = &s2.children[0];
    assert_eq!(s3.mv.unwrap().to_iccs(), "b0c2");
    assert_eq!(s3.comment.as_deref(), Some("正马防守"));
}

#[test]
fn export_escapes_metadata_and_comment_delimiters() {
    let pgn = r##"
[Event "A \"quoted\" \\ path"]

1. h2e2 {close \} slash \\} *
"##;

    let manual = PgnParser::parse(pgn).unwrap();
    let exported = PgnExporter::export(&manual, true);
    let roundtrip = PgnParser::parse(&exported).unwrap();

    assert_eq!(roundtrip.title, r#"A "quoted" \ path"#);
    assert_eq!(
        roundtrip.root.children[0].comment.as_deref(),
        Some(r#"close } slash \"#)
    );
}

#[test]
fn export_and_roundtrip_preserves_root_variation_and_multiline_comments() {
    let pgn = r#"
{根节点说明
第二行} {根节点补充}
1. h2e2 {主线说明} h9g7 (1... b9c7 {变例说明} 2. b0c2 {变例主线} (2. a0a1 {嵌套变例})) 2. b0c2 {黑方说明} *
"#;

    let manual1 = PgnParser::parse(pgn).unwrap();
    let exported = PgnExporter::export(&manual1, true);
    let manual2 = PgnParser::parse(&exported).unwrap();

    assert_eq!(manual2.root.comment, manual1.root.comment);
    fn assert_same_tree(
        actual: &canglang_app::manual::ManualNode,
        expected: &canglang_app::manual::ManualNode,
    ) {
        assert_eq!(actual.mv, expected.mv);
        assert_eq!(actual.comment, expected.comment);
        assert_eq!(actual.children.len(), expected.children.len());
        for (actual_child, expected_child) in actual.children.iter().zip(&expected.children) {
            assert_same_tree(actual_child, expected_child);
        }
    }

    assert_same_tree(&manual2.root, &manual1.root);
}
