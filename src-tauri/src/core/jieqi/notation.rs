use super::{JieqiPosition, JieqiRules};
use crate::core::piece::{Color, PieceKind};
use crate::core::position::Move;

const RED_DIGITS: [&str; 10] = ["", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
const BLACK_DIGITS: [&str; 10] = ["", "１", "２", "３", "４", "５", "６", "７", "８", "９"];

pub struct JieqiNotation;

impl JieqiNotation {
    pub fn format(position: &JieqiPosition, mv: Move) -> String {
        let Some(piece) = JieqiRules::piece_at(position, mv.from) else {
            return mv.to_iccs();
        };
        let kind = JieqiRules::public_kind(position, piece);
        let mut text = String::new();
        if let Some(prefix) = disambiguation(position, piece.id, kind) {
            text.push_str(&prefix);
            text.push_str(piece_name(piece.color, kind));
        } else {
            text.push_str(piece_name(piece.color, kind));
            text.push_str(digit(file_number(piece.color, mv.from.col), piece.color));
        }
        if mv.from.row == mv.to.row {
            text.push('平');
            text.push_str(digit(file_number(piece.color, mv.to.col), piece.color));
            return text;
        }
        let advance = if piece.color.is_red() {
            mv.to.row < mv.from.row
        } else {
            mv.to.row > mv.from.row
        };
        text.push(if advance { '進' } else { '退' });
        if matches!(
            kind,
            PieceKind::Knight | PieceKind::Bishop | PieceKind::Advisor
        ) {
            text.push_str(digit(file_number(piece.color, mv.to.col), piece.color));
        } else {
            text.push_str(digit(mv.from.row.abs_diff(mv.to.row) as usize, piece.color));
        }
        text
    }
}

fn disambiguation(position: &JieqiPosition, piece_id: u8, kind: PieceKind) -> Option<String> {
    let piece = position
        .pieces()
        .iter()
        .find(|piece| piece.id == piece_id)?;
    let mut same_file = position
        .pieces()
        .iter()
        .filter(|candidate| {
            candidate.color == piece.color
                && candidate.position.col == piece.position.col
                && JieqiRules::public_kind(position, candidate) == kind
        })
        .collect::<Vec<_>>();
    if same_file.len() <= 1 {
        return None;
    }
    same_file.sort_by_key(|candidate| candidate.position.row);
    let index = same_file
        .iter()
        .position(|candidate| candidate.id == piece_id)?;
    let front_index = if piece.color.is_red() {
        index
    } else {
        same_file.len() - index - 1
    };
    Some(match (same_file.len(), front_index) {
        (2, 0) => "前".to_string(),
        (2, _) => "后".to_string(),
        (3, 0) => "前".to_string(),
        (3, 1) => "中".to_string(),
        (3, _) => "后".to_string(),
        (_, rank) => digit(rank + 1, piece.color).to_string(),
    })
}

fn piece_name(color: Color, kind: PieceKind) -> &'static str {
    match (color, kind) {
        (Color::Red, PieceKind::King) => "帥",
        (Color::Black, PieceKind::King) => "將",
        (Color::Red, PieceKind::Advisor) => "仕",
        (Color::Black, PieceKind::Advisor) => "士",
        (Color::Red, PieceKind::Bishop) => "相",
        (Color::Black, PieceKind::Bishop) => "象",
        (_, PieceKind::Knight) => "馬",
        (_, PieceKind::Rook) => "車",
        (_, PieceKind::Cannon) => "炮",
        (Color::Red, PieceKind::Pawn) => "兵",
        (Color::Black, PieceKind::Pawn) => "卒",
    }
}

fn file_number(color: Color, col: u8) -> usize {
    if color.is_red() {
        (9 - col) as usize
    } else {
        (col + 1) as usize
    }
}

fn digit(value: usize, color: Color) -> &'static str {
    if !(1..=9).contains(&value) {
        return "?";
    }
    if color.is_red() {
        RED_DIGITS[value]
    } else {
        BLACK_DIGITS[value]
    }
}
