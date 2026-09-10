pub mod board;
pub mod game;
pub mod hash;
pub mod notation;
pub mod piece;
pub mod position;
pub mod rules;

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("Invalid position: row {row}, col {col}")]
    InvalidPosition { row: u8, col: u8 },

    #[error("Invalid ICCS move string: {0}")]
    InvalidIccsMove(String),

    #[error("Invalid FEN string: {0}")]
    InvalidFen(String),

    #[error("Invalid Chinese notation: {0}")]
    InvalidChineseNotation(String),

    #[error("Illegal move: {0}")]
    IllegalMove(String),
}
