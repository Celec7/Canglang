use super::{JieqiIdentitySource, JieqiPiece, JieqiPosition};
use crate::core::board::{COL_COUNT, ROW_COUNT};
use crate::core::piece::{Color, PieceKind};
use crate::core::position::{Move, Position};
use crate::core::rules::{GeometryPolicy, can_piece_move_geometry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JieqiMoveRejection {
    InvalidPosition,
    EmptySource,
    WrongTurn,
    FriendlyOccupied,
    CannotCaptureKing,
    InvalidGeometry,
    ExposesKing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JieqiEndState {
    Ongoing,
    Checkmate { winner: Color },
    Stalemate { winner: Color },
}

pub struct JieqiRules;

impl JieqiRules {
    pub fn public_kind(position: &JieqiPosition, piece: &JieqiPiece) -> PieceKind {
        if !piece.revealed {
            return piece.move_as;
        }
        match position.identities() {
            JieqiIdentitySource::Assigned(values) => values
                .iter()
                .find(|value| value.piece_id == piece.id)
                .map(|value| value.kind)
                .unwrap_or(piece.move_as),
            JieqiIdentitySource::RecordedReveals(values) => values
                .iter()
                .find(|value| value.piece_id == piece.id)
                .map(|value| value.kind)
                .unwrap_or(piece.move_as),
        }
    }

    pub fn piece_at(position: &JieqiPosition, square: Position) -> Option<&JieqiPiece> {
        position
            .pieces()
            .iter()
            .find(|piece| piece.position == square)
    }

    pub fn validate_move(position: &JieqiPosition, mv: Move) -> Result<(), JieqiMoveRejection> {
        Self::validate_candidate(position, mv)?;
        let piece = Self::piece_at(position, mv.from).expect("候选校验保证起点存在");
        let next = moved_position(position, mv, false);
        if Self::is_in_check(&next, piece.color) {
            return Err(JieqiMoveRejection::ExposesKing);
        }
        Ok(())
    }

    fn validate_candidate(position: &JieqiPosition, mv: Move) -> Result<(), JieqiMoveRejection> {
        if !mv.from.is_valid() || !mv.to.is_valid() || mv.from == mv.to {
            return Err(JieqiMoveRejection::InvalidPosition);
        }
        let piece = Self::piece_at(position, mv.from).ok_or(JieqiMoveRejection::EmptySource)?;
        if piece.color != position.turn() {
            return Err(JieqiMoveRejection::WrongTurn);
        }
        if let Some(target) = Self::piece_at(position, mv.to) {
            if target.color == piece.color {
                return Err(JieqiMoveRejection::FriendlyOccupied);
            }
            if Self::public_kind(position, target) == PieceKind::King {
                return Err(JieqiMoveRejection::CannotCaptureKing);
            }
        }
        let kind = Self::public_kind(position, piece);
        if !Self::can_piece_attack(position, piece, kind, mv.from, mv.to) {
            return Err(JieqiMoveRejection::InvalidGeometry);
        }
        Ok(())
    }

    pub fn can_move(position: &JieqiPosition, mv: Move) -> bool {
        Self::validate_move(position, mv).is_ok()
    }

    pub fn apply_move(
        position: &JieqiPosition,
        mv: Move,
    ) -> Result<JieqiPosition, JieqiMoveRejection> {
        Self::validate_move(position, mv)?;
        Ok(moved_position(position, mv, true))
    }

    pub fn legal_targets(position: &JieqiPosition, from: Position) -> Vec<Position> {
        Self::targets_matching(position, from, Self::validate_move)
    }

    /// 返回符合走子几何的展示候选，保留可能送将的位置供落子时解释
    pub fn candidate_targets(position: &JieqiPosition, from: Position) -> Vec<Position> {
        Self::targets_matching(position, from, Self::validate_candidate)
    }

    fn targets_matching(
        position: &JieqiPosition,
        from: Position,
        validate: fn(&JieqiPosition, Move) -> Result<(), JieqiMoveRejection>,
    ) -> Vec<Position> {
        let Some(piece) = Self::piece_at(position, from) else {
            return Vec::new();
        };
        if piece.color != position.turn() {
            return Vec::new();
        }
        let mut targets = Vec::new();
        for row in 0..ROW_COUNT as u8 {
            for col in 0..COL_COUNT as u8 {
                let to = Position::new(row, col);
                if validate(position, Move::new(from, to)).is_ok() {
                    targets.push(to);
                }
            }
        }
        targets
    }

    pub fn is_in_check(position: &JieqiPosition, color: Color) -> bool {
        let Some(king) = position
            .pieces()
            .iter()
            .find(|piece| piece.color == color && piece.move_as == PieceKind::King)
        else {
            return false;
        };
        let opponent_king = position
            .pieces()
            .iter()
            .find(|piece| piece.color == color.opposite() && piece.move_as == PieceKind::King);
        if let Some(opponent_king) = opponent_king
            && opponent_king.position.col == king.position.col
            && count_intervening(position, opponent_king.position, king.position) == 0
        {
            return true;
        }
        position.pieces().iter().any(|piece| {
            piece.color == color.opposite()
                && Self::can_piece_attack(
                    position,
                    piece,
                    Self::public_kind(position, piece),
                    piece.position,
                    king.position,
                )
        })
    }

    pub fn has_any_legal_move(position: &JieqiPosition, color: Color) -> bool {
        let mut candidate = position.clone();
        candidate.turn = color;
        candidate.pieces().iter().any(|piece| {
            piece.color == color && !Self::legal_targets(&candidate, piece.position).is_empty()
        })
    }

    pub fn end_state(position: &JieqiPosition) -> JieqiEndState {
        let side = position.turn();
        if Self::has_any_legal_move(position, side) {
            JieqiEndState::Ongoing
        } else if Self::is_in_check(position, side) {
            JieqiEndState::Checkmate {
                winner: side.opposite(),
            }
        } else {
            JieqiEndState::Stalemate {
                winner: side.opposite(),
            }
        }
    }

    fn can_piece_attack(
        position: &JieqiPosition,
        piece: &JieqiPiece,
        kind: PieceKind,
        from: Position,
        to: Position,
    ) -> bool {
        can_piece_move_geometry(
            piece.color,
            kind,
            from,
            to,
            if piece.revealed && matches!(kind, PieceKind::Advisor | PieceKind::Bishop) {
                GeometryPolicy::JieqiRevealed
            } else {
                GeometryPolicy::Xiangqi
            },
            |square| Self::piece_at(position, square).is_some(),
        )
    }
}

fn moved_position(position: &JieqiPosition, mv: Move, reveal: bool) -> JieqiPosition {
    let mut next = position.clone();
    next.pieces.retain(|piece| piece.position != mv.to);
    if let Some(piece) = next
        .pieces
        .iter_mut()
        .find(|piece| piece.position == mv.from)
    {
        piece.position = mv.to;
        if reveal {
            piece.revealed = true;
        }
    }
    next.turn = next.turn.opposite();
    next
}

fn count_intervening(position: &JieqiPosition, from: Position, to: Position) -> usize {
    if from.row == to.row {
        let start = from.col.min(to.col) + 1;
        let end = from.col.max(to.col);
        return (start..end)
            .filter(|col| JieqiRules::piece_at(position, Position::new(from.row, *col)).is_some())
            .count();
    }
    if from.col == to.col {
        let start = from.row.min(to.row) + 1;
        let end = from.row.max(to.row);
        return (start..end)
            .filter(|row| JieqiRules::piece_at(position, Position::new(*row, from.col)).is_some())
            .count();
    }
    usize::MAX
}
