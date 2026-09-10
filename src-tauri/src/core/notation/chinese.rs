use crate::core::CoreError;
use crate::core::board::{BoardState, COL_COUNT, ROW_COUNT};
use crate::core::piece::{Color, Piece, PieceKind};
use crate::core::position::{Move, Position};
use crate::core::rules::MoveValidator;

const RED_DIGITS: [&str; 10] = ["", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
const BLACK_DIGITS: [&str; 10] = ["", "１", "２", "３", "４", "５", "６", "７", "８", "９"]; // 黑方传统用全角阿拉伯数字

pub struct NotationConverter;

impl NotationConverter {
    /// 将指定走法转换为符合象棋标准的 4 字符中文记法（如「炮二平五」、「馬８進７」）
    pub fn to_chinese_notation(before: &BoardState, mv: &Move) -> Result<String, CoreError> {
        let piece = before
            .get_piece(mv.from)
            .ok_or(CoreError::InvalidPosition {
                row: mv.from.row,
                col: mv.from.col,
            })?;

        let is_red = piece.color.is_red();
        let piece_name = Self::get_piece_name(piece);

        let mut res = String::with_capacity(16);

        // 1. 判断是否需要前缀（同列多子消歧）
        if let Some(prefix) = Self::get_disambiguation_prefix(before, mv.from, piece, is_red) {
            res.push_str(&prefix);
            res.push_str(piece_name);
        } else {
            res.push_str(piece_name);
            let from_file = Self::get_file_number(mv.from.col, is_red);
            res.push_str(&Self::format_digit(from_file, is_red));
        }

        // 2. 判断动词：平、进、退
        if mv.from.row == mv.to.row {
            res.push('平');
            let to_file = Self::get_file_number(mv.to.col, is_red);
            res.push_str(&Self::format_digit(to_file, is_red));
        } else {
            let is_advance = if is_red {
                mv.to.row < mv.from.row
            } else {
                mv.to.row > mv.from.row
            };
            res.push(if is_advance { '進' } else { '退' });

            let is_diagonal = matches!(
                piece.kind,
                PieceKind::Knight | PieceKind::Bishop | PieceKind::Advisor
            );

            if is_diagonal {
                let to_file = Self::get_file_number(mv.to.col, is_red);
                res.push_str(&Self::format_digit(to_file, is_red));
            } else {
                // 直线棋子（車、炮、兵/卒、帥/將）：使用进退的垂直步数
                let steps = (mv.to.row as i32 - mv.from.row as i32).unsigned_abs() as usize;
                res.push_str(&Self::format_digit(steps, is_red));
            }
        }

        Ok(res)
    }

    /// 将中文走法解析为 Move
    pub fn from_chinese_notation(board: &BoardState, cn: &str) -> Result<Move, CoreError> {
        let cn = cn.trim();
        if cn.is_empty() {
            return Err(CoreError::InvalidChineseNotation(cn.to_string()));
        }

        // 统一全角与半角数字
        let normalized = Self::normalize_numbers(cn);
        let char_count = normalized.chars().count();
        if char_count != 4 {
            return Err(CoreError::InvalidChineseNotation(format!(
                "Expected 4 characters, got {char_count}: '{cn}'"
            )));
        }

        let is_red = board.turn.is_red();

        // 优先在当前方所有合法走法中匹配
        let legal_moves = MoveValidator::get_all_legal_moves(board);
        for mv in legal_moves {
            if let Ok(generated_cn) = Self::to_chinese_notation(board, &mv)
                && Self::normalize_numbers(&generated_cn) == normalized
            {
                return Ok(mv);
            }
        }

        // 若合法走法中未匹配（可能是非严格合规局面输入），尝试根据规则几何解析
        if let Some(parsed) = Self::try_geom_parse(board, &normalized, is_red) {
            return Ok(parsed);
        }

        Err(CoreError::InvalidChineseNotation(format!(
            "Could not resolve move for '{cn}' on current board"
        )))
    }

    /// 连续推演一组 ICCS 走法序列（变例 PV 链），转换为用空格分隔的中文记法列表
    /// 例如 ["h2e2", "h9g7", "i0i1"] -> ["炮二平五", "馬８進７", "車一進一"]
    pub fn deduce_pv_chinese(start_board: &BoardState, iccs_moves: &[String]) -> Vec<String> {
        let mut board = *start_board;
        let mut results = Vec::with_capacity(iccs_moves.len());

        for iccs in iccs_moves {
            if let Ok(mv) = Move::from_iccs(iccs) {
                if board.get_piece(mv.from).is_none() {
                    break;
                }
                if let Ok(cn) = Self::to_chinese_notation(&board, &mv) {
                    results.push(cn);
                    let (next_board, _) = board.apply_move(mv);
                    board = next_board;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        results
    }

    fn get_piece_name(piece: Piece) -> &'static str {
        match (piece.color, piece.kind) {
            (Color::Red, PieceKind::King) => "帥",
            (Color::Red, PieceKind::Advisor) => "仕",
            (Color::Red, PieceKind::Bishop) => "相",
            (Color::Red, PieceKind::Knight) => "馬",
            (Color::Red, PieceKind::Rook) => "車",
            (Color::Red, PieceKind::Cannon) => "炮",
            (Color::Red, PieceKind::Pawn) => "兵",

            (Color::Black, PieceKind::King) => "將",
            (Color::Black, PieceKind::Advisor) => "士",
            (Color::Black, PieceKind::Bishop) => "象",
            (Color::Black, PieceKind::Knight) => "馬",
            (Color::Black, PieceKind::Rook) => "車",
            (Color::Black, PieceKind::Cannon) => "炮",
            (Color::Black, PieceKind::Pawn) => "卒",
        }
    }

    #[inline]
    fn get_file_number(col: u8, is_red: bool) -> usize {
        // 红方从右往左：col 8 -> 1, col 0 -> 9
        // 黑方从右往左（黑方视角）：col 0 -> 1, col 8 -> 9
        if is_red {
            (9 - col) as usize
        } else {
            (col + 1) as usize
        }
    }

    #[inline]
    fn get_col_from_file_number(file: usize, is_red: bool) -> Option<u8> {
        if !(1..=9).contains(&file) {
            return None;
        }
        if is_red {
            Some((9 - file) as u8)
        } else {
            Some((file - 1) as u8)
        }
    }

    fn format_digit(digit: usize, is_red: bool) -> String {
        if !(1..=9).contains(&digit) {
            return digit.to_string();
        }
        if is_red {
            RED_DIGITS[digit].to_string()
        } else {
            BLACK_DIGITS[digit].to_string()
        }
    }

    fn parse_digit(ch: char) -> usize {
        match ch {
            '一' | '１' | '1' => 1,
            '二' | '２' | '2' => 2,
            '三' | '３' | '3' => 3,
            '四' | '４' | '4' => 4,
            '五' | '５' | '5' => 5,
            '六' | '６' | '6' => 6,
            '七' | '７' | '7' => 7,
            '八' | '８' | '8' => 8,
            '九' | '９' | '9' => 9,
            _ => 0,
        }
    }

    fn normalize_numbers(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '1'..='9' => (('１' as u32) + (c as u32 - '1' as u32))
                    .try_into()
                    .unwrap_or(c),
                _ => c,
            })
            .collect()
    }

    fn get_disambiguation_prefix(
        board: &BoardState,
        from: Position,
        piece: Piece,
        is_red: bool,
    ) -> Option<String> {
        // 只有车、马、炮、兵/卒同列可能需要消歧
        match piece.kind {
            PieceKind::Rook | PieceKind::Knight | PieceKind::Cannon | PieceKind::Pawn => {}
            _ => return None,
        }

        // 收集同列相同棋子（按行号升序：0..=9）
        let mut same_col_rows = Vec::new();
        for r in 0..ROW_COUNT {
            if let Some(p) = board.get_piece(Position::new(r as u8, from.col))
                && p == piece
            {
                same_col_rows.push(r as u8);
            }
        }

        if same_col_rows.len() <= 1 {
            return None;
        }

        if same_col_rows.len() == 2 {
            // 两子同列
            // 红方朝向 row 0 进攻：row 较小的为「前」，较大的为「后」
            // 黑方朝向 row 9 进攻：row 较大的为「前」，较小的为「后」
            let is_front = if is_red {
                from.row == same_col_rows[0]
            } else {
                from.row == same_col_rows[1]
            };
            return Some(if is_front { "前" } else { "后" }.to_string());
        }

        if same_col_rows.len() == 3 {
            // 三子同列（通常为兵/卒）
            if is_red {
                if from.row == same_col_rows[0] {
                    return Some("前".to_string());
                }
                if from.row == same_col_rows[1] {
                    return Some("中".to_string());
                }
                return Some("后".to_string());
            } else {
                if from.row == same_col_rows[2] {
                    return Some("前".to_string());
                }
                if from.row == same_col_rows[1] {
                    return Some("中".to_string());
                }
                return Some("后".to_string());
            }
        }

        // 4 或 5 子同列
        let idx = same_col_rows.iter().position(|&r| r == from.row)?;
        let rank = if is_red {
            idx + 1
        } else {
            same_col_rows.len() - idx
        };
        Some(Self::format_digit(rank, is_red))
    }

    #[inline]
    fn is_advance_verb(ch: char) -> bool {
        matches!(ch, '进' | '進')
    }

    fn try_geom_parse(board: &BoardState, cn: &str, is_red: bool) -> Option<Move> {
        let chars: Vec<char> = cn.chars().collect();
        if chars.len() != 4 {
            return None;
        }

        let a = chars[0];
        let b = chars[1];
        let c = chars[2];
        let d = chars[3];

        let piece: Piece;
        let mut from_col = None;
        let mut from_row = None;

        if matches!(a, '前' | '后' | '中') {
            piece = Self::resolve_piece(b, is_red)?;

            // 寻找含有多个该棋子的列
            for col in 0..COL_COUNT as u8 {
                let mut rows = Vec::new();
                for r in 0..ROW_COUNT as u8 {
                    if board.get_piece(Position::new(r, col)) == Some(piece) {
                        rows.push(r);
                    }
                }

                if rows.len() >= 2 {
                    from_col = Some(col);
                    if rows.len() == 2 {
                        let is_front = a == '前';
                        from_row = Some(if is_red {
                            if is_front { rows[0] } else { rows[1] }
                        } else {
                            if is_front { rows[1] } else { rows[0] }
                        });
                    } else if rows.len() == 3 {
                        from_row = Some(if a == '前' {
                            if is_red { rows[0] } else { rows[2] }
                        } else if a == '中' {
                            rows[1]
                        } else {
                            if is_red { rows[2] } else { rows[0] }
                        });
                    }
                    break;
                }
            }
        } else {
            piece = Self::resolve_piece(a, is_red)?;
            let file = Self::parse_digit(b);
            if file == 0 {
                return None;
            }
            let col = Self::get_col_from_file_number(file, is_red)?;
            from_col = Some(col);

            // 在该列寻找该棋子
            for r in 0..ROW_COUNT as u8 {
                if board.get_piece(Position::new(r, col)) == Some(piece) {
                    from_row = Some(r);
                    break;
                }
            }
        }

        let from_row = from_row?;
        let from_col = from_col?;
        let from = Position::new(from_row, from_col);

        let fourth_digit = Self::parse_digit(d);
        if fourth_digit == 0 {
            return None;
        }

        let mut to_row = None;
        let mut to_col = None;

        if c == '平' {
            to_row = Some(from_row);
            to_col = Self::get_col_from_file_number(fourth_digit, is_red);
        } else if matches!(c, '进' | '進' | '退') {
            let is_diagonal = matches!(
                piece.kind,
                PieceKind::Knight | PieceKind::Bishop | PieceKind::Advisor
            );

            if is_diagonal {
                let target_col = Self::get_col_from_file_number(fourth_digit, is_red)?;
                to_col = Some(target_col);
                let delta_col = (target_col as i32 - from_col as i32).abs();

                match piece.kind {
                    PieceKind::Knight => {
                        let delta_row = if delta_col == 1 { 2 } else { 1 };
                        let r = if is_red {
                            if Self::is_advance_verb(c) {
                                from_row as i32 - delta_row
                            } else {
                                from_row as i32 + delta_row
                            }
                        } else {
                            if Self::is_advance_verb(c) {
                                from_row as i32 + delta_row
                            } else {
                                from_row as i32 - delta_row
                            }
                        };
                        to_row = Some(r as u8);
                    }
                    PieceKind::Bishop => {
                        let r = if is_red {
                            if Self::is_advance_verb(c) {
                                from_row as i32 - 2
                            } else {
                                from_row as i32 + 2
                            }
                        } else {
                            if Self::is_advance_verb(c) {
                                from_row as i32 + 2
                            } else {
                                from_row as i32 - 2
                            }
                        };
                        to_row = Some(r as u8);
                    }
                    PieceKind::Advisor => {
                        let r = if is_red {
                            if Self::is_advance_verb(c) {
                                from_row as i32 - 1
                            } else {
                                from_row as i32 + 1
                            }
                        } else {
                            if Self::is_advance_verb(c) {
                                from_row as i32 + 1
                            } else {
                                from_row as i32 - 1
                            }
                        };
                        to_row = Some(r as u8);
                    }
                    _ => {}
                }
            } else {
                to_col = Some(from_col);
                let steps = fourth_digit as i32;
                let r = if is_red {
                    if Self::is_advance_verb(c) {
                        from_row as i32 - steps
                    } else {
                        from_row as i32 + steps
                    }
                } else {
                    if Self::is_advance_verb(c) {
                        from_row as i32 + steps
                    } else {
                        from_row as i32 - steps
                    }
                };
                to_row = Some(r as u8);
            }
        }

        let to_row = to_row?;
        let to_col = to_col?;

        if to_row < ROW_COUNT as u8 && to_col < COL_COUNT as u8 {
            Some(Move::new(from, Position::new(to_row, to_col)))
        } else {
            None
        }
    }

    fn resolve_piece(name: char, is_red: bool) -> Option<Piece> {
        let color = if is_red { Color::Red } else { Color::Black };
        let kind = match name {
            '车' | '車' | '俥' => PieceKind::Rook,
            '马' | '馬' | '傌' => PieceKind::Knight,
            '炮' | '砲' => PieceKind::Cannon,
            '兵' => PieceKind::Pawn,
            '卒' => PieceKind::Pawn,
            '相' => PieceKind::Bishop,
            '象' => PieceKind::Bishop,
            '仕' => PieceKind::Advisor,
            '士' => PieceKind::Advisor,
            '帅' | '帥' => PieceKind::King,
            '将' | '將' => PieceKind::King,
            _ => return None,
        };
        Some(Piece::new(color, kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_chinese_notation_standard_opening_moves() {
        let board = BoardState::initial();

        // 炮二平五 (7, 7) -> (7, 4) "h2e2"
        let c_move = Move::from_iccs("h2e2").unwrap();
        assert_eq!(
            NotationConverter::to_chinese_notation(&board, &c_move).unwrap(),
            "炮二平五"
        );

        // 車一進一 (9, 8) -> (8, 8) "i0i1"
        let r_move = Move::from_iccs("i0i1").unwrap();
        assert_eq!(
            NotationConverter::to_chinese_notation(&board, &r_move).unwrap(),
            "車一進一"
        );

        // 馬八進七 (9, 1) -> (7, 2) "b0c2"
        let n_move = Move::from_iccs("b0c2").unwrap();
        assert_eq!(
            NotationConverter::to_chinese_notation(&board, &n_move).unwrap(),
            "馬八進七"
        );

        // 兵七進一 (6, 2) -> (5, 2) "c3c4"
        let p_move = Move::from_iccs("c3c4").unwrap();
        assert_eq!(
            NotationConverter::to_chinese_notation(&board, &p_move).unwrap(),
            "兵七進一"
        );
    }

    #[test]
    fn test_to_chinese_notation_black_knight_move() {
        let board = BoardState::initial();
        // 执行红方 炮二平五 后轮到黑方
        let (b1, _) = board.apply_move(Move::from_iccs("h2e2").unwrap());
        assert!(!b1.turn.is_red());

        // 黑方 馬８進７ (0, 7) -> (2, 6) "h9g7"
        let b_move = Move::from_iccs("h9g7").unwrap();
        let cn = NotationConverter::to_chinese_notation(&b1, &b_move).unwrap();
        assert_eq!(cn, "馬８進７");
    }

    #[test]
    fn test_from_chinese_notation_resolves_correct_move() {
        let board = BoardState::initial();

        let move1 = NotationConverter::from_chinese_notation(&board, "炮二平五").unwrap();
        assert_eq!(move1.to_iccs(), "h2e2");

        let move2 = NotationConverter::from_chinese_notation(&board, "馬八進七").unwrap();
        assert_eq!(move2.to_iccs(), "b0c2");

        // 简体写法亦可通过几何解析兜底识别
        let move2_simplified =
            NotationConverter::from_chinese_notation(&board, "马八进七").unwrap();
        assert_eq!(move2_simplified.to_iccs(), "b0c2");

        let move3 = NotationConverter::from_chinese_notation(&board, "車一進一").unwrap();
        assert_eq!(move3.to_iccs(), "i0i1");

        // 切换至黑方走
        let (b1, _) = board.apply_move(move1);
        let black_move = NotationConverter::from_chinese_notation(&b1, "馬８進７").unwrap();
        assert_eq!(black_move.to_iccs(), "h9g7");

        // 半角支持
        let black_move_half = NotationConverter::from_chinese_notation(&b1, "馬8進7").unwrap();
        assert_eq!(black_move_half.to_iccs(), "h9g7");
    }

    #[test]
    fn test_disambiguation_double_rooks_front_and_back() {
        // 设置红方在 0 列（九路）有两个车：(5, 0) 和 (8, 0)
        let fen = "4k4/9/9/9/9/R8/9/9/R8/4K4 w";
        let board = BoardState::from_fen(fen).unwrap();

        // 前車 (5, 0) 進一至 (4, 0)
        let front_move = Move::new(Position::new(5, 0), Position::new(4, 0));
        assert_eq!(
            NotationConverter::to_chinese_notation(&board, &front_move).unwrap(),
            "前車進一"
        );

        // 后車 (8, 0) 進一至 (7, 0)
        let back_move = Move::new(Position::new(8, 0), Position::new(7, 0));
        assert_eq!(
            NotationConverter::to_chinese_notation(&board, &back_move).unwrap(),
            "后車進一"
        );

        // 反向解析
        assert_eq!(
            NotationConverter::from_chinese_notation(&board, "前車進一").unwrap(),
            front_move
        );
        assert_eq!(
            NotationConverter::from_chinese_notation(&board, "后車進一").unwrap(),
            back_move
        );
    }

    #[test]
    fn test_deduce_pv_chinese() {
        let board = BoardState::initial();
        let pv = vec!["h2e2".to_string(), "h9g7".to_string(), "b0c2".to_string()];

        let result = NotationConverter::deduce_pv_chinese(&board, &pv);
        assert_eq!(result, vec!["炮二平五", "馬８進７", "馬八進七"]);
    }
}
