use super::session::{SessionError, SessionErrorCode, SessionSnapshot, SessionToken};
use super::{SessionMutation, XiangqiGame};
use crate::core::board::BoardState;
use crate::core::jieqi::{JieqiGame, JieqiPlayMode};
use crate::core::rules::RuleProfile;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_GAME_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
// GameState 始终只持有一个活动对局，小型内联状态比全链路 Box 间接访问更清晰
#[allow(clippy::large_enum_variant)]
pub enum ActiveGame {
    Xiangqi(XiangqiGame),
    Jieqi(JieqiGame),
}

#[derive(Debug, Clone)]
pub struct GameState {
    game_id: u64,
    revision: u64,
    content_revision: u64,
    active: ActiveGame,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new(BoardState::initial())
    }
}

impl GameState {
    pub fn new(board: BoardState) -> Self {
        Self::new_with_profile(board, RuleProfile::default())
    }

    pub fn new_with_profile(board: BoardState, profile: RuleProfile) -> Self {
        Self::from_active(ActiveGame::Xiangqi(XiangqiGame::new_with_profile(
            board, profile,
        )))
    }

    pub fn new_jieqi(game: JieqiGame) -> Self {
        Self::from_active(ActiveGame::Jieqi(game))
    }

    fn from_active(active: ActiveGame) -> Self {
        Self {
            game_id: NEXT_GAME_ID.fetch_add(1, Ordering::Relaxed),
            revision: 0,
            content_revision: 0,
            active,
        }
    }

    pub fn token(&self) -> SessionToken {
        SessionToken {
            game_id: self.game_id.to_string(),
            expected_revision: self.revision.to_string(),
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot::from_state(self)
    }

    pub fn active(&self) -> &ActiveGame {
        &self.active
    }

    pub fn xiangqi(&self) -> Result<&XiangqiGame, SessionError> {
        match &self.active {
            ActiveGame::Xiangqi(game) => Ok(game),
            ActiveGame::Jieqi(_) => Err(SessionError::new(
                SessionErrorCode::OperationUnavailable,
                "当前会话不是普通象棋",
            )),
        }
    }

    pub fn xiangqi_mut(&mut self) -> Result<&mut XiangqiGame, SessionError> {
        match &mut self.active {
            ActiveGame::Xiangqi(game) => Ok(game),
            ActiveGame::Jieqi(_) => Err(SessionError::new(
                SessionErrorCode::OperationUnavailable,
                "当前会话不是普通象棋",
            )),
        }
    }

    pub fn jieqi(&self) -> Result<&JieqiGame, SessionError> {
        match &self.active {
            ActiveGame::Jieqi(game) => Ok(game),
            ActiveGame::Xiangqi(_) => Err(SessionError::new(
                SessionErrorCode::OperationUnavailable,
                "当前会话不是揭棋",
            )),
        }
    }

    pub fn jieqi_mut(&mut self) -> Result<&mut JieqiGame, SessionError> {
        match &mut self.active {
            ActiveGame::Jieqi(game) => Ok(game),
            ActiveGame::Xiangqi(_) => Err(SessionError::new(
                SessionErrorCode::OperationUnavailable,
                "当前会话不是揭棋",
            )),
        }
    }

    pub fn check_token(&self, token: &SessionToken) -> Result<(), SessionError> {
        if token.game_id == self.game_id.to_string()
            && token.expected_revision == self.revision.to_string()
        {
            Ok(())
        } else {
            Err(SessionError::new(
                SessionErrorCode::StaleSession,
                "会话已变化，请刷新后重试",
            ))
        }
    }

    pub fn replace(
        &mut self,
        token: &SessionToken,
        candidate: ActiveGame,
    ) -> Result<SessionSnapshot, SessionError> {
        self.check_token(token)?;
        self.game_id = NEXT_GAME_ID.fetch_add(1, Ordering::Relaxed);
        self.revision = 0;
        self.content_revision = 0;
        self.active = candidate;
        Ok(self.snapshot())
    }

    pub fn mutate<T>(
        &mut self,
        token: &SessionToken,
        mutation: SessionMutation,
        apply: impl FnOnce(&mut ActiveGame) -> Result<T, SessionError>,
    ) -> Result<SessionSnapshot, SessionError> {
        self.check_token(token)?;
        apply(&mut self.active)?;
        self.revision += 1;
        if mutation == SessionMutation::Content {
            self.content_revision += 1;
        }
        Ok(self.snapshot())
    }

    pub(crate) const fn game_id(&self) -> u64 {
        self.game_id
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) const fn content_revision(&self) -> u64 {
        self.content_revision
    }

    pub fn jieqi_play_mode(&self) -> Option<JieqiPlayMode> {
        self.jieqi().ok().map(JieqiGame::play_mode)
    }
}
