use crate::core::piece::{Color, PieceKind};
use crate::core::position::Position;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JieqiError {
    #[error("揭棋运行棋子数量应在 2 到 32 之间，实际为 {0}")]
    InvalidPieceCount(usize),
    #[error("揭棋身份数量应为 32，实际为 {0}")]
    InvalidIdentityCount(usize),
    #[error("棋子 {0} 使用了无效的稳定 ID")]
    InvalidPieceId(u8),
    #[error("棋子 {id} 的阵营与初始棋位不一致")]
    InvalidPieceColor { id: u8 },
    #[error("棋子 {id} 的公开角色与初始棋位不一致")]
    InvalidMoveRole { id: u8 },
    #[error("棋位越界: {0}")]
    InvalidPosition(Position),
    #[error("重复棋位: {0}")]
    DuplicatePosition(Position),
    #[error("重复棋子 ID: {0}")]
    DuplicatePieceId(u8),
    #[error("重复身份 ID: {0}")]
    DuplicateIdentityId(u8),
    #[error("将帅 {0} 必须固定、明置且身份与角色一致")]
    InvalidKing(u8),
    #[error("{color} 方身份集合不合法，{kind:?} 数量应为 {expected}，实际为 {actual}")]
    InvalidIdentityMultiset {
        color: Color,
        kind: PieceKind,
        expected: usize,
        actual: usize,
    },
    #[error("公开揭子记录引用了无效棋子 ID: {0}")]
    InvalidRevealId(u8),
    #[error("公开揭子记录不能把暗子揭为将帅: {0}")]
    RecordedKingReveal(u8),
    #[error("重复公开揭子记录: {0}")]
    DuplicateRevealId(u8),
}
