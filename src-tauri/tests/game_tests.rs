use canglang_app::core::board::BoardState;
use canglang_app::core::game::{GameResult, XiangqiGame};
use canglang_app::core::hash::ZobristHasher;
use canglang_app::core::piece::Color;

const EXPECTED_INITIAL_RED_HASH: u64 = 0x628D04D7C9C144AE;

#[test]
fn initial_state_is_correct() {
    let game = XiangqiGame::default();

    assert!(game.is_red_to_move());
    assert_eq!(game.result(), GameResult::Ongoing);
    assert!(game.history().is_empty());
    assert_eq!(
        ZobristHasher::compute(&game.current_board(), false),
        EXPECTED_INITIAL_RED_HASH
    );
    assert!(!game.is_in_check());
}

#[test]
fn make_move_and_undo_move_maintains_consistency() {
    let mut game = XiangqiGame::default();

    // 1. 红走 炮二平五
    assert!(game.make_move_iccs("h2e2"));
    assert!(!game.is_red_to_move());
    assert_eq!(game.history().len(), 1);
    assert_eq!(game.history()[0].chinese_notation, "炮二平五");

    // 2. 黑走 馬８進７
    assert!(game.make_move_iccs("h9g7"));
    assert!(game.is_red_to_move());
    assert_eq!(game.history().len(), 2);
    assert_eq!(game.history()[1].chinese_notation, "馬８進７");

    // 3. 尝试非法走棋（当前红走，黑馬试图走）
    assert!(!game.make_move_iccs("b9c7"));
    assert_eq!(game.history().len(), 2);

    // 4. 单步悔棋
    assert!(game.undo_move());
    assert!(!game.is_red_to_move());
    assert_eq!(game.history().len(), 1);
    assert_eq!(game.history()[0].chinese_notation, "炮二平五");

    // 5. 再次悔棋至初始状态
    assert!(game.undo_move());
    assert!(game.is_red_to_move());
    assert!(game.history().is_empty());
    assert_eq!(
        ZobristHasher::compute(&game.current_board(), false),
        EXPECTED_INITIAL_RED_HASH
    );

    // 6. 无法进一步悔棋
    assert!(!game.undo_move());
}

#[test]
fn redo_move_replays_an_undone_move() {
    let mut game = XiangqiGame::default();

    assert!(game.make_move_iccs("h2e2"));
    assert!(game.make_move_iccs("h9g7"));
    assert_eq!(game.history().len(), 2);

    // 悔棋一步
    assert!(game.undo_move());
    assert_eq!(game.history().len(), 1);

    // 重做恢复
    assert!(game.redo_move());
    assert_eq!(game.history().len(), 2);
    assert!(game.is_red_to_move());
}

#[test]
fn successful_new_move_clears_redo_history() {
    let mut game = XiangqiGame::default();

    assert!(game.make_move_iccs("h2e2"));
    assert!(game.undo_move());
    assert!(game.can_redo());

    assert!(!game.make_move_iccs("a0a0"));
    assert!(game.can_redo());

    assert!(game.make_move_iccs("h2e2"));
    assert!(!game.can_redo());
}

#[test]
fn checkmate_terminates_game_with_winner() {
    // 经典双車錯绝杀局面
    let fen = "3k5/1R7/R8/9/9/9/9/9/9/5K3 w";
    let board = BoardState::from_fen(fen).unwrap();
    let mut game = XiangqiGame::new(board);

    assert_eq!(game.result(), GameResult::Ongoing);

    // 走 "a7a9"：红車沉底将军，形成双車錯绝杀
    assert!(game.make_move_iccs("a7a9"));

    assert_eq!(game.result(), GameResult::RedWin);

    // 游戏结束后不能继续走子
    assert!(!game.make_move_iccs("d9d8"));

    // 悔棋后恢复为 ongoing 状态
    assert!(game.undo_move());
    assert_eq!(game.result(), GameResult::Ongoing);
    assert!(game.is_red_to_move());
}

#[test]
fn resign_sets_correct_result() {
    let mut game = XiangqiGame::default();
    game.resign(Color::Red);
    assert_eq!(game.result(), GameResult::BlackWin);
}
