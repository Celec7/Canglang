use canglang_app::core::game::GameState;
use canglang_app::core::rules::{MoveEffect, RuleExplanationCode, RuleProfile, RuleStatus};

#[test]
fn rule_profile_and_basic_effects_survive_a_game_session() {
    let mut game = GameState::new_with_profile(
        canglang_app::core::board::BoardState::initial(),
        RuleProfile::Asian2017,
    );

    assert!(game.make_move_iccs("h2e2"));
    assert!(game.make_move_iccs("h9g7"));
    assert_eq!(game.rule_profile(), RuleProfile::Asian2017);
    assert_eq!(game.history()[0].effect, MoveEffect::Chase);
    assert_eq!(game.rule_assessment().status, RuleStatus::Ongoing);
}

#[test]
fn repeated_position_returns_structured_pending_explanation() {
    let mut game = GameState::default();
    for iccs in [
        "h2e2", "h9g7", "e2h2", "g7h9", "h2e2", "h9g7", "e2h2", "g7h9",
    ] {
        assert!(game.make_move_iccs(iccs));
    }

    let assessment = game.rule_assessment();
    let explanation = assessment.explanation.expect("repetition explanation");

    assert_eq!(assessment.status, RuleStatus::RepetitionPending);
    assert_eq!(
        explanation.code,
        RuleExplanationCode::RepetitionNeedsJudgement
    );
    assert_eq!(explanation.category, Some(MoveEffect::Pending));
    assert_eq!(explanation.cycle_length, Some(4));
}

#[test]
fn legal_perpetual_check_cycle_identifies_the_checking_side() {
    let board =
        canglang_app::core::board::BoardState::from_fen("3k5/4R4/9/9/4P4/9/9/9/9/4K4 w").unwrap();
    let mut game = GameState::new_with_profile(board, RuleProfile::China2020);

    for (index, iccs) in [
        "e8d8", "d9e9", "d8e8", "e9d9", "e8d8", "d9e9", "d8e8", "e9d9",
    ]
    .into_iter()
    .enumerate()
    {
        assert!(game.make_move_iccs(iccs), "failed move {index}: {iccs}");
    }

    let assessment = game.rule_assessment();
    let explanation = assessment.explanation.expect("perpetual check explanation");
    assert_eq!(assessment.status, RuleStatus::RedLossByRule);
    assert_eq!(explanation.side.as_deref(), Some("red"));
    assert_eq!(explanation.category, Some(MoveEffect::Check));
    // 规则参考与客观终局解耦：裁判状态判负供提示参考，对局物理层不强行中断
    assert_eq!(game.result(), canglang_app::core::game::GameResult::Ongoing);
}

#[test]
fn legal_perpetual_chase_cycle_is_an_asian_same_kind_exception() {
    let board =
        canglang_app::core::board::BoardState::from_fen("3k5/9/9/9/5r3/4R4/4p4/9/9/4K4 w").unwrap();
    let mut game = GameState::new_with_profile(board, RuleProfile::Asian2017);

    for iccs in [
        "e4f4", "f5e5", "f4e4", "e5f5", "e4f4", "f5e5", "f4e4", "e5f5",
    ] {
        assert!(game.make_move_iccs(iccs));
    }

    let assessment = game.rule_assessment();
    let explanation = assessment.explanation.expect("perpetual chase explanation");
    assert_eq!(assessment.status, RuleStatus::DrawByRule);
    assert_eq!(explanation.side.as_deref(), Some("red"));
    assert_eq!(explanation.category, Some(MoveEffect::Chase));
    assert_eq!(game.result(), canglang_app::core::game::GameResult::Ongoing);
}

#[test]
fn legal_perpetual_chase_cycle_is_a_china_single_side_loss() {
    let board =
        canglang_app::core::board::BoardState::from_fen("3k5/9/9/9/5r3/4R4/4p4/9/9/4K4 w").unwrap();
    let mut game = GameState::new_with_profile(board, RuleProfile::China2020);

    for iccs in [
        "e4f4", "f5e5", "f4e4", "e5f5", "e4f4", "f5e5", "f4e4", "e5f5",
    ] {
        assert!(game.make_move_iccs(iccs));
    }

    let assessment = game.rule_assessment();
    let explanation = assessment.explanation.expect("perpetual chase explanation");
    assert_eq!(assessment.status, RuleStatus::RedLossByRule);
    assert_eq!(explanation.side.as_deref(), Some("red"));
    assert_eq!(explanation.category, Some(MoveEffect::Chase));
    assert_eq!(game.result(), canglang_app::core::game::GameResult::Ongoing);
}
