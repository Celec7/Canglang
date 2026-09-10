use super::{ActiveGame, GameResult, GameState};
use crate::core::jieqi::{
    DrawOffer, JieqiCapability, JieqiCapabilityReason, JieqiGameResult, JieqiOperation,
    JieqiPlayMode, JieqiPositionViewV1, JieqiResultReason, JieqiSource, PublicPly,
};
use crate::core::piece::Color;
use crate::core::rules::{MoveEffect, RuleExplanation, RuleProfile, RuleStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SessionToken {
    pub game_id: String,
    pub expected_revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionErrorCode {
    StaleSession,
    InvalidInput,
    IllegalMove,
    OperationUnavailable,
    GameFinished,
    IoFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SessionError {
    pub code: SessionErrorCode,
    pub message: String,
}

impl SessionError {
    pub fn new(code: SessionErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMutation {
    Presentation,
    Content,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum PositionView {
    Xiangqi { fen: String },
    Jieqi { position: JieqiPositionViewV1 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PlyRecord {
    pub ply: u32,
    pub iccs: String,
    pub notation: String,
    pub mover: String,
    pub is_capture: bool,
    pub is_check: bool,
    pub fen: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "variant", content = "ply", rename_all = "snake_case")]
pub enum SessionPly {
    Xiangqi(PlyRecord),
    Jieqi(PublicPly),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionSource {
    Local,
    PublicReplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum SessionRules {
    Xiangqi { profile: RuleProfile },
    JieqiCasualV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionResult {
    Ongoing,
    Winner {
        winner: Color,
        reason: SessionResultReason,
    },
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionResultReason {
    Checkmate,
    Stalemate,
    Resignation,
    Agreement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct XiangqiAssessment {
    pub repetition_count: u32,
    pub repetition_explanation: Option<String>,
    pub rule_status: RuleStatus,
    pub rule_explanation: Option<RuleExplanation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SessionCapabilities {
    pub r#move: JieqiCapability,
    pub undo: JieqiCapability,
    pub redo: JieqiCapability,
    pub jump: JieqiCapability,
    pub resign: JieqiCapability,
    pub offer_draw: JieqiCapability,
    pub save_private: JieqiCapability,
    pub save_public: JieqiCapability,
    pub edit_annotations: JieqiCapability,
    pub analyze: JieqiCapability,
    pub query_book: JieqiCapability,
    pub edit_position: JieqiCapability,
    pub use_fen: JieqiCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SessionSnapshot {
    pub game_id: String,
    pub revision: String,
    pub content_revision: String,
    pub position: PositionView,
    pub start_position: PositionView,
    pub head_ply: u32,
    pub current_ply: u32,
    pub source: SessionSource,
    pub rules: SessionRules,
    pub play_mode: JieqiPlayMode,
    pub result: SessionResult,
    pub in_check: bool,
    pub history: Vec<SessionPly>,
    pub xiangqi_assessment: Option<XiangqiAssessment>,
    pub capabilities: SessionCapabilities,
    pub draw_offer: Option<DrawOffer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum NewGameOptions {
    Xiangqi {
        fen: Option<String>,
        rule_profile: RuleProfile,
    },
    Jieqi {
        play_mode: JieqiPlayMode,
    },
}

impl SessionSnapshot {
    pub(crate) fn from_state(state: &GameState) -> Self {
        match state.active() {
            ActiveGame::Xiangqi(game) => {
                let full_history = game.full_history();
                let history = full_history
                    .iter()
                    .enumerate()
                    .map(|(index, record)| {
                        SessionPly::Xiangqi(PlyRecord {
                            ply: (index + 1) as u32,
                            iccs: record.mv.to_iccs(),
                            notation: record.chinese_notation.clone(),
                            mover: if record.mover.is_red() {
                                "red"
                            } else {
                                "black"
                            }
                            .to_string(),
                            is_capture: record.captured.is_some(),
                            is_check: matches!(
                                record.effect,
                                MoveEffect::Check | MoveEffect::Checkmate
                            ),
                            fen: record.position_fen.clone(),
                        })
                    })
                    .collect();
                let assessment = game.rule_assessment();
                let enabled = capability(true, None);
                let wrong_variant = capability(false, Some(JieqiCapabilityReason::WrongVariant));
                Self {
                    game_id: state.game_id().to_string(),
                    revision: state.revision().to_string(),
                    content_revision: state.content_revision().to_string(),
                    position: PositionView::Xiangqi {
                        fen: game.current_board().to_fen(),
                    },
                    start_position: PositionView::Xiangqi {
                        fen: game.start_fen(),
                    },
                    head_ply: full_history.len() as u32,
                    current_ply: game.current_ply() as u32,
                    source: SessionSource::Local,
                    rules: SessionRules::Xiangqi {
                        profile: game.rule_profile(),
                    },
                    play_mode: JieqiPlayMode::Training,
                    result: xiangqi_result(game.result()),
                    in_check: game.is_in_check(),
                    history,
                    xiangqi_assessment: Some(XiangqiAssessment {
                        repetition_count: game.repetition_count(),
                        repetition_explanation: game.repetition_assessment().explanation,
                        rule_status: assessment.status,
                        rule_explanation: assessment.explanation,
                    }),
                    capabilities: SessionCapabilities {
                        r#move: capability(game.result() == GameResult::Ongoing, None),
                        undo: capability(game.can_undo(), None),
                        redo: capability(game.can_redo(), None),
                        jump: enabled,
                        resign: capability(game.result() == GameResult::Ongoing, None),
                        offer_draw: wrong_variant,
                        save_private: wrong_variant,
                        save_public: enabled,
                        edit_annotations: enabled,
                        analyze: enabled,
                        query_book: enabled,
                        edit_position: enabled,
                        use_fen: enabled,
                    },
                    draw_offer: None,
                }
            }
            ActiveGame::Jieqi(game) => {
                let disabled = capability(false, Some(JieqiCapabilityReason::WrongVariant));
                Self {
                    game_id: state.game_id().to_string(),
                    revision: state.revision().to_string(),
                    content_revision: state.content_revision().to_string(),
                    position: PositionView::Jieqi {
                        position: game.position().public_view(),
                    },
                    start_position: PositionView::Jieqi {
                        position: game.initial_position().public_view(),
                    },
                    head_ply: game.head_ply() as u32,
                    current_ply: game.current_ply() as u32,
                    source: match game.source() {
                        JieqiSource::Local => SessionSource::Local,
                        JieqiSource::PublicReplay => SessionSource::PublicReplay,
                    },
                    rules: SessionRules::JieqiCasualV1,
                    play_mode: game.play_mode(),
                    result: jieqi_result(game.result()),
                    in_check: crate::core::jieqi::JieqiRules::is_in_check(
                        game.position(),
                        game.position().turn(),
                    ),
                    history: game
                        .history()
                        .iter()
                        .cloned()
                        .map(SessionPly::Jieqi)
                        .collect(),
                    xiangqi_assessment: None,
                    capabilities: SessionCapabilities {
                        r#move: game.capability(JieqiOperation::Move),
                        undo: game.capability(JieqiOperation::Undo),
                        redo: game.capability(JieqiOperation::Redo),
                        jump: game.capability(JieqiOperation::Jump),
                        resign: game.capability(JieqiOperation::Resign),
                        offer_draw: game.capability(JieqiOperation::OfferDraw),
                        save_private: game.capability(JieqiOperation::SavePrivate),
                        save_public: game.capability(JieqiOperation::SavePublic),
                        edit_annotations: game.capability(JieqiOperation::EditAnnotations),
                        analyze: disabled,
                        query_book: disabled,
                        edit_position: disabled,
                        use_fen: disabled,
                    },
                    draw_offer: game.draw_offer().cloned(),
                }
            }
        }
    }
}

fn capability(enabled: bool, reason: Option<JieqiCapabilityReason>) -> JieqiCapability {
    JieqiCapability { enabled, reason }
}

fn xiangqi_result(result: GameResult) -> SessionResult {
    match result {
        GameResult::Ongoing => SessionResult::Ongoing,
        GameResult::RedWin => SessionResult::Winner {
            winner: Color::Red,
            reason: SessionResultReason::Checkmate,
        },
        GameResult::BlackWin => SessionResult::Winner {
            winner: Color::Black,
            reason: SessionResultReason::Checkmate,
        },
        GameResult::Draw => SessionResult::Draw,
    }
}

fn jieqi_result(result: Option<JieqiGameResult>) -> SessionResult {
    match result {
        None => SessionResult::Ongoing,
        Some(JieqiGameResult::Winner { winner, reason }) => SessionResult::Winner {
            winner,
            reason: match reason {
                JieqiResultReason::Checkmate => SessionResultReason::Checkmate,
                JieqiResultReason::Stalemate => SessionResultReason::Stalemate,
                JieqiResultReason::Resignation => SessionResultReason::Resignation,
            },
        },
        Some(JieqiGameResult::DrawAgreement) => SessionResult::Draw,
    }
}
