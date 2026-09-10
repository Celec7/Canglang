use crate::core::board::BoardState;
use crate::core::piece::{Color, Piece, PieceKind};
use crate::core::position::{Move, Position};
use serde::{Deserialize, Serialize};

use super::MoveValidator;

/// 规则裁判关注的走法作用
/// 可直接确认的类别与待细化的规则类别分开保存
/// `Capture`、`Check` 和 `Checkmate` 可以从当前棋盘直接确认；兑、捉、献、拦
/// 等需要结合循环中的前后手、被攻击子力和规则案例判断，先保留为稳定的分类值，
/// 由后续裁判器逐步细化，不在信息不足时臆判
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum MoveEffect {
    Check,
    Checkmate,
    Capture,
    Exchange,
    Chase,
    JointChase,
    Sacrifice,
    Block,
    Idle,
    #[default]
    Pending,
}

/// 一次捉候选中被攻击目标的稳定描述
///
/// 不使用棋盘坐标作为身份，因为目标棋子可以在循环中移动；当同一阵营存在多个
/// 同类棋子时，裁判器会保持待裁判而不会臆测它们是同一枚棋子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChaseTarget {
    pub color: Color,
    pub kind: PieceKind,
    pub ambiguous: bool,
    #[serde(default)]
    pub attacker_kind: Option<PieceKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveClassification {
    pub effect: MoveEffect,
    pub chase_targets: Vec<ChaseTarget>,
}

/// 对一手合法棋步执行可直接验证的基础分类
pub fn classify_move(board: &BoardState, mv: Move) -> MoveEffect {
    classify_move_details(board, mv).effect
}

/// 返回走法作用及其可验证的捉目标上下文
pub fn classify_move_details(board: &BoardState, mv: Move) -> MoveClassification {
    let captured = board.get_piece(mv.to).is_some();
    let (next_board, _) = board.apply_move(mv);
    let opponent = board.turn.opposite();

    if MoveValidator::is_checkmate(&next_board, opponent) {
        MoveClassification {
            effect: MoveEffect::Checkmate,
            chase_targets: Vec::new(),
        }
    } else if MoveValidator::is_in_check(&next_board, opponent) {
        MoveClassification {
            effect: MoveEffect::Check,
            chase_targets: Vec::new(),
        }
    } else if captured {
        MoveClassification {
            effect: if offers_moved_piece(&next_board, mv) {
                MoveEffect::Exchange
            } else {
                MoveEffect::Capture
            },
            chase_targets: Vec::new(),
        }
    } else if let Some((effect, chase_targets)) = classify_chase(board, &next_board) {
        MoveClassification {
            effect,
            chase_targets,
        }
    } else if offers_moved_piece(&next_board, mv) {
        MoveClassification {
            effect: MoveEffect::Sacrifice,
            chase_targets: Vec::new(),
        }
    } else {
        MoveClassification {
            effect: MoveEffect::Idle,
            chase_targets: Vec::new(),
        }
    }
}

/// 判断目标棋子是否属于象棋规则中受“捉”限制的有效目标
///
/// 依据象棋竞赛规则：
/// 1. 凡走子企图在下一招吃掉对方棋子，并对其产生现实威胁者，统称“捉”；
/// 2. 下列情况不按“捉”论（属于“闲”）：
///    - 将（帅）不受“捉”的定义约束（将被攻击为将军而非捉；将帅产生的吃子威胁一律按闲论）；
///    - 追捉士、相（象），无论有根无根，一律按闲论
pub fn is_valid_chase_target(
    attacker: Option<Piece>,
    target: Piece,
    _target_pos: Position,
) -> bool {
    if target.kind == PieceKind::King {
        return false;
    }

    if let Some(att) = attacker
        && att.kind == PieceKind::King
    {
        return false;
    }

    // 追捉士、相（象）一律不算捉，按“闲”论
    if matches!(target.kind, PieceKind::Advisor | PieceKind::Bishop) {
        return false;
    }

    true
}

/// 判断走法是否首次制造了对非将帅棋子的下一手合法吃子威胁
///
/// 这是“捉”的可计算子集：兑、献、拦、联合捉子和有根/假根判断仍需结合循环
/// 上下文，不能仅凭一个局面下结论
fn classify_chase(
    before: &BoardState,
    after: &BoardState,
) -> Option<(MoveEffect, Vec<ChaseTarget>)> {
    let mover = before.turn;
    let before_targets = capture_threats(before, mover);
    let after_targets = capture_threats(after, mover);
    let mut targets = Vec::new();
    let mut joint = false;

    for threat in after_targets.iter().filter(|threat| {
        !before_targets
            .iter()
            .any(|previous| previous.from == threat.from && previous.to == threat.to)
    }) {
        let Some(piece) = after.get_piece(threat.to) else {
            continue;
        };
        let attacker = after.get_piece(threat.from);
        if !is_valid_chase_target(attacker, piece, threat.to)
            || has_real_root(after, mover, *threat)
        {
            continue;
        }

        let target = ChaseTarget {
            color: piece.color,
            kind: piece.kind,
            ambiguous: after
                .pieces
                .iter()
                .flatten()
                .filter(|candidate| candidate.color == piece.color && candidate.kind == piece.kind)
                .count()
                > 1,
            attacker_kind: attacker.map(|attacker| attacker.kind),
        };
        if !targets.contains(&target) {
            targets.push(target);
        }
        if after_targets
            .iter()
            .filter(|candidate| candidate.to == threat.to)
            .count()
            >= 2
        {
            joint = true;
        }
    }

    if targets.is_empty() {
        None
    } else {
        Some((
            if joint {
                MoveEffect::JointChase
            } else {
                MoveEffect::Chase
            },
            targets,
        ))
    }
}

fn capture_threats(
    board: &BoardState,
    color: crate::core::piece::Color,
) -> Vec<crate::core::position::Move> {
    MoveValidator::get_all_legal_moves_for_side(board, color)
        .into_iter()
        .filter(|mv| {
            let Some(piece) = board.get_piece(mv.to) else {
                return false;
            };
            let attacker = board.get_piece(mv.from);
            piece.color != color && is_valid_chase_target(attacker, piece, mv.to)
        })
        .collect()
}

/// 判断刚走的棋子是否进入对方下一手的合法吃子范围，作为“献”的保守候选
fn offers_moved_piece(after: &BoardState, mv: Move) -> bool {
    let Some(piece) = after.get_piece(mv.to) else {
        return false;
    };

    MoveValidator::get_all_legal_moves_for_side(after, piece.color.opposite())
        .into_iter()
        .any(|reply| reply.to == mv.to)
}

/// 判断目标棋子是否有一枚真正能够吃回攻击者的保护子
///
/// 必须在“攻击者已经吃到目标”的假想局面中重新生成合法着法；只按伪攻击判断
/// 会把被牵制的保护子误认为真根，从而漏掉长捉候选
fn has_real_root(
    board: &BoardState,
    attacker: crate::core::piece::Color,
    threat: crate::core::position::Move,
) -> bool {
    let mut attack_board = *board;
    attack_board.turn = attacker;
    let (after_capture, _) = attack_board.apply_move(threat);
    let defender = attacker.opposite();

    MoveValidator::get_all_legal_moves_for_side(&after_capture, defender)
        .into_iter()
        .any(|reply| reply.to == threat.to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::board::BoardState;

    #[test]
    fn classifies_a_capture_without_overclaiming_rule_effect() {
        let board = BoardState::from_fen("3k5/9/9/9/9/4p4/4R4/9/9/5K3 w").unwrap();
        let mv = Move::from_iccs("e3e4").unwrap();

        assert_eq!(classify_move(&board, mv), MoveEffect::Capture);
    }

    #[test]
    fn classifies_a_check() {
        let board = BoardState::from_fen("4k4/9/9/9/9/9/9/9/4R4/4K4 w").unwrap();
        let mv = Move::from_iccs("e1e8").unwrap();

        assert_eq!(classify_move(&board, mv), MoveEffect::Check);
    }

    #[test]
    fn classifies_a_new_capture_threat_as_chase() {
        let board = BoardState::from_fen("4k4/9/9/9/9/9/9/4p4/R8/4K4 w").unwrap();
        let mv = Move::from_iccs("a1e1").unwrap();

        assert_eq!(classify_move(&board, mv), MoveEffect::Chase);
    }

    #[test]
    fn does_not_classify_a_capture_threat_on_a_real_root_as_chase() {
        let board = BoardState::from_fen("4k4/4r4/4r4/9/9/9/9/9/R8/2K6 w").unwrap();
        let mv = Move::from_iccs("a1e1").unwrap();

        assert_eq!(classify_move(&board, mv), MoveEffect::Sacrifice);
    }

    #[test]
    fn treats_a_pinned_root_as_a_false_root() {
        let board = BoardState::from_fen("4k4/3rr4/1N7/9/9/9/9/9/R8/2K1R4 w").unwrap();
        let mv = Move::from_iccs("a1d1").unwrap();
        let (after, _) = board.apply_move(mv);
        let threat = Move::from_iccs("d1d8").unwrap();

        assert!(!has_real_root(&after, board.turn, threat));
        assert_eq!(classify_move(&board, mv), MoveEffect::JointChase);
    }

    #[test]
    fn classifies_two_attackers_on_one_unrooted_target_as_joint_chase() {
        let board = BoardState::from_fen("4k4/3r5/1N7/9/9/9/9/9/R8/2K6 w").unwrap();
        let mv = Move::from_iccs("a1d1").unwrap();

        assert_eq!(classify_move(&board, mv), MoveEffect::JointChase);
    }

    #[test]
    fn classifies_a_move_offering_its_piece_as_sacrifice() {
        let board = BoardState::from_fen("4k4/9/9/4P4/9/2n6/R8/9/9/4K4 w").unwrap();
        let mv = Move::from_iccs("a3e3").unwrap();

        assert_eq!(classify_move(&board, mv), MoveEffect::Sacrifice);
    }

    #[test]
    fn classifies_an_immediate_recapture_as_exchange_candidate() {
        let board = BoardState::from_fen("4k4/9/9/4P4/9/2n6/R3p4/9/9/4K4 w").unwrap();
        let mv = Move::from_iccs("a3e3").unwrap();

        assert_eq!(classify_move(&board, mv), MoveEffect::Exchange);
    }
}
