use crate::core::CoreError;
use crate::core::board::{BoardState, TOTAL_SQUARES};
use crate::core::piece::{Color, Piece, PieceKind};
use crate::core::position::{Move, Position};

use super::MoveValidator;

impl MoveValidator {
    /// 判断指定走法在当前棋盘上是否完全合法
    pub fn can_move(board: &BoardState, mv: Move) -> bool {
        if !mv.from.is_valid() || !mv.to.is_valid() || mv.from == mv.to {
            return false;
        }

        let piece = match board.get_piece(mv.from) {
            Some(p) => p,
            None => return false,
        };

        if piece.color != board.turn {
            return false;
        }

        if let Some(target) = board.get_piece(mv.to)
            && (target.color == piece.color || target.kind == PieceKind::King)
        {
            return false;
        }

        if !Self::can_piece_move_pseudo(board, piece, mv.from, mv.to) {
            return false;
        }

        let (next_board, _) = board.apply_move(mv);
        !Self::is_in_check(&next_board, board.turn)
    }

    /// 校验棋子几何走法，不包含走后己方是否被将军
    pub fn can_piece_move_pseudo(
        board: &BoardState,
        piece: Piece,
        from: Position,
        to: Position,
    ) -> bool {
        let dr = to.row as i32 - from.row as i32;
        let dc = to.col as i32 - from.col as i32;
        let abs_dr = dr.abs();
        let abs_dc = dc.abs();

        match (piece.color, piece.kind) {
            (Color::Red, PieceKind::King) | (Color::Black, PieceKind::King) => {
                let in_palace = if piece.color.is_red() {
                    (7..=9).contains(&to.row)
                } else {
                    (0..=2).contains(&to.row)
                } && (3..=5).contains(&to.col);
                in_palace && ((abs_dr == 1 && abs_dc == 0) || (abs_dr == 0 && abs_dc == 1))
            }
            (Color::Red, PieceKind::Advisor) | (Color::Black, PieceKind::Advisor) => {
                let in_palace = if piece.color.is_red() {
                    (7..=9).contains(&to.row)
                } else {
                    (0..=2).contains(&to.row)
                } && (3..=5).contains(&to.col);
                in_palace && abs_dr == 1 && abs_dc == 1
            }
            (Color::Red, PieceKind::Bishop) | (Color::Black, PieceKind::Bishop) => {
                let on_own_side = if piece.color.is_red() {
                    (5..=9).contains(&to.row)
                } else {
                    (0..=4).contains(&to.row)
                };
                if !on_own_side || abs_dr != 2 || abs_dc != 2 {
                    return false;
                }
                let eye = Position::new((from.row + to.row) / 2, (from.col + to.col) / 2);
                board.get_piece(eye).is_none()
            }
            (_, PieceKind::Knight) => {
                if !((abs_dr == 2 && abs_dc == 1) || (abs_dr == 1 && abs_dc == 2)) {
                    return false;
                }
                let leg_row = (from.row as i32 + if abs_dr == 2 { dr.signum() } else { 0 }) as u8;
                let leg_col = (from.col as i32 + if abs_dc == 2 { dc.signum() } else { 0 }) as u8;
                board.get_piece(Position::new(leg_row, leg_col)).is_none()
            }
            (_, PieceKind::Rook) => {
                if abs_dr != 0 && abs_dc != 0 {
                    return false;
                }
                Self::count_intervening_pieces(board, from, to) == 0
            }
            (_, PieceKind::Cannon) => {
                if abs_dr != 0 && abs_dc != 0 {
                    return false;
                }
                let count = Self::count_intervening_pieces(board, from, to);
                if board.get_piece(to).is_none() {
                    count == 0
                } else {
                    count == 1
                }
            }
            (Color::Red, PieceKind::Pawn) => {
                // 河界位于第 4、5 行之间；红兵落到第 4 行后才算过河
                if from.row >= 5 {
                    dr == -1 && dc == 0
                } else {
                    (dr == -1 && dc == 0) || (dr == 0 && abs_dc == 1)
                }
            }
            (Color::Black, PieceKind::Pawn) => {
                // 河界位于第 4、5 行之间；黑卒落到第 5 行后才算过河
                if from.row <= 4 {
                    dr == 1 && dc == 0
                } else {
                    (dr == 1 && dc == 0) || (dr == 0 && abs_dc == 1)
                }
            }
        }
    }

    /// 判断指定方是否处于绝杀状态
    pub fn is_checkmate(board: &BoardState, color: Color) -> bool {
        Self::is_in_check(board, color) && !Self::has_any_legal_move(board, color)
    }

    /// 判断指定方是否处于困毙状态
    pub fn is_stalemate(board: &BoardState, color: Color) -> bool {
        !Self::is_in_check(board, color) && !Self::has_any_legal_move(board, color)
    }

    /// 校验棋盘的完整棋局结构，不改变 `BoardState::from_fen` 对诊断局面的可表示性
    pub fn validate_board(board: &BoardState) -> Result<(), CoreError> {
        let mut red_king = None;
        let mut black_king = None;
        for i in 0..TOTAL_SQUARES {
            if let Some(piece) = board.pieces[i]
                && piece.kind == PieceKind::King
            {
                let slot = if piece.color.is_red() {
                    &mut red_king
                } else {
                    &mut black_king
                };
                if slot.replace(Position::from_index(i)).is_some() {
                    return Err(CoreError::InvalidFen(
                        "Each side must have exactly one king".to_string(),
                    ));
                }
            }
        }

        let red_king =
            red_king.ok_or_else(|| CoreError::InvalidFen("Red king missing".to_string()))?;
        let black_king =
            black_king.ok_or_else(|| CoreError::InvalidFen("Black king missing".to_string()))?;

        if !(7..=9).contains(&red_king.row) || !(3..=5).contains(&red_king.col) {
            return Err(CoreError::InvalidFen("Red king not in palace".to_string()));
        }
        if !(0..=2).contains(&black_king.row) || !(3..=5).contains(&black_king.col) {
            return Err(CoreError::InvalidFen(
                "Black king not in palace".to_string(),
            ));
        }
        if red_king.col == black_king.col
            && Self::count_intervening_pieces(board, red_king, black_king) == 0
        {
            return Err(CoreError::InvalidFen(
                "Kings are facing each other directly".to_string(),
            ));
        }

        if Self::is_in_check(board, board.turn.opposite()) {
            return Err(CoreError::InvalidFen(
                "Side not to move cannot be in check".to_string(),
            ));
        }

        Ok(())
    }
}
