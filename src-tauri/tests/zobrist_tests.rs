use canglang_app::core::board::BoardState;
use canglang_app::core::hash::ZobristHasher;
use canglang_app::core::piece::Piece;
use canglang_app::core::position::{Move, Position};

const EXPECTED_INITIAL_RED_HASH: u64 = 0x628D04D7C9C144AE;
const EXPECTED_INITIAL_BLACK_HASH: u64 = 0xC2432E2EC5846BF6;

#[test]
fn compute_initial_board_matches_benchmark_values() {
    let board = BoardState::initial();

    let normal_red = ZobristHasher::compute_with_turn(&board, true, false);
    let mirror_red = ZobristHasher::compute_with_turn(&board, true, true);
    assert_eq!(normal_red, EXPECTED_INITIAL_RED_HASH);
    assert_eq!(mirror_red, EXPECTED_INITIAL_RED_HASH);

    let normal_black = ZobristHasher::compute_with_turn(&board, false, false);
    let mirror_black = ZobristHasher::compute_with_turn(&board, false, true);
    assert_eq!(normal_black, EXPECTED_INITIAL_BLACK_HASH);
    assert_eq!(mirror_black, EXPECTED_INITIAL_BLACK_HASH);
}

#[test]
fn compute_asymmetric_position_mirrored_differs() {
    let board = BoardState::initial();
    // 炮二平五 打破棋盘左右对称
    let (next_board, _) = board.apply_move(Move::from_iccs("h2e2").unwrap());

    let normal = ZobristHasher::compute(&next_board, false);
    let mirror = ZobristHasher::compute(&next_board, true);
    assert_ne!(normal, mirror);
}

#[test]
fn update_matches_compute_for_non_capture_move() {
    let board = BoardState::initial();
    let mv = Move::from_iccs("h2e2").unwrap();
    let (next_board, _) = board.apply_move(mv);

    let initial_hash = ZobristHasher::compute(&board, false);
    let moving_piece = board.get_piece(mv.from).unwrap();
    let updated_hash =
        ZobristHasher::update(initial_hash, mv.from, mv.to, moving_piece, None, false);
    let computed_hash = ZobristHasher::compute(&next_board, false);

    assert_eq!(computed_hash, updated_hash);
}

#[test]
fn update_matches_compute_for_capture_move() {
    // 设置車吃卒局面
    let fen = "4k4/9/9/9/9/4p4/4R4/9/9/4K4 w";
    let board = BoardState::from_fen(fen).unwrap();

    // 红车 (6, 4) 进一吃黑卒 (5, 4)
    let mv = Move::new(Position::new(6, 4), Position::new(5, 4));
    let (next_board, captured) = board.apply_move(mv);

    let current_hash = ZobristHasher::compute(&board, false);
    let moving_piece = board.get_piece(mv.from).unwrap();
    let updated_hash =
        ZobristHasher::update(current_hash, mv.from, mv.to, moving_piece, captured, false);
    let computed_hash = ZobristHasher::compute(&next_board, false);

    assert_eq!(computed_hash, updated_hash);
}

#[test]
fn update_handles_mirrored_as_well() {
    // 验证镜像增量与全盘镜像重算一致
    let board = BoardState::initial();
    let mv = Move::from_iccs("h2e2").unwrap();
    let (next_board, _) = board.apply_move(mv);

    let initial_hash = ZobristHasher::compute(&board, true);
    let moving_piece = board.get_piece(mv.from).unwrap();
    let updated_hash =
        ZobristHasher::update(initial_hash, mv.from, mv.to, moving_piece, None, true);
    let computed_hash = ZobristHasher::compute(&next_board, true);

    assert_eq!(computed_hash, updated_hash);
}

#[test]
fn get_move_from_vmove_matches_coordinates() {
    // 首字节为 0xc3（a0），次字节为 0xb3（a1）
    let vmove: u16 = (0xc3 << 8) | 0xb3;

    let normal = ZobristHasher::get_move_from_vmove(vmove, false).unwrap();
    assert_eq!(normal.to_iccs(), "a0a1");

    let mirrored = ZobristHasher::get_move_from_vmove(vmove, true).unwrap();
    assert_eq!(mirrored.to_iccs(), "i0i1");
}

#[test]
fn moving_piece_identity_mismatch_detects_via_piece() {
    // 增量更新使用的棋子即走子方棋子本身
    let board = BoardState::initial();
    let mv = Move::from_iccs("h2e2").unwrap();
    let moving_piece: Piece = board.get_piece(mv.from).unwrap();

    assert!(moving_piece.is_red());
}
