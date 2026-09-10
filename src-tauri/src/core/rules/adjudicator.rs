use crate::core::board::BoardState;
use crate::core::piece::{Color, PieceKind};

use super::effects::{ChaseTarget, MoveEffect};
use super::repetition::assess_repetition;
use super::{MoveValidator, RuleProfile};
use serde::{Deserialize, Serialize};

/// 当前规则裁判状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RuleStatus {
    Ongoing,
    Checkmate,
    Stalemate,
    RepetitionPending,
    ProhibitedMovePending,
    RedLossByRule,
    BlackLossByRule,
    DrawByRule,
}

/// 面向调用方的规则解释码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RuleExplanationCode {
    Checkmate,
    Stalemate,
    RepetitionNeedsJudgement,
    ProhibitedMoveCandidate,
    RuleLoss,
    RuleDraw,
}

/// 一条可展示、可记录的规则解释
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RuleExplanation {
    pub code: RuleExplanationCode,
    pub side: Option<String>,
    pub category: Option<MoveEffect>,
    pub message: String,
    pub cycle_length: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleAssessment {
    pub status: RuleStatus,
    pub explanation: Option<RuleExplanation>,
}

/// 循环中的一手走法及其可验证的捉目标上下文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleMove {
    pub side: Color,
    pub effect: MoveEffect,
    pub chase_targets: Vec<ChaseTarget>,
}

/// 依据基础胜负状态与版本化循环政策生成当前规则结果
/// 结果只在规则服务中生成，前端消费其结构化表示
/// 该入口自动确认基础终局，以及可以从走法作用中无歧义确认的纯长将与允许着法循环；
/// 长捉、兑、献、拦等需要根子和攻防上下文的复杂责任仍返回待裁判状态
pub fn assess_position(
    profile: RuleProfile,
    board: &BoardState,
    repetition_count: u32,
    cycle_length: Option<u32>,
    cycle_moves: &[(Color, MoveEffect)],
) -> RuleAssessment {
    let context: Vec<CycleMove> = cycle_moves
        .iter()
        .map(|(side, effect)| CycleMove {
            side: *side,
            effect: *effect,
            chase_targets: Vec::new(),
        })
        .collect();
    assess_position_with_context(profile, board, repetition_count, cycle_length, &context)
}

/// 使用完整走法上下文执行循环裁判
pub fn assess_position_with_context(
    profile: RuleProfile,
    board: &BoardState,
    repetition_count: u32,
    cycle_length: Option<u32>,
    cycle_moves: &[CycleMove],
) -> RuleAssessment {
    let turn = board.turn;

    if MoveValidator::is_checkmate(board, turn) {
        return RuleAssessment {
            status: RuleStatus::Checkmate,
            explanation: Some(explanation(
                RuleExplanationCode::Checkmate,
                Some(turn),
                Some(MoveEffect::Checkmate),
                "当前走方被将死，对局结束。".to_string(),
                None,
            )),
        };
    }

    if MoveValidator::is_stalemate(board, turn) {
        return RuleAssessment {
            status: RuleStatus::Stalemate,
            explanation: Some(explanation(
                RuleExplanationCode::Stalemate,
                Some(turn),
                None,
                "当前走方无合法着法，按象棋困毙规则对局结束。".to_string(),
                None,
            )),
        };
    }

    if let Some(repetition_message) =
        assess_repetition(profile, repetition_count, cycle_length).explanation
    {
        return match classify_cycle(cycle_length, cycle_moves) {
            Some(CycleDisposition::SingleSideCheck(side)) => RuleAssessment {
                status: if side.is_red() {
                    RuleStatus::RedLossByRule
                } else {
                    RuleStatus::BlackLossByRule
                },
                explanation: Some(explanation(
                    RuleExplanationCode::RuleLoss,
                    Some(side),
                    Some(MoveEffect::Check),
                    format!(
                        "检测到{}连续长将（循环规则：{}），该方按规则判负。",
                        side_name(side),
                        profile.label()
                    ),
                    cycle_length,
                )),
            },
            Some(CycleDisposition::MutualCheck) => RuleAssessment {
                status: RuleStatus::DrawByRule,
                explanation: Some(explanation(
                    RuleExplanationCode::RuleDraw,
                    None,
                    Some(MoveEffect::Check),
                    format!(
                        "检测到双方连续长将（循环规则：{}），双方均不变着时判和。",
                        profile.label()
                    ),
                    cycle_length,
                )),
            },
            Some(CycleDisposition::MutualStableChase) => RuleAssessment {
                status: RuleStatus::DrawByRule,
                explanation: Some(explanation(
                    RuleExplanationCode::RuleDraw,
                    None,
                    Some(MoveEffect::Chase),
                    format!(
                        "检测到双方均为稳定目标的连续捉子（循环规则：{}），双方均不变着时判和。",
                        profile.label()
                    ),
                    cycle_length,
                )),
            },
            Some(CycleDisposition::Allowed) => RuleAssessment {
                status: RuleStatus::DrawByRule,
                explanation: Some(explanation(
                    RuleExplanationCode::RuleDraw,
                    None,
                    Some(MoveEffect::Idle),
                    format!(
                        "检测到双方均为允许着法的循环（循环规则：{}），双方不变着判和。",
                        profile.label()
                    ),
                    cycle_length,
                )),
            },
            Some(CycleDisposition::SingleSideChase(side, category, stable_target)) => {
                let stable_outcome = (category == MoveEffect::Chase)
                    .then(|| stable_target.and_then(|target| stable_chase_outcome(profile, target)))
                    .flatten();
                let decisive = stable_outcome == Some(true);
                let stable_draw = stable_outcome == Some(false);
                RuleAssessment {
                    status: if decisive {
                        if side.is_red() {
                            RuleStatus::RedLossByRule
                        } else {
                            RuleStatus::BlackLossByRule
                        }
                    } else if stable_draw {
                        RuleStatus::DrawByRule
                    } else {
                        RuleStatus::ProhibitedMovePending
                    },
                    explanation: Some(explanation(
                        if decisive {
                            RuleExplanationCode::RuleLoss
                        } else if stable_draw {
                            RuleExplanationCode::RuleDraw
                        } else {
                            RuleExplanationCode::ProhibitedMoveCandidate
                        },
                        Some(side),
                        Some(category),
                        if decisive {
                            format!(
                                "检测到{}连续追捉同一无根/假根目标（循环规则：{}），该方按规则判负。",
                                side_name(side),
                                profile.label()
                            )
                        } else if stable_draw {
                            format!(
                                "检测到{}连续追捉同一目标（循环规则：{}），该场景属于该规则档案的长捉例外，判和。",
                                side_name(side),
                                profile.label()
                            )
                        } else {
                            cycle_candidate_message(profile, Some(side), Some(category), false)
                        },
                        cycle_length,
                    )),
                }
            }
            Some(CycleDisposition::MutualAttack(category))
            | Some(CycleDisposition::MixedAttack(category)) => RuleAssessment {
                status: RuleStatus::RepetitionPending,
                explanation: Some(explanation(
                    RuleExplanationCode::RepetitionNeedsJudgement,
                    None,
                    Some(category),
                    cycle_candidate_message(profile, None, Some(category), true),
                    cycle_length,
                )),
            },
            None => RuleAssessment {
                status: RuleStatus::RepetitionPending,
                explanation: Some(explanation(
                    RuleExplanationCode::RepetitionNeedsJudgement,
                    None,
                    Some(MoveEffect::Pending),
                    repetition_message,
                    cycle_length,
                )),
            },
        };
    }

    RuleAssessment {
        status: RuleStatus::Ongoing,
        explanation: None,
    }
}

fn explanation(
    code: RuleExplanationCode,
    side: Option<Color>,
    category: Option<MoveEffect>,
    message: String,
    cycle_length: Option<u32>,
) -> RuleExplanation {
    RuleExplanation {
        code,
        side: side.map(|side| if side.is_red() { "red" } else { "black" }.to_string()),
        category,
        message,
        cycle_length,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CycleDisposition {
    SingleSideCheck(Color),
    MutualCheck,
    MutualStableChase,
    SingleSideChase(Color, MoveEffect, Option<ChaseTarget>),
    MutualAttack(MoveEffect),
    MixedAttack(MoveEffect),
    Allowed,
}

fn classify_cycle(
    cycle_length: Option<u32>,
    cycle_moves: &[CycleMove],
) -> Option<CycleDisposition> {
    let length = cycle_length? as usize;
    if length == 0 || cycle_moves.len() < length {
        return None;
    }

    let cycle = &cycle_moves[cycle_moves.len() - length..];
    let red: Vec<&CycleMove> = cycle
        .iter()
        .filter(|move_record| move_record.side.is_red())
        .collect();
    let black: Vec<&CycleMove> = cycle
        .iter()
        .filter(|move_record| move_record.side.is_black())
        .collect();

    let red_attack = attack_cycle(&red);
    let black_attack = attack_cycle(&black);
    let red_allowed = allowed_cycle(&red);
    let black_allowed = allowed_cycle(&black);

    match (red_attack, black_attack, red_allowed, black_allowed) {
        (Some(MoveEffect::Check), Some(MoveEffect::Check), _, _) => {
            Some(CycleDisposition::MutualCheck)
        }
        (Some(MoveEffect::Check), None, _, true) => {
            Some(CycleDisposition::SingleSideCheck(Color::Red))
        }
        (None, Some(MoveEffect::Check), true, _) => {
            Some(CycleDisposition::SingleSideCheck(Color::Black))
        }
        (Some(MoveEffect::Chase), Some(MoveEffect::Chase), _, _)
            if has_stable_chase_target(&red).is_some()
                && has_stable_chase_target(&black).is_some() =>
        {
            Some(CycleDisposition::MutualStableChase)
        }
        (Some(category @ (MoveEffect::Chase | MoveEffect::JointChase)), None, _, true) => Some(
            CycleDisposition::SingleSideChase(Color::Red, category, has_stable_chase_target(&red)),
        ),
        (None, Some(category @ (MoveEffect::Chase | MoveEffect::JointChase)), true, _) => {
            Some(CycleDisposition::SingleSideChase(
                Color::Black,
                category,
                has_stable_chase_target(&black),
            ))
        }
        (Some(red_category), Some(black_category), _, _) if red_category == black_category => {
            Some(CycleDisposition::MutualAttack(red_category))
        }
        (Some(red_category), Some(black_category), _, _) => Some(CycleDisposition::MixedAttack(
            if red_category == black_category {
                red_category
            } else {
                MoveEffect::Pending
            },
        )),
        (None, None, true, true) => Some(CycleDisposition::Allowed),
        _ => None,
    }
}

fn attack_cycle(moves: &[&CycleMove]) -> Option<MoveEffect> {
    if moves.is_empty()
        || !moves.iter().all(|move_record| {
            matches!(
                move_record.effect,
                MoveEffect::Check
                    | MoveEffect::Checkmate
                    | MoveEffect::Chase
                    | MoveEffect::JointChase
            )
        })
    {
        return None;
    }

    if moves.iter().all(|move_record| {
        matches!(
            move_record.effect,
            MoveEffect::Check | MoveEffect::Checkmate
        )
    }) {
        Some(MoveEffect::Check)
    } else if moves
        .iter()
        .all(|move_record| move_record.effect == MoveEffect::Chase)
    {
        Some(MoveEffect::Chase)
    } else if moves
        .iter()
        .all(|move_record| move_record.effect == MoveEffect::JointChase)
    {
        Some(MoveEffect::JointChase)
    } else {
        Some(MoveEffect::Pending)
    }
}

fn allowed_cycle(moves: &[&CycleMove]) -> bool {
    !moves.is_empty()
        && moves.iter().all(|move_record| {
            !matches!(
                move_record.effect,
                MoveEffect::Check
                    | MoveEffect::Checkmate
                    | MoveEffect::Chase
                    | MoveEffect::JointChase
            )
        })
}

fn has_stable_chase_target(moves: &[&CycleMove]) -> Option<ChaseTarget> {
    let first_target = moves.first().and_then(|move_record| {
        if move_record.chase_targets.len() == 1 && !move_record.chase_targets[0].ambiguous {
            Some(move_record.chase_targets[0])
        } else {
            None
        }
    })?;

    moves
        .iter()
        .all(|move_record| {
            move_record.chase_targets.len() == 1 && move_record.chase_targets[0] == first_target
        })
        .then_some(first_target)
}

/// 亚洲规则对部分稳定单子长捉保留例外，中国规则则将同一场景作为单方长捉处理
///
/// 这里只对已经确认“同一目标、单一攻击子、无歧义”的稳定候选给出差异化结论；
/// 联合捉、多目标和缺少攻击子上下文的记录仍交给待裁判流程
fn stable_chase_outcome(profile: RuleProfile, target: ChaseTarget) -> Option<bool> {
    match profile {
        RuleProfile::China2020 => Some(true),
        RuleProfile::Asian2017 => target.attacker_kind.map(|attacker_kind| {
            !matches!(attacker_kind, PieceKind::King | PieceKind::Pawn)
                && attacker_kind != target.kind
        }),
    }
}

fn side_name(side: Color) -> &'static str {
    if side.is_red() { "红方" } else { "黑方" }
}

fn cycle_candidate_message(
    profile: RuleProfile,
    side: Option<Color>,
    category: Option<MoveEffect>,
    both: bool,
) -> String {
    let profile_name = profile.label();
    let category_name = match category {
        Some(MoveEffect::Check) => "连续将军",
        Some(MoveEffect::Chase) => "连续捉子",
        Some(MoveEffect::JointChase) => "连续联合捉子",
        Some(MoveEffect::Pending) => "连续攻击",
        _ => "攻击性循环",
    };

    if both {
        format!(
            "检测到双方均有{category_name}候选（循环规则：{profile_name}），属于双方攻击性循环，仍需按规则裁判。"
        )
    } else {
        let side_name = match side {
            Some(color) if color.is_red() => "红方",
            Some(_) => "黑方",
            None => "一方",
        };
        format!(
            "检测到{side_name}{category_name}候选（循环规则：{profile_name}），属于禁止着法候选，需变着或由裁判确认。"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_repetition_as_pending_judgement() {
        let assessment = assess_position(
            RuleProfile::China2020,
            &BoardState::initial(),
            3,
            Some(4),
            &[],
        );

        assert_eq!(assessment.status, RuleStatus::RepetitionPending);
        let explanation = assessment.explanation.unwrap();
        assert_eq!(
            explanation.code,
            RuleExplanationCode::RepetitionNeedsJudgement
        );
        assert_eq!(explanation.category, Some(MoveEffect::Pending));
        assert_eq!(explanation.side, None);
        assert_eq!(explanation.cycle_length, Some(4));
    }

    #[test]
    fn reports_checkmate_before_repetition() {
        let board = BoardState::from_fen("3Rk4/9/9/9/9/9/9/9/3R5/4K4 b").unwrap();
        let assessment = assess_position(RuleProfile::Asian2017, &board, 3, Some(4), &[]);

        assert_eq!(assessment.status, RuleStatus::Checkmate);
        assert_eq!(
            assessment.explanation.unwrap().code,
            RuleExplanationCode::Checkmate
        );
    }

    #[test]
    fn identifies_one_side_perpetual_check_candidate() {
        let cycle_moves = [
            (Color::Red, MoveEffect::Check),
            (Color::Black, MoveEffect::Idle),
            (Color::Red, MoveEffect::Check),
            (Color::Black, MoveEffect::Idle),
        ];
        let assessment = assess_position(
            RuleProfile::China2020,
            &BoardState::initial(),
            3,
            Some(4),
            &cycle_moves,
        );

        let explanation = assessment.explanation.unwrap();
        assert_eq!(assessment.status, RuleStatus::RedLossByRule);
        assert_eq!(explanation.code, RuleExplanationCode::RuleLoss);
        assert_eq!(explanation.side.as_deref(), Some("red"));
        assert_eq!(explanation.category, Some(MoveEffect::Check));
    }

    #[test]
    fn keeps_mutual_attack_cycle_in_judgement_state() {
        let cycle_moves = [
            (Color::Red, MoveEffect::Check),
            (Color::Black, MoveEffect::Check),
            (Color::Red, MoveEffect::Check),
            (Color::Black, MoveEffect::Check),
        ];
        let assessment = assess_position(
            RuleProfile::Asian2017,
            &BoardState::initial(),
            3,
            Some(4),
            &cycle_moves,
        );

        let explanation = assessment.explanation.unwrap();
        assert_eq!(assessment.status, RuleStatus::DrawByRule);
        assert_eq!(explanation.side, None);
        assert_eq!(explanation.category, Some(MoveEffect::Check));
    }

    #[test]
    fn adjudicates_an_allowed_cycle_as_a_draw() {
        let cycle_moves = [
            (Color::Red, MoveEffect::Idle),
            (Color::Black, MoveEffect::Idle),
            (Color::Red, MoveEffect::Idle),
            (Color::Black, MoveEffect::Idle),
        ];
        let assessment = assess_position(
            RuleProfile::China2020,
            &BoardState::initial(),
            3,
            Some(4),
            &cycle_moves,
        );

        let explanation = assessment.explanation.unwrap();
        assert_eq!(assessment.status, RuleStatus::DrawByRule);
        assert_eq!(explanation.code, RuleExplanationCode::RuleDraw);
        assert_eq!(explanation.category, Some(MoveEffect::Idle));
    }

    #[test]
    fn keeps_mixed_check_and_chase_cycle_for_judgement() {
        let cycle_moves = [
            (Color::Red, MoveEffect::Check),
            (Color::Black, MoveEffect::Chase),
            (Color::Red, MoveEffect::Check),
            (Color::Black, MoveEffect::Chase),
        ];
        let assessment = assess_position(
            RuleProfile::China2020,
            &BoardState::initial(),
            3,
            Some(4),
            &cycle_moves,
        );

        assert_eq!(assessment.status, RuleStatus::RepetitionPending);
        assert_eq!(
            assessment.explanation.unwrap().category,
            Some(MoveEffect::Pending)
        );
    }

    #[test]
    fn exposes_a_single_side_joint_chase_as_a_prohibited_candidate() {
        let cycle_moves = [
            (Color::Red, MoveEffect::JointChase),
            (Color::Black, MoveEffect::Idle),
            (Color::Red, MoveEffect::JointChase),
            (Color::Black, MoveEffect::Idle),
        ];
        let assessment = assess_position(
            RuleProfile::China2020,
            &BoardState::initial(),
            3,
            Some(4),
            &cycle_moves,
        );

        let explanation = assessment.explanation.unwrap();
        assert_eq!(assessment.status, RuleStatus::ProhibitedMovePending);
        assert_eq!(explanation.side.as_deref(), Some("red"));
        assert_eq!(explanation.category, Some(MoveEffect::JointChase));
    }

    #[test]
    fn keeps_ambiguous_same_kind_chase_for_judgement() {
        let target = ChaseTarget {
            color: Color::Black,
            kind: crate::core::piece::PieceKind::Rook,
            ambiguous: true,
            attacker_kind: Some(PieceKind::Rook),
        };
        let cycle_moves = [
            CycleMove {
                side: Color::Red,
                effect: MoveEffect::Chase,
                chase_targets: vec![target],
            },
            CycleMove {
                side: Color::Black,
                effect: MoveEffect::Idle,
                chase_targets: Vec::new(),
            },
            CycleMove {
                side: Color::Red,
                effect: MoveEffect::Chase,
                chase_targets: vec![target],
            },
            CycleMove {
                side: Color::Black,
                effect: MoveEffect::Idle,
                chase_targets: Vec::new(),
            },
        ];
        let assessment = assess_position_with_context(
            RuleProfile::China2020,
            &BoardState::initial(),
            3,
            Some(4),
            &cycle_moves,
        );

        assert_eq!(assessment.status, RuleStatus::ProhibitedMovePending);
    }

    #[test]
    fn adjudicates_two_stable_single_piece_chases_as_a_draw() {
        let red_target = ChaseTarget {
            color: Color::Black,
            kind: crate::core::piece::PieceKind::Rook,
            ambiguous: false,
            attacker_kind: Some(PieceKind::Rook),
        };
        let black_target = ChaseTarget {
            color: Color::Red,
            kind: crate::core::piece::PieceKind::Rook,
            ambiguous: false,
            attacker_kind: Some(PieceKind::Rook),
        };
        let cycle_moves = [
            CycleMove {
                side: Color::Red,
                effect: MoveEffect::Chase,
                chase_targets: vec![red_target],
            },
            CycleMove {
                side: Color::Black,
                effect: MoveEffect::Chase,
                chase_targets: vec![black_target],
            },
            CycleMove {
                side: Color::Red,
                effect: MoveEffect::Chase,
                chase_targets: vec![red_target],
            },
            CycleMove {
                side: Color::Black,
                effect: MoveEffect::Chase,
                chase_targets: vec![black_target],
            },
        ];
        let assessment = assess_position_with_context(
            RuleProfile::Asian2017,
            &BoardState::initial(),
            3,
            Some(4),
            &cycle_moves,
        );

        assert_eq!(assessment.status, RuleStatus::DrawByRule);
        assert_eq!(
            assessment.explanation.unwrap().code,
            RuleExplanationCode::RuleDraw
        );
    }

    #[test]
    fn differentiates_same_kind_stable_chase_by_rule_profile() {
        let target = ChaseTarget {
            color: Color::Black,
            kind: PieceKind::Rook,
            ambiguous: false,
            attacker_kind: Some(PieceKind::Rook),
        };
        let cycle_moves = [
            CycleMove {
                side: Color::Red,
                effect: MoveEffect::Chase,
                chase_targets: vec![target],
            },
            CycleMove {
                side: Color::Black,
                effect: MoveEffect::Idle,
                chase_targets: Vec::new(),
            },
            CycleMove {
                side: Color::Red,
                effect: MoveEffect::Chase,
                chase_targets: vec![target],
            },
            CycleMove {
                side: Color::Black,
                effect: MoveEffect::Idle,
                chase_targets: Vec::new(),
            },
        ];

        let asian = assess_position_with_context(
            RuleProfile::Asian2017,
            &BoardState::initial(),
            3,
            Some(4),
            &cycle_moves,
        );
        assert_eq!(asian.status, RuleStatus::DrawByRule);

        let china = assess_position_with_context(
            RuleProfile::China2020,
            &BoardState::initial(),
            3,
            Some(4),
            &cycle_moves,
        );
        assert_eq!(china.status, RuleStatus::RedLossByRule);
    }
}
