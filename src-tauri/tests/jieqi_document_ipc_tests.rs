use canglang_app::core::game::{GameState, SessionErrorCode, SessionToken};
use canglang_app::core::jieqi::{
    JieqiGame, JieqiIdentity, JieqiPlayMode, JieqiPosition, standard_identity_kinds,
};
use canglang_app::ipc::engine::EngineState;
use canglang_app::ipc::manual::{jieqi_document_open_with_states, jieqi_document_save_with_state};
use canglang_app::manual::jieqi::{JieqiDocumentKind, JieqiDocumentMetadata};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

fn state() -> Mutex<GameState> {
    let identities = standard_identity_kinds()
        .into_iter()
        .enumerate()
        .map(|(piece_id, kind)| JieqiIdentity {
            piece_id: piece_id as u8,
            kind,
        })
        .collect();
    Mutex::new(GameState::new_jieqi(JieqiGame::new(
        JieqiPosition::standard_assigned(identities).unwrap(),
        JieqiPlayMode::Training,
    )))
}

fn path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "canglang_jieqi_ipc_{label}_{}_{}.cjq",
        std::process::id(),
        nanos
    ))
}

#[tokio::test]
async fn save_receipt_tracks_the_exported_revisions_without_returning_identities() {
    let state = state();
    let token = state.lock().unwrap().token();
    let path = path("private");
    let receipt = jieqi_document_save_with_state(
        token.clone(),
        path.to_string_lossy().into_owned(),
        JieqiDocumentKind::PrivateGame,
        JieqiDocumentMetadata::default(),
        BTreeMap::new(),
        "edit-7".into(),
        &state,
    )
    .await
    .unwrap();

    assert_eq!(receipt.game_id, token.game_id);
    assert_eq!(receipt.content_revision, "0");
    assert_eq!(receipt.edit_revision, "edit-7");
    let receipt_json = serde_json::to_string(&receipt).unwrap();
    assert!(!receipt_json.contains("assignments"));
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("assignments")
    );
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn public_open_replaces_session_and_returns_no_assigned_identity() {
    let state = state();
    let save_token = state.lock().unwrap().token();
    let path = path("public");
    jieqi_document_save_with_state(
        save_token.clone(),
        path.to_string_lossy().into_owned(),
        JieqiDocumentKind::PublicReplay,
        JieqiDocumentMetadata::default(),
        BTreeMap::new(),
        "0".into(),
        &state,
    )
    .await
    .unwrap();

    let result = jieqi_document_open_with_states(
        save_token.clone(),
        path.to_string_lossy().into_owned(),
        &state,
        &EngineState::default(),
    )
    .await
    .unwrap();
    assert_ne!(result.snapshot.game_id, save_token.game_id);
    assert_eq!(result.document.kind, JieqiDocumentKind::PublicReplay);
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("Assigned"));
    assert!(!json.contains("assigned"));
    assert!(!json.contains("piece_id"));
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn invalid_or_stale_open_never_replaces_the_current_session() {
    let state = state();
    let before = state.lock().unwrap().snapshot();
    let invalid_path = path("invalid");
    std::fs::write(&invalid_path, b"not json").unwrap();
    let invalid_token = state.lock().unwrap().token();
    let error = jieqi_document_open_with_states(
        invalid_token,
        invalid_path.to_string_lossy().into_owned(),
        &state,
        &EngineState::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, SessionErrorCode::InvalidInput);
    assert_eq!(state.lock().unwrap().snapshot(), before);
    std::fs::remove_file(invalid_path).unwrap();

    let valid_path = path("stale");
    let live_token = state.lock().unwrap().token();
    jieqi_document_save_with_state(
        live_token.clone(),
        valid_path.to_string_lossy().into_owned(),
        JieqiDocumentKind::PublicReplay,
        JieqiDocumentMetadata::default(),
        BTreeMap::new(),
        "0".into(),
        &state,
    )
    .await
    .unwrap();
    let stale = canglang_app::core::game::SessionToken {
        expected_revision: "999".into(),
        ..live_token
    };
    let error = jieqi_document_open_with_states(
        stale,
        valid_path.to_string_lossy().into_owned(),
        &state,
        &EngineState::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, SessionErrorCode::StaleSession);
    assert_eq!(state.lock().unwrap().snapshot(), before);
    std::fs::remove_file(valid_path).unwrap();
}

#[tokio::test]
async fn public_replay_cannot_be_saved_as_a_private_continuation() {
    let local = state();
    let token = local.lock().unwrap().token();
    let public_path = path("readonly");
    jieqi_document_save_with_state(
        token.clone(),
        public_path.to_string_lossy().into_owned(),
        JieqiDocumentKind::PublicReplay,
        JieqiDocumentMetadata::default(),
        BTreeMap::new(),
        "0".into(),
        &local,
    )
    .await
    .unwrap();
    let result = jieqi_document_open_with_states(
        token,
        public_path.to_string_lossy().into_owned(),
        &local,
        &EngineState::default(),
    )
    .await
    .unwrap();
    let public_token = SessionToken {
        game_id: result.snapshot.game_id.clone(),
        expected_revision: result.snapshot.revision.clone(),
    };
    let error = jieqi_document_save_with_state(
        public_token,
        path("forbidden").to_string_lossy().into_owned(),
        JieqiDocumentKind::PrivateGame,
        JieqiDocumentMetadata::default(),
        BTreeMap::new(),
        "0".into(),
        &local,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, SessionErrorCode::OperationUnavailable);
    std::fs::remove_file(public_path).unwrap();
}
