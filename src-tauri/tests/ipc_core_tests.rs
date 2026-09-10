// 无状态核心规则 IPC 命令的集成测试
//
// 这些命令是基于 FEN/ICCS 字符串的纯函数，无需 Tauri 运行时即可直接测试
// 有状态命令（make_move、会话命令、引擎与开局库命令）由更完整的 Rust 模块测试覆盖

use canglang_app::core::rules::RuleProfile;
use canglang_app::ipc::core::{
    PreviewRequest, get_initial_board, get_legal_moves, is_in_check, parse_fen, preview_line,
    to_chinese_notation,
};

const INITIAL: &str = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w";

#[test]
fn get_initial_board_returns_canonical_fen() {
    assert_eq!(get_initial_board(), INITIAL);
}

#[test]
fn parse_fen_validates_and_canonicalizes() {
    assert_eq!(parse_fen(INITIAL.to_string()).unwrap(), INITIAL);
    assert!(parse_fen("not a fen".to_string()).is_err());
    assert!(parse_fen(String::new()).is_err());
}

#[test]
fn get_legal_moves_initial_is_44() {
    assert_eq!(get_legal_moves(get_initial_board()).unwrap().len(), 44);
}

#[test]
fn to_chinese_notation_pao_ping_wu() {
    assert_eq!(
        to_chinese_notation(get_initial_board(), "h2e2".to_string()).unwrap(),
        "炮二平五"
    );
}

#[test]
fn is_in_check_initial_is_false() {
    assert!(!is_in_check(get_initial_board()).unwrap());
}

#[test]
fn invalid_fen_is_not_coerced_to_an_empty_result() {
    assert!(get_legal_moves("not a fen".to_string()).is_err());
    assert!(is_in_check("not a fen".to_string()).is_err());
}

#[test]
fn preview_line_replays_history_and_pv_without_state() {
    let snapshot = preview_line(PreviewRequest {
        start_fen: INITIAL.to_string(),
        history: vec!["h2e2".to_string()],
        pv: vec!["h9g7".to_string()],
        rule_profile: RuleProfile::China2020,
    })
    .unwrap();

    assert_eq!(snapshot.last_move_iccs.as_deref(), Some("h9g7"));
    assert_eq!(snapshot.applied_pv_len, 1);
    assert_ne!(snapshot.fen, INITIAL);
}

#[test]
fn preview_line_rejects_illegal_pv() {
    let result = preview_line(PreviewRequest {
        start_fen: INITIAL.to_string(),
        history: Vec::new(),
        pv: vec!["a0a9".to_string()],
        rule_profile: RuleProfile::China2020,
    });
    assert!(result.is_err());
}
