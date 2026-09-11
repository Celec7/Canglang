use canglang_app::core::jieqi::{
    JieqiCapabilityReason, JieqiGame, JieqiGameError, JieqiGameResult, JieqiIdentity,
    JieqiIdentitySource, JieqiOperation, JieqiPiece, JieqiPlayMode, JieqiPosition, JieqiPublicKind,
    JieqiResultReason, JieqiReveal, JieqiRules, JieqiSource, PublicPly, STANDARD_INITIAL_SLOTS,
    standard_identity_kinds,
};
use canglang_app::core::piece::{Color, PieceKind};
use canglang_app::core::position::{Move, Position};

fn identities() -> Vec<JieqiIdentity> {
    standard_identity_kinds()
        .into_iter()
        .enumerate()
        .map(|(piece_id, kind)| JieqiIdentity {
            piece_id: piece_id as u8,
            kind,
        })
        .collect()
}

fn piece(id: u8, row: u8, col: u8, revealed: bool) -> JieqiPiece {
    let slot = STANDARD_INITIAL_SLOTS[id as usize];
    JieqiPiece {
        id,
        color: slot.color,
        move_as: slot.move_as,
        position: Position::new(row, col),
        revealed,
    }
}

fn assigned_position(pieces: Vec<JieqiPiece>, assigned: Vec<JieqiIdentity>) -> JieqiPosition {
    JieqiPosition::from_parts(pieces, JieqiIdentitySource::Assigned(assigned), Color::Red).unwrap()
}

fn simple_position() -> JieqiPosition {
    assigned_position(
        vec![
            piece(4, 0, 3, true),
            piece(27, 9, 5, true),
            piece(23, 7, 0, false),
            piece(8, 2, 8, false),
        ],
        identities(),
    )
}

#[test]
fn failed_move_is_atomic() {
    let mut game = JieqiGame::new(simple_position(), JieqiPlayMode::Training);
    let before = game.clone();
    let result = game.make_move(Move::new(Position::new(7, 0), Position::new(6, 1)));
    assert_eq!(result, Err(JieqiGameError::IllegalMove));
    assert_eq!(game, before);
}

#[test]
fn move_exposing_king_has_specific_error_and_is_atomic() {
    let initial = assigned_position(
        vec![
            piece(4, 0, 3, true),
            piece(27, 9, 4, true),
            piece(8, 0, 4, true),
            piece(21, 5, 4, true),
        ],
        identities(),
    );
    let mut game = JieqiGame::new(initial, JieqiPlayMode::Training);
    let before = game.clone();

    let result = game.make_move(Move::new(Position::new(5, 4), Position::new(5, 5)));

    assert_eq!(result, Err(JieqiGameError::ExposesKing));
    assert_eq!(game, before);
}

#[test]
fn hidden_capture_undo_and_redo_restore_identical_identity() {
    let mut assigned = identities();
    let rook = assigned[23].kind;
    assigned[23].kind = assigned[24].kind;
    assigned[24].kind = rook;
    let initial = assigned_position(
        vec![
            piece(4, 0, 3, true),
            piece(27, 9, 5, true),
            piece(23, 6, 4, false),
            piece(11, 5, 4, false),
        ],
        assigned,
    );
    let initial_view = initial.public_view();
    let mut game = JieqiGame::new(initial, JieqiPlayMode::Training);
    let mv = Move::new(Position::new(6, 4), Position::new(5, 4));
    let ply = game.make_move(mv).unwrap().clone();
    assert!(matches!(
        ply.captured,
        canglang_app::core::jieqi::CapturedPieceView::Hidden
    ));
    let revealed_kind = JieqiRules::public_kind(
        game.position(),
        JieqiRules::piece_at(game.position(), Position::new(5, 4)).unwrap(),
    );
    assert_eq!(revealed_kind, PieceKind::Knight);
    let moved_view = game.position().public_view();

    game.undo().unwrap();
    assert_eq!(game.position().public_view(), initial_view);
    game.redo().unwrap();
    assert_eq!(game.position().public_view(), moved_view);
    assert_eq!(
        JieqiRules::public_kind(
            game.position(),
            JieqiRules::piece_at(game.position(), Position::new(5, 4)).unwrap()
        ),
        PieceKind::Knight
    );
}

#[test]
fn training_branch_replaces_future_main_line() {
    let mut game = JieqiGame::new(simple_position(), JieqiPlayMode::Training);
    game.make_move(Move::new(Position::new(7, 0), Position::new(6, 0)))
        .unwrap();
    game.make_move(Move::new(Position::new(2, 8), Position::new(3, 8)))
        .unwrap();
    game.undo().unwrap();
    game.make_move(Move::new(Position::new(2, 8), Position::new(2, 7)))
        .unwrap();

    assert_eq!(game.head_ply(), 2);
    assert_eq!(game.current_ply(), 2);
    assert_eq!(game.history()[1].iccs, "i7h7");
    assert!(!game.capability(JieqiOperation::Redo).enabled);
}

#[test]
fn duel_forbids_undo_and_finished_review_never_reopens_play() {
    let mut game = JieqiGame::new(simple_position(), JieqiPlayMode::Duel);
    assert_eq!(
        game.undo(),
        Err(JieqiGameError::OperationUnavailable(
            JieqiCapabilityReason::DuelPolicy
        ))
    );
    game.resign(Color::Red).unwrap();
    assert_eq!(
        game.result(),
        Some(JieqiGameResult::Winner {
            winner: Color::Black,
            reason: canglang_app::core::jieqi::JieqiResultReason::Resignation,
        })
    );
    game.jump_to(0).unwrap();
    assert!(!game.capability(JieqiOperation::Move).enabled);
}

#[test]
fn draw_offer_pauses_game_and_validates_participants_and_id() {
    let mut game = JieqiGame::new(simple_position(), JieqiPlayMode::Duel);
    let offer_id = game.offer_draw(Color::Red).unwrap().id.clone();
    assert_eq!(game.position().turn(), Color::Red);
    assert_eq!(
        game.make_move(Move::new(Position::new(7, 0), Position::new(6, 0))),
        Err(JieqiGameError::OperationUnavailable(
            JieqiCapabilityReason::PendingDraw
        ))
    );
    assert_eq!(
        game.respond_draw(&offer_id, Color::Red, true),
        Err(JieqiGameError::WrongDrawResponder)
    );
    assert_eq!(
        game.respond_draw("old", Color::Black, true),
        Err(JieqiGameError::StaleDrawOffer)
    );
    game.respond_draw(&offer_id, Color::Black, false).unwrap();
    assert!(game.draw_offer().is_none());
    assert!(game.capability(JieqiOperation::Move).enabled);

    let accepted = game.offer_draw(Color::Red).unwrap().id.clone();
    game.respond_draw(&accepted, Color::Black, true).unwrap();
    assert_eq!(game.result(), Some(JieqiGameResult::DrawAgreement));
}

#[test]
fn public_replay_rejects_interactive_moves() {
    let mut game = JieqiGame::new_public_replay(simple_position());
    assert_eq!(game.source(), JieqiSource::PublicReplay);
    assert_eq!(
        game.make_move(Move::new(Position::new(7, 0), Position::new(6, 0))),
        Err(JieqiGameError::OperationUnavailable(
            JieqiCapabilityReason::ReadOnly
        ))
    );
    assert!(game.capability(JieqiOperation::Jump).enabled);
}

#[test]
fn public_replay_only_commits_matching_recorded_event() {
    let position = JieqiPosition::from_parts(
        vec![
            piece(4, 0, 3, true),
            piece(27, 9, 5, true),
            piece(23, 7, 0, false),
        ],
        JieqiIdentitySource::RecordedReveals(vec![JieqiReveal {
            piece_id: 23,
            kind: PieceKind::Rook,
        }]),
        Color::Red,
    )
    .unwrap();
    let mut game = JieqiGame::new_public_replay(position);
    let recorded = PublicPly {
        ply: 1,
        iccs: "a2a3".to_string(),
        mover: Color::Red,
        notation: "車九進一".to_string(),
        revealed: Some(JieqiPublicKind::Rook),
        captured: canglang_app::core::jieqi::CapturedPieceView::None,
        is_check: false,
    };
    let mut mismatched = recorded.clone();
    mismatched.revealed = Some(JieqiPublicKind::Knight);
    let before = game.clone();
    assert_eq!(
        game.apply_recorded_move(mismatched),
        Err(JieqiGameError::RecordedPlyMismatch)
    );
    assert_eq!(game, before);

    game.apply_recorded_move(recorded).unwrap();
    assert_eq!(game.current_ply(), 1);
    assert_eq!(game.history()[0].revealed, Some(JieqiPublicKind::Rook));
}

#[test]
fn training_undo_restores_result_and_redo_restores_it_again() {
    let mut game = JieqiGame::new(simple_position(), JieqiPlayMode::Training);
    game.make_move(Move::new(Position::new(7, 0), Position::new(6, 0)))
        .unwrap();
    game.resign(Color::Black).unwrap();
    assert_eq!(
        game.result(),
        Some(JieqiGameResult::Winner {
            winner: Color::Red,
            reason: JieqiResultReason::Resignation,
        })
    );
    game.undo().unwrap();
    assert_eq!(game.result(), None);
    game.redo().unwrap();
    assert_eq!(
        game.result(),
        Some(JieqiGameResult::Winner {
            winner: Color::Red,
            reason: JieqiResultReason::Resignation,
        })
    );
}
