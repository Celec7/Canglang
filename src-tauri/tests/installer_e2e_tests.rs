//! 用官方发布包端到端验证内置引擎装配流水线
//!
//! 依赖环境变量：
//! - `PIKAFISH_ARCHIVE`：官方 `Pikafish.<date>.7z` 压缩包绝对路径（本地装配测试必需）
//!
//! 完整在线流水线测试会直接从档案登记的官方地址下载约 52MB
//! 在线测试还需要显式设置 `PIKAFISH_ONLINE=1`，避免普通 `--ignored` 批量运行时
//! 意外触发网络下载
//!
//! 默认 `#[ignore]`，需显式运行：
//!   `PIKAFISH_ARCHIVE=/path/to/Pikafish.2026-09-06.7z \
//!    cargo test --test installer_e2e_tests -- --ignored`

use canglang_app::engine::models::EngineConfig;
use canglang_app::engine::{EngineInstaller, EngineProfile, EngineSession};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// 装配结果必须真的能握手，与内置引擎安装后的探针路径一致
async fn assert_handshake(binary: &Path, nnue: &Path) {
    let config = EngineConfig::new(binary.to_string_lossy().into_owned(), "uci")
        .with_nnue(nnue.to_string_lossy().into_owned());
    let mut session = EngineSession::new();
    let info = tokio::time::timeout(Duration::from_secs(60), session.start(&config))
        .await
        .expect("引擎握手超时")
        .expect("引擎握手失败");
    session.stop().await;

    assert!(info.ready, "引擎未报告 ready: {info:?}");
    assert_eq!(info.protocol, "uci");
}

#[tokio::test]
#[ignore]
async fn extract_and_locate_official_pikafish_archive() {
    let archive = match std::env::var("PIKAFISH_ARCHIVE") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            eprintln!("skip: PIKAFISH_ARCHIVE not set");
            return;
        }
    };
    if !archive.is_file() {
        eprintln!("skip: archive not found at {archive:?}");
        return;
    }

    let dir = std::env::temp_dir().join(format!("canglang_installer_e2e_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    EngineInstaller::extract_archive(&archive, &dir).expect("解压官方压缩包失败");

    let (binary, nnue) = EngineInstaller::locate_runtime(&dir).expect("定位可执行文件失败");
    assert!(binary.is_file());

    let file_name = binary.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        file_name.contains("Pikafish"),
        "意外的可执行文件: {file_name}"
    );
    assert!(
        !file_name.contains("Android"),
        "不得选中 Android 构建: {file_name}"
    );

    let nnue = nnue.expect("官方压缩包应包含 pikafish.nnue");
    assert_eq!(nnue.file_name().unwrap(), "pikafish.nnue");

    assert_handshake(&binary, &nnue).await;

    let _ = std::fs::remove_dir_all(&dir);
}

/// 完整在线流水线：下载官方压缩包、解压装配、上报阶段，再握手验证
#[tokio::test]
#[ignore]
async fn install_builtin_downloads_and_assembles() {
    if std::env::var("PIKAFISH_ONLINE").ok().as_deref() != Some("1") {
        eprintln!("skip: set PIKAFISH_ONLINE=1 to enable the online download test");
        return;
    }
    let profile = EngineProfile::builtin_pikafish();
    let dir =
        std::env::temp_dir().join(format!("canglang_installer_online_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let stages = Mutex::new(Vec::new());
    let installed = EngineInstaller::install_with_progress(&profile, &dir, |payload| {
        assert_eq!(payload.profile_id, "builtin-pikafish");
        assert!(payload.percent >= 0.0 && payload.percent <= 100.0);
        stages.lock().unwrap().push(payload.stage);
    })
    .await
    .expect("在线安装内置引擎失败");

    let stages = stages.into_inner().unwrap();
    assert!(
        stages.iter().any(|stage| stage == "downloading"),
        "未上报下载阶段: {stages:?}"
    );
    assert!(
        stages.iter().any(|stage| stage == "extracting"),
        "未上报解压阶段: {stages:?}"
    );
    assert_eq!(stages.last().map(String::as_str), Some("ready"));

    let binary = PathBuf::from(&installed.path);
    assert!(binary.is_file(), "装配后的可执行文件不存在: {binary:?}");
    assert!(
        !dir.join(format!("{}-download.7z", profile.install_key()))
            .exists(),
        "装配完成后不应残留临时压缩包"
    );

    let nnue = PathBuf::from(installed.nnue_path.expect("装配后应绑定 NNUE 权重"));
    assert!(nnue.is_file());

    assert_handshake(&binary, &nnue).await;

    let _ = std::fs::remove_dir_all(&dir);
}
