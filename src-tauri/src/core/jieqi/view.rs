use super::{JieqiIdentitySource, JieqiPosition};
use crate::core::piece::{Color, PieceKind};
use crate::core::position::Position;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum JieqiPublicKind {
    King,
    Advisor,
    Bishop,
    Knight,
    Rook,
    Cannon,
    Pawn,
}

impl From<PieceKind> for JieqiPublicKind {
    fn from(value: PieceKind) -> Self {
        match value {
            PieceKind::King => Self::King,
            PieceKind::Advisor => Self::Advisor,
            PieceKind::Bishop => Self::Bishop,
            PieceKind::Knight => Self::Knight,
            PieceKind::Rook => Self::Rook,
            PieceKind::Cannon => Self::Cannon,
            PieceKind::Pawn => Self::Pawn,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum JieqiPieceView {
    Hidden {
        position: Position,
        color: Color,
        move_as: JieqiPublicKind,
    },
    Revealed {
        position: Position,
        color: Color,
        kind: JieqiPublicKind,
    },
}

impl JieqiPieceView {
    fn position(&self) -> Position {
        match self {
            Self::Hidden { position, .. } | Self::Revealed { position, .. } => *position,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct JieqiPositionViewV1 {
    pub turn: Color,
    pub pieces: Vec<JieqiPieceView>,
}

impl JieqiPositionViewV1 {
    pub(crate) fn from_position(position: &JieqiPosition) -> Self {
        let assigned = match position.identities() {
            JieqiIdentitySource::Assigned(values) => Some(values.as_slice()),
            JieqiIdentitySource::RecordedReveals(_) => None,
        };
        let recorded = match position.identities() {
            JieqiIdentitySource::RecordedReveals(values) => Some(values.as_slice()),
            JieqiIdentitySource::Assigned(_) => None,
        };
        let mut pieces = position
            .pieces()
            .iter()
            .map(|piece| {
                if piece.revealed {
                    let kind = assigned
                        .and_then(|values| values.iter().find(|value| value.piece_id == piece.id))
                        .map(|value| value.kind)
                        .or_else(|| {
                            recorded.and_then(|values| {
                                values
                                    .iter()
                                    .find(|value| value.piece_id == piece.id)
                                    .map(|value| value.kind)
                            })
                        })
                        .unwrap_or(piece.move_as);
                    JieqiPieceView::Revealed {
                        position: piece.position,
                        color: piece.color,
                        kind: kind.into(),
                    }
                } else {
                    JieqiPieceView::Hidden {
                        position: piece.position,
                        color: piece.color,
                        move_as: piece.move_as.into(),
                    }
                }
            })
            .collect::<Vec<_>>();
        pieces.sort_by_key(|piece| {
            let position = piece.position();
            (position.row, position.col)
        });
        Self {
            turn: position.turn(),
            pieces,
        }
    }
}
