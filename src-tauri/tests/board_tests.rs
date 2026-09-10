use canglang_app::core::board::{BoardState, INITIAL_FEN};
use canglang_app::core::piece::{Color, Piece, PieceKind};
use canglang_app::core::position::{Move, Position};

fn red(kind: PieceKind) -> Piece {
    Piece::new(Color::Red, kind)
}

fn black(kind: PieceKind) -> Piece {
    Piece::new(Color::Black, kind)
}

#[test]
fn from_fen_initial_position_parses_pieces_correctly() {
    let board = BoardState::from_fen(INITIAL_FEN).unwrap();
    assert!(board.turn.is_red());

    // 黑方底线
    assert_eq!(
        board.get_piece(Position::new(0, 0)),
        Some(black(PieceKind::Rook))
    );
    assert_eq!(
        board.get_piece(Position::new(0, 1)),
        Some(black(PieceKind::Knight))
    );
    assert_eq!(
        board.get_piece(Position::new(0, 2)),
        Some(black(PieceKind::Bishop))
    );
    assert_eq!(
        board.get_piece(Position::new(0, 3)),
        Some(black(PieceKind::Advisor))
    );
    assert_eq!(
        board.get_piece(Position::new(0, 4)),
        Some(black(PieceKind::King))
    );
    assert_eq!(
        board.get_piece(Position::new(0, 5)),
        Some(black(PieceKind::Advisor))
    );
    assert_eq!(
        board.get_piece(Position::new(0, 6)),
        Some(black(PieceKind::Bishop))
    );
    assert_eq!(
        board.get_piece(Position::new(0, 7)),
        Some(black(PieceKind::Knight))
    );
    assert_eq!(
        board.get_piece(Position::new(0, 8)),
        Some(black(PieceKind::Rook))
    );

    // 黑炮
    assert_eq!(
        board.get_piece(Position::new(2, 1)),
        Some(black(PieceKind::Cannon))
    );
    assert_eq!(
        board.get_piece(Position::new(2, 7)),
        Some(black(PieceKind::Cannon))
    );

    // 黑卒
    for col in [0, 2, 4, 6, 8] {
        assert_eq!(
            board.get_piece(Position::new(3, col)),
            Some(black(PieceKind::Pawn))
        );
    }

    // 空格
    assert_eq!(board.get_piece(Position::new(1, 0)), None);
    assert_eq!(board.get_piece(Position::new(4, 4)), None);
    assert_eq!(board.get_piece(Position::new(5, 4)), None);

    // 红兵
    for col in [0, 2, 4, 6, 8] {
        assert_eq!(
            board.get_piece(Position::new(6, col)),
            Some(red(PieceKind::Pawn))
        );
    }

    // 红炮
    assert_eq!(
        board.get_piece(Position::new(7, 1)),
        Some(red(PieceKind::Cannon))
    );
    assert_eq!(
        board.get_piece(Position::new(7, 7)),
        Some(red(PieceKind::Cannon))
    );

    // 红方底线
    assert_eq!(
        board.get_piece(Position::new(9, 0)),
        Some(red(PieceKind::Rook))
    );
    assert_eq!(
        board.get_piece(Position::new(9, 1)),
        Some(red(PieceKind::Knight))
    );
    assert_eq!(
        board.get_piece(Position::new(9, 2)),
        Some(red(PieceKind::Bishop))
    );
    assert_eq!(
        board.get_piece(Position::new(9, 3)),
        Some(red(PieceKind::Advisor))
    );
    assert_eq!(
        board.get_piece(Position::new(9, 4)),
        Some(red(PieceKind::King))
    );
    assert_eq!(
        board.get_piece(Position::new(9, 5)),
        Some(red(PieceKind::Advisor))
    );
    assert_eq!(
        board.get_piece(Position::new(9, 6)),
        Some(red(PieceKind::Bishop))
    );
    assert_eq!(
        board.get_piece(Position::new(9, 7)),
        Some(red(PieceKind::Knight))
    );
    assert_eq!(
        board.get_piece(Position::new(9, 8)),
        Some(red(PieceKind::Rook))
    );
}

#[test]
fn to_fen_initial_position_matches_original_fen() {
    let board = BoardState::from_fen(INITIAL_FEN).unwrap();
    assert_eq!(board.to_fen(), INITIAL_FEN);
}

#[test]
fn apply_and_undo_move_executed_and_reverted() {
    let board = BoardState::initial();
    // 红炮二平五 (7, 7) -> (7, 4)
    let mv = Move::new(Position::new(7, 7), Position::new(7, 4));

    let (next_board, captured) = board.apply_move(mv);

    assert_eq!(next_board.get_piece(Position::new(7, 7)), None);
    assert_eq!(
        next_board.get_piece(Position::new(7, 4)),
        Some(red(PieceKind::Cannon))
    );
    assert!(next_board.turn.is_black());
    assert_eq!(captured, None);

    // 原棋盘未变（Copy 语义）
    assert_eq!(
        board.get_piece(Position::new(7, 7)),
        Some(red(PieceKind::Cannon))
    );
    assert_eq!(board.get_piece(Position::new(7, 4)), None);
    assert!(board.turn.is_red());

    // 撤销
    let reverted_board = next_board.undo_move(mv, captured);
    assert_eq!(
        reverted_board.get_piece(Position::new(7, 7)),
        Some(red(PieceKind::Cannon))
    );
    assert_eq!(reverted_board.get_piece(Position::new(7, 4)), None);
    assert!(reverted_board.turn.is_red());
}

#[test]
fn fen_roundtrip_of_non_initial_position_is_stable() {
    // 构造一个非对称局面，验证 FEN 导出后再导入完全一致
    let fen = "4k4/9/9/9/9/R8/9/9/9/4K4 w";
    let board = BoardState::from_fen(fen).unwrap();
    assert_eq!(board.to_fen(), fen);

    let reparsed = BoardState::from_fen(&board.to_fen()).unwrap();
    assert_eq!(reparsed, board);
}

#[test]
fn from_fen_relative_turn_markers_are_accepted() {
    // 兼容 'w' / 'r' / 'b' 走方标识
    assert!(
        BoardState::from_fen("4k4/9/9/9/9/9/9/9/9/4K4 w")
            .unwrap()
            .turn
            .is_red()
    );
    assert!(
        BoardState::from_fen("4k4/9/9/9/9/9/9/9/9/4K4 r")
            .unwrap()
            .turn
            .is_red()
    );
    assert!(
        BoardState::from_fen("4k4/9/9/9/9/9/9/9/9/4K4 b")
            .unwrap()
            .turn
            .is_black()
    );
}

#[test]
fn from_fen_accepts_standard_six_part_fen() {
    let six_part = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";
    let board = BoardState::from_fen(six_part).unwrap();
    assert!(board.turn.is_red());
    assert_eq!(board.to_fen(), INITIAL_FEN);
}

#[test]
fn from_fen_invalid_fen_returns_error() {
    let invalid_fens = [
        "",
        "   ",
        "invalid/fen",
        "4k4/9/9/9/9/9/9/9/9/4K4 x",
        "4k4/9/9/9/9/9/9/9/9/4K4 w extra1 extra2 extra3 extra4 extra5",
        "4k4/9/9/9/9/9/9/9/9/4K4 0",
        "4k4/0/9/9/9/9/9/9/9/4K4 w",
        "4k4/9/9/9/9/9/9/9/4K4/4K4 w",
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR/extra w",
        // 行数不足（只有 9 行）
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9 w",
        // 某行列数为 10（超过 9）
        "rnbakabnrr/9/9/9/9/9/9/9/9/RNBAKABNR w",
        // 非法字符
        "rnbakabnr/9/9/9/9/9/9/9/9/RNBAKABXA w",
    ];

    for fen in invalid_fens {
        assert!(BoardState::from_fen(fen).is_err(), "should reject: {fen}");
    }
}
