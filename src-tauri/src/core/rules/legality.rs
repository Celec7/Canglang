use crate::core::CoreError;
use crate::core::board::{BoardState, TOTAL_SQUARES};
use crate::core::piece::{Color, Piece, PieceKind};
use crate::core::position::{Move, Position};

use super::MoveValidator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeometryPolicy {
    Xiangqi,
    JieqiRevealed,
}

pub(crate) fn can_piece_move_geometry(
    color: Color,
    kind: PieceKind,
    from: Position,
    to: Position,
    policy: GeometryPolicy,
    is_occupied: impl Fn(Position) -> bool,
) -> bool {
    let dr = to.row as i32 - from.row as i32;
    let dc = to.col as i32 - from.col as i32;
    let abs_dr = dr.abs();
    let abs_dc = dc.abs();

    match kind {
        PieceKind::King => {
            in_palace(color, to) && ((abs_dr == 1 && abs_dc == 0) || (abs_dr == 0 && abs_dc == 1))
        }
        PieceKind::Advisor => {
            (policy == GeometryPolicy::JieqiRevealed || in_palace(color, to))
                && abs_dr == 1
                && abs_dc == 1
        }
        PieceKind::Bishop => {
            if (policy != GeometryPolicy::JieqiRevealed && !on_own_side(color, to))
                || abs_dr != 2
                || abs_dc != 2
            {
                return false;
            }
            let eye = Position::new((from.row + to.row) / 2, (from.col + to.col) / 2);
            !is_occupied(eye)
        }
        PieceKind::Knight => {
            if !((abs_dr == 2 && abs_dc == 1) || (abs_dr == 1 && abs_dc == 2)) {
                return false;
            }
            let leg = Position::new(
                (from.row as i32 + if abs_dr == 2 { dr.signum() } else { 0 }) as u8,
                (from.col as i32 + if abs_dc == 2 { dc.signum() } else { 0 }) as u8,
            );
            !is_occupied(leg)
        }
        PieceKind::Rook => {
            (abs_dr == 0 || abs_dc == 0) && count_intervening_geometry(from, to, &is_occupied) == 0
        }
        PieceKind::Cannon => {
            if abs_dr != 0 && abs_dc != 0 {
                return false;
            }
            let screens = count_intervening_geometry(from, to, &is_occupied);
            if is_occupied(to) {
                screens == 1
            } else {
                screens == 0
            }
        }
        PieceKind::Pawn => match color {
            Color::Red if from.row >= 5 => dr == -1 && dc == 0,
            Color::Red => (dr == -1 && dc == 0) || (dr == 0 && abs_dc == 1),
            Color::Black if from.row <= 4 => dr == 1 && dc == 0,
            Color::Black => (dr == 1 && dc == 0) || (dr == 0 && abs_dc == 1),
        },
    }
}

fn count_intervening_geometry(
    from: Position,
    to: Position,
    is_occupied: &impl Fn(Position) -> bool,
) -> usize {
    if from.row == to.row {
        return (from.col.min(to.col) + 1..from.col.max(to.col))
            .filter(|col| is_occupied(Position::new(from.row, *col)))
            .count();
    }
    if from.col == to.col {
        return (from.row.min(to.row) + 1..from.row.max(to.row))
            .filter(|row| is_occupied(Position::new(*row, from.col)))
            .count();
    }
    usize::MAX
}

fn in_palace(color: Color, position: Position) -> bool {
    (3..=5).contains(&position.col)
        && if color.is_red() {
            (7..=9).contains(&position.row)
        } else {
            (0..=2).contains(&position.row)
        }
}

fn on_own_side(color: Color, position: Position) -> bool {
    if color.is_red() {
        (5..=9).contains(&position.row)
    } else {
        (0..=4).contains(&position.row)
    }
}

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
        can_piece_move_geometry(
            piece.color,
            piece.kind,
            from,
            to,
            GeometryPolicy::Xiangqi,
            |position| board.get_piece(position).is_some(),
        )
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
