use serde::{Deserialize, Serialize};
use std::fmt;

/// 棋子阵营（红方 / 黑方）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    Red,
    Black,
}

impl Color {
    #[inline]
    pub const fn opposite(&self) -> Self {
        match self {
            Self::Red => Self::Black,
            Self::Black => Self::Red,
        }
    }

    #[inline]
    pub const fn is_red(&self) -> bool {
        matches!(self, Self::Red)
    }

    #[inline]
    pub const fn is_black(&self) -> bool {
        matches!(self, Self::Black)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Red => write!(f, "Red"),
            Self::Black => write!(f, "Black"),
        }
    }
}

/// 棋子种类（将、士、象、马、车、炮、卒）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PieceKind {
    King,    // 帅/将
    Advisor, // 仕/士
    Bishop,  // 相/象
    Knight,  // 傌/马
    Rook,    // 俥/车
    Cannon,  // 炮
    Pawn,    // 兵/卒
}

/// 棋子（阵营 + 种类，单字节紧凑值对象）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

impl Piece {
    pub const fn new(color: Color, kind: PieceKind) -> Self {
        Self { color, kind }
    }

    pub const fn red(kind: PieceKind) -> Self {
        Self {
            color: Color::Red,
            kind,
        }
    }

    pub const fn black(kind: PieceKind) -> Self {
        Self {
            color: Color::Black,
            kind,
        }
    }

    /// 转换为标准 FEN 字符表示（大写为红方，小写为黑方）
    pub const fn to_char(&self) -> char {
        match (self.color, self.kind) {
            (Color::Red, PieceKind::King) => 'K',
            (Color::Red, PieceKind::Advisor) => 'A',
            (Color::Red, PieceKind::Bishop) => 'B',
            (Color::Red, PieceKind::Knight) => 'N',
            (Color::Red, PieceKind::Rook) => 'R',
            (Color::Red, PieceKind::Cannon) => 'C',
            (Color::Red, PieceKind::Pawn) => 'P',

            (Color::Black, PieceKind::King) => 'k',
            (Color::Black, PieceKind::Advisor) => 'a',
            (Color::Black, PieceKind::Bishop) => 'b',
            (Color::Black, PieceKind::Knight) => 'n',
            (Color::Black, PieceKind::Rook) => 'r',
            (Color::Black, PieceKind::Cannon) => 'c',
            (Color::Black, PieceKind::Pawn) => 'p',
        }
    }

    /// 从 FEN 字符解析出棋子
    pub const fn from_char(c: char) -> Option<Self> {
        match c {
            'K' => Some(Self::red(PieceKind::King)),
            'A' => Some(Self::red(PieceKind::Advisor)),
            'B' => Some(Self::red(PieceKind::Bishop)),
            'N' => Some(Self::red(PieceKind::Knight)),
            'R' => Some(Self::red(PieceKind::Rook)),
            'C' => Some(Self::red(PieceKind::Cannon)),
            'P' => Some(Self::red(PieceKind::Pawn)),

            'k' => Some(Self::black(PieceKind::King)),
            'a' => Some(Self::black(PieceKind::Advisor)),
            'b' => Some(Self::black(PieceKind::Bishop)),
            'n' => Some(Self::black(PieceKind::Knight)),
            'r' => Some(Self::black(PieceKind::Rook)),
            'c' => Some(Self::black(PieceKind::Cannon)),
            'p' => Some(Self::black(PieceKind::Pawn)),

            _ => None,
        }
    }

    #[inline]
    pub const fn is_red(&self) -> bool {
        self.color.is_red()
    }

    #[inline]
    pub const fn is_black(&self) -> bool {
        self.color.is_black()
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}
