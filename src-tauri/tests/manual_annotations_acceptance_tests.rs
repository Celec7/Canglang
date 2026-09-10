use canglang_app::core::board::BoardState;
use canglang_app::core::position::Move;
use canglang_app::manual::{ChessManual, ManualNode, PgnExporter, PgnParser, XqfExporter};

fn assert_tree(actual: &ManualNode, expected: &ManualNode) {
    assert_eq!(actual.mv, expected.mv);
    assert_eq!(actual.comment, expected.comment);
    assert_eq!(actual.children.len(), expected.children.len());
    for (actual_child, expected_child) in actual.children.iter().zip(&expected.children) {
        assert_tree(actual_child, expected_child);
    }
}

fn sample_manual() -> ChessManual {
    let mut root = ManualNode::new(0, None, "开始局面").with_comment("根备注\n第二行");
    let mut main = ManualNode::new(1, Some(Move::from_iccs("h2e2").unwrap()), "炮二平五")
        .with_comment(r"主线含反斜杠 \ 和右花括号 }");
    let mut main_reply = ManualNode::new(2, Some(Move::from_iccs("h9g7").unwrap()), "马８进７")
        .with_comment("主线第二手");
    main_reply.children.push(
        ManualNode::new(4, Some(Move::from_iccs("b0c2").unwrap()), "马八进七")
            .with_comment("嵌套变例"),
    );
    main.children.push(main_reply);
    root.children.push(main);
    root.children.push(
        ManualNode::new(3, Some(Move::from_iccs("b0c2").unwrap()), "马八进七")
            .with_comment("第一分支"),
    );

    ChessManual::new(
        "验收棋谱",
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w",
        root,
    )
}

#[test]
fn annotations_survive_pgn_tree_roundtrip_across_all_node_kinds() {
    let manual = sample_manual();
    let text = PgnExporter::export(&manual, false);
    let loaded = PgnParser::parse(&text).unwrap();

    assert_tree(&loaded.root, &manual.root);
}

#[test]
fn whitespace_comments_are_empty_but_multiline_special_text_is_preserved() {
    let whitespace = PgnParser::parse("1. h2e2 { \n\t } *").unwrap();
    assert_eq!(whitespace.root.comment, None);
    assert_eq!(whitespace.root.children[0].comment, None);

    let mut manual = ChessManual::new(
        "特殊字符",
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w",
        ManualNode::new(0, None, "开始局面"),
    );
    manual.root.comment = Some("第一行\n第二行 \\ }".to_string());
    let loaded = PgnParser::parse(&PgnExporter::export(&manual, false)).unwrap();
    assert_eq!(loaded.root.comment, manual.root.comment);
}

#[test]
fn comments_do_not_change_mainline_position_evolution() {
    let with_comments = sample_manual();
    let mut without_comments = with_comments.clone();
    fn clear_comments(node: &mut ManualNode) {
        node.comment = None;
        for child in &mut node.children {
            clear_comments(child);
        }
    }
    clear_comments(&mut without_comments.root);

    fn apply_mainline(manual: &ChessManual) -> String {
        let mut board = BoardState::from_fen(&manual.start_fen).unwrap();
        let mut node = &manual.root;
        while let Some(next) = node.children.first() {
            if let Some(mv) = next.mv {
                board = board.apply_move(mv).0;
            }
            node = next;
        }
        board.to_fen()
    }

    assert_eq!(
        apply_mainline(&with_comments),
        apply_mainline(&without_comments)
    );
}

#[test]
fn xqf_export_acceptance_keeps_unsupported_variations_explicit() {
    let error = XqfExporter::export(&sample_manual(), 10)
        .unwrap_err()
        .to_string();
    assert!(error.contains("does not support variations"));
}
