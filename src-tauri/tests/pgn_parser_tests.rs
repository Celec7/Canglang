use canglang_app::manual::PgnParser;

#[test]
fn parse_standard_iccs_pgn_with_tags_and_comments() {
    let pgn = r#"
[Game "Chinese Chess"]
[Event "大师邀请赛"]
[Date "2025.01.01"]
[Red "王天一"]
[Black "郑惟桐"]
[Result "1-0"]

1. h2e2 h9g7 2. b0c2 {起正马护中卒} *
"#;

    let manual = PgnParser::parse(pgn).unwrap();

    assert_eq!(manual.title, "大师邀请赛");
    assert_eq!(manual.date.as_deref(), Some("2025.01.01"));
    assert_eq!(manual.red_player.as_deref(), Some("王天一"));
    assert_eq!(manual.black_player.as_deref(), Some("郑惟桐"));

    // 主线树结构
    assert_eq!(manual.root.children.len(), 1);
    let step1 = &manual.root.children[0];
    assert_eq!(step1.mv.unwrap().to_iccs(), "h2e2");
    assert_eq!(step1.chinese_notation, "炮二平五");

    assert_eq!(step1.children.len(), 1);
    let step2 = &step1.children[0];
    assert_eq!(step2.mv.unwrap().to_iccs(), "h9g7");
    assert_eq!(step2.chinese_notation, "馬８進７");

    assert_eq!(step2.children.len(), 1);
    let step3 = &step2.children[0];
    assert_eq!(step3.mv.unwrap().to_iccs(), "b0c2");
    assert_eq!(step3.comment.as_deref(), Some("起正马护中卒"));
}

#[test]
fn parse_root_mainline_and_variation_comments_preserves_order() {
    let pgn = r#"
{开局说明
第二行} {补充说明}
1. h2e2 {主线说明
细节} (1. b0c2 {变例说明}) h9g7 *
"#;

    let manual = PgnParser::parse(pgn).unwrap();

    assert_eq!(
        manual.root.comment.as_deref(),
        Some("开局说明\n第二行\n补充说明")
    );
    assert_eq!(manual.root.children.len(), 2);

    let mainline = &manual.root.children[0];
    assert_eq!(mainline.mv.unwrap().to_iccs(), "h2e2");
    assert_eq!(mainline.comment.as_deref(), Some("主线说明\n细节"));

    let variation = &manual.root.children[1];
    assert_eq!(variation.mv.unwrap().to_iccs(), "b0c2");
    assert_eq!(variation.comment.as_deref(), Some("变例说明"));
}

#[test]
fn parse_nested_variation_comments_attach_to_each_branch_node() {
    let pgn =
        "1. h2e2 h9g7 (1... b9c7 {黑方变例} 2. b0c2 {变例主线} (2. a0a1 {嵌套变例})) 2. b0c2 *";

    let manual = PgnParser::parse(pgn).unwrap();
    let first = &manual.root.children[0];
    assert_eq!(first.children.len(), 2);

    let main_black = &first.children[0];
    assert_eq!(main_black.mv.unwrap().to_iccs(), "h9g7");

    let variation_black = &first.children[1];
    assert_eq!(variation_black.mv.unwrap().to_iccs(), "b9c7");
    assert_eq!(variation_black.comment.as_deref(), Some("黑方变例"));
    assert_eq!(variation_black.children.len(), 2);

    let variation_main_red = &variation_black.children[0];
    assert_eq!(variation_main_red.mv.unwrap().to_iccs(), "b0c2");
    assert_eq!(variation_main_red.comment.as_deref(), Some("变例主线"));

    let nested_variation = &variation_black.children[1];
    assert_eq!(nested_variation.mv.unwrap().to_iccs(), "a0a1");
    assert_eq!(nested_variation.comment.as_deref(), Some("嵌套变例"));
}

#[test]
fn parse_chinese_notation_pgn_translates_to_iccs() {
    let pgn = r#"
[Event "全国甲级联赛"]

1. 炮二平五 马８进７ 2. 马八进七 *
"#;

    let manual = PgnParser::parse(pgn).unwrap();

    assert_eq!(manual.root.children.len(), 1);
    let m1 = &manual.root.children[0];
    assert_eq!(m1.mv.unwrap().to_iccs(), "h2e2");

    let m2 = &m1.children[0];
    assert_eq!(m2.mv.unwrap().to_iccs(), "h9g7");

    let m3 = &m2.children[0];
    assert_eq!(m3.mv.unwrap().to_iccs(), "b0c2");
}

#[test]
fn parse_nested_variations_builds_multi_branch_tree() {
    let pgn = "1. h2e2 (1. b0c2 b9c7) h9g7 *";

    let manual = PgnParser::parse(pgn).unwrap();

    // 根节点下有两个分支：主线 h2e2 与变例 b0c2
    assert_eq!(manual.root.children.len(), 2);
    assert_eq!(manual.root.children[0].mv.unwrap().to_iccs(), "h2e2");
    assert_eq!(manual.root.children[1].mv.unwrap().to_iccs(), "b0c2");
}

#[test]
fn invalid_moves_are_not_silently_skipped() {
    let invalid_iccs = "1. h2e2 not-a-move *";
    let illegal_iccs = "1. h2a2 *";

    assert!(PgnParser::parse(invalid_iccs).is_err());
    assert!(PgnParser::parse(illegal_iccs).is_err());
}

#[test]
fn malformed_pgn_structure_is_rejected() {
    assert!(PgnParser::parse("[Event broken]\n1. h2e2 *").is_err());
    assert!(PgnParser::parse("1. h2e2 (1. b0c2 *").is_err());
    assert!(PgnParser::parse("1. h2e2 {unterminated").is_err());
}

#[test]
fn variation_after_the_first_move_attaches_to_the_correct_parent() {
    let pgn = "1. h2e2 h9g7 (1... b9c7 2. b0c2) 2. b0c2 *";
    let manual = PgnParser::parse(pgn).unwrap();

    let first = &manual.root.children[0];
    assert_eq!(first.mv.unwrap().to_iccs(), "h2e2");
    assert_eq!(first.children.len(), 2);
    assert_eq!(first.children[0].mv.unwrap().to_iccs(), "h9g7");
    assert_eq!(first.children[1].mv.unwrap().to_iccs(), "b9c7");
}

#[test]
fn compact_move_number_and_annotations_are_parsed_correctly() {
    // 紧凑步数格式 1.h2e2! (1.b0c2!?) 1...h9g7+ 2.b0c2#
    let pgn = "1.h2e2! (1.b0c2!?) 1...h9g7+ 2.b0c2# *";
    let manual = PgnParser::parse(pgn).unwrap();

    assert_eq!(manual.root.children.len(), 2);
    let m1 = &manual.root.children[0];
    let v1 = &manual.root.children[1];
    assert_eq!(m1.mv.unwrap().to_iccs(), "h2e2");
    assert_eq!(v1.mv.unwrap().to_iccs(), "b0c2");

    assert_eq!(m1.children.len(), 1);
    let m2 = &m1.children[0];
    assert_eq!(m2.mv.unwrap().to_iccs(), "h9g7");

    assert_eq!(m2.children.len(), 1);
    let m3 = &m2.children[0];
    assert_eq!(m3.mv.unwrap().to_iccs(), "b0c2");
}
