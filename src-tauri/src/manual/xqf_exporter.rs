use crate::core::board::BoardState;
use crate::core::piece::Color;
use crate::core::position::Position;
use crate::manual::encoding::encode_gbk_strict;
use crate::manual::error::ManualError;
use crate::manual::models::{ChessManual, ManualNode};

const HEADER_SIZE: usize = 1024;
const FEN_PIECES: &[u8] = b"RNBAKABNRCCPPPPPrnbakabnrccppppp";

/// XQF v10 规范化导出器
///
/// v10 的公开布局只表达一条线，因此多分支棋谱必须在编码前显式拒绝
pub struct XqfExporter;

impl XqfExporter {
    pub fn export(manual: &ChessManual, version: u8) -> Result<Vec<u8>, ManualError> {
        if version != 10 {
            return Err(ManualError::InvalidData {
                msg: format!("XQF export only supports canonical version 10, got {version}"),
            });
        }
        if manual.root.mv.is_some() {
            return Err(ManualError::InvalidData {
                msg: "XQF export requires a root node without a move".to_string(),
            });
        }
        validate_single_line(&manual.root)?;

        let board =
            BoardState::from_fen(&manual.start_fen).map_err(|error| ManualError::InvalidData {
                msg: format!("XQF export cannot encode start FEN: {error}"),
            })?;
        let mut mainline = Vec::new();
        collect_mainline(&manual.root, &mut mainline);
        if mainline.len() > u16::MAX as usize {
            return Err(ManualError::InvalidData {
                msg: "XQF export contains too many moves".to_string(),
            });
        }

        let mut output = vec![0u8; HEADER_SIZE];
        output[0..2].copy_from_slice(b"XQ");
        output[2] = version;
        output[16..48].copy_from_slice(&piece_positions(&board)?);
        let move_count = mainline.len() as u16;
        output[48..50].copy_from_slice(&move_count.to_le_bytes());
        output[50] = if board.turn == Color::Black { 1 } else { 0 };

        write_gbk_field(&mut output, 80, 64, &manual.title, "title")?;
        write_gbk_field(
            &mut output,
            208,
            64,
            manual.event_name.as_deref().unwrap_or(""),
            "event",
        )?;
        write_gbk_field(
            &mut output,
            272,
            16,
            manual.date.as_deref().unwrap_or(""),
            "date",
        )?;
        write_gbk_field(
            &mut output,
            304,
            16,
            manual.red_player.as_deref().unwrap_or(""),
            "red player",
        )?;
        write_gbk_field(
            &mut output,
            320,
            16,
            manual.black_player.as_deref().unwrap_or(""),
            "black player",
        )?;

        append_record(
            &mut output,
            0x18,
            0x20,
            !mainline.is_empty(),
            manual.root.comment.as_deref(),
            true,
        )?;
        for (index, node) in mainline.iter().enumerate() {
            let mv = node.mv.ok_or_else(|| ManualError::InvalidData {
                msg: "XQF export encountered a move-less non-root node".to_string(),
            })?;
            if !mv.from.is_valid() || !mv.to.is_valid() {
                return Err(ManualError::InvalidData {
                    msg: format!("XQF export encountered an invalid move: {mv}"),
                });
            }
            let from = to_xqf_position(mv.from);
            let to = to_xqf_position(mv.to);
            append_record(
                &mut output,
                from.saturating_add(24),
                to.saturating_add(32),
                index + 1 < mainline.len(),
                node.comment.as_deref(),
                false,
            )?;
        }

        Ok(output)
    }
}

fn validate_single_line(node: &ManualNode) -> Result<(), ManualError> {
    if node.children.len() > 1 {
        return Err(ManualError::InvalidData {
            msg: "canonical XQF v10 export does not support variations".to_string(),
        });
    }
    for child in &node.children {
        if child.mv.is_none() {
            return Err(ManualError::InvalidData {
                msg: "XQF export requires every non-root node to contain a move".to_string(),
            });
        }
        validate_single_line(child)?;
    }
    Ok(())
}

fn collect_mainline<'a>(root: &'a ManualNode, result: &mut Vec<&'a ManualNode>) {
    let mut current = root.children.first();
    while let Some(node) = current {
        result.push(node);
        current = node.children.first();
    }
}

fn piece_positions(board: &BoardState) -> Result<[u8; 32], ManualError> {
    let mut positions = [0xff; 32];
    let mut used = [false; 90];

    for (slot, expected) in FEN_PIECES.iter().enumerate() {
        for (index, piece) in board.pieces.iter().enumerate() {
            if !used[index] && piece.map(|value| value.to_char() as u8) == Some(*expected) {
                positions[slot] = to_xqf_position(Position::from_index(index));
                used[index] = true;
                break;
            }
        }
    }

    if board
        .pieces
        .iter()
        .enumerate()
        .any(|(index, piece)| piece.is_some() && !used[index])
    {
        return Err(ManualError::InvalidData {
            msg: "start FEN contains more pieces than canonical XQF can represent".to_string(),
        });
    }
    Ok(positions)
}

fn to_xqf_position(position: Position) -> u8 {
    position.col.saturating_mul(10) + (9 - position.row)
}

fn append_record(
    output: &mut Vec<u8>,
    from: u8,
    to: u8,
    has_next: bool,
    comment: Option<&str>,
    root: bool,
) -> Result<(), ManualError> {
    let comment = comment.filter(|text| !text.trim().is_empty());
    let encoded = match comment {
        Some(text) => encode_gbk_strict(text, "comment")?,
        None => Vec::new(),
    };
    let length = i32::try_from(encoded.len()).map_err(|_| ManualError::InvalidData {
        msg: "XQF comment is too long".to_string(),
    })?;
    output.extend([
        from,
        to,
        if has_next { 0xf0 } else { 0 },
        if root { 0xff } else { 0 },
    ]);
    output.extend(length.to_le_bytes());
    output.extend(encoded);
    Ok(())
}

fn write_gbk_field(
    output: &mut [u8],
    offset: usize,
    capacity: usize,
    text: &str,
    field: &str,
) -> Result<(), ManualError> {
    let encoded = encode_gbk_strict(text, field)?;
    if encoded.len() >= capacity {
        return Err(ManualError::InvalidData {
            msg: format!(
                "XQF {field} is too long: {} bytes, maximum is {}",
                encoded.len(),
                capacity - 1
            ),
        });
    }
    output[offset] = encoded.len() as u8;
    output[offset + 1..offset + 1 + encoded.len()].copy_from_slice(&encoded);
    Ok(())
}
