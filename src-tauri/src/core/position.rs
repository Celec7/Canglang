use crate::core::CoreError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 表示象棋棋盘上的一个坐标位置
/// 行：0..=9（0 为黑方底线，9 为红方底线）
/// 列：0..=8（0 为 a 列/黑方左翼，8 为 i 列/黑方右翼）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub struct Position {
    pub row: u8,
    pub col: u8,
}

impl Position {
    pub const fn new(row: u8, col: u8) -> Self {
        Self { row, col }
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.row < 10 && self.col < 9
    }

    #[inline]
    pub const fn to_index(&self) -> usize {
        (self.row as usize) * 9 + (self.col as usize)
    }

    #[inline]
    pub const fn from_index(index: usize) -> Self {
        debug_assert!(index < 90);
        Self {
            row: (index / 9) as u8,
            col: (index % 9) as u8,
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.row, self.col)
    }
}

/// 表示一步走法（起始格到目标格）
/// 支持 ICCS 4 字符（如 "h2e2"）编解码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub struct Move {
    pub from: Position,
    pub to: Position,
}

impl Move {
    pub const fn new(from: Position, to: Position) -> Self {
        Self { from, to }
    }

    /// 获取 ICCS 4 字符走法字符串（如 "h2e2"）
    /// 列：a..i (0..8)
    /// 行：0..9（0 为红方底线 row=9，9 为黑方底线 row=0）
    pub fn to_iccs(&self) -> String {
        let fc = (b'a' + self.from.col) as char;
        let fr = (b'0' + (9 - self.from.row)) as char;
        let tc = (b'a' + self.to.col) as char;
        let tr = (b'0' + (9 - self.to.row)) as char;
        format!("{fc}{fr}{tc}{tr}")
    }

    /// 从 ICCS 4 字符走法字符串解析出 Move
    pub fn from_iccs(iccs: &str) -> Result<Self, CoreError> {
        let s = iccs.trim();
        if s.len() != 4 {
            return Err(CoreError::InvalidIccsMove(iccs.to_string()));
        }

        let bytes = s.as_bytes();
        let fc = bytes[0];
        let fr = bytes[1];
        let tc = bytes[2];
        let tr = bytes[3];

        if !(b'a'..=b'i').contains(&fc)
            || !(b'a'..=b'i').contains(&tc)
            || !fr.is_ascii_digit()
            || !tr.is_ascii_digit()
        {
            return Err(CoreError::InvalidIccsMove(iccs.to_string()));
        }

        let from_col = fc - b'a';
        let from_row = 9 - (fr - b'0');
        let to_col = tc - b'a';
        let to_row = 9 - (tr - b'0');

        Ok(Self {
            from: Position::new(from_row, from_col),
            to: Position::new(to_row, to_col),
        })
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_iccs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_index_conversion() {
        let p0 = Position::new(0, 0);
        assert_eq!(p0.to_index(), 0);
        assert_eq!(Position::from_index(0), p0);

        let p_end = Position::new(9, 8);
        assert_eq!(p_end.to_index(), 89);
        assert_eq!(Position::from_index(89), p_end);

        let p_mid = Position::new(5, 4);
        assert_eq!(p_mid.to_index(), 49);
        assert_eq!(Position::from_index(49), p_mid);
    }

    #[test]
    fn test_move_iccs_roundtrip() {
        // h2e2：从第 7 行第 7 列到第 7 行第 4 列（炮二平五）
        let mv = Move::from_iccs("h2e2").unwrap();
        assert_eq!(mv.from, Position::new(7, 7));
        assert_eq!(mv.to, Position::new(7, 4));
        assert_eq!(mv.to_iccs(), "h2e2");

        // a0a1：从第 9 行第 0 列到第 8 行第 0 列（车九进一）
        let mv2 = Move::from_iccs("a0a1").unwrap();
        assert_eq!(mv2.from, Position::new(9, 0));
        assert_eq!(mv2.to, Position::new(8, 0));
        assert_eq!(mv2.to_iccs(), "a0a1");
    }

    #[test]
    fn test_invalid_iccs() {
        assert!(Move::from_iccs("").is_err());
        assert!(Move::from_iccs("h2e").is_err());
        assert!(Move::from_iccs("j2e2").is_err()); // 'j' out of bounds
        assert!(Move::from_iccs("h2eA").is_err());
    }
}
