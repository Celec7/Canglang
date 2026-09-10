use crate::core::board::INITIAL_FEN;
use crate::manual::models::{ChessManual, ManualNode};

/// 将 ChessManual 变例树导出为标准化中国象棋 PGN 文本
pub struct PgnExporter;

impl PgnExporter {
    pub fn export(manual: &ChessManual, use_chinese_notation: bool) -> String {
        let mut sb = String::new();

        // 1. Tag 元数据
        push_line(&mut sb, "[Game \"Chinese Chess\"]");
        push_line(
            &mut sb,
            &format!("[Event \"{}\"]", escape_tag_value(&manual.title)),
        );
        if let Some(date) = non_empty(&manual.date) {
            push_line(&mut sb, &format!("[Date \"{}\"]", escape_tag_value(date)));
        }
        if let Some(red) = non_empty(&manual.red_player) {
            push_line(&mut sb, &format!("[Red \"{}\"]", escape_tag_value(red)));
        }
        if let Some(black) = non_empty(&manual.black_player) {
            push_line(&mut sb, &format!("[Black \"{}\"]", escape_tag_value(black)));
        }
        push_line(&mut sb, "[Result \"*\"]");

        if !manual.start_fen.trim().is_empty()
            && !manual.start_fen.trim().eq_ignore_ascii_case(INITIAL_FEN)
        {
            push_line(
                &mut sb,
                &format!("[FEN \"{}\"]", escape_tag_value(manual.start_fen.trim())),
            );
        }

        push_line(
            &mut sb,
            &format!(
                "[Format \"{}\"]",
                if use_chinese_notation {
                    "Chinese"
                } else {
                    "ICCS"
                }
            ),
        );
        sb.push('\n');

        // 2. 走法区与变例树
        let mut move_sb = String::new();
        if let Some(remark) = non_empty(&manual.root.comment) {
            move_sb.push('{');
            move_sb.push_str(&escape_comment(remark));
            move_sb.push_str("} ");
        }

        if !manual.root.children.is_empty() {
            export_children(&mut move_sb, &manual.root.children, 1, use_chinese_notation);
        }

        move_sb.push_str(" *");
        push_line(&mut sb, move_sb.trim());

        sb
    }
}

fn non_empty(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.trim().is_empty())
}

fn escape_tag_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_comment(value: &str) -> String {
    value.replace('\\', "\\\\").replace('}', "\\}")
}

fn push_line(sb: &mut String, line: &str) {
    sb.push_str(line);
    sb.push('\n');
}

fn export_children(sb: &mut String, children: &[ManualNode], ply: usize, use_chinese: bool) {
    if children.is_empty() {
        return;
    }

    let main = &children[0];
    append_move_node(sb, main, ply, use_chinese);

    for var_node in children.iter().skip(1) {
        sb.push_str(" (");
        append_move_node(sb, var_node, ply, use_chinese);
        if !var_node.children.is_empty() {
            export_children(sb, &var_node.children, ply + 1, use_chinese);
        }
        sb.push(')');
    }

    if !main.children.is_empty() {
        export_children(sb, &main.children, ply + 1, use_chinese);
    }
}

fn append_move_node(sb: &mut String, node: &ManualNode, ply: usize, use_chinese: bool) {
    let is_red = ply % 2 == 1;
    let move_number = ply.div_ceil(2);

    if is_red {
        if sb.chars().last().is_some_and(|c| c != '(' && c != ' ') {
            sb.push(' ');
        }
        sb.push_str(&format!("{move_number}. "));
    } else if sb.chars().last().is_some_and(|c| c != ' ') {
        sb.push(' ');
    }

    let notation = if use_chinese {
        if !node.chinese_notation.trim().is_empty() {
            node.chinese_notation.clone()
        } else {
            node.mv.map(|m| m.to_iccs()).unwrap_or_default()
        }
    } else {
        node.mv
            .map(|m| m.to_iccs())
            .unwrap_or_else(|| node.chinese_notation.clone())
    };

    sb.push_str(&notation);

    if let Some(remark) = non_empty(&node.comment) {
        sb.push_str(" {");
        sb.push_str(&escape_comment(remark));
        sb.push('}');
    }
}
