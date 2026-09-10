use crate::core::board::{BoardState, INITIAL_FEN};
use crate::core::notation::NotationConverter;
use crate::core::position::Move;
use crate::core::rules::MoveValidator;
use crate::manual::error::ManualError;
use crate::manual::models::{ChessManual, ManualNode};
use std::collections::HashMap;

/// 中国象棋 PGN 文本解析器：支持 Tag 元数据、ICCS/中文招法识别、嵌套变例分支与走法注释
pub struct PgnParser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    Move,
    Comment,
    OpenParen,
    CloseParen,
    Result,
}

#[derive(Debug, Clone)]
struct Token {
    ttype: TokenType,
    text: String,
}

struct ArenaNode {
    id: u32,
    mv: Option<Move>,
    chinese_notation: String,
    comment: Option<String>,
    children: Vec<usize>,
}

struct TreeArena {
    nodes: Vec<ArenaNode>,
}

impl TreeArena {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn push(&mut self, node: ArenaNode) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }
}

impl PgnParser {
    pub fn parse(pgn_text: &str) -> Result<ChessManual, ManualError> {
        if pgn_text.trim().is_empty() {
            return Err(ManualError::Parse {
                msg: "empty PGN text".to_string(),
            });
        }

        let mut tags: HashMap<String, String> = HashMap::new();
        let mut move_text = String::new();
        let mut in_tags = true;

        for raw_line in pgn_text.lines() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                if !tags.is_empty() {
                    in_tags = false;
                }
                continue;
            }

            if in_tags && trimmed.starts_with('[') {
                let tag = parse_tag(trimmed).ok_or_else(|| ManualError::Parse {
                    msg: format!("invalid PGN tag: {trimmed}"),
                })?;
                tags.insert(tag.0, tag.1);
                continue;
            }

            in_tags = false;
            move_text.push_str(raw_line);
            move_text.push('\n');
        }

        let mut start_fen = tags
            .get("FEN")
            .cloned()
            .unwrap_or_else(|| INITIAL_FEN.to_string());
        start_fen = start_fen.trim().to_string();
        let board = BoardState::from_fen(&start_fen)
            .map_err(|e| ManualError::Parse { msg: e.to_string() })?;

        let mut arena = TreeArena::new();
        arena.push(ArenaNode {
            id: 0,
            mv: None,
            chinese_notation: "开始局面".to_string(),
            comment: None,
            children: Vec::new(),
        });

        let tokens = tokenize(&move_text)?;
        let mut index = 0;
        let mut next_id = 1u32;
        if !tokens.is_empty() {
            parse_branch(
                &tokens,
                &mut index,
                &mut arena,
                0,
                &board,
                &mut next_id,
                false,
            )?;
        }
        if index != tokens.len() {
            return Err(ManualError::Parse {
                msg: "unexpected tokens after move text".to_string(),
            });
        }

        let root_node = arena_to_node(&arena, 0);

        Ok(ChessManual {
            title: tags
                .get("Event")
                .or_else(|| tags.get("Title"))
                .cloned()
                .unwrap_or_else(|| "无题".to_string()),
            date: tags.get("Date").cloned(),
            red_player: tags.get("Red").cloned(),
            black_player: tags.get("Black").cloned(),
            event_name: tags.get("Event").cloned(),
            start_fen,
            root: root_node,
        })
    }
}

fn parse_tag(line: &str) -> Option<(String, String)> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?.trim();
    let space = inner.find(char::is_whitespace)?;
    if space == 0 {
        return None;
    }
    let name = inner[..space].to_string();
    let value_part = inner[space..].trim_start();
    let chars: Vec<char> = value_part.chars().collect();
    if chars.first() != Some(&'"') {
        return None;
    }
    let mut value = String::new();
    let mut escaped = false;
    for (index, ch) in chars.iter().copied().enumerate().skip(1) {
        if escaped {
            value.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            if chars[index + 1..].iter().any(|rest| !rest.is_whitespace()) {
                return None;
            }
            return Some((name, value));
        } else {
            value.push(ch);
        }
    }
    None
}

fn tokenize(s: &str) -> Result<Vec<Token>, ManualError> {
    let chars: Vec<char> = s.chars().collect();
    let mut list = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c == '(' {
            list.push(Token {
                ttype: TokenType::OpenParen,
                text: "(".to_string(),
            });
            i += 1;
            continue;
        }

        if c == ')' {
            list.push(Token {
                ttype: TokenType::CloseParen,
                text: ")".to_string(),
            });
            i += 1;
            continue;
        }

        if c == '{' {
            let mut end = i + 1;
            let mut escaped = false;
            while end < chars.len() {
                if escaped {
                    escaped = false;
                } else if chars[end] == '\\' {
                    escaped = true;
                } else if chars[end] == '}' {
                    break;
                }
                end += 1;
            }
            if end == chars.len() {
                return Err(ManualError::Parse {
                    msg: "unterminated PGN comment".to_string(),
                });
            }
            let raw_comment: String = chars[i + 1..end].iter().collect();
            let comment = unescape_comment(&raw_comment);
            list.push(Token {
                ttype: TokenType::Comment,
                text: comment.trim().to_string(),
            });
            i = if end < chars.len() { end + 1 } else { end };
            continue;
        }

        let start = i;
        while i < chars.len()
            && !chars[i].is_whitespace()
            && chars[i] != '('
            && chars[i] != ')'
            && chars[i] != '{'
            && chars[i] != '}'
        {
            i += 1;
        }

        let word_str: String = chars[start..i].iter().collect();
        let mut word: &str = word_str.trim();
        if word.is_empty() {
            continue;
        }

        if let Some(rest) = strip_move_number_prefix(word) {
            if rest.is_empty() {
                continue;
            }
            word = rest;
        } else if is_move_number(word) {
            continue;
        }

        if matches!(word, "1-0" | "0-1" | "1/2-1/2" | "*") {
            list.push(Token {
                ttype: TokenType::Result,
                text: word.to_string(),
            });
            continue;
        }

        list.push(Token {
            ttype: TokenType::Move,
            text: word.to_string(),
        });
    }

    Ok(list)
}

fn unescape_comment(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut escaped = false;
    for ch in raw.chars() {
        if escaped {
            result.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            result.push(ch);
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

fn strip_move_number_prefix(word: &str) -> Option<&str> {
    let trimmed = word.trim_start();
    let dot_pos = trimmed.find('.')?;
    let (prefix, rest) = trimmed.split_at(dot_pos);
    if prefix.parse::<u64>().is_ok() {
        Some(rest.trim_start_matches('.'))
    } else {
        None
    }
}

fn is_move_number(word: &str) -> bool {
    if word.ends_with('.') {
        return true;
    }
    if let Some((head, _)) = word.split_once('.') {
        return head.parse::<i64>().is_ok();
    }
    false
}

#[allow(clippy::too_many_lines)]
fn parse_branch(
    tokens: &[Token],
    index: &mut usize,
    arena: &mut TreeArena,
    branch_root: usize,
    initial_board: &BoardState,
    next_id: &mut u32,
    expect_close: bool,
) -> Result<(), ManualError> {
    let mut current_node = branch_root;
    let mut current_board = *initial_board;
    let mut last_move_node: Option<usize> = None;
    let mut last_move_context: Option<(usize, BoardState)> = None;

    while *index < tokens.len() {
        let token = &tokens[*index].clone();

        match token.ttype {
            TokenType::CloseParen => {
                if !expect_close {
                    return Err(ManualError::Parse {
                        msg: "unexpected closing variation".to_string(),
                    });
                }
                *index += 1;
                return Ok(());
            }

            TokenType::Comment => {
                let text = token.text.clone();
                if text.trim().is_empty() {
                    *index += 1;
                    continue;
                }
                let target = last_move_node.unwrap_or(branch_root);
                let node = &mut arena.nodes[target];
                node.comment = Some(match node.comment.take() {
                    Some(existing) => format!("{existing}\n{text}"),
                    None => text,
                });
                *index += 1;
                continue;
            }

            TokenType::OpenParen => {
                *index += 1;
                let (variation_root, variation_board) =
                    last_move_context.unwrap_or((current_node, current_board));
                parse_branch(
                    tokens,
                    index,
                    arena,
                    variation_root,
                    &variation_board,
                    next_id,
                    true,
                )?;
                continue;
            }

            TokenType::Result => {
                *index += 1;
                break;
            }

            TokenType::Move => {
                *index += 1;
                let text = token.text.clone();
                let (mv, cn) =
                    try_parse_move(&current_board, &text).map_err(|reason| ManualError::Parse {
                        msg: format!("invalid move '{text}': {reason}"),
                    })?;
                let parent_node = current_node;
                let board_before_move = current_board;
                let new_idx = arena.push(ArenaNode {
                    id: *next_id,
                    mv: Some(mv),
                    chinese_notation: cn,
                    comment: None,
                    children: Vec::new(),
                });
                *next_id += 1;
                arena.nodes[current_node].children.push(new_idx);
                current_board = current_board.apply_move(mv).0;
                current_node = new_idx;
                last_move_node = Some(new_idx);
                last_move_context = Some((parent_node, board_before_move));
            }
        }
    }

    if expect_close {
        return Err(ManualError::Parse {
            msg: "unterminated variation".to_string(),
        });
    }

    Ok(())
}

fn try_parse_move(board: &BoardState, text: &str) -> Result<(Move, String), String> {
    let raw = text.trim();
    let stripped = raw.trim_end_matches(['+', '#', '!', '?']);
    let clean = stripped.replace('-', "");

    if let Ok(mv) = Move::from_iccs(&clean) {
        if !MoveValidator::can_move(board, mv) {
            return Err("move is not legal in the current position".to_string());
        }
        let cn = NotationConverter::to_chinese_notation(board, &mv)
            .map_err(|error| error.to_string())?;
        return Ok((mv, cn));
    }

    if let Ok(mv) = NotationConverter::from_chinese_notation(board, stripped) {
        if !MoveValidator::can_move(board, mv) {
            return Err("move is not legal in the current position".to_string());
        }
        return Ok((mv, stripped.to_string()));
    }

    Err("unrecognized ICCS or Chinese notation".to_string())
}

fn arena_to_node(arena: &TreeArena, idx: usize) -> ManualNode {
    let n = &arena.nodes[idx];
    ManualNode {
        id: n.id,
        mv: n.mv,
        chinese_notation: n.chinese_notation.clone(),
        comment: n.comment.clone(),
        score: None,
        children: n
            .children
            .iter()
            .map(|&c| arena_to_node(arena, c))
            .collect(),
    }
}
