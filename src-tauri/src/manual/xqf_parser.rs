use crate::core::board::BoardState;
use crate::core::notation::NotationConverter;
use crate::core::position::Move;
use crate::core::rules::MoveValidator;
use crate::manual::encoding::decode_gbk;
use crate::manual::encoding::read_gbk_string;
use crate::manual::error::ManualError;
use crate::manual::models::{ChessManual, ManualNode};

const FEN_PIECES: &str = "RNBAKABNRCCPPPPPrnbakabnrccppppp";
const KEY_SEED: &str = "[(C) Copyright Mr. Dong Shiwei.]";

/// XQF (象棋演播室) 二进制棋谱解析器
/// 严格处理文件头密钥解算、FEN 计算、字节级解密，并在首个 `\0` 处截断 GBK 定长字符串
pub struct XqfParser;

#[derive(Default)]
struct XqfHeader {
    version: usize,
    key_mask: usize,
    key_or: [usize; 4],
    key_sum: usize,
    key_xy_p: usize,
    key_xy_f: usize,
    key_xy_t: usize,
    qizi_xy: [u8; 32],
    play_step_no: usize,
    who_play: usize,
    play_result: usize,
    title: String,
    match_name: String,
    match_time: String,
    match_addr: String,
    red_player: String,
    blk_player: String,
    author: String,
}

#[derive(Default)]
struct XqfKey {
    f32: [u8; 32],
    xy_p: u8,
    xy_f: u8,
    xy_t: u8,
    rmk: u16,
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

impl XqfParser {
    pub fn parse(buffer: &[u8]) -> Result<ChessManual, ManualError> {
        if buffer.len() < 2 || buffer[0] != b'X' || buffer[1] != b'Q' {
            return Err(ManualError::InvalidData {
                msg: "Invalid XQF file header: missing 'XQ' magic bytes.".to_string(),
            });
        }

        if buffer.len() < 512 {
            return Err(ManualError::InvalidData {
                msg: "XQF file too short for header.".to_string(),
            });
        }

        let header = parse_header(buffer);
        let key = calculate_key(&header);
        let start_fen = calculate_fen(&header, &key);

        let root = parse_moves(buffer, &header, &key, &start_fen)?;

        Ok(ChessManual {
            title: if header.title.trim().is_empty() {
                "无题".to_string()
            } else {
                header.title.clone()
            },
            date: non_empty(&header.match_time),
            red_player: non_empty(&header.red_player),
            black_player: non_empty(&header.blk_player),
            event_name: non_empty(&header.match_name),
            start_fen,
            root,
        })
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn parse_header(buffer: &[u8]) -> XqfHeader {
    let mut header = XqfHeader {
        version: buffer[2] as usize,
        key_mask: buffer[3] as usize,
        key_or: [0; 4],
        ..Default::default()
    };

    for i in 0..4 {
        header.key_or[i] = buffer[8 + i] as usize;
    }
    header.key_sum = buffer[12] as usize;
    header.key_xy_p = buffer[13] as usize;
    header.key_xy_f = buffer[14] as usize;
    header.key_xy_t = buffer[15] as usize;

    header.qizi_xy.copy_from_slice(&buffer[16..48]);
    header.play_step_no = (buffer[49] as usize) * 256 + (buffer[48] as usize);
    header.who_play = buffer[50] as usize;
    header.play_result = buffer[51] as usize;

    header.title = read_gbk_string(buffer, 80, 128);
    header.match_name = read_gbk_string(buffer, 208, 64);
    header.match_time = read_gbk_string(buffer, 272, 16);
    header.match_addr = read_gbk_string(buffer, 288, 16);
    header.red_player = read_gbk_string(buffer, 304, 16);
    header.blk_player = read_gbk_string(buffer, 320, 16);
    header.author = read_gbk_string(buffer, 480, 16);

    header
}

fn calculate_key(header: &XqfHeader) -> XqfKey {
    let mut key = XqfKey::default();
    if header.version <= 10 {
        return key;
    }

    let p = header.key_xy_p;
    let f = header.key_xy_f;
    let t = header.key_xy_t;

    key.xy_p = (((p * p * 54 + 221) * p) & 0xFF) as u8;
    key.xy_f = (((f * f * 54 + 221) * (key.xy_p as usize)) & 0xFF) as u8;
    key.xy_t = (((t * t * 54 + 221) * (key.xy_f as usize)) & 0xFF) as u8;
    key.rmk = ((((header.key_sum * 256 + header.key_xy_p) % 32000) + 767) & 0xFFFF) as u16;

    let fkey = [
        (header.key_sum & header.key_mask) | (header.key_or[0] & 0xFF),
        (header.key_xy_p & header.key_mask) | (header.key_or[1] & 0xFF),
        (header.key_xy_f & header.key_mask) | (header.key_or[2] & 0xFF),
        (header.key_xy_t & header.key_mask) | (header.key_or[3] & 0xFF),
    ];

    let seed_bytes = KEY_SEED.as_bytes();
    for i in 0..32 {
        key.f32[i] = (fkey[i % 4] & (seed_bytes[i] as usize)) as u8;
    }

    key
}

fn calculate_fen(header: &XqfHeader, key: &XqfKey) -> String {
    let mut fen_array = [b' '; 90];
    let pieces = FEN_PIECES.as_bytes();

    for i in 0..32 {
        let piece_key: usize;
        let piece_pos: usize;

        if header.version > 10 {
            piece_key = (key.xy_p as usize + i + 1) & 31;
            piece_pos = ((header.qizi_xy[i] as usize).wrapping_sub(key.xy_p as usize)) & 0xFF;
        } else {
            piece_key = i;
            piece_pos = header.qizi_xy[i] as usize;
        }

        if piece_pos < 90 {
            let x = piece_pos / 10;
            let y = 9usize.saturating_sub(piece_pos % 10);
            let index = y * 9 + x;
            if index < 90 {
                fen_array[index] = pieces[piece_key];
            }
        }
    }

    let mut out = String::with_capacity(64);
    for row in 0..10 {
        if row > 0 {
            out.push('/');
        }
        let mut empty = 0usize;
        for col in 0..9 {
            let c = fen_array[row * 9 + col] as char;
            if c == ' ' {
                empty += 1;
            } else {
                if empty > 0 {
                    out.push_str(&empty.to_string());
                    empty = 0;
                }
                out.push(c);
            }
        }
        if empty > 0 {
            out.push_str(&empty.to_string());
        }
    }

    out.push_str(if header.who_play == 1 { " b" } else { " w" });
    out
}

fn parse_moves(
    buffer: &[u8],
    header: &XqfHeader,
    key: &XqfKey,
    start_fen: &str,
) -> Result<ManualNode, ManualError> {
    let mut arena = TreeArena::new();
    let root_idx = arena.push(ArenaNode {
        id: 0,
        mv: None,
        chinese_notation: "开始局面".to_string(),
        comment: None,
        children: Vec::new(),
    });

    if buffer.len() <= 1024 {
        return Ok(arena_to_node(&arena, root_idx));
    }

    let data_len = buffer.len() - 1024;
    let mut decode = vec![0u8; data_len];
    for i in 0..data_len {
        if header.version > 10 {
            decode[i] =
                ((buffer[1024 + i] as usize).wrapping_sub(key.f32[i % 32] as usize) & 0xFF) as u8;
        } else {
            decode[i] = buffer[1024 + i];
        }
    }

    let mut change_stack: Vec<(usize, BoardState)> = Vec::new();
    let mut parent = root_idx;
    let mut current_board = match BoardState::from_fen(start_fen) {
        Ok(b) => b,
        Err(error) => {
            return Err(ManualError::InvalidData {
                msg: format!("XQF start position is invalid: {error}"),
            });
        }
    };

    let mut pos = 0usize;
    let mut move_id = 0u32;

    while pos + 4 <= decode.len() {
        let mut comment_len: i32 = 0;
        let mut next_offset: usize = 4;
        let has_next: bool;
        let has_change: bool;

        if header.version > 10 {
            has_next = (decode[pos + 2] & 0x80) != 0;
            has_change = (decode[pos + 2] & 0x40) != 0;
            if (decode[pos + 2] & 0x20) != 0 {
                comment_len = read_int32(&decode, pos + 4) - key.rmk as i32;
                if comment_len < 0 {
                    comment_len = 0;
                }
                next_offset = comment_len as usize + 8;
            }
        } else {
            has_next = (decode[pos + 2] & 0xF0) != 0;
            has_change = (decode[pos + 2] & 0x0F) != 0;
            comment_len = read_int32(&decode, pos + 4);
            if comment_len < 0 {
                comment_len = 0;
            }
            next_offset = comment_len as usize + 8;
        }

        let comment = if comment_len > 0 && pos + 8 + comment_len as usize <= decode.len() {
            let raw = decode_gbk(&decode[pos + 8..pos + 8 + comment_len as usize]);
            Some(raw.trim_end_matches(['\0', ' ', '\r', '\n']).to_string())
        } else {
            None
        };

        if pos == 0 {
            if let Some(c) = &comment
                && !c.is_empty()
            {
                arena.nodes[root_idx].comment = Some(c.clone());
            }
            pos += if has_next { next_offset } else { decode.len() };
            continue;
        }

        let pf = ((decode[pos] as i32 - 24 - key.xy_f as i32) & 0xFF) as usize;
        let pt = ((decode[pos + 1] as i32 - 32 - key.xy_t as i32) & 0xFF) as usize;

        let from_x = (pf / 10).clamp(0, 8);
        let from_y = (pf % 10).clamp(0, 9);
        let to_x = (pt / 10).clamp(0, 8);
        let to_y = (pt % 10).clamp(0, 9);

        let iccs = format!(
            "{}{}{}{}",
            (b'a' + from_x as u8) as char,
            from_y,
            (b'a' + to_x as u8) as char,
            to_y
        );

        let mv = Move::from_iccs(&iccs).map_err(|error| ManualError::InvalidData {
            msg: format!("XQF contains invalid move '{iccs}': {error}"),
        })?;
        if !MoveValidator::can_move(&current_board, mv) {
            return Err(ManualError::InvalidData {
                msg: format!("XQF contains illegal move '{iccs}'"),
            });
        }

        move_id += 1;
        let cn = NotationConverter::to_chinese_notation(&current_board, &mv)
            .unwrap_or_else(|_| iccs.clone());

        let new_idx = arena.push(ArenaNode {
            id: move_id,
            mv: Some(mv),
            chinese_notation: cn,
            comment,
            children: Vec::new(),
        });
        arena.nodes[parent].children.push(new_idx);

        let next_board = current_board.apply_move(mv).0;

        if has_next {
            if has_change {
                change_stack.push((parent, current_board));
            }
            parent = new_idx;
            current_board = next_board;
        } else if !has_change && !change_stack.is_empty() {
            let (last_node, last_board) =
                change_stack.pop().ok_or_else(|| ManualError::InvalidData {
                    msg: "XQF variation stack is inconsistent".to_string(),
                })?;
            parent = last_node;
            current_board = last_board;
        }

        pos += next_offset;
    }

    Ok(arena_to_node(&arena, root_idx))
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

fn read_int32(data: &[u8], offset: usize) -> i32 {
    if offset + 4 > data.len() {
        return 0;
    }
    (data[offset] as i32)
        | ((data[offset + 1] as i32) << 8)
        | ((data[offset + 2] as i32) << 16)
        | ((data[offset + 3] as i32) << 24)
}
