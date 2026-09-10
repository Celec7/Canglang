use crate::core::board::{BoardState, COL_COUNT, ROW_COUNT, TOTAL_SQUARES};
use crate::core::piece::{Piece, PieceKind};
use crate::core::position::{Move, Position};

use super::MoveValidator;

impl MoveValidator {
    /// 获取指定起始格上棋子的所有合法走法
    pub fn get_legal_moves(board: &BoardState, from: Position) -> Vec<Move> {
        if !from.is_valid() {
            return Vec::new();
        }

        let piece = match board.get_piece(from) {
            Some(p) => p,
            None => return Vec::new(),
        };

        if piece.color != board.turn {
            return Vec::new();
        }

        let mut list = Vec::with_capacity(16);
        for to in generate_candidate_targets(board, piece, from) {
            let mv = Move::new(from, to);
            if Self::can_move(board, mv) {
                list.push(mv);
            }
        }

        list
    }

    /// 获取指定起始格上棋子的所有候选目标走法（包含会送将/未应将的点位，供 UI 展示落点与反馈）
    pub fn get_candidate_moves(board: &BoardState, from: Position) -> Vec<Move> {
        if !from.is_valid() {
            return Vec::new();
        }

        let piece = match board.get_piece(from) {
            Some(p) => p,
            None => return Vec::new(),
        };

        if piece.color != board.turn {
            return Vec::new();
        }

        let mut list = Vec::with_capacity(16);
        for to in generate_candidate_targets(board, piece, from) {
            if let Some(target) = board.get_piece(to)
                && (target.color == piece.color || target.kind == PieceKind::King)
            {
                continue;
            }
            if Self::can_piece_move_pseudo(board, piece, from, to) {
                list.push(Move::new(from, to));
            }
        }

        list
    }

    /// 获取当前走方所有棋子的全部合法走法
    pub fn get_all_legal_moves(board: &BoardState) -> Vec<Move> {
        Self::get_all_legal_moves_for_side(board, board.turn)
    }

    /// 获取指定方所有棋子的全部合法走法
    pub fn get_all_legal_moves_for_side(
        board: &BoardState,
        color: crate::core::piece::Color,
    ) -> Vec<Move> {
        let mut board_copy = *board;
        board_copy.turn = color;

        let mut list = Vec::with_capacity(64);
        for i in 0..TOTAL_SQUARES {
            if let Some(piece) = board_copy.pieces[i]
                && piece.color == color
            {
                let from = Position::from_index(i);
                list.extend(Self::get_legal_moves(&board_copy, from));
            }
        }

        list
    }

    /// 快速检测指定方是否存在至少一步合法走法
    pub fn has_any_legal_move(board: &BoardState, color: crate::core::piece::Color) -> bool {
        let mut board_copy = *board;
        board_copy.turn = color;

        for i in 0..TOTAL_SQUARES {
            if let Some(piece) = board_copy.pieces[i]
                && piece.color == color
            {
                let from = Position::from_index(i);
                for to in generate_candidate_targets(&board_copy, piece, from) {
                    let mv = Move::new(from, to);
                    if Self::can_move(&board_copy, mv) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// 根据棋子几何走法生成候选目标格，供合法性过滤和快速存在性查询使用
pub(super) fn generate_candidate_targets(
    board: &BoardState,
    piece: Piece,
    from: Position,
) -> Vec<Position> {
    let mut targets = Vec::with_capacity(16);
    let r_i = from.row as i32;
    let c_i = from.col as i32;

    match piece.kind {
        PieceKind::Rook | PieceKind::Cannon => {
            const DRS: [i32; 4] = [-1, 1, 0, 0];
            const DCS: [i32; 4] = [0, 0, -1, 1];

            for i in 0..4 {
                let mut r = r_i + DRS[i];
                let mut c = c_i + DCS[i];

                while (0..ROW_COUNT as i32).contains(&r) && (0..COL_COUNT as i32).contains(&c) {
                    let to = Position::new(r as u8, c as u8);
                    targets.push(to);

                    if board.get_piece(to).is_some() {
                        if piece.kind == PieceKind::Cannon {
                            let mut r2 = r + DRS[i];
                            let mut c2 = c + DCS[i];
                            while (0..ROW_COUNT as i32).contains(&r2)
                                && (0..COL_COUNT as i32).contains(&c2)
                            {
                                let to2 = Position::new(r2 as u8, c2 as u8);
                                if board.get_piece(to2).is_some() {
                                    targets.push(to2);
                                    break;
                                }
                                r2 += DRS[i];
                                c2 += DCS[i];
                            }
                        }
                        break;
                    }

                    r += DRS[i];
                    c += DCS[i];
                }
            }
        }
        PieceKind::Knight => {
            const NDR: [i32; 8] = [-2, -2, -1, -1, 1, 1, 2, 2];
            const NDC: [i32; 8] = [-1, 1, -2, 2, -2, 2, -1, 1];
            for i in 0..8 {
                let r = r_i + NDR[i];
                let c = c_i + NDC[i];
                if (0..ROW_COUNT as i32).contains(&r) && (0..COL_COUNT as i32).contains(&c) {
                    targets.push(Position::new(r as u8, c as u8));
                }
            }
        }
        PieceKind::Bishop => {
            const BDR: [i32; 4] = [-2, -2, 2, 2];
            const BDC: [i32; 4] = [-2, 2, -2, 2];
            for i in 0..4 {
                let r = r_i + BDR[i];
                let c = c_i + BDC[i];
                if (0..ROW_COUNT as i32).contains(&r) && (0..COL_COUNT as i32).contains(&c) {
                    targets.push(Position::new(r as u8, c as u8));
                }
            }
        }
        PieceKind::Advisor => {
            const ADR: [i32; 4] = [-1, -1, 1, 1];
            const ADC: [i32; 4] = [-1, 1, -1, 1];
            for i in 0..4 {
                let r = r_i + ADR[i];
                let c = c_i + ADC[i];
                if (0..ROW_COUNT as i32).contains(&r) && (0..COL_COUNT as i32).contains(&c) {
                    targets.push(Position::new(r as u8, c as u8));
                }
            }
        }
        PieceKind::King => {
            const KDR: [i32; 4] = [-1, 1, 0, 0];
            const KDC: [i32; 4] = [0, 0, -1, 1];
            for i in 0..4 {
                let r = r_i + KDR[i];
                let c = c_i + KDC[i];
                if (0..ROW_COUNT as i32).contains(&r) && (0..COL_COUNT as i32).contains(&c) {
                    targets.push(Position::new(r as u8, c as u8));
                }
            }
        }
        PieceKind::Pawn => {
            let forward_dr = if piece.color.is_red() { -1 } else { 1 };
            let forward_r = r_i + forward_dr;
            if (0..ROW_COUNT as i32).contains(&forward_r) {
                targets.push(Position::new(forward_r as u8, from.col));
            }

            let crossed_river = if piece.color.is_red() {
                from.row <= 4
            } else {
                from.row >= 5
            };

            if crossed_river {
                if from.col > 0 {
                    targets.push(Position::new(from.row, from.col - 1));
                }
                if from.col < (COL_COUNT - 1) as u8 {
                    targets.push(Position::new(from.row, from.col + 1));
                }
            }
        }
    }

    targets
}
