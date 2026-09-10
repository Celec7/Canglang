use canglang_app::core::board::BoardState;
use canglang_app::core::piece::{Color, Piece, PieceKind};
use canglang_app::core::position::{Move, Position};
use canglang_app::core::rules::{MoveEffect, MoveValidator};

fn red(kind: PieceKind) -> Piece {
    Piece::new(Color::Red, kind)
}

#[test]
fn initial_position_standard_moves_are_legal() {
    let board = BoardState::initial();

    // 炮二平五 (7, 7) -> (7, 4)
    assert!(MoveValidator::can_move(
        &board,
        Move::from_iccs("h2e2").unwrap()
    ));

    // 馬八進七 (9, 1) -> (7, 2)
    assert!(MoveValidator::can_move(
        &board,
        Move::from_iccs("b0c2").unwrap()
    ));

    // 兵七進一 (6, 2) -> (5, 2)
    assert!(MoveValidator::can_move(
        &board,
        Move::from_iccs("c3c4").unwrap()
    ));

    // 相七進五 (9, 2) -> (7, 4) 象眼 (8, 3) 空
    assert!(MoveValidator::can_move(
        &board,
        Move::from_iccs("c0e2").unwrap()
    ));

    // 車一進一 (9, 8) -> (8, 8)
    assert!(MoveValidator::can_move(
        &board,
        Move::from_iccs("i0i1").unwrap()
    ));
}

#[test]
fn knight_leg_blocked_is_illegal() {
    let mut board = BoardState::initial();

    // 在 (8, 1) 放置己方兵蹩马腿
    board.set_piece(Position::new(8, 1), Some(red(PieceKind::Pawn)));

    assert!(!MoveValidator::can_move(
        &board,
        Move::from_iccs("b0a2").unwrap()
    ));
    assert!(!MoveValidator::can_move(
        &board,
        Move::from_iccs("b0c2").unwrap()
    ));
}

#[test]
fn elephant_eye_blocked_is_illegal() {
    let mut board = BoardState::initial();

    // 红相 (9, 2) 跳 (7, 4)，象眼为 (8, 3)，放置黑卒塞象眼
    board.set_piece(
        Position::new(8, 3),
        Some(Piece::new(Color::Black, PieceKind::Pawn)),
    );

    assert!(!MoveValidator::can_move(
        &board,
        Move::from_iccs("c0e2").unwrap()
    ));
}

#[test]
fn elephant_cannot_cross_river() {
    // 相在 (5, 2)，试图跳 (3, 4) 过河
    let fen = "4k4/9/9/9/9/2B6/9/9/9/4K4 w";
    let board = BoardState::from_fen(fen).unwrap();

    let cross_river_move = Move::new(Position::new(5, 2), Position::new(3, 4));
    assert!(!MoveValidator::can_move(&board, cross_river_move));
}

#[test]
fn kings_facing_each_other_is_illegal() {
    // 红帅从 (9, 3) 走到 (9, 4) 将与黑将 (0, 4) 直接照面
    let fen = "4k4/9/9/9/9/9/9/9/9/3K5 w";
    let board = BoardState::from_fen(fen).unwrap();

    let mv = Move::new(Position::new(9, 3), Position::new(9, 4));
    assert!(!MoveValidator::can_move(&board, mv));
}

#[test]
fn moving_pinned_piece_exposing_king_is_illegal() {
    // 黑将位于 (1, 3)，黑车在 (0, 4)，中路仅红炮在 (5, 4)，红帅在 (9, 4)
    let fen = "4r4/3k5/9/9/9/4C4/9/9/9/4K4 w";
    let board = BoardState::from_fen(fen).unwrap();

    // 红炮横移暴露红帅给黑车，非法
    let move_sideways = Move::new(Position::new(5, 4), Position::new(5, 0));
    assert!(!MoveValidator::can_move(&board, move_sideways));

    // 红炮沿该列竖移仍阻挡黑车，合法
    let move_along_file = Move::new(Position::new(5, 4), Position::new(6, 4));
    assert!(MoveValidator::can_move(&board, move_along_file));
}

#[test]
fn cannot_capture_the_enemy_king_directly() {
    let fen = "4k4/4R4/9/9/9/9/9/9/9/4K4 w";
    let board = BoardState::from_fen(fen).unwrap();

    assert!(!MoveValidator::can_move(
        &board,
        Move::new(Position::new(1, 4), Position::new(0, 4))
    ));
}

#[test]
fn cannon_capture_requires_exactly_one_screen() {
    let fen = "4k4/9/9/4p4/4P4/4C4/9/9/9/4K4 w";
    let mut board = BoardState::from_fen(fen).unwrap();
    let capture = Move::new(Position::new(5, 4), Position::new(3, 4));

    assert!(MoveValidator::can_move(&board, capture));

    board.set_piece(Position::new(4, 4), None);
    assert!(!MoveValidator::can_move(&board, capture));
}

#[test]
fn facing_kings_are_representable_but_fail_full_board_validation() {
    let fen = "4k4/9/9/9/9/9/9/9/9/4K4 w";
    let board = BoardState::from_fen(fen).unwrap();

    assert!(MoveValidator::is_in_check(&board, Color::Red));
    assert!(MoveValidator::is_in_check(&board, Color::Black));
    assert!(MoveValidator::validate_board(&board).is_err());
}

#[test]
fn pawn_movement_before_and_after_river() {
    // 红兵在河边 (5, 4)：仍未过河，只能向前，不能横移
    let fen = "3k5/9/9/9/9/4P4/9/9/9/5K3 w";
    let board = BoardState::from_fen(fen).unwrap();
    assert!(MoveValidator::can_move(
        &board,
        Move::new(Position::new(5, 4), Position::new(4, 4))
    ));
    assert!(!MoveValidator::can_move(
        &board,
        Move::new(Position::new(5, 4), Position::new(5, 5))
    ));

    // 黑卒在河边 (4, 4)：仍未过河，只能向前，不能横移
    let black_bank_fen = "3k5/9/9/9/4p4/9/9/9/9/5K3 b";
    let black_bank_board = BoardState::from_fen(black_bank_fen).unwrap();
    assert!(MoveValidator::can_move(
        &black_bank_board,
        Move::new(Position::new(4, 4), Position::new(5, 4))
    ));
    assert!(!MoveValidator::can_move(
        &black_bank_board,
        Move::new(Position::new(4, 4), Position::new(4, 5))
    ));

    // 已过河红兵在 (4, 0)：可前进、可向右横移，不能后退
    let crossed_fen = "3k5/9/9/9/P8/9/9/9/9/5K3 w";
    let crossed_board = BoardState::from_fen(crossed_fen).unwrap();
    assert!(MoveValidator::can_move(
        &crossed_board,
        Move::new(Position::new(4, 0), Position::new(3, 0))
    ));
    assert!(MoveValidator::can_move(
        &crossed_board,
        Move::new(Position::new(4, 0), Position::new(4, 1))
    ));
    assert!(!MoveValidator::can_move(
        &crossed_board,
        Move::new(Position::new(4, 0), Position::new(5, 0))
    ));
}

#[test]
fn legal_move_generation_includes_captures() {
    let fen = "3k5/9/9/9/9/4p4/4R4/9/9/5K3 w";
    let board = BoardState::from_fen(fen).unwrap();
    let moves = MoveValidator::get_legal_moves(&board, Position::new(6, 4));

    assert!(moves.contains(&Move::new(Position::new(6, 4), Position::new(5, 4))));
}

#[test]
fn is_in_check_and_is_checkmate_detected() {
    // 双車错绝杀局面：黑将在 (0, 4)，红車锁喉 + 红帅照面封路
    let fen = "3Rk4/9/9/9/9/9/9/9/3R5/4K4 b";
    let board = BoardState::from_fen(fen).unwrap();

    assert!(MoveValidator::is_in_check(&board, Color::Black));
    assert!(MoveValidator::is_checkmate(&board, Color::Black));
}

#[test]
fn get_legal_moves_returns_expected_moves() {
    let board = BoardState::initial();

    // 红炮二 (7, 7)
    let moves = MoveValidator::get_legal_moves(&board, Position::new(7, 7));

    // 炮可平走至 (7, 6), (7, 5), (7, 4), (7, 3), (7, 2)，亦可垂直进退
    assert!(!moves.is_empty());
    assert!(moves.contains(&Move::new(Position::new(7, 7), Position::new(7, 4))));
}

#[test]
fn stalemate_is_detected_when_not_in_check_and_no_legal_move() {
    // 困毙局面：黑将 (0, 3) 未被将军，但无合法着法
    let fen = "3k5/4P4/4K4/9/9/9/9/9/9/9 b";
    let board = BoardState::from_fen(fen).unwrap();

    assert!(!MoveValidator::is_in_check(&board, Color::Black));
    assert!(!MoveValidator::has_any_legal_move(&board, Color::Black));
    assert!(MoveValidator::is_stalemate(&board, Color::Black));
    // 困毙不是绝杀：未被将军
    assert!(!MoveValidator::is_checkmate(&board, Color::Black));
}

#[test]
fn all_initial_legal_moves_are_actually_legal() {
    let board = BoardState::initial();
    let legal_moves = MoveValidator::get_all_legal_moves(&board);

    assert!(!legal_moves.is_empty());
    for mv in &legal_moves {
        assert!(
            MoveValidator::can_move(&board, *mv),
            "illegal move {}",
            mv.to_iccs()
        );
    }
}

#[test]
fn test_user_fen_is_not_red_win() {
    let fen = "4ka3/4a3c/3nb4/p8/4NRb2/9/P3P4/2C1BA3/5K3/2BA3r1 b";
    let board = BoardState::from_fen(fen).expect("parse fen");
    let state = canglang_app::core::game::XiangqiGame::new(board);
    assert_eq!(
        state.result(),
        canglang_app::core::game::GameResult::Ongoing
    );
    assert!(!MoveValidator::is_in_check(&board, Color::Black));
    assert!(!MoveValidator::is_checkmate(&board, Color::Black));
    assert!(!MoveValidator::is_stalemate(&board, Color::Black));

    // 局面：红帅在 (8, 5) f1，红仕在 (9, 3) d0，红相在 (9, 2) c0
    // 黑车从 (8, 7) h1 走到 (9, 7) h0，产生对红仕、红相的攻击，按规则一律按“闲”论，不算“捉”
    let before_fen = "4ka3/4a3c/3nb4/p8/4NRb2/9/P3P4/2C1BA3/5K1r1/2BA5 b";
    let board_before = BoardState::from_fen(before_fen).expect("parse fen");
    let mv = Move::from_iccs("h1h0").unwrap();
    let classification =
        canglang_app::core::rules::effects::classify_move_details(&board_before, mv);
    assert_eq!(classification.effect, MoveEffect::Idle);
    assert!(classification.chase_targets.is_empty());
}

#[test]
fn validate_board_rejects_side_not_to_move_in_check() {
    // 轮到红方走棋 (w)，但黑将已被红车将军 (0, 4) vs (1, 4) 同列无阻隔
    let fen = "4k4/4R4/9/9/9/9/9/9/9/4K4 w";
    let board = BoardState::from_fen(fen).unwrap();
    assert!(MoveValidator::validate_board(&board).is_err());

    // 轮到黑方走棋 (b)，黑将被红车将军，此时是合法的（黑方应将状态）
    let fen_legal = "4k4/4R4/9/9/9/9/9/9/9/4K4 b";
    let board_legal = BoardState::from_fen(fen_legal).unwrap();
    assert!(MoveValidator::validate_board(&board_legal).is_ok());
}
