use canglang_app::core::jieqi::{
    JieqiIdentity, JieqiIdentitySource, JieqiNotation, JieqiPiece, JieqiPosition,
    STANDARD_INITIAL_SLOTS, standard_identity_kinds,
};
use canglang_app::core::piece::Color;
use canglang_app::core::position::{Move, Position};

fn position(pieces: Vec<JieqiPiece>) -> JieqiPosition {
    let identities = standard_identity_kinds()
        .into_iter()
        .enumerate()
        .map(|(piece_id, kind)| JieqiIdentity {
            piece_id: piece_id as u8,
            kind,
        })
        .collect();
    JieqiPosition::from_parts(
        pieces,
        JieqiIdentitySource::Assigned(identities),
        Color::Red,
    )
    .unwrap()
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

#[test]
fn notation_uses_pre_move_public_role_outside_normal_boundaries() {
    let advisor = position(vec![
        piece(4, 0, 3, true),
        piece(27, 9, 5, true),
        piece(26, 6, 4, true),
    ]);
    assert_eq!(
        JieqiNotation::format(
            &advisor,
            Move::new(Position::new(6, 4), Position::new(5, 3))
        ),
        "仕五進六"
    );

    let hidden_rook = position(vec![
        piece(4, 0, 3, true),
        piece(27, 9, 5, true),
        piece(23, 6, 4, false),
    ]);
    assert_eq!(
        JieqiNotation::format(
            &hidden_rook,
            Move::new(Position::new(6, 4), Position::new(5, 4))
        ),
        "車五進一"
    );
}

#[test]
fn notation_disambiguates_multiple_revealed_pieces_on_one_file() {
    let rooks = position(vec![
        piece(4, 0, 3, true),
        piece(27, 9, 5, true),
        piece(23, 4, 0, true),
        piece(31, 7, 0, true),
    ]);
    assert_eq!(
        JieqiNotation::format(&rooks, Move::new(Position::new(4, 0), Position::new(3, 0))),
        "前車進一"
    );
    assert_eq!(
        JieqiNotation::format(&rooks, Move::new(Position::new(7, 0), Position::new(6, 0))),
        "后車進一"
    );
}

#[test]
fn missing_source_falls_back_to_iccs() {
    let board = position(vec![piece(4, 0, 3, true), piece(27, 9, 5, true)]);
    let mv = Move::new(Position::new(8, 8), Position::new(7, 8));
    assert_eq!(JieqiNotation::format(&board, mv), "i1i2");
}
