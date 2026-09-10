use crate::core::CoreError;
use crate::core::piece::{Color, Piece, PieceKind};
use crate::core::position::{Move, Position};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Index, IndexMut};

pub const ROW_COUNT: usize = 10;
pub const COL_COUNT: usize = 9;
pub const TOTAL_SQUARES: usize = ROW_COUNT * COL_COUNT;

pub const INITIAL_FEN: &str = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w";

/// 表示中国象棋 10×9 棋盘状态（单块内存、纯值语义、支持 Copy）
/// 行索引 0..=9（0 为黑方底线，9 为红方底线）
/// 列索引 0..=8（0 为 a 列/黑方左翼，8 为 i 列/黑方右翼）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoardState {
    pub pieces: [Option<Piece>; TOTAL_SQUARES],
    pub turn: Color,
}

impl Serialize for BoardState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_fen())
    }
}

impl<'de> Deserialize<'de> for BoardState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_fen(&s).map_err(serde::de::Error::custom)
    }
}

impl Default for BoardState {
    fn default() -> Self {
        Self::initial()
    }
}

impl BoardState {
    /// 空棋盘，红方先行
    pub const fn empty() -> Self {
        Self {
            pieces: [None; TOTAL_SQUARES],
            turn: Color::Red,
        }
    }

    /// 创建标准初始局面
    pub fn initial() -> Self {
        Self::from_fen(INITIAL_FEN).expect("Initial FEN must be valid")
    }

    #[inline]
    pub fn get_piece(&self, pos: Position) -> Option<Piece> {
        if pos.is_valid() {
            self.pieces[pos.to_index()]
        } else {
            None
        }
    }

    #[inline]
    pub fn set_piece(&mut self, pos: Position, piece: Option<Piece>) {
        if pos.is_valid() {
            self.pieces[pos.to_index()] = piece;
        }
    }

    /// 从 FEN 字符串解析出局面
    /// 兼容标准中国象棋 FEN：以 '/' 分隔 10 行，支持带 'w'/'r'/'b' 走方标识
    pub fn from_fen(fen: &str) -> Result<Self, CoreError> {
        let trimmed = fen.trim();
        if trimmed.is_empty() {
            return Err(CoreError::InvalidFen("Empty FEN string".to_string()));
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() || parts.len() > 6 {
            return Err(CoreError::InvalidFen(
                "Expected between 1 and 6 fields in FEN string".to_string(),
            ));
        }
        let rows: Vec<&str> = parts[0].split('/').collect();

        if rows.len() != ROW_COUNT {
            return Err(CoreError::InvalidFen(format!(
                "Expected {ROW_COUNT} rows, got {}",
                rows.len()
            )));
        }

        let mut pieces = [None; TOTAL_SQUARES];
        let mut red_king: Option<Position> = None;
        let mut black_king: Option<Position> = None;

        for (r, row_str) in rows.iter().enumerate() {
            let mut c = 0;
            for ch in row_str.chars() {
                if let Some(digit) = ch.to_digit(10) {
                    if digit == 0 {
                        return Err(CoreError::InvalidFen(
                            "Empty runs must be between 1 and 9".to_string(),
                        ));
                    }
                    let empty_count = digit as usize;
                    c += empty_count;
                    if c > COL_COUNT {
                        return Err(CoreError::InvalidFen(format!(
                            "Row {r} exceeds {COL_COUNT} columns"
                        )));
                    }
                } else if let Some(piece) = Piece::from_char(ch) {
                    if c >= COL_COUNT {
                        return Err(CoreError::InvalidFen(format!(
                            "Row {r} exceeds {COL_COUNT} columns"
                        )));
                    }
                    pieces[r * COL_COUNT + c] = Some(piece);
                    if piece.kind == PieceKind::King {
                        let position = Position::new(r as u8, c as u8);
                        let king_slot = if piece.color.is_red() {
                            &mut red_king
                        } else {
                            &mut black_king
                        };
                        if king_slot.replace(position).is_some() {
                            return Err(CoreError::InvalidFen(
                                "Each side must have exactly one king".to_string(),
                            ));
                        }
                    }
                    c += 1;
                } else {
                    return Err(CoreError::InvalidFen(format!(
                        "Invalid character '{ch}' in FEN"
                    )));
                }
            }

            if c != COL_COUNT {
                return Err(CoreError::InvalidFen(format!(
                    "Row {r} does not have exactly {COL_COUNT} columns (got {c})"
                )));
            }
        }

        let turn = match parts.get(1).map(|value| value.to_ascii_lowercase()) {
            None => Color::Red,
            Some(value) if value == "w" || value == "r" => Color::Red,
            Some(value) if value == "b" => Color::Black,
            Some(value) => {
                return Err(CoreError::InvalidFen(format!(
                    "Invalid side-to-move marker '{value}'"
                )));
            }
        };

        red_king.ok_or_else(|| CoreError::InvalidFen("Red king is missing".to_string()))?;
        black_king.ok_or_else(|| CoreError::InvalidFen("Black king is missing".to_string()))?;

        Ok(Self { pieces, turn })
    }

    /// 序列化为 Xiangqi FEN 字符串（2 段式："<board> <w/b>"）
    pub fn to_fen(&self) -> String {
        let mut fen = String::with_capacity(64);

        for r in 0..ROW_COUNT {
            if r > 0 {
                fen.push('/');
            }

            let mut empty_count = 0;
            for c in 0..COL_COUNT {
                let piece = self.pieces[r * COL_COUNT + c];
                match piece {
                    None => empty_count += 1,
                    Some(p) => {
                        if empty_count > 0 {
                            fen.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        fen.push(p.to_char());
                    }
                }
            }

            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }
        }

        fen.push(' ');
        fen.push(if self.turn.is_red() { 'w' } else { 'b' });

        fen
    }

    /// 执行一步走法，返回 (新局面, 被吃掉的棋子)
    pub fn apply_move(&self, mv: Move) -> (Self, Option<Piece>) {
        let mut next = *self;
        let from_idx = mv.from.to_index();
        let to_idx = mv.to.to_index();

        let moving_piece = next.pieces[from_idx];
        let captured = next.pieces[to_idx];

        next.pieces[from_idx] = None;
        next.pieces[to_idx] = moving_piece;
        next.turn = next.turn.opposite();

        (next, captured)
    }

    /// 撤销一步走法，恢复被吃棋子并反转走方
    pub fn undo_move(&self, mv: Move, captured: Option<Piece>) -> Self {
        let mut prev = *self;
        let from_idx = mv.from.to_index();
        let to_idx = mv.to.to_index();

        let moved_piece = prev.pieces[to_idx];
        prev.pieces[from_idx] = moved_piece;
        prev.pieces[to_idx] = captured;
        prev.turn = prev.turn.opposite();

        prev
    }

    /// 寻找某一方将/帅的位置
    pub fn find_king(&self, color: Color) -> Option<Position> {
        let target_kind = crate::core::piece::PieceKind::King;
        for i in 0..TOTAL_SQUARES {
            if let Some(p) = self.pieces[i]
                && p.color == color
                && p.kind == target_kind
            {
                return Some(Position::from_index(i));
            }
        }
        None
    }
}

impl Index<Position> for BoardState {
    type Output = Option<Piece>;

    #[inline]
    fn index(&self, pos: Position) -> &Self::Output {
        &self.pieces[pos.to_index()]
    }
}

impl IndexMut<Position> for BoardState {
    #[inline]
    fn index_mut(&mut self, pos: Position) -> &mut Self::Output {
        let idx = pos.to_index();
        &mut self.pieces[idx]
    }
}

impl Index<(usize, usize)> for BoardState {
    type Output = Option<Piece>;

    #[inline]
    fn index(&self, (row, col): (usize, usize)) -> &Self::Output {
        &self.pieces[row * COL_COUNT + col]
    }
}

impl IndexMut<(usize, usize)> for BoardState {
    #[inline]
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Self::Output {
        &mut self.pieces[row * COL_COUNT + col]
    }
}

impl fmt::Display for BoardState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_fen())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::piece::{Color, PieceKind};

    #[test]
    fn test_initial_board() {
        let board = BoardState::initial();
        assert_eq!(board.turn, Color::Red);

        // 检查红方将位于 (9, 4)
        let red_king = board.get_piece(Position::new(9, 4));
        assert_eq!(red_king, Some(Piece::new(Color::Red, PieceKind::King)));

        // 检查黑方将位于 (0, 4)
        let black_king = board.get_piece(Position::new(0, 4));
        assert_eq!(black_king, Some(Piece::new(Color::Black, PieceKind::King)));

        // 验证 FEN 往返转换
        assert_eq!(board.to_fen(), INITIAL_FEN);
    }

    #[test]
    fn test_apply_and_undo_move() {
        let board = BoardState::initial();
        let mv = Move::from_iccs("h2e2").unwrap(); // 炮二平五
        let (next, captured) = board.apply_move(mv);

        assert_eq!(captured, None);
        assert_eq!(next.turn, Color::Black);
        assert_eq!(next.get_piece(mv.from), None);
        assert_eq!(
            next.get_piece(mv.to),
            Some(Piece::new(Color::Red, PieceKind::Cannon))
        );

        let restored = next.undo_move(mv, captured);
        assert_eq!(restored, board);
    }

    #[test]
    fn test_find_king() {
        let board = BoardState::initial();
        assert_eq!(board.find_king(Color::Red), Some(Position::new(9, 4)));
        assert_eq!(board.find_king(Color::Black), Some(Position::new(0, 4)));
    }
}
