use canglang_app::core::jieqi::{
    JieqiError, JieqiIdentity, JieqiIdentitySource, JieqiPiece, JieqiPosition,
    STANDARD_INITIAL_SLOTS, standard_identity_kinds,
};
use canglang_app::core::piece::{Color, PieceKind};
use canglang_app::core::position::Position;
use canglang_app::services::jieqi_setup::create_random_jieqi_position;

fn standard_assignments() -> Vec<JieqiIdentity> {
    standard_identity_kinds()
        .into_iter()
        .enumerate()
        .map(|(piece_id, kind)| JieqiIdentity {
            piece_id: piece_id as u8,
            kind,
        })
        .collect()
}

fn standard_pieces() -> Vec<JieqiPiece> {
    STANDARD_INITIAL_SLOTS
        .iter()
        .map(|slot| JieqiPiece {
            id: slot.id,
            color: slot.color,
            move_as: slot.move_as,
            position: slot.position,
            revealed: slot.move_as == PieceKind::King,
        })
        .collect()
}

#[test]
fn random_setup_preserves_all_pieces_and_public_information() {
    for _ in 0..32 {
        let position = create_random_jieqi_position().unwrap();
        assert_eq!(position.pieces().len(), 32);
        assert_eq!(
            position
                .pieces()
                .iter()
                .filter(|piece| piece.revealed)
                .count(),
            2
        );
        assert_eq!(
            position
                .pieces()
                .iter()
                .filter(|piece| !piece.revealed)
                .count(),
            30
        );
        let view = position.public_view();
        assert_eq!(view.pieces.len(), 32);
        assert_eq!(view.turn, Color::Red);
    }
}

#[test]
fn hidden_identity_swaps_have_identical_public_serialization() {
    let left = JieqiPosition::standard_assigned(standard_assignments()).unwrap();
    let mut swapped = standard_assignments();
    swapped.swap(0, 1);
    let first_id = swapped[0].piece_id;
    swapped[0].piece_id = swapped[1].piece_id;
    swapped[1].piece_id = first_id;
    let right = JieqiPosition::standard_assigned(swapped).unwrap();

    assert_eq!(
        serde_json::to_value(left.public_view()).unwrap(),
        serde_json::to_value(right.public_view()).unwrap()
    );
}

#[test]
fn rejects_duplicate_identity_and_invalid_multiset() {
    let mut duplicate = standard_assignments();
    duplicate[1].piece_id = duplicate[0].piece_id;
    assert!(matches!(
        JieqiPosition::standard_assigned(duplicate),
        Err(JieqiError::DuplicateIdentityId(_))
    ));

    let mut invalid = standard_assignments();
    invalid[0].kind = PieceKind::Knight;
    assert!(matches!(
        JieqiPosition::standard_assigned(invalid),
        Err(JieqiError::InvalidIdentityMultiset { .. })
    ));
}

#[test]
fn rejects_duplicate_and_out_of_bounds_positions() {
    let mut duplicate = standard_pieces();
    duplicate[1].position = duplicate[0].position;
    assert!(matches!(
        JieqiPosition::from_parts(
            duplicate,
            JieqiIdentitySource::Assigned(standard_assignments()),
            Color::Red,
        ),
        Err(JieqiError::DuplicatePosition(_))
    ));

    let mut invalid = standard_pieces();
    invalid[0].position = Position::new(10, 0);
    assert!(matches!(
        JieqiPosition::from_parts(
            invalid,
            JieqiIdentitySource::Assigned(standard_assignments()),
            Color::Red,
        ),
        Err(JieqiError::InvalidPosition(_))
    ));
}

#[test]
fn accepts_valid_runtime_positions_after_initial_setup() {
    let mut moved = standard_pieces();
    moved[0].position = Position::new(1, 0);
    moved[0].revealed = true;
    let position = JieqiPosition::from_parts(
        moved,
        JieqiIdentitySource::Assigned(standard_assignments()),
        Color::Black,
    )
    .unwrap();

    assert_eq!(position.turn(), Color::Black);
    assert_eq!(position.pieces()[0].position, Position::new(1, 0));
}

#[test]
fn accepts_captured_runtime_position_with_both_kings() {
    let pieces = standard_pieces()
        .into_iter()
        .filter(|piece| matches!(piece.id, 0 | 4 | 27))
        .collect();
    let position = JieqiPosition::from_parts(
        pieces,
        JieqiIdentitySource::Assigned(standard_assignments()),
        Color::Red,
    )
    .unwrap();

    assert_eq!(position.pieces().len(), 3);
}
