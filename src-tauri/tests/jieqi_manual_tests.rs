use canglang_app::core::jieqi::{
    JieqiGame, JieqiIdentity, JieqiPlayMode, JieqiPosition, JieqiReveal, standard_identity_kinds,
};
use canglang_app::core::piece::PieceKind;
use canglang_app::core::position::Move;
use canglang_app::manual::jieqi::{
    JieqiDocumentCodec, JieqiDocumentKind, JieqiDocumentMetadata, JieqiDocumentService,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn game() -> JieqiGame {
    let mut game = JieqiGame::new(
        JieqiPosition::standard_assigned(assignments()).unwrap(),
        JieqiPlayMode::Training,
    );
    game.make_move(Move::from_iccs("a3a4").unwrap()).unwrap();
    game.make_move(Move::from_iccs("a6a5").unwrap()).unwrap();
    game
}

fn metadata() -> JieqiDocumentMetadata {
    JieqiDocumentMetadata {
        title: "固定揭棋".into(),
        date: "2026-09-11".into(),
        red_player: "红方".into(),
        black_player: "黑方".into(),
        event_name: "测试".into(),
    }
}

fn annotations() -> BTreeMap<u32, String> {
    BTreeMap::from([(0, "起始备注".into()), (2, "末端备注".into())])
}

fn temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "canglang_jieqi_{label}_{}_{}.cjq",
        std::process::id(),
        nanos
    ))
}

#[test]
fn private_and_public_documents_round_trip_the_verified_main_line() {
    let source = game();
    for kind in [
        JieqiDocumentKind::PrivateGame,
        JieqiDocumentKind::PublicReplay,
    ] {
        let bytes = JieqiDocumentCodec::encode(&source, kind, metadata(), annotations()).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        let (loaded, public) = JieqiDocumentCodec::decode(&bytes).unwrap();

        assert_eq!(loaded.history(), source.history());
        assert_eq!(loaded.head_ply(), 2);
        assert_eq!(loaded.current_ply(), 2);
        assert_eq!(public.kind, kind);
        assert_eq!(public.annotations, annotations());
        match kind {
            JieqiDocumentKind::PrivateGame => assert!(text.contains("assignments")),
            JieqiDocumentKind::PublicReplay => {
                assert!(!text.contains("assignments"));
                assert!(!text.contains("piece_id"));
            }
        }
    }
}

#[test]
fn public_replay_rejects_missing_and_king_reveals() {
    let bytes = JieqiDocumentCodec::encode(
        &game(),
        JieqiDocumentKind::PublicReplay,
        metadata(),
        BTreeMap::new(),
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    value["moves"][0]["revealed"] = serde_json::Value::Null;
    assert!(JieqiDocumentCodec::decode(&serde_json::to_vec(&value).unwrap()).is_err());

    value["moves"][0]["revealed"] = serde_json::json!("king");
    assert!(JieqiDocumentCodec::decode(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn recorded_identity_source_rejects_reveals_above_the_side_multiset() {
    let reveals = [16, 17, 18]
        .into_iter()
        .map(|piece_id| JieqiReveal {
            piece_id,
            kind: PieceKind::Rook,
        })
        .collect();
    assert!(JieqiPosition::standard_recorded(reveals).is_err());
}

#[test]
fn private_replay_rejects_reveal_that_disagrees_with_fixed_identity() {
    let bytes = JieqiDocumentCodec::encode(
        &game(),
        JieqiDocumentKind::PrivateGame,
        metadata(),
        BTreeMap::new(),
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["moves"][0]["revealed"] = serde_json::json!("knight");

    assert!(JieqiDocumentCodec::decode(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn rejects_future_version_duplicate_annotations_and_out_of_range_text() {
    let bytes = JieqiDocumentCodec::encode(
        &game(),
        JieqiDocumentKind::PublicReplay,
        metadata(),
        BTreeMap::new(),
    )
    .unwrap();
    let text = String::from_utf8(bytes).unwrap();

    let future = text.replace("\"version\": 1", "\"version\": 2");
    assert!(JieqiDocumentCodec::decode(future.as_bytes()).is_err());

    let duplicate = text.replace(
        "\"annotations\": {}",
        "\"annotations\": {\"0\":\"甲\",\"0\":\"乙\"}",
    );
    assert!(JieqiDocumentCodec::decode(duplicate.as_bytes()).is_err());

    let mut out_of_range = serde_json::from_str::<serde_json::Value>(&text).unwrap();
    out_of_range["annotations"] = serde_json::json!({"3": "越界"});
    assert!(JieqiDocumentCodec::decode(&serde_json::to_vec(&out_of_range).unwrap()).is_err());
}

#[test]
fn service_atomically_replaces_existing_file_and_leaves_no_sibling_artifacts() {
    let path = temp_path("atomic");
    std::fs::write(&path, b"old content").unwrap();
    JieqiDocumentService::save(
        &game(),
        path.to_str().unwrap(),
        JieqiDocumentKind::PrivateGame,
        metadata(),
        annotations(),
    )
    .unwrap();

    let (loaded, _) = JieqiDocumentService::load(path.to_str().unwrap()).unwrap();
    assert_eq!(loaded.head_ply(), 2);
    let name = path.file_name().unwrap().to_string_lossy();
    let siblings = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            let candidate = entry.file_name();
            let candidate = candidate.to_string_lossy();
            candidate.starts_with(&format!(".{name}.tmp"))
                || candidate.starts_with(&format!(".{name}.bak"))
        })
        .count();
    assert_eq!(siblings, 0);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn size_and_move_count_limits_are_enforced_before_replay() {
    let path = temp_path("oversize");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(16 * 1024 * 1024 + 1).unwrap();
    assert!(JieqiDocumentService::load(path.to_str().unwrap()).is_err());
    std::fs::remove_file(path).unwrap();

    let bytes = JieqiDocumentCodec::encode(
        &game(),
        JieqiDocumentKind::PublicReplay,
        metadata(),
        BTreeMap::new(),
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["moves"] = serde_json::Value::Array(vec![
        serde_json::json!({
            "iccs": "a3a4",
            "revealed": "pawn"
        });
        10_001
    ]);
    assert!(JieqiDocumentCodec::decode(&serde_json::to_vec(&value).unwrap()).is_err());
}
