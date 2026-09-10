use super::{JieqiEndState, JieqiNotation, JieqiPosition, JieqiPublicKind, JieqiRules};
use crate::core::jieqi::history::JieqiHistory;
use crate::core::piece::Color;
use crate::core::position::Move;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JieqiPlayMode {
    Duel,
    Training,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JieqiSource {
    Local,
    PublicReplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JieqiResultReason {
    Checkmate,
    Stalemate,
    Resignation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JieqiGameResult {
    Winner {
        winner: Color,
        reason: JieqiResultReason,
    },
    DrawAgreement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CapturedPieceView {
    None,
    Revealed { kind: JieqiPublicKind },
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicPly {
    pub ply: usize,
    pub iccs: String,
    pub mover: Color,
    pub notation: String,
    pub revealed: Option<JieqiPublicKind>,
    pub captured: CapturedPieceView,
    pub is_check: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawOffer {
    pub id: String,
    pub proposer: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JieqiOperation {
    Move,
    Undo,
    Redo,
    Jump,
    Resign,
    OfferDraw,
    SavePrivate,
    SavePublic,
    EditAnnotations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JieqiCapabilityReason {
    ReadOnly,
    DuelPolicy,
    Finished,
    NotAtHead,
    PendingDraw,
    NoHistory,
    NoFuture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct JieqiCapability {
    pub enabled: bool,
    pub reason: Option<JieqiCapabilityReason>,
}

impl JieqiCapability {
    const fn enabled() -> Self {
        Self {
            enabled: true,
            reason: None,
        }
    }

    const fn disabled(reason: JieqiCapabilityReason) -> Self {
        Self {
            enabled: false,
            reason: Some(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JieqiGameError {
    #[error("揭棋操作当前不可用: {0:?}")]
    OperationUnavailable(JieqiCapabilityReason),
    #[error("揭棋走法不合法")]
    IllegalMove,
    #[error("走方参数与当前走方不一致")]
    WrongSide,
    #[error("历史位置越界: {0}")]
    InvalidPly(usize),
    #[error("求和请求已过期")]
    StaleDrawOffer,
    #[error("求和回应方不正确")]
    WrongDrawResponder,
    #[error("求和取消方不正确")]
    WrongDrawCanceller,
    #[error("公开回放记录与规则推导结果不一致")]
    RecordedPlyMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JieqiGame {
    history: JieqiHistory,
    play_mode: JieqiPlayMode,
    source: JieqiSource,
    final_result: Option<JieqiGameResult>,
    draw_offer: Option<DrawOffer>,
    next_draw_offer_id: u64,
}

impl JieqiGame {
    pub fn new(initial: JieqiPosition, play_mode: JieqiPlayMode) -> Self {
        Self {
            history: JieqiHistory::new(initial),
            play_mode,
            source: JieqiSource::Local,
            final_result: None,
            draw_offer: None,
            next_draw_offer_id: 1,
        }
    }

    pub fn new_public_replay(initial: JieqiPosition) -> Self {
        Self {
            history: JieqiHistory::new(initial),
            play_mode: JieqiPlayMode::Duel,
            source: JieqiSource::PublicReplay,
            final_result: None,
            draw_offer: None,
            next_draw_offer_id: 1,
        }
    }

    pub fn position(&self) -> &JieqiPosition {
        self.history.current()
    }

    pub fn history(&self) -> &[PublicPly] {
        &self.history.plies
    }

    pub const fn play_mode(&self) -> JieqiPlayMode {
        self.play_mode
    }

    pub const fn source(&self) -> JieqiSource {
        self.source
    }

    pub fn head_ply(&self) -> usize {
        self.history.head()
    }

    pub const fn current_ply(&self) -> usize {
        self.history.cursor
    }

    pub fn result(&self) -> Option<JieqiGameResult> {
        if self.play_mode == JieqiPlayMode::Training {
            self.history.results[self.history.cursor]
        } else {
            self.final_result
        }
    }

    pub fn draw_offer(&self) -> Option<&DrawOffer> {
        self.draw_offer.as_ref()
    }

    pub fn capability(&self, operation: JieqiOperation) -> JieqiCapability {
        if self.draw_offer.is_some()
            && matches!(
                operation,
                JieqiOperation::Move
                    | JieqiOperation::Undo
                    | JieqiOperation::Redo
                    | JieqiOperation::Jump
                    | JieqiOperation::Resign
                    | JieqiOperation::OfferDraw
                    | JieqiOperation::SavePrivate
                    | JieqiOperation::SavePublic
            )
        {
            return JieqiCapability::disabled(JieqiCapabilityReason::PendingDraw);
        }
        if self.source == JieqiSource::PublicReplay {
            return match operation {
                JieqiOperation::Jump
                | JieqiOperation::SavePublic
                | JieqiOperation::EditAnnotations => JieqiCapability::enabled(),
                _ => JieqiCapability::disabled(JieqiCapabilityReason::ReadOnly),
            };
        }
        match operation {
            JieqiOperation::Move => {
                if self.result().is_some() {
                    JieqiCapability::disabled(JieqiCapabilityReason::Finished)
                } else if self.play_mode == JieqiPlayMode::Duel
                    && self.current_ply() != self.head_ply()
                {
                    JieqiCapability::disabled(JieqiCapabilityReason::NotAtHead)
                } else {
                    JieqiCapability::enabled()
                }
            }
            JieqiOperation::Undo => {
                if self.play_mode == JieqiPlayMode::Duel {
                    JieqiCapability::disabled(JieqiCapabilityReason::DuelPolicy)
                } else if self.current_ply() == 0 {
                    JieqiCapability::disabled(JieqiCapabilityReason::NoHistory)
                } else {
                    JieqiCapability::enabled()
                }
            }
            JieqiOperation::Redo => {
                if self.play_mode == JieqiPlayMode::Duel {
                    JieqiCapability::disabled(JieqiCapabilityReason::DuelPolicy)
                } else if self.current_ply() == self.head_ply() {
                    JieqiCapability::disabled(JieqiCapabilityReason::NoFuture)
                } else {
                    JieqiCapability::enabled()
                }
            }
            JieqiOperation::Jump => {
                if self.play_mode == JieqiPlayMode::Training || self.final_result.is_some() {
                    JieqiCapability::enabled()
                } else {
                    JieqiCapability::disabled(JieqiCapabilityReason::DuelPolicy)
                }
            }
            JieqiOperation::Resign | JieqiOperation::OfferDraw => {
                if self.result().is_some() {
                    JieqiCapability::disabled(JieqiCapabilityReason::Finished)
                } else if self.current_ply() != self.head_ply() {
                    JieqiCapability::disabled(JieqiCapabilityReason::NotAtHead)
                } else {
                    JieqiCapability::enabled()
                }
            }
            JieqiOperation::SavePrivate
            | JieqiOperation::SavePublic
            | JieqiOperation::EditAnnotations => JieqiCapability::enabled(),
        }
    }

    pub fn make_move(&mut self, mv: Move) -> Result<&PublicPly, JieqiGameError> {
        self.require(JieqiOperation::Move)?;
        let (next, result, ply) = self.prepare_move(mv)?;

        if self.play_mode == JieqiPlayMode::Training && self.current_ply() < self.head_ply() {
            self.history.truncate_future();
            self.final_result = None;
        }
        self.history.push(next, result, ply);
        if result.is_some() {
            self.final_result = result;
        }
        Ok(self.history.plies.last().expect("刚提交的历史记录存在"))
    }

    pub fn apply_recorded_move(&mut self, expected: PublicPly) -> Result<(), JieqiGameError> {
        if self.source != JieqiSource::PublicReplay || self.current_ply() != self.head_ply() {
            return Err(JieqiGameError::OperationUnavailable(
                JieqiCapabilityReason::ReadOnly,
            ));
        }
        let mv = Move::from_iccs(&expected.iccs).map_err(|_| JieqiGameError::IllegalMove)?;
        let (next, result, actual) = self.prepare_move(mv)?;
        if actual != expected {
            return Err(JieqiGameError::RecordedPlyMismatch);
        }
        self.history.push(next, result, actual);
        if result.is_some() {
            self.final_result = result;
        }
        Ok(())
    }

    fn prepare_move(
        &self,
        mv: Move,
    ) -> Result<(JieqiPosition, Option<JieqiGameResult>, PublicPly), JieqiGameError> {
        JieqiRules::validate_move(self.position(), mv).map_err(|_| JieqiGameError::IllegalMove)?;

        let before = self.position();
        let moving = JieqiRules::piece_at(before, mv.from)
            .expect("走法校验保证起点存在")
            .to_owned();
        let captured = JieqiRules::piece_at(before, mv.to).map(|piece| {
            if piece.revealed {
                CapturedPieceView::Revealed {
                    kind: JieqiRules::public_kind(before, piece).into(),
                }
            } else {
                CapturedPieceView::Hidden
            }
        });
        let notation = JieqiNotation::format(before, mv);
        let next = JieqiRules::apply_move(before, mv).map_err(|_| JieqiGameError::IllegalMove)?;
        let revealed = (!moving.revealed).then(|| {
            JieqiRules::public_kind(
                &next,
                JieqiRules::piece_at(&next, mv.to).expect("移动后的落点包含移动子"),
            )
            .into()
        });
        let is_check = JieqiRules::is_in_check(&next, next.turn());
        let result = natural_result(&next);
        let ply = PublicPly {
            ply: self.history.cursor + 1,
            iccs: mv.to_iccs(),
            mover: moving.color,
            notation,
            revealed,
            captured: captured.unwrap_or(CapturedPieceView::None),
            is_check,
        };

        Ok((next, result, ply))
    }

    pub fn undo(&mut self) -> Result<(), JieqiGameError> {
        self.require(JieqiOperation::Undo)?;
        self.history.cursor -= 1;
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), JieqiGameError> {
        self.require(JieqiOperation::Redo)?;
        self.history.cursor += 1;
        Ok(())
    }

    pub fn jump_to(&mut self, ply: usize) -> Result<(), JieqiGameError> {
        self.require(JieqiOperation::Jump)?;
        if ply > self.head_ply() {
            return Err(JieqiGameError::InvalidPly(ply));
        }
        self.history.cursor = ply;
        Ok(())
    }

    pub fn resign(&mut self, side: Color) -> Result<(), JieqiGameError> {
        self.require(JieqiOperation::Resign)?;
        if side != self.position().turn() {
            return Err(JieqiGameError::WrongSide);
        }
        let result = JieqiGameResult::Winner {
            winner: side.opposite(),
            reason: JieqiResultReason::Resignation,
        };
        self.final_result = Some(result);
        self.history.results[self.history.cursor] = Some(result);
        Ok(())
    }

    pub fn offer_draw(&mut self, side: Color) -> Result<&DrawOffer, JieqiGameError> {
        self.require(JieqiOperation::OfferDraw)?;
        if side != self.position().turn() {
            return Err(JieqiGameError::WrongSide);
        }
        let offer = DrawOffer {
            id: self.next_draw_offer_id.to_string(),
            proposer: side,
        };
        self.next_draw_offer_id += 1;
        self.draw_offer = Some(offer);
        Ok(self.draw_offer.as_ref().expect("刚创建的求和请求存在"))
    }

    pub fn respond_draw(
        &mut self,
        offer_id: &str,
        side: Color,
        accept: bool,
    ) -> Result<(), JieqiGameError> {
        let offer = self.match_offer(offer_id)?.clone();
        if side != offer.proposer.opposite() {
            return Err(JieqiGameError::WrongDrawResponder);
        }
        self.draw_offer = None;
        if accept {
            self.final_result = Some(JieqiGameResult::DrawAgreement);
            self.history.results[self.history.cursor] = Some(JieqiGameResult::DrawAgreement);
        }
        Ok(())
    }

    pub fn cancel_draw(&mut self, offer_id: &str, side: Color) -> Result<(), JieqiGameError> {
        let offer = self.match_offer(offer_id)?;
        if side != offer.proposer {
            return Err(JieqiGameError::WrongDrawCanceller);
        }
        self.draw_offer = None;
        Ok(())
    }

    fn match_offer(&self, offer_id: &str) -> Result<&DrawOffer, JieqiGameError> {
        self.draw_offer
            .as_ref()
            .filter(|offer| offer.id == offer_id)
            .ok_or(JieqiGameError::StaleDrawOffer)
    }

    fn require(&self, operation: JieqiOperation) -> Result<(), JieqiGameError> {
        let capability = self.capability(operation);
        if capability.enabled {
            Ok(())
        } else {
            Err(JieqiGameError::OperationUnavailable(
                capability.reason.expect("禁用能力必须包含原因"),
            ))
        }
    }
}

fn natural_result(position: &JieqiPosition) -> Option<JieqiGameResult> {
    match JieqiRules::end_state(position) {
        JieqiEndState::Ongoing => None,
        JieqiEndState::Checkmate { winner } => Some(JieqiGameResult::Winner {
            winner,
            reason: JieqiResultReason::Checkmate,
        }),
        JieqiEndState::Stalemate { winner } => Some(JieqiGameResult::Winner {
            winner,
            reason: JieqiResultReason::Stalemate,
        }),
    }
}
