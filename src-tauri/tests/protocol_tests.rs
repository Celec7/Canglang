use canglang_app::engine::Protocol;
use canglang_app::engine::models::{AnalysisConfig, AnalysisMode};
use canglang_app::engine::protocols::{UcciProtocol, UciProtocol};

fn mv_list(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn uci_protocol_format_commands_match_expected() {
    let uci = UciProtocol::new();

    let pos1 = uci.format_position(Some("startpos"), Some(&mv_list(&["h2e2", "h9g7"])));
    assert_eq!(pos1, "position startpos moves h2e2 h9g7");

    let pos2 = uci.format_position(
        Some("rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w"),
        None,
    );
    assert_eq!(
        pos2,
        "position fen rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w"
    );

    let go1 = uci.format_go(&AnalysisConfig::new(AnalysisMode::FixedTime, 3000), None);
    assert_eq!(go1, "go movetime 3000");

    let go2 = uci.format_go(
        &AnalysisConfig::new(AnalysisMode::FixedDepth, 15),
        Some(&mv_list(&["h2e2", "b0c2"])),
    );
    assert_eq!(go2, "go depth 15 searchmoves h2e2 b0c2");

    let go_inf = uci.format_go(&AnalysisConfig::new(AnalysisMode::Infinite, 0), None);
    assert_eq!(go_inf, "go infinite");
    assert_eq!(
        uci.format_button_option("Clear Hash"),
        "setoption name Clear Hash"
    );
}

#[test]
fn uci_protocol_parse_info_standard_and_out_of_order() {
    let uci = UciProtocol::new();

    // 标准 info
    let line1 = "info depth 18 seldepth 25 multipv 1 score cp 38 nodes 281938 nps 1850000 time 152 pv h2e2 h9g7 b0c2";
    let data1 = uci.parse_info(line1).unwrap();

    assert_eq!(data1.depth, 18);
    assert_eq!(data1.score, 38);
    assert_eq!(data1.mate_in, None);
    assert_eq!(data1.multi_pv, 1);
    assert_eq!(data1.nps, 1850000);
    assert_eq!(data1.time_ms, 152);
    assert_eq!(data1.pv, vec!["h2e2", "h9g7", "b0c2"]);

    // 乱序 info
    let line2 = "info time 500 pv h2e2 score cp -45 depth 12 nps 900000 multipv 2";
    let data2 = uci.parse_info(line2).unwrap();
    assert_eq!(data2.time_ms, 500);
    assert_eq!(data2.depth, 12);
    assert_eq!(data2.score, -45);
    assert_eq!(data2.multi_pv, 2);
    assert_eq!(data2.nps, 900000);
    assert_eq!(data2.pv, vec!["h2e2"]);
}

#[test]
fn uci_protocol_parse_info_mate_scores() {
    let uci = UciProtocol::new();

    // 己方将杀对方
    let data = uci
        .parse_info("info depth 10 score mate 3 time 100 pv d8d9")
        .unwrap();
    assert_eq!(data.mate_in, Some(3));
    assert!(data.score > 20000);

    // 己方被将杀
    let data_mated = uci
        .parse_info("info depth 8 score mate -2 time 80 pv e9e8")
        .unwrap();
    assert_eq!(data_mated.mate_in, Some(-2));
    assert!(data_mated.score < -20000);
}

#[test]
fn uci_protocol_parse_best_move_valid_and_none() {
    let uci = UciProtocol::new();

    assert_eq!(
        uci.parse_best_move("bestmove h2e2 ponder h9g7").as_deref(),
        Some("h2e2")
    );
    assert_eq!(
        uci.parse_best_move("bestmove b0c2").as_deref(),
        Some("b0c2")
    );
    assert_eq!(uci.parse_best_move("bestmove (none)"), None);
    assert_eq!(uci.parse_best_move("bestmove nobestmove"), None);
    assert_eq!(uci.parse_best_move("info depth 10"), None);
}

#[test]
fn ucci_protocol_format_and_parse_compatibility() {
    let ucci = UcciProtocol::new();

    // UCCI Go 使用 time 而非 movetime
    let go_time = ucci.format_go(&AnalysisConfig::new(AnalysisMode::FixedTime, 2500), None);
    assert_eq!(go_time, "go time 2500");
    assert_eq!(
        ucci.format_button_option("Clear Hash"),
        "setoption Clear Hash"
    );

    // UCCI 直接分值
    let data = ucci
        .parse_info("info depth 12 score 42 time 180 nodes 50000 nps 270000 pv h2e2 h9g7")
        .unwrap();
    assert_eq!(data.depth, 12);
    assert_eq!(data.score, 42);
    assert_eq!(data.time_ms, 180);
    assert_eq!(data.pv, vec!["h2e2", "h9g7"]);

    // UCCI nobestmove
    assert_eq!(ucci.parse_best_move("nobestmove"), None);
    assert_eq!(
        ucci.parse_best_move("bestmove h2e2").as_deref(),
        Some("h2e2")
    );
}
