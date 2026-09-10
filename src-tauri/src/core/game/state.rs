use crate::core::board::BoardState;
use crate::core::hash::ZobristHasher;
use crate::core::notation::NotationConverter;
use crate::core::piece::{Color, Piece};
use crate::core::position::Move;
use crate::core::rules::MoveValidator;
use crate::core::rules::adjudicator::{CycleMove, assess_position_with_context};
use crate::core::rules::effects::{ChaseTarget, classify_move_details};
use crate::core::rules::repetition::{RepetitionAssessment, assess_repetition};
use crate::core::rules::{MoveEffect, RuleAssessment, RuleProfile};
use serde::{Deserialize, Serialize};

/// 对弈胜负状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameResult {
    Ongoing,
    RedWin,
    BlackWin,
    Draw,
}

/// 历史走法记录条目
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRecord {
    pub mv: Move,
    #[serde(default = "default_mover")]
    pub mover: Color,
    pub captured: Option<Piece>,
    pub chinese_notation: String,
    pub zobrist_hash: u64,
    #[serde(default)]
    pub position_fen: String,
    #[serde(default)]
    pub effect: MoveEffect,
    #[serde(default)]
    pub chase_targets: Vec<ChaseTarget>,
}

fn default_mover() -> Color {
    Color::Red
}

/// 棋局流程与状态机控制器：维护局面走棋、悔棋、重做、胜负判定与历史快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    #[serde(default)]
    initial_board: BoardState,
    current_board: BoardState,
    rule_profile: RuleProfile,
    result: GameResult,
    resigned: Option<Color>,
    history: Vec<MoveRecord>,
    #[serde(skip)]
    undo_stack: Vec<(BoardState, Move, Option<Piece>)>,
    #[serde(skip)]
    redo_history: Vec<MoveRecord>,
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

    pub fn new_with_profile(board: BoardState, rule_profile: RuleProfile) -> Self {
        let mut state = Self {
            initial_board: board,
            current_board: board,
            rule_profile,
            result: GameResult::Ongoing,
            resigned: None,
            history: Vec::new(),
            undo_stack: Vec::new(),
            redo_history: Vec::new(),
        };
        state.update_game_result();
        state
    }

    #[inline]
    pub fn rule_profile(&self) -> RuleProfile {
        self.rule_profile
    }

    pub fn set_rule_profile(&mut self, profile: RuleProfile) {
        self.rule_profile = profile;
        self.update_game_result();
    }

    pub fn repetition_count(&self) -> u32 {
        let current_hash = ZobristHasher::compute(&self.current_board, false);
        let current_fen = self.current_board.to_fen();
        let history_matches = self
            .history
            .iter()
            .filter(|record| {
                record.zobrist_hash == current_hash && record.position_fen == current_fen
            })
            .count() as u32;
        history_matches + u32::from(self.initial_board.to_fen() == current_fen)
    }

    /// 返回当前局面与上一次相同局面之间的完整回合步数
    pub fn repetition_cycle_length(&self) -> Option<u32> {
        let current_fen = self.current_board.to_fen();
        let matches: Vec<usize> = self
            .history
            .iter()
            .enumerate()
            .filter(|(_, record)| record.position_fen == current_fen)
            .map(|(index, _)| index)
            .collect();

        let &current_index = matches.last()?;
        if let Some(&previous_index) = matches.iter().rev().nth(1) {
            Some((current_index - previous_index) as u32)
        } else if self.initial_board.to_fen() == current_fen {
            Some((current_index + 1) as u32)
        } else {
            None
        }
    }

    pub fn repetition_assessment(&self) -> RepetitionAssessment {
        assess_repetition(
            self.rule_profile,
            self.repetition_count(),
            self.repetition_cycle_length(),
        )
    }

    pub fn rule_assessment(&self) -> RuleAssessment {
        let cycle_moves: Vec<CycleMove> = self
            .history
            .iter()
            .map(|record| CycleMove {
                side: record.mover,
                effect: record.effect,
                chase_targets: record.chase_targets.clone(),
            })
            .collect();
        assess_position_with_context(
            self.rule_profile,
            &self.current_board,
            self.repetition_count(),
            self.repetition_cycle_length(),
            &cycle_moves,
        )
    }

    /// 返回当前棋盘的值快照
    #[inline]
    pub fn current_board(&self) -> BoardState {
        self.current_board
    }

    /// 返回初始棋盘的引用
    #[inline]
    pub fn initial_board(&self) -> &BoardState {
        &self.initial_board
    }

    /// 返回对局起始局面的 FEN
    #[inline]
    pub fn start_fen(&self) -> String {
        self.initial_board.to_fen()
    }

    /// 返回当前已走步数（游标位置，0 为起始局面）
    #[inline]
    pub fn current_ply(&self) -> usize {
        self.history.len()
    }

    /// 返回包含已走步与未来可重做步的完整主线序列
    pub fn full_history(&self) -> Vec<MoveRecord> {
        let mut full = self.history.clone();
        for record in self.redo_history.iter().rev() {
            full.push(record.clone());
        }
        full
    }

    /// 返回当前对局结果
    #[inline]
    pub fn result(&self) -> GameResult {
        self.result
    }

    /// 返回主线历史的只读视图
    #[inline]
    pub fn history(&self) -> &[MoveRecord] {
        &self.history
    }

    #[inline]
    pub fn is_red_to_move(&self) -> bool {
        self.current_board.turn.is_red()
    }

    #[inline]
    pub fn is_in_check(&self) -> bool {
        MoveValidator::is_in_check(&self.current_board, self.current_board.turn)
    }

    /// 尝试走一步棋。如果合法则更新局面状态并返回 true，否则返回 false
    pub fn make_move(&mut self, mv: Move) -> bool {
        if self.result != GameResult::Ongoing {
            return false;
        }

        if !MoveValidator::can_move(&self.current_board, mv) {
            return false;
        }

        let captured = self.current_board.get_piece(mv.to);
        let previous_board = self.current_board;
        let cn = NotationConverter::to_chinese_notation(&previous_board, &mv).unwrap_or_default();

        // 执行走棋
        let (next_board, _) = self.current_board.apply_move(mv);
        self.current_board = next_board;

        self.undo_stack.push((previous_board, mv, captured));
        self.redo_history.clear();
        self.resigned = None;

        let hash = ZobristHasher::compute(&self.current_board, false);
        let classification = classify_move_details(&previous_board, mv);
        self.history.push(MoveRecord {
            mv,
            mover: previous_board.turn,
            captured,
            chinese_notation: cn,
            zobrist_hash: hash,
            position_fen: self.current_board.to_fen(),
            effect: classification.effect,
            chase_targets: classification.chase_targets,
        });

        self.update_game_result();
        true
    }

    /// 尝试通过 ICCS 字符串（如 "h2e2"）走棋
    pub fn make_move_iccs(&mut self, iccs: &str) -> bool {
        if let Ok(mv) = Move::from_iccs(iccs) {
            self.make_move(mv)
        } else {
            false
        }
    }

    /// 内部单步回退（不触发终局判定）
    fn undo_step(&mut self) -> bool {
        self.resigned = None;
        if let Some((previous_board, _, _)) = self.undo_stack.pop() {
            if let Some(record) = self.history.pop() {
                self.redo_history.push(record);
            }
            self.current_board = previous_board;
            true
        } else {
            false
        }
    }

    /// 内部单步重做（不触发终局判定）
    fn redo_step(&mut self) -> bool {
        let Some(record) = self.redo_history.last() else {
            return false;
        };
        let mv = record.mv;
        if MoveValidator::can_move(&self.current_board, mv) {
            let record = self.redo_history.pop().unwrap();
            let captured = self.current_board.get_piece(mv.to);
            let previous_board = self.current_board;
            let (next_board, _) = self.current_board.apply_move(mv);

            self.current_board = next_board;
            self.undo_stack.push((previous_board, mv, captured));
            self.history.push(record);
            return true;
        }
        false
    }

    /// 悔棋（回退至上一步局面）。成功返回 true，已至根局面返回 false
    pub fn undo_move(&mut self) -> bool {
        if self.undo_step() {
            self.update_game_result();
            true
        } else {
            false
        }
    }

    /// 重做（恢复刚才悔棋的一步）
    pub fn redo_move(&mut self) -> bool {
        if self.redo_step() {
            self.update_game_result();
            true
        } else {
            false
        }
    }

    /// 将游标快速定位到目标步数（0 表示起始局面）
    /// 在内存中批量前进或后退，中间步跳过复杂的终局判定，仅在最终目标位置裁决
    pub fn jump_to(&mut self, target_ply: usize) {
        let max_ply = self.history.len() + self.redo_history.len();
        let target = target_ply.min(max_ply);
        while self.history.len() > target {
            if !self.undo_step() {
                break;
            }
        }
        while self.history.len() < target {
            if !self.redo_step() {
                break;
            }
        }
        self.update_game_result();
    }

    /// 当前是否可以进行悔棋（存在可回退的已走步）
    #[inline]
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// 当前是否可以进行重做（存在刚悔掉的步）
    #[inline]
    pub fn can_redo(&self) -> bool {
        !self.redo_history.is_empty()
    }

    /// 玩家认输
    pub fn resign(&mut self, color: Color) {
        if self.result == GameResult::Ongoing {
            self.resigned = Some(color);
            self.result = if color.is_red() {
                GameResult::BlackWin
            } else {
                GameResult::RedWin
            };
        }
    }

    /// 检查并更新客观胜负判定（绝杀或困毙）
    ///
    /// 象棋走法物理层与胜负判定严格解耦于赛事裁判规则：对局结果只由绝杀（被将军无合法解法）
    /// 与困毙（无子可走）以及玩家主动认输决定。长将、长捉等复杂赛事裁决规则保留在
    /// `rule_assessment()` 作为参考与建议展示给用户，不再强制中止对局或篡改 `result`
    pub fn update_game_result(&mut self) {
        if let Some(color) = self.resigned {
            self.result = if color.is_red() {
                GameResult::BlackWin
            } else {
                GameResult::RedWin
            };
            return;
        }

        let turn = self.current_board.turn;
        let checkmate = MoveValidator::is_checkmate(&self.current_board, turn);
        let stalemate = MoveValidator::is_stalemate(&self.current_board, turn);

        if checkmate || stalemate {
            // 象棋规则：无子可走方（困毙）或被绝杀方负
            self.result = if turn.is_red() {
                GameResult::BlackWin
            } else {
                GameResult::RedWin
            };
        } else {
            self.result = GameResult::Ongoing;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_make_move_and_undo() {
        let mut game = GameState::default();
        assert_eq!(game.result(), GameResult::Ongoing);
        assert!(game.is_red_to_move());

        // 炮二平五: h2e2
        assert!(game.make_move_iccs("h2e2"));
        assert!(!game.is_red_to_move());
        assert_eq!(game.history().len(), 1);

        // 马８进７: b9c7
        assert!(game.make_move_iccs("b9c7"));
        assert!(game.is_red_to_move());
        assert_eq!(game.history().len(), 2);

        // 撤销 马８进７
        assert!(game.undo_move());
        assert!(!game.is_red_to_move());
        assert_eq!(game.history().len(), 1);

        // 重做 马８进７
        assert!(game.redo_move());
        assert!(game.is_red_to_move());
        assert_eq!(game.history().len(), 2);
    }

    #[test]
    fn test_game_resign() {
        let mut game = GameState::default();
        game.make_move_iccs("h2e2");
        game.resign(Color::Red);
        assert_eq!(game.result(), GameResult::BlackWin);
        // 认输后无法继续走子
        assert!(!game.make_move_iccs("b9c7"));
        // 切换规则或 jump_to 当前步不会将认输冲刷为 ongoing
        game.set_rule_profile(RuleProfile::Asian2017);
        assert_eq!(game.result(), GameResult::BlackWin);
        game.jump_to(1);
        assert_eq!(game.result(), GameResult::BlackWin);

        // 但若主动悔棋回到先前步，认输状态清除
        assert!(game.undo_move());
        assert_eq!(game.result(), GameResult::Ongoing);
    }

    #[test]
    fn game_state_keeps_rule_profile() {
        let state = GameState::new_with_profile(BoardState::initial(), RuleProfile::Asian2017);
        assert_eq!(state.rule_profile(), RuleProfile::Asian2017);
    }

    #[test]
    fn failed_redo_preserves_the_redo_stack() {
        let mut game = GameState::default();
        let invalid_move = Move::from_iccs("a0a0").unwrap();
        game.redo_history.push(MoveRecord {
            mv: invalid_move,
            mover: Color::Red,
            captured: None,
            chinese_notation: "炮二平五".to_string(),
            zobrist_hash: 0,
            position_fen: BoardState::initial().to_fen(),
            effect: crate::core::rules::MoveEffect::Idle,
            chase_targets: Vec::new(),
        });

        assert!(!game.redo_move());
        assert!(game.can_redo());
    }

    #[test]
    fn jump_to_navigates_smoothly() {
        let mut game = GameState::default();
        assert!(game.make_move_iccs("h2e2")); // ply 1
        assert!(game.make_move_iccs("h9g7")); // ply 2
        assert!(game.make_move_iccs("b0c2")); // ply 3
        assert_eq!(game.current_ply(), 3);
        assert_eq!(game.full_history().len(), 3);

        // 跳转到根局面
        game.jump_to(0);
        assert_eq!(game.current_ply(), 0);
        assert_eq!(game.full_history().len(), 3);
        assert_eq!(
            game.current_board().to_fen(),
            BoardState::initial().to_fen()
        );
        assert!(game.can_redo());
        assert!(!game.can_undo());

        // 跳转到第 2 个半回合
        game.jump_to(2);
        assert_eq!(game.current_ply(), 2);
        assert!(game.can_undo());
        assert!(game.can_redo());

        // 超出范围时限制到第 3 个半回合
        game.jump_to(100);
        assert_eq!(game.current_ply(), 3);
        assert!(!game.can_redo());
        assert!(game.can_undo());
    }

    #[test]
    fn exact_repetition_reports_cycle_length() {
        let mut game = GameState::default();
        // 通过两个无冲突的炮步往返形成同一局面，重复计数仍需完整 FEN 确认
        for iccs in [
            "h2e2", "h9g7", "e2h2", "g7h9", "h2e2", "h9g7", "e2h2", "g7h9",
        ] {
            assert!(game.make_move_iccs(iccs));
        }

        assert_eq!(game.repetition_count(), 3);
        assert_eq!(game.repetition_cycle_length(), Some(4));
        assert_eq!(
            game.rule_assessment().status,
            crate::core::rules::RuleStatus::RepetitionPending
        );
    }
}
