use crate::core::jieqi::{
    JieqiError, JieqiIdentity, JieqiPosition, STANDARD_INITIAL_SLOTS, standard_identity_kinds,
};
use crate::core::piece::{Color, PieceKind};
use rand::seq::SliceRandom;

pub fn create_random_jieqi_position() -> Result<JieqiPosition, JieqiError> {
    let mut rng = rand::rng();
    create_random_jieqi_position_with(&mut rng)
}

fn create_random_jieqi_position_with<R: rand::Rng + ?Sized>(
    rng: &mut R,
) -> Result<JieqiPosition, JieqiError> {
    let mut kinds = standard_identity_kinds();
    for color in [Color::Black, Color::Red] {
        let mut shuffled = kinds
            .iter()
            .enumerate()
            .filter(|(id, kind)| {
                STANDARD_INITIAL_SLOTS[*id].color == color && **kind != PieceKind::King
            })
            .map(|(_, kind)| *kind)
            .collect::<Vec<_>>();
        shuffled.shuffle(rng);
        let mut shuffled = shuffled.into_iter();
        for (id, kind) in kinds.iter_mut().enumerate() {
            let slot = STANDARD_INITIAL_SLOTS[id];
            if slot.color == color && slot.move_as != PieceKind::King {
                *kind = shuffled.next().expect("暗子身份数量由固定初始棋位保证");
            }
        }
    }
    let assignments = kinds
        .into_iter()
        .enumerate()
        .map(|(piece_id, kind)| JieqiIdentity {
            piece_id: piece_id as u8,
            kind,
        })
        .collect();
    JieqiPosition::standard_assigned(assignments)
}
