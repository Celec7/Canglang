use canglang_app::core::board::BoardState;
use canglang_app::core::notation::NotationConverter;
use canglang_app::core::position::{Move, Position};

#[test]
fn to_chinese_notation_standard_opening_moves() {
    let board = BoardState::initial();

    // 炮二平五 (7, 7) -> (7, 4)
    let c_move = Move::from_iccs("h2e2").unwrap();
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &c_move).unwrap(),
        "炮二平五"
    );

    // 車一進一 (9, 8) -> (8, 8)
    let r_move = Move::from_iccs("i0i1").unwrap();
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &r_move).unwrap(),
        "車一進一"
    );

    // 馬八進七 (9, 1) -> (7, 2)
    let n_move = Move::from_iccs("b0c2").unwrap();
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &n_move).unwrap(),
        "馬八進七"
    );

    // 兵七進一 (6, 2) -> (5, 2)
    let p_move = Move::from_iccs("c3c4").unwrap();
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &p_move).unwrap(),
        "兵七進一"
    );
}

#[test]
fn to_chinese_notation_black_knight_move() {
    let board = BoardState::initial();
    // 执行红方 炮二平五 后轮到黑方
    let (b1, _) = board.apply_move(Move::from_iccs("h2e2").unwrap());
    assert!(!b1.turn.is_red());

    // 黑方 馬８進７ (0, 7) -> (2, 6) "h9g7"
    let b_move = Move::from_iccs("h9g7").unwrap();
    let cn = NotationConverter::to_chinese_notation(&b1, &b_move).unwrap();
    assert_eq!(cn, "馬８進７");
}

#[test]
fn from_chinese_notation_resolves_correct_move() {
    let board = BoardState::initial();

    let move1 = NotationConverter::from_chinese_notation(&board, "炮二平五").unwrap();
    assert_eq!(move1.to_iccs(), "h2e2");

    let move2 = NotationConverter::from_chinese_notation(&board, "馬八進七").unwrap();
    assert_eq!(move2.to_iccs(), "b0c2");

    // 简体写法亦可通过几何解析兜底识别
    let move2_simplified = NotationConverter::from_chinese_notation(&board, "马八进七").unwrap();
    assert_eq!(move2_simplified.to_iccs(), "b0c2");

    let move3 = NotationConverter::from_chinese_notation(&board, "車一進一").unwrap();
    assert_eq!(move3.to_iccs(), "i0i1");

    // 切换至黑方走
    let (b1, _) = board.apply_move(move1);
    let black_move = NotationConverter::from_chinese_notation(&b1, "馬８進７").unwrap();
    assert_eq!(black_move.to_iccs(), "h9g7");

    // 半角支持
    let black_move_half = NotationConverter::from_chinese_notation(&b1, "馬8進7").unwrap();
    assert_eq!(black_move_half.to_iccs(), "h9g7");
}

#[test]
fn cannon_piece_name_uses_pao_for_both_sides() {
    // 红黑双方的炮均记作「炮」
    let board = BoardState::initial();
    let c_move = Move::from_iccs("h2e2").unwrap();
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &c_move).unwrap(),
        "炮二平五"
    );

    let fen = "4k4/9/2c6/9/9/9/9/9/9/5K3 b";
    let black_board = BoardState::from_fen(fen).unwrap();
    // 黑炮 (2, 2) 平至 (2, 3)
    let b_move = Move::new(Position::new(2, 2), Position::new(2, 3));
    let cn = NotationConverter::to_chinese_notation(&black_board, &b_move).unwrap();
    assert_eq!(cn, "炮３平４");
}

#[test]
fn disambiguation_double_rooks_front_and_back() {
    // 设置红方在 0 列（九路）有两个車：(5, 0) 和 (8, 0)
    let fen = "4k4/9/9/9/9/R8/9/9/R8/4K4 w";
    let board = BoardState::from_fen(fen).unwrap();

    // 前車 (5, 0) 進一至 (4, 0)
    let front_move = Move::new(Position::new(5, 0), Position::new(4, 0));
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &front_move).unwrap(),
        "前車進一"
    );

    // 后車 (8, 0) 進一至 (7, 0)
    let back_move = Move::new(Position::new(8, 0), Position::new(7, 0));
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &back_move).unwrap(),
        "后車進一"
    );

    // 反向解析
    assert_eq!(
        NotationConverter::from_chinese_notation(&board, "前車進一").unwrap(),
        front_move
    );
    assert_eq!(
        NotationConverter::from_chinese_notation(&board, "后車進一").unwrap(),
        back_move
    );
}

#[test]
fn disambiguation_three_pawns_front_middle_back() {
    // 红方三兵同列（九路）：(2, 0)、(4, 0)、(6, 0)
    let fen = "3k5/9/P8/9/P8/9/P8/9/9/5K3 w";
    let board = BoardState::from_fen(fen).unwrap();

    // 前兵 (2, 0) 進一至 (1, 0)
    let front_move = Move::new(Position::new(2, 0), Position::new(1, 0));
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &front_move).unwrap(),
        "前兵進一"
    );

    // 中兵 (4, 0) 進一至 (3, 0)
    let middle_move = Move::new(Position::new(4, 0), Position::new(3, 0));
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &middle_move).unwrap(),
        "中兵進一"
    );

    // 后兵 (6, 0) 進一至 (5, 0)
    let back_move = Move::new(Position::new(6, 0), Position::new(5, 0));
    assert_eq!(
        NotationConverter::to_chinese_notation(&board, &back_move).unwrap(),
        "后兵進一"
    );

    // 反向解析全部可还原
    assert_eq!(
        NotationConverter::from_chinese_notation(&board, "前兵進一").unwrap(),
        front_move
    );
    assert_eq!(
        NotationConverter::from_chinese_notation(&board, "中兵進一").unwrap(),
        middle_move
    );
    assert_eq!(
        NotationConverter::from_chinese_notation(&board, "后兵進一").unwrap(),
        back_move
    );
}

#[test]
fn deduce_pv_chinese() {
    let board = BoardState::initial();
    let pv = vec!["h2e2".to_string(), "h9g7".to_string(), "b0c2".to_string()];

    let result = NotationConverter::deduce_pv_chinese(&board, &pv);
    assert_eq!(result, vec!["炮二平五", "馬８進７", "馬八進七"]);
}
