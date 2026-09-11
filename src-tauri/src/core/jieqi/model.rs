use super::JieqiError;
use super::view::JieqiPositionViewV1;
use crate::core::piece::{Color, PieceKind};
use crate::core::position::Position;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitialSlot {
    pub id: u8,
    pub color: Color,
    pub move_as: PieceKind,
    pub position: Position,
}

const fn slot(id: u8, color: Color, move_as: PieceKind, row: u8, col: u8) -> InitialSlot {
    InitialSlot {
        id,
        color,
        move_as,
        position: Position::new(row, col),
    }
}

pub const STANDARD_INITIAL_SLOTS: [InitialSlot; 32] = [
    slot(0, Color::Black, PieceKind::Rook, 0, 0),
    slot(1, Color::Black, PieceKind::Knight, 0, 1),
    slot(2, Color::Black, PieceKind::Bishop, 0, 2),
    slot(3, Color::Black, PieceKind::Advisor, 0, 3),
    slot(4, Color::Black, PieceKind::King, 0, 4),
    slot(5, Color::Black, PieceKind::Advisor, 0, 5),
    slot(6, Color::Black, PieceKind::Bishop, 0, 6),
    slot(7, Color::Black, PieceKind::Knight, 0, 7),
    slot(8, Color::Black, PieceKind::Rook, 0, 8),
    slot(9, Color::Black, PieceKind::Cannon, 2, 1),
    slot(10, Color::Black, PieceKind::Cannon, 2, 7),
    slot(11, Color::Black, PieceKind::Pawn, 3, 0),
    slot(12, Color::Black, PieceKind::Pawn, 3, 2),
    slot(13, Color::Black, PieceKind::Pawn, 3, 4),
    slot(14, Color::Black, PieceKind::Pawn, 3, 6),
    slot(15, Color::Black, PieceKind::Pawn, 3, 8),
    slot(16, Color::Red, PieceKind::Pawn, 6, 0),
    slot(17, Color::Red, PieceKind::Pawn, 6, 2),
    slot(18, Color::Red, PieceKind::Pawn, 6, 4),
    slot(19, Color::Red, PieceKind::Pawn, 6, 6),
    slot(20, Color::Red, PieceKind::Pawn, 6, 8),
    slot(21, Color::Red, PieceKind::Cannon, 7, 1),
    slot(22, Color::Red, PieceKind::Cannon, 7, 7),
    slot(23, Color::Red, PieceKind::Rook, 9, 0),
    slot(24, Color::Red, PieceKind::Knight, 9, 1),
    slot(25, Color::Red, PieceKind::Bishop, 9, 2),
    slot(26, Color::Red, PieceKind::Advisor, 9, 3),
    slot(27, Color::Red, PieceKind::King, 9, 4),
    slot(28, Color::Red, PieceKind::Advisor, 9, 5),
    slot(29, Color::Red, PieceKind::Bishop, 9, 6),
    slot(30, Color::Red, PieceKind::Knight, 9, 7),
    slot(31, Color::Red, PieceKind::Rook, 9, 8),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JieqiPiece {
    pub id: u8,
    pub color: Color,
    pub move_as: PieceKind,
    pub position: Position,
    pub revealed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JieqiIdentity {
    pub piece_id: u8,
    pub kind: PieceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JieqiReveal {
    pub piece_id: u8,
    pub kind: PieceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JieqiIdentitySource {
    Assigned(Vec<JieqiIdentity>),
    RecordedReveals(Vec<JieqiReveal>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JieqiPosition {
    pub(crate) pieces: Vec<JieqiPiece>,
    pub(crate) identities: JieqiIdentitySource,
    pub(crate) turn: Color,
}

impl JieqiPosition {
    pub fn standard_assigned(assignments: Vec<JieqiIdentity>) -> Result<Self, JieqiError> {
        let pieces = STANDARD_INITIAL_SLOTS
            .iter()
            .map(|slot| JieqiPiece {
                id: slot.id,
                color: slot.color,
                move_as: slot.move_as,
                position: slot.position,
                revealed: slot.move_as == PieceKind::King,
            })
            .collect();
        Self::from_parts(
            pieces,
            JieqiIdentitySource::Assigned(assignments),
            Color::Red,
        )
    }

    pub fn standard_recorded(reveals: Vec<JieqiReveal>) -> Result<Self, JieqiError> {
        let pieces = STANDARD_INITIAL_SLOTS
            .iter()
            .map(|slot| JieqiPiece {
                id: slot.id,
                color: slot.color,
                move_as: slot.move_as,
                position: slot.position,
                revealed: slot.move_as == PieceKind::King,
            })
            .collect();
        Self::from_parts(
            pieces,
            JieqiIdentitySource::RecordedReveals(reveals),
            Color::Red,
        )
    }

    pub fn from_parts(
        pieces: Vec<JieqiPiece>,
        identities: JieqiIdentitySource,
        turn: Color,
    ) -> Result<Self, JieqiError> {
        validate_pieces(&pieces)?;
        validate_identities(&identities)?;
        Ok(Self {
            pieces,
            identities,
            turn,
        })
    }

    pub fn pieces(&self) -> &[JieqiPiece] {
        &self.pieces
    }

    pub fn identities(&self) -> &JieqiIdentitySource {
        &self.identities
    }

    pub const fn turn(&self) -> Color {
        self.turn
    }

    pub fn public_view(&self) -> JieqiPositionViewV1 {
        JieqiPositionViewV1::from_position(self)
    }
}

pub fn standard_identity_kinds() -> Vec<PieceKind> {
    STANDARD_INITIAL_SLOTS
        .iter()
        .map(|slot| slot.move_as)
        .collect()
}

fn validate_pieces(pieces: &[JieqiPiece]) -> Result<(), JieqiError> {
    if !(2..=STANDARD_INITIAL_SLOTS.len()).contains(&pieces.len()) {
        return Err(JieqiError::InvalidPieceCount(pieces.len()));
    }
    let mut ids = HashSet::with_capacity(32);
    let mut positions = HashSet::with_capacity(32);
    for piece in pieces {
        if !piece.position.is_valid() {
            return Err(JieqiError::InvalidPosition(piece.position));
        }
        if !ids.insert(piece.id) {
            return Err(JieqiError::DuplicatePieceId(piece.id));
        }
        if !positions.insert(piece.position) {
            return Err(JieqiError::DuplicatePosition(piece.position));
        }
        let Some(initial) = STANDARD_INITIAL_SLOTS.get(piece.id as usize) else {
            return Err(JieqiError::InvalidPieceId(piece.id));
        };
        if piece.color != initial.color {
            return Err(JieqiError::InvalidPieceColor { id: piece.id });
        }
        if piece.move_as != initial.move_as {
            return Err(JieqiError::InvalidMoveRole { id: piece.id });
        }
        let king = piece.move_as == PieceKind::King;
        if king && !piece.revealed {
            return Err(JieqiError::InvalidKing(piece.id));
        }
    }
    for king_id in [4, 27] {
        if !ids.contains(&king_id) {
            return Err(JieqiError::InvalidKing(king_id));
        }
    }
    Ok(())
}

fn validate_identities(source: &JieqiIdentitySource) -> Result<(), JieqiError> {
    match source {
        JieqiIdentitySource::Assigned(assignments) => validate_assignments(assignments),
        JieqiIdentitySource::RecordedReveals(reveals) => validate_reveals(reveals),
    }
}

fn validate_assignments(assignments: &[JieqiIdentity]) -> Result<(), JieqiError> {
    if assignments.len() != STANDARD_INITIAL_SLOTS.len() {
        return Err(JieqiError::InvalidIdentityCount(assignments.len()));
    }
    let mut ids = HashSet::with_capacity(32);
    for identity in assignments {
        let Some(slot) = STANDARD_INITIAL_SLOTS.get(identity.piece_id as usize) else {
            return Err(JieqiError::InvalidPieceId(identity.piece_id));
        };
        if !ids.insert(identity.piece_id) {
            return Err(JieqiError::DuplicateIdentityId(identity.piece_id));
        }
        if slot.move_as == PieceKind::King && identity.kind != PieceKind::King {
            return Err(JieqiError::InvalidKing(identity.piece_id));
        }
        if slot.move_as != PieceKind::King && identity.kind == PieceKind::King {
            return Err(JieqiError::InvalidKing(identity.piece_id));
        }
    }
    for color in [Color::Red, Color::Black] {
        for kind in [
            PieceKind::King,
            PieceKind::Advisor,
            PieceKind::Bishop,
            PieceKind::Knight,
            PieceKind::Rook,
            PieceKind::Cannon,
            PieceKind::Pawn,
        ] {
            let expected = STANDARD_INITIAL_SLOTS
                .iter()
                .filter(|slot| slot.color == color && slot.move_as == kind)
                .count();
            let actual = assignments
                .iter()
                .filter(|identity| {
                    STANDARD_INITIAL_SLOTS[identity.piece_id as usize].color == color
                        && identity.kind == kind
                })
                .count();
            if actual != expected {
                return Err(JieqiError::InvalidIdentityMultiset {
                    color,
                    kind,
                    expected,
                    actual,
                });
            }
        }
    }
    Ok(())
}

fn validate_reveals(reveals: &[JieqiReveal]) -> Result<(), JieqiError> {
    let mut ids = HashSet::with_capacity(reveals.len());
    for reveal in reveals {
        let Some(slot) = STANDARD_INITIAL_SLOTS.get(reveal.piece_id as usize) else {
            return Err(JieqiError::InvalidRevealId(reveal.piece_id));
        };
        if slot.move_as == PieceKind::King || reveal.kind == PieceKind::King {
            return Err(JieqiError::RecordedKingReveal(reveal.piece_id));
        }
        if !ids.insert(reveal.piece_id) {
            return Err(JieqiError::DuplicateRevealId(reveal.piece_id));
        }
    }
    for color in [Color::Red, Color::Black] {
        for kind in [
            PieceKind::Advisor,
            PieceKind::Bishop,
            PieceKind::Knight,
            PieceKind::Rook,
            PieceKind::Cannon,
            PieceKind::Pawn,
        ] {
            let available = STANDARD_INITIAL_SLOTS
                .iter()
                .filter(|slot| slot.color == color && slot.move_as == kind)
                .count();
            let actual = reveals
                .iter()
                .filter(|reveal| {
                    STANDARD_INITIAL_SLOTS[reveal.piece_id as usize].color == color
                        && reveal.kind == kind
                })
                .count();
            if actual > available {
                return Err(JieqiError::InvalidIdentityMultiset {
                    color,
                    kind,
                    expected: available,
                    actual,
                });
            }
        }
    }
    Ok(())
}
