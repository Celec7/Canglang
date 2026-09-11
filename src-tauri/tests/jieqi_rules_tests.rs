use canglang_app::core::jieqi::{
    JieqiEndState, JieqiIdentity, JieqiIdentitySource, JieqiMoveRejection, JieqiPiece,
    JieqiPosition, JieqiRules, STANDARD_INITIAL_SLOTS, standard_identity_kinds,
};
use canglang_app::core::piece::{Color, PieceKind};
use canglang_app::core::position::{Move, Position};

fn assignments() -> Vec<JieqiIdentity> {
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

fn position(pieces: Vec<JieqiPiece>, turn: Color) -> JieqiPosition {
    JieqiPosition::from_parts(pieces, JieqiIdentitySource::Assigned(assignments()), turn).unwrap()
}

fn kings() -> Vec<JieqiPiece> {
    vec![piece(4, 0, 3, true), piece(27, 9, 5, true)]
}

#[test]
fn hidden_and_revealed_advisors_and_bishops_use_different_boundaries() {
    let mut hidden_advisor = kings();
    hidden_advisor.push(piece(26, 7, 4, false));
    let hidden_advisor = position(hidden_advisor, Color::Red);
    assert!(!JieqiRules::can_move(
        &hidden_advisor,
        Move::new(Position::new(7, 4), Position::new(6, 3))
    ));

    let mut revealed_advisor = kings();
    revealed_advisor.push(piece(26, 7, 4, true));
    let revealed_advisor = position(revealed_advisor, Color::Red);
    assert!(JieqiRules::can_move(
        &revealed_advisor,
        Move::new(Position::new(7, 4), Position::new(6, 3))
    ));

    let mut hidden_bishop = kings();
    hidden_bishop.push(piece(25, 5, 2, false));
    let hidden_bishop = position(hidden_bishop, Color::Red);
    assert!(!JieqiRules::can_move(
        &hidden_bishop,
        Move::new(Position::new(5, 2), Position::new(3, 4))
    ));

    let mut revealed_bishop = kings();
    revealed_bishop.push(piece(25, 5, 2, true));
    let revealed_bishop = position(revealed_bishop, Color::Red);
    assert!(JieqiRules::can_move(
        &revealed_bishop,
        Move::new(Position::new(5, 2), Position::new(3, 4))
    ));
}

#[test]
fn horse_leg_bishop_eye_cannon_screen_and_facing_kings_are_enforced() {
    let mut horse = kings();
    horse.push(piece(24, 7, 4, true));
    horse.push(piece(16, 6, 4, false));
    let horse = position(horse, Color::Red);
    assert!(!JieqiRules::can_move(
        &horse,
        Move::new(Position::new(7, 4), Position::new(5, 3))
    ));

    let mut bishop = kings();
    bishop.push(piece(25, 7, 2, true));
    bishop.push(piece(16, 6, 3, false));
    let bishop = position(bishop, Color::Red);
    assert!(!JieqiRules::can_move(
        &bishop,
        Move::new(Position::new(7, 2), Position::new(5, 4))
    ));

    let mut cannon = kings();
    cannon.push(piece(21, 7, 4, true));
    cannon.push(piece(16, 5, 4, false));
    cannon.push(piece(11, 3, 4, false));
    let cannon = position(cannon, Color::Red);
    assert!(JieqiRules::can_move(
        &cannon,
        Move::new(Position::new(7, 4), Position::new(3, 4))
    ));

    let facing = position(
        vec![piece(4, 0, 4, true), piece(27, 9, 4, true)],
        Color::Red,
    );
    assert!(JieqiRules::is_in_check(&facing, Color::Red));
    assert!(JieqiRules::is_in_check(&facing, Color::Black));
}

#[test]
fn king_safety_and_stalemate_are_derived_from_public_geometry() {
    let mut pinned = vec![piece(4, 0, 3, true), piece(27, 9, 4, true)];
    pinned.push(piece(8, 0, 4, true));
    pinned.push(piece(21, 5, 4, true));
    let pinned = position(pinned, Color::Red);
    assert!(!JieqiRules::can_move(
        &pinned,
        Move::new(Position::new(5, 4), Position::new(5, 5))
    ));

    let stalemate = position(
        vec![
            piece(4, 0, 4, true),
            piece(27, 9, 3, true),
            piece(23, 1, 3, true),
            piece(31, 1, 5, true),
            piece(21, 1, 4, true),
        ],
        Color::Black,
    );
    assert_eq!(
        JieqiRules::end_state(&stalemate),
        JieqiEndState::Stalemate { winner: Color::Red }
    );
}

#[test]
fn candidate_targets_show_self_check_moves_while_validation_still_rejects_them() {
    let mut pinned = vec![piece(4, 0, 3, true), piece(27, 9, 4, true)];
    pinned.push(piece(8, 0, 4, true));
    pinned.push(piece(21, 5, 4, true));
    let pinned = position(pinned, Color::Red);
    let exposes_king = Move::new(Position::new(5, 4), Position::new(5, 5));

    assert!(JieqiRules::candidate_targets(&pinned, exposes_king.from).contains(&exposes_king.to));
    assert!(!JieqiRules::legal_targets(&pinned, exposes_king.from).contains(&exposes_king.to));
    assert_eq!(
        JieqiRules::validate_move(&pinned, exposes_king),
        Err(JieqiMoveRejection::ExposesKing)
    );
}

#[test]
fn checkmate_is_detected_without_capturing_the_king() {
    let checkmate = position(
        vec![
            piece(4, 0, 4, true),
            piece(27, 9, 3, true),
            piece(23, 0, 0, true),
            piece(31, 1, 5, true),
            piece(21, 1, 4, true),
        ],
        Color::Black,
    );
    assert_eq!(
        JieqiRules::end_state(&checkmate),
        JieqiEndState::Checkmate { winner: Color::Red }
    );
    assert!(!JieqiRules::can_move(
        &checkmate,
        Move::new(Position::new(0, 4), Position::new(0, 0))
    ));
}

#[test]
fn hidden_identity_does_not_change_targets_or_rejections() {
    let pieces = vec![
        piece(4, 0, 3, true),
        piece(27, 9, 5, true),
        piece(23, 7, 4, false),
    ];
    let left = position(pieces.clone(), Color::Red);
    let mut swapped = assignments();
    let rook = swapped[23].kind;
    swapped[23].kind = swapped[24].kind;
    swapped[24].kind = rook;
    let right =
        JieqiPosition::from_parts(pieces, JieqiIdentitySource::Assigned(swapped), Color::Red)
            .unwrap();
    assert_eq!(
        JieqiRules::legal_targets(&left, Position::new(7, 4)),
        JieqiRules::legal_targets(&right, Position::new(7, 4))
    );
    let rejected = Move::new(Position::new(7, 4), Position::new(6, 5));
    assert_eq!(
        JieqiRules::validate_move(&left, rejected),
        JieqiRules::validate_move(&right, rejected)
    );
}

#[test]
fn first_move_uses_hidden_role_then_revealed_identity_gives_check() {
    let mut identities = assignments();
    let rook = identities[23].kind;
    identities[23].kind = identities[24].kind;
    identities[24].kind = rook;
    let before = JieqiPosition::from_parts(
        vec![
            piece(4, 0, 3, true),
            piece(27, 9, 5, true),
            piece(23, 3, 4, false),
        ],
        JieqiIdentitySource::Assigned(identities),
        Color::Red,
    )
    .unwrap();
    let mv = Move::new(Position::new(3, 4), Position::new(2, 4));
    assert!(JieqiRules::can_move(&before, mv));
    let after = JieqiRules::apply_move(&before, mv).unwrap();
    assert_eq!(
        JieqiRules::public_kind(
            &after,
            JieqiRules::piece_at(&after, Position::new(2, 4)).unwrap()
        ),
        PieceKind::Knight
    );
    assert!(JieqiRules::is_in_check(&after, Color::Black));
}
