use super::{JieqiDocumentCodec, JieqiDocumentKind, JieqiDocumentMetadata, JieqiDocumentPublic};
use crate::core::jieqi::JieqiGame;
use crate::manual::ManualError;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

pub struct JieqiDocumentService;

impl JieqiDocumentService {
    pub fn load(path: &str) -> Result<(JieqiGame, JieqiDocumentPublic), ManualError> {
        validate_path(path)?;
        let target = Path::new(path);
        let metadata = std::fs::metadata(target).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                ManualError::NotExist { path: path.into() }
            } else {
                ManualError::Io { source }
            }
        })?;
        if metadata.len() > super::codec::MAX_DOCUMENT_BYTES as u64 {
            return Err(ManualError::InvalidData {
                msg: "揭棋文件超过 16 MiB 限制".into(),
            });
        }
        let bytes = std::fs::read(target).map_err(|source| ManualError::Io { source })?;
        JieqiDocumentCodec::decode(&bytes)
    }

    pub fn save(
        game: &JieqiGame,
        path: &str,
        kind: JieqiDocumentKind,
        metadata: JieqiDocumentMetadata,
        annotations: BTreeMap<u32, String>,
    ) -> Result<(), ManualError> {
        validate_path(path)?;
        let bytes = JieqiDocumentCodec::encode(game, kind, metadata, annotations)?;
        if bytes.len() > super::codec::MAX_DOCUMENT_BYTES {
            return Err(ManualError::InvalidData {
                msg: "揭棋文件超过 16 MiB 限制".into(),
            });
        }
        atomic_write(Path::new(path), &bytes)
    }
}

fn validate_path(path: &str) -> Result<(), ManualError> {
    if path.trim().is_empty() {
        return Err(ManualError::InvalidData {
            msg: "揭棋文件路径为空".into(),
        });
    }
    let ext = Path::new(path)
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if ext != "cjq" {
        return Err(ManualError::UnsupportedFormat { ext });
    }
    Ok(())
}

fn sibling_path(target: &Path, suffix: &str) -> PathBuf {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    target.with_file_name(format!(".{name}.{suffix}.{}.{}", std::process::id(), id))
}

fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), ManualError> {
    if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|source| ManualError::Io { source })?;
    }
    let temp = sibling_path(target, "tmp");
    let backup = sibling_path(target, "bak");
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|source| ManualError::Io { source })?;
        file.write_all(bytes)
            .map_err(|source| ManualError::Io { source })?;
        file.sync_all()
            .map_err(|source| ManualError::Io { source })?;
        drop(file);

        let had_target = target.exists();
        if had_target {
            std::fs::rename(target, &backup).map_err(|source| ManualError::Io { source })?;
        }
        if let Err(source) = std::fs::rename(&temp, target) {
            if had_target {
                let _ = std::fs::rename(&backup, target);
            }
            return Err(ManualError::Io { source });
        }
        if had_target {
            std::fs::remove_file(&backup).map_err(|source| ManualError::Io { source })?;
        }
        if let Some(parent) = target.parent()
            && let Ok(directory) = File::open(parent)
        {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
