use crate::core::board::{BoardState, COL_COUNT, TOTAL_SQUARES};
use crate::core::piece::Color;
use crate::core::position::Position;

use super::MoveValidator;

impl MoveValidator {
    /// 检测指定方（color）的将/帅当前是否正处于受攻击（被将军）状态
    pub fn is_in_check(board: &BoardState, color: Color) -> bool {
        let king_pos = match board.find_king(color) {
            Some(p) => p,
            None => return false,
        };

        if let Some(opp_king_pos) = board.find_king(color.opposite())
            && opp_king_pos.col == king_pos.col
            && Self::count_intervening_pieces(board, king_pos, opp_king_pos) == 0
        {
            return true;
        }

        let opp_color = color.opposite();
        for i in 0..TOTAL_SQUARES {
            if let Some(piece) = board.pieces[i]
                && piece.color == opp_color
            {
                let from = Position::from_index(i);
                if Self::can_piece_move_pseudo(board, piece, from, king_pos) {
                    return true;
                }
            }
        }

        false
    }

    /// 计算两格之间（同一行或同一列）的夹子数量（不含端点）
    pub fn count_intervening_pieces(board: &BoardState, p1: Position, p2: Position) -> usize {
        let mut count = 0;
        if p1.row == p2.row {
            let min_col = p1.col.min(p2.col) as usize;
            let max_col = p1.col.max(p2.col) as usize;
            for c in (min_col + 1)..max_col {
                if board.pieces[(p1.row as usize) * COL_COUNT + c].is_some() {
                    count += 1;
                }
            }
        } else if p1.col == p2.col {
            let min_row = p1.row.min(p2.row) as usize;
            let max_row = p1.row.max(p2.row) as usize;
            for r in (min_row + 1)..max_row {
                if board.pieces[r * COL_COUNT + (p1.col as usize)].is_some() {
                    count += 1;
                }
            }
        }
        count
    }
}
