use super::model::{
    JieqiDocument, JieqiDocumentInitial, JieqiDocumentKind, JieqiDocumentMetadata,
    JieqiDocumentMove, JieqiDocumentPublic, JieqiDocumentTermination, JieqiFileIdentity,
};
use crate::core::jieqi::{
    JieqiGame, JieqiGameResult, JieqiIdentity, JieqiIdentitySource, JieqiPosition, JieqiPublicKind,
    JieqiResultReason, JieqiReveal, STANDARD_INITIAL_SLOTS,
};
use crate::core::piece::PieceKind;
use crate::core::position::Move;
use crate::manual::ManualError;
use std::collections::{BTreeMap, HashMap};

pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_MOVES: usize = 10_000;
const MAX_METADATA_BYTES: usize = 4 * 1024;
const MAX_ANNOTATION_BYTES: usize = 64 * 1024;

pub struct JieqiDocumentCodec;

impl JieqiDocumentCodec {
    pub fn encode(
        game: &JieqiGame,
        kind: JieqiDocumentKind,
        metadata: JieqiDocumentMetadata,
        mut annotations: BTreeMap<u32, String>,
    ) -> Result<Vec<u8>, ManualError> {
        annotations.retain(|_, text| !text.trim().is_empty());
        validate_public_fields(&metadata, &annotations, game.head_ply())?;
        let initial = match kind {
            JieqiDocumentKind::PrivateGame => match game.initial_position().identities() {
                JieqiIdentitySource::Assigned(values) => JieqiDocumentInitial::Private {
                    assignments: values
                        .iter()
                        .map(|value| JieqiFileIdentity {
                            piece_id: value.piece_id,
                            kind: value.kind,
                        })
                        .collect(),
                },
                JieqiIdentitySource::RecordedReveals(_) => {
                    return invalid("公开回放不包含可导出的私有身份");
                }
            },
            JieqiDocumentKind::PublicReplay => JieqiDocumentInitial::Public,
        };
        let moves = game
            .history()
            .iter()
            .map(|ply| JieqiDocumentMove {
                iccs: ply.iccs.clone(),
                revealed: ply.revealed,
            })
            .collect();
        let termination = match game.final_result() {
            Some(JieqiGameResult::Winner {
                winner,
                reason: JieqiResultReason::Resignation,
            }) => Some(JieqiDocumentTermination::Resignation {
                side: winner.opposite(),
                ply: game.head_ply() as u32,
            }),
            Some(JieqiGameResult::DrawAgreement) => Some(JieqiDocumentTermination::DrawAgreement {
                ply: game.head_ply() as u32,
            }),
            _ => None,
        };
        let document = JieqiDocument {
            format: "canglang-jieqi".into(),
            version: 1,
            kind,
            rules: "jieqi_casual_v1".into(),
            play_mode: game.play_mode(),
            metadata,
            initial,
            moves,
            annotations,
            termination,
        };
        let bytes =
            serde_json::to_vec_pretty(&document).map_err(|error| ManualError::InvalidData {
                msg: error.to_string(),
            })?;
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return invalid("揭棋文件超过 16 MiB 限制");
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<(JieqiGame, JieqiDocumentPublic), ManualError> {
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return invalid("揭棋文件超过 16 MiB 限制");
        }
        let mut document: JieqiDocument =
            serde_json::from_slice(bytes).map_err(|error| ManualError::Parse {
                msg: error.to_string(),
            })?;
        validate_header(&document)?;
        validate_public_fields(
            &document.metadata,
            &document.annotations,
            document.moves.len(),
        )?;
        if document.moves.len() > MAX_MOVES {
            return invalid("揭棋文件超过 10000 个半回合");
        }
        document
            .annotations
            .retain(|_, text| !text.trim().is_empty());

        let mut game = match (&document.kind, &document.initial) {
            (JieqiDocumentKind::PrivateGame, JieqiDocumentInitial::Private { assignments }) => {
                let identities = assignments
                    .iter()
                    .map(|value| JieqiIdentity {
                        piece_id: value.piece_id,
                        kind: value.kind,
                    })
                    .collect();
                JieqiGame::new(
                    JieqiPosition::standard_assigned(identities).map_err(invalid_display)?,
                    document.play_mode,
                )
            }
            (JieqiDocumentKind::PublicReplay, JieqiDocumentInitial::Public) => {
                let reveals = collect_public_reveals(&document.moves)?;
                JieqiGame::new_public_replay_with_mode(
                    JieqiPosition::standard_recorded(reveals).map_err(invalid_display)?,
                    document.play_mode,
                )
            }
            _ => return invalid("文件 kind 与 initial 类型不匹配"),
        };

        for entry in &document.moves {
            match document.kind {
                JieqiDocumentKind::PrivateGame => {
                    let actual = game
                        .make_move(Move::from_iccs(&entry.iccs).map_err(invalid_display)?)
                        .map_err(invalid_display)?;
                    if actual.revealed != entry.revealed {
                        return invalid("私有棋谱的揭子结果与固定身份不一致");
                    }
                }
                JieqiDocumentKind::PublicReplay => game
                    .apply_recorded_evidence(&entry.iccs, entry.revealed)
                    .map_err(invalid_display)?,
            }
        }
        apply_termination(&mut game, document.termination, document.moves.len())?;

        let public = JieqiDocumentPublic {
            kind: document.kind,
            metadata: document.metadata,
            annotations: document.annotations,
        };
        Ok((game, public))
    }
}

fn validate_header(document: &JieqiDocument) -> Result<(), ManualError> {
    if document.format != "canglang-jieqi" {
        return invalid("未知揭棋文件格式");
    }
    if document.version != 1 {
        return invalid("不支持的揭棋文件版本");
    }
    if document.rules != "jieqi_casual_v1" {
        return invalid("不支持的揭棋规则标识");
    }
    Ok(())
}

fn validate_public_fields(
    metadata: &JieqiDocumentMetadata,
    annotations: &BTreeMap<u32, String>,
    head_ply: usize,
) -> Result<(), ManualError> {
    for value in [
        &metadata.title,
        &metadata.date,
        &metadata.red_player,
        &metadata.black_player,
        &metadata.event_name,
    ] {
        if value.len() > MAX_METADATA_BYTES {
            return invalid("元数据字段超过 4 KiB");
        }
    }
    for (ply, text) in annotations {
        if *ply as usize > head_ply {
            return invalid("备注位置超出主线范围");
        }
        if text.len() > MAX_ANNOTATION_BYTES {
            return invalid("单条备注超过 64 KiB");
        }
    }
    Ok(())
}

fn collect_public_reveals(moves: &[JieqiDocumentMove]) -> Result<Vec<JieqiReveal>, ManualError> {
    let mut positions: HashMap<_, _> = STANDARD_INITIAL_SLOTS
        .iter()
        .map(|slot| (slot.position, slot.id))
        .collect();
    let mut revealed = BTreeMap::new();
    for entry in moves {
        let mv = Move::from_iccs(&entry.iccs).map_err(invalid_display)?;
        let Some(piece_id) = positions.remove(&mv.from) else {
            return invalid("公开棋谱走法起点没有棋子");
        };
        positions.remove(&mv.to);
        positions.insert(mv.to, piece_id);
        if let Some(kind) = entry.revealed {
            if kind == JieqiPublicKind::King
                || revealed.insert(piece_id, to_piece_kind(kind)).is_some()
            {
                return invalid("公开棋谱包含非法或重复揭子事件");
            }
        }
    }
    Ok(revealed
        .into_iter()
        .map(|(piece_id, kind)| JieqiReveal { piece_id, kind })
        .collect())
}

fn apply_termination(
    game: &mut JieqiGame,
    termination: Option<JieqiDocumentTermination>,
    moves: usize,
) -> Result<(), ManualError> {
    let Some(termination) = termination else {
        return Ok(());
    };
    if game.final_result().is_some() {
        return invalid("自然终局不能再附加人为终局");
    }
    let result = match termination {
        JieqiDocumentTermination::Resignation { side, ply } => {
            if ply as usize != moves || side != game.position().turn() {
                return invalid("认输终局与主线末端不一致");
            }
            JieqiGameResult::Winner {
                winner: side.opposite(),
                reason: JieqiResultReason::Resignation,
            }
        }
        JieqiDocumentTermination::DrawAgreement { ply } => {
            if ply as usize != moves {
                return invalid("协议和棋与主线末端不一致");
            }
            JieqiGameResult::DrawAgreement
        }
    };
    game.restore_document_result(result)
        .map_err(invalid_display)
}

fn to_piece_kind(kind: JieqiPublicKind) -> PieceKind {
    match kind {
        JieqiPublicKind::King => PieceKind::King,
        JieqiPublicKind::Advisor => PieceKind::Advisor,
        JieqiPublicKind::Bishop => PieceKind::Bishop,
        JieqiPublicKind::Knight => PieceKind::Knight,
        JieqiPublicKind::Rook => PieceKind::Rook,
        JieqiPublicKind::Cannon => PieceKind::Cannon,
        JieqiPublicKind::Pawn => PieceKind::Pawn,
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, ManualError> {
    Err(ManualError::InvalidData {
        msg: message.into(),
    })
}

fn invalid_display(error: impl std::fmt::Display) -> ManualError {
    ManualError::InvalidData {
        msg: error.to_string(),
    }
}
