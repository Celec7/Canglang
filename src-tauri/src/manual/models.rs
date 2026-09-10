use crate::core::position::Move;
use serde::{Deserialize, Serialize};

/// 棋谱变例树节点（多叉树结构）
/// `children[0]` 为主线后继节点，`children[1..]` 为变例分支节点
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ManualNode {
    pub id: u32,
    pub mv: Option<Move>,
    pub chinese_notation: String,
    pub comment: Option<String>,
    pub score: Option<i32>,
    pub children: Vec<ManualNode>,
}

impl ManualNode {
    pub fn new(id: u32, mv: Option<Move>, chinese_notation: impl Into<String>) -> Self {
        Self {
            id,
            mv,
            chinese_notation: chinese_notation.into(),
            comment: None,
            score: None,
            children: Vec::new(),
        }
    }

    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    pub fn with_score(mut self, score: i32) -> Self {
        self.score = Some(score);
        self
    }
}

/// 完整棋谱数据模型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ChessManual {
    pub title: String,
    pub date: Option<String>,
    pub red_player: Option<String>,
    pub black_player: Option<String>,
    pub event_name: Option<String>,
    pub start_fen: String,
    pub root: ManualNode,
}

impl ChessManual {
    pub fn new(title: impl Into<String>, start_fen: impl Into<String>, root: ManualNode) -> Self {
        Self {
            title: title.into(),
            date: None,
            red_player: None,
            black_player: None,
            event_name: None,
            start_fen: start_fen.into(),
            root,
        }
    }
}
