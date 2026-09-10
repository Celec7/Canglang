//! 内置引擎的在线获取与就地装配
//!
//! 内置档案（见 [`crate::engine::config::default_builtin_profiles`]）在未下载时
//! 只有注册信息，没有本地路径。用户点击“一键在线获取”后，本模块负责流式下载
//! 官方压缩包、解压到引擎安装目录、按当前操作系统挑选可执行文件、配置执行权限
//! 并绑定同目录的 `.nnue` 权重，最终返回一个已就绪的 [`EngineProfile`]
//!
//! 安装目录由调用方（IPC 层）依据 [`ConfigService`](crate::engine::config::ConfigService)
//! 的便携/标准模式解析后传入，因此下载期间无需持有配置锁

use crate::engine::config::EngineProfile;
use crate::engine::error::EngineError;
use serde::{Deserialize, Serialize};
use specta_typescript::Number;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::Emitter;

/// 引擎在线安装进度事件名
pub const DOWNLOAD_PROGRESS_EVENT: &str = "engine://download-progress";

/// 官方发布包中 Linux 平台的通用可执行文件名
const LINUX_BINARY: &str = "Pikafish-Linux-x86-64-universal";
/// 官方发布包中 macOS 平台的通用可执行文件名
const MACOS_BINARY: &str = "Pikafish-MacOS-universal";
/// 官方发布包中 Windows 平台的通用可执行文件名
const WINDOWS_BINARY: &str = "Pikafish-Windows-x86-64-universal.exe";
/// 官方随包分发的 NNUE 权重文件名
const NNUE_FILE: &str = "pikafish.nnue";
/// 递归搜索可执行文件与权重的最大目录深度
const MAX_SCAN_DEPTH: usize = 4;

/// 引擎在线安装进度载荷，经 `engine://download-progress` 事件推送
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressPayload {
    pub profile_id: String,
    /// 阶段：`downloading` | `extracting` | `ready` | `error`
    pub stage: String,
    #[specta(type = Number)]
    pub downloaded_bytes: u64,
    #[specta(type = Option<Number>)]
    pub total_bytes: Option<u64>,
    pub percent: f64,
    #[specta(type = Number)]
    pub speed_bytes_per_sec: u64,
    pub message: Option<String>,
}

pub struct EngineInstaller;

/// 已完成下载、解压和装配但尚未替换正式目录的安装结果
pub struct PreparedInstallation {
    pub profile: EngineProfile,
    staging_dir: PathBuf,
    target_dir: PathBuf,
}

/// 已替换正式目录但尚未完成配置提交的安装结果
///
/// 保留旧目录直到调用方完成配置持久化；配置写入失败时可回滚，避免磁盘上的
/// 新引擎与 `config.json` 中的 Profile 脱节
pub struct CommittedInstallation {
    pub profile: EngineProfile,
    target_dir: PathBuf,
    backup_dir: PathBuf,
    had_backup: bool,
}

impl PreparedInstallation {
    pub fn cleanup(self) {
        let _ = std::fs::remove_dir_all(self.staging_dir);
    }
}

impl EngineInstaller {
    /// 执行内置引擎的下载、解压与装配流水线，并把进度经
    /// `engine://download-progress` 推送给前端
    ///
    /// `install_dir` 为引擎专属目录（如 `engines/pikafish`），不存在时自动创建
    pub async fn install_builtin(
        profile: &EngineProfile,
        install_dir: &Path,
        app_handle: &tauri::AppHandle,
    ) -> Result<EngineProfile, EngineError> {
        Self::install_with_progress(profile, install_dir, |payload| {
            let _ = app_handle.emit(DOWNLOAD_PROGRESS_EVENT, payload);
        })
        .await
    }

    /// 与 [`Self::install_builtin`] 相同的流水线，进度经回调上报
    ///
    /// 回调解耦让流水线可以脱离 Tauri 运行，便于用官方发布包做端到端验证
    pub async fn install_with_progress<F>(
        profile: &EngineProfile,
        install_dir: &Path,
        on_progress: F,
    ) -> Result<EngineProfile, EngineError>
    where
        F: Fn(DownloadProgressPayload) + Send + Sync,
    {
        let prepared = Self::prepare_with_progress(profile, install_dir, on_progress).await?;
        let committed = Self::commit_prepared(prepared)?;
        committed.finalize()
    }

    /// 下载并装配到同级 staging 目录，供调用方探活成功后再提交
    pub async fn prepare_with_progress<F>(
        profile: &EngineProfile,
        install_dir: &Path,
        on_progress: F,
    ) -> Result<PreparedInstallation, EngineError>
    where
        F: Fn(DownloadProgressPayload) + Send + Sync,
    {
        let download_url = resolve_download_url(profile)?.to_string();
        let parent = install_dir.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let staging_dir = parent.join(format!(
            ".{}.staging-{}-{}",
            profile.install_key(),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&staging_dir)?;
        let archive_path = staging_dir.join(format!("{}-download.7z", profile.install_key()));
        let result = Self::download_and_assemble(
            profile,
            &download_url,
            &archive_path,
            &staging_dir,
            &on_progress,
        )
        .await;
        match result {
            Ok(installed) => Ok(PreparedInstallation {
                profile: installed,
                staging_dir,
                target_dir: install_dir.to_path_buf(),
            }),
            Err(error) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                Err(error)
            }
        }
    }

    /// 原子替换正式目录，返回一个可在配置提交失败时回滚的事务结果
    pub fn commit_prepared(
        mut prepared: PreparedInstallation,
    ) -> Result<CommittedInstallation, EngineError> {
        let backup_dir = prepared
            .target_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!(
                ".{}-backup",
                prepared
                    .target_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ));
        let had_backup = prepared.target_dir.exists();
        let _ = std::fs::remove_dir_all(&backup_dir);
        if had_backup {
            std::fs::rename(&prepared.target_dir, &backup_dir)?;
        }
        if let Err(error) = std::fs::rename(&prepared.staging_dir, &prepared.target_dir) {
            if backup_dir.exists() {
                let _ = std::fs::rename(&backup_dir, &prepared.target_dir);
            }
            return Err(error.into());
        }
        let old_staging = prepared.staging_dir.clone();
        if let Ok(relative) =
            std::path::Path::new(&prepared.profile.path).strip_prefix(&old_staging)
        {
            prepared.profile.path = prepared
                .target_dir
                .join(relative)
                .to_string_lossy()
                .into_owned();
        }
        if let Some(nnue) = prepared.profile.nnue_path.as_mut()
            && let Ok(relative) = std::path::Path::new(nnue).strip_prefix(&old_staging)
        {
            *nnue = prepared
                .target_dir
                .join(relative)
                .to_string_lossy()
                .into_owned();
        }
        Ok(CommittedInstallation {
            profile: prepared.profile,
            target_dir: prepared.target_dir,
            backup_dir,
            had_backup,
        })
    }

    async fn download_and_assemble<F>(
        profile: &EngineProfile,
        download_url: &str,
        archive_path: &Path,
        install_dir: &Path,
        on_progress: &F,
    ) -> Result<EngineProfile, EngineError>
    where
        F: Fn(DownloadProgressPayload) + Send + Sync,
    {
        let (downloaded_bytes, total_bytes) =
            Self::download_archive(profile, download_url, archive_path, on_progress).await?;

        on_progress(progress_payload(
            &profile.id,
            "extracting",
            downloaded_bytes,
            total_bytes,
            100.0,
            0,
            "下载完成，正在解压并配置引擎权限...",
        ));

        Self::extract_archive(archive_path, install_dir)?;

        // 压缩包仅用于装配，解压完成后立即清理
        let _ = std::fs::remove_file(archive_path);

        let (binary_path, nnue_path) = Self::locate_runtime(install_dir)?;

        let mut installed = profile.clone();
        installed.path = binary_path.to_string_lossy().into_owned();
        installed.nnue_path = nnue_path.map(|p| p.to_string_lossy().into_owned());
        installed.installed_revision = installed.release_revision.clone();

        on_progress(progress_payload(
            &profile.id,
            "ready",
            downloaded_bytes,
            total_bytes,
            100.0,
            0,
            "内置引擎已下载并装配完成。",
        ));

        Ok(installed)
    }

    /// 把官方 `.7z` 压缩包解压到引擎安装目录
    pub fn extract_archive(archive_path: &Path, install_dir: &Path) -> Result<(), EngineError> {
        sevenz_rust::decompress_file(archive_path, install_dir)
            .map_err(|e| EngineError::InvalidConfig(format!("解压 .7z 压缩包失败: {e}")))
    }

    /// 在已解压的安装目录中定位当前平台的可执行文件与 `.nnue` 权重，
    /// 并为可执行文件补上类 Unix 平台的执行权限
    pub fn locate_runtime(install_dir: &Path) -> Result<(PathBuf, Option<PathBuf>), EngineError> {
        let binary_path = find_executable(install_dir)?;
        grant_execute_permission(&binary_path);
        Ok((binary_path, find_nnue(install_dir)))
    }

    /// 流式下载压缩包并按 150ms 节流上报进度，返回已下载字节数与总字节数
    async fn download_archive<F>(
        profile: &EngineProfile,
        download_url: &str,
        archive_path: &Path,
        on_progress: &F,
    ) -> Result<(u64, Option<u64>), EngineError>
    where
        F: Fn(DownloadProgressPayload) + Send + Sync,
    {
        let client = reqwest::Client::builder()
            .user_agent(concat!("Canglang/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(600))
            .build()
            .unwrap_or_default();

        let mut response = client
            .get(download_url)
            .send()
            .await
            .map_err(|e| EngineError::InvalidConfig(format!("下载网络请求失败: {e}")))?;

        if !response.status().is_success() {
            return Err(EngineError::InvalidConfig(format!(
                "下载服务器返回错误状态码: {}",
                response.status()
            )));
        }

        let total_bytes = response.content_length();
        let mut downloaded_bytes: u64 = 0;
        let mut last_emit = Instant::now();
        let mut last_bytes: u64 = 0;
        let mut speed: u64 = 0;

        let mut file = File::create(archive_path)?;

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| EngineError::InvalidConfig(format!("下载数据传输中断: {e}")))?
        {
            file.write_all(&chunk)?;
            downloaded_bytes += chunk.len() as u64;

            if last_emit.elapsed() >= Duration::from_millis(150) {
                let interval_secs = last_emit.elapsed().as_secs_f64();
                if interval_secs > 0.0 {
                    speed = ((downloaded_bytes - last_bytes) as f64 / interval_secs) as u64;
                }
                last_bytes = downloaded_bytes;
                last_emit = Instant::now();

                on_progress(progress_payload(
                    &profile.id,
                    "downloading",
                    downloaded_bytes,
                    total_bytes,
                    percent_of(downloaded_bytes, total_bytes),
                    speed,
                    "正在下载引擎压缩包...",
                ));
            }
        }

        file.flush()?;
        drop(file);

        // 结束时补发一次终值，避免最后一帧进度停留在 99%
        on_progress(progress_payload(
            &profile.id,
            "downloading",
            downloaded_bytes,
            total_bytes,
            percent_of(downloaded_bytes, total_bytes),
            0,
            "正在下载引擎压缩包...",
        ));

        if let Some(total) = total_bytes
            && total > 0
            && downloaded_bytes != total
        {
            return Err(EngineError::InvalidConfig(format!(
                "下载不完整：期望 {total} 字节，实际 {downloaded_bytes} 字节"
            )));
        }

        Ok((downloaded_bytes, total_bytes))
    }
}

impl CommittedInstallation {
    /// 配置持久化成功后删除旧版本目录并完成提交
    pub fn finalize(self) -> Result<EngineProfile, EngineError> {
        if self.had_backup {
            std::fs::remove_dir_all(&self.backup_dir)?;
        }
        Ok(self.profile)
    }

    /// 配置持久化失败时删除新目录并恢复旧版本
    pub fn rollback(self) -> Result<(), EngineError> {
        if self.target_dir.exists() {
            std::fs::remove_dir_all(&self.target_dir)?;
        }
        if self.had_backup && self.backup_dir.exists() {
            std::fs::rename(&self.backup_dir, &self.target_dir)?;
        }
        Ok(())
    }
}

/// 校验内置档案并返回其下载地址
fn resolve_download_url(profile: &EngineProfile) -> Result<&str, EngineError> {
    if !profile.is_builtin {
        return Err(EngineError::InvalidConfig(format!(
            "引擎档案 {} 不是内置在线引擎",
            profile.id
        )));
    }

    profile
        .download_url
        .as_deref()
        .filter(|url| !url.trim().is_empty())
        .ok_or_else(|| EngineError::InvalidConfig(format!("内置引擎 {} 缺少下载地址", profile.id)))
}

fn percent_of(downloaded: u64, total: Option<u64>) -> f64 {
    match total {
        Some(total) if total > 0 => ((downloaded as f64 / total as f64) * 100.0).min(100.0),
        _ => 0.0,
    }
}

/// 构造一帧安装进度载荷
fn progress_payload(
    profile_id: &str,
    stage: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    percent: f64,
    speed_bytes_per_sec: u64,
    message: &str,
) -> DownloadProgressPayload {
    DownloadProgressPayload {
        profile_id: profile_id.to_string(),
        stage: stage.to_string(),
        downloaded_bytes,
        total_bytes,
        percent,
        speed_bytes_per_sec,
        message: Some(message.to_string()),
    }
}

/// 当前操作系统在官方发布包中对应的可执行文件名
fn platform_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        WINDOWS_BINARY
    } else if cfg!(target_os = "macos") {
        MACOS_BINARY
    } else {
        LINUX_BINARY
    }
}

/// 判断某个文件名是否是当前平台可能的可执行引擎文件
fn is_platform_candidate(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();

    // 排除移动端平台打包产物
    if lower.contains("android") || lower.contains("ios") {
        return false;
    }

    if cfg!(target_os = "windows") {
        return lower.ends_with(".exe");
    }

    // Unix 平台的引擎二进制没有扩展名；该规则同时排除 .nnue/.md/.txt/.json 等附属文件
    if Path::new(file_name).extension().is_some() {
        return false;
    }

    if cfg!(target_os = "macos") {
        return !lower.contains("linux") && !lower.contains("windows");
    }

    !lower.contains("macos") && !lower.contains("darwin") && !lower.contains("windows")
}

/// 在安装目录中定位当前平台的可执行文件
///
/// 优先精确匹配官方发布包中的文件名；否则递归搜索符合平台特征的候选，
/// 并按名称排序取第一个，保证结果与目录遍历顺序无关
fn find_executable(install_dir: &Path) -> Result<PathBuf, EngineError> {
    let expected = platform_binary_name();
    let direct = install_dir.join(expected);
    if direct.is_file() {
        return Ok(direct);
    }

    let mut candidates = Vec::new();
    collect_files(install_dir, 0, &mut candidates);
    candidates.sort();

    candidates
        .into_iter()
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(is_platform_candidate)
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            EngineError::InvalidConfig(format!(
                "解压后未找到适配当前操作系统的引擎可执行文件（期望 {expected}）"
            ))
        })
}

/// 在安装目录中定位 `.nnue` 权重文件，优先官方 `pikafish.nnue`
fn find_nnue(install_dir: &Path) -> Option<PathBuf> {
    let official = install_dir.join(NNUE_FILE);
    if official.is_file() {
        return Some(official);
    }

    let mut candidates = Vec::new();
    collect_files(install_dir, 0, &mut candidates);
    candidates.sort();

    candidates.into_iter().find(|path| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("nnue"))
            .unwrap_or(false)
    })
}

/// 递归收集目录下的文件，深度受 [`MAX_SCAN_DEPTH`] 限制
fn collect_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_SCAN_DEPTH {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, depth + 1, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// 在类 Unix 平台为引擎补上可执行权限
fn grant_execute_permission(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = std::fs::set_permissions(path, permissions);
        }
    }

    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "canglang_installer_test_{}_{}_{n}",
                std::process::id(),
                tag
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn touch(&self, relative: &str) -> PathBuf {
            let path = self.path.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, b"stub").unwrap();
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_find_executable_prefers_exact_platform_binary() {
        let dir = TempDir::new("exact");
        dir.touch(LINUX_BINARY);
        dir.touch(MACOS_BINARY);
        dir.touch(WINDOWS_BINARY);
        dir.touch("Pikafish-Android-arm64-universal");
        dir.touch(NNUE_FILE);

        let found = find_executable(&dir.path).unwrap();
        assert_eq!(
            found.file_name().unwrap().to_str().unwrap(),
            platform_binary_name()
        );
    }

    #[test]
    fn test_find_executable_falls_back_to_platform_candidate() {
        let dir = TempDir::new("fallback");
        // 官方发布包结构变化时的兜底：名称含 pikafish 且属于当前平台
        let candidate = if cfg!(target_os = "windows") {
            "dist/Pikafish-Windows-x86-64-avx2.exe"
        } else if cfg!(target_os = "macos") {
            "dist/Pikafish-MacOS-arm64"
        } else {
            "dist/Pikafish-Linux-x86-64-avx2"
        };
        let expected = dir.touch(candidate);

        assert_eq!(find_executable(&dir.path).unwrap(), expected);
    }

    #[test]
    fn test_find_executable_ignores_weights_docs_and_other_platforms() {
        let dir = TempDir::new("ignore");
        dir.touch(NNUE_FILE);
        dir.touch("Copying.txt");
        dir.touch("Wiki/Home.md");
        dir.touch("Pikafish-Android-arm64-universal");

        let error = find_executable(&dir.path).unwrap_err();
        assert!(matches!(error, EngineError::InvalidConfig(_)));
    }

    #[test]
    fn test_find_nnue_prefers_official_weight() {
        let dir = TempDir::new("nnue_official");
        let nested = dir.touch("dist/other.nnue");
        let official = dir.touch(NNUE_FILE);

        assert_eq!(find_nnue(&dir.path).unwrap(), official);
        assert_ne!(find_nnue(&dir.path).unwrap(), nested);
    }

    #[test]
    fn test_find_nnue_falls_back_to_nested_weight() {
        let dir = TempDir::new("nnue_nested");
        let nested = dir.touch("dist/custom.nnue");
        dir.touch("README.md");

        assert_eq!(find_nnue(&dir.path).unwrap(), nested);
    }

    #[test]
    fn test_find_nnue_returns_none_without_weight() {
        let dir = TempDir::new("nnue_missing");
        dir.touch("Pikafish-Linux-x86-64-universal");

        assert!(find_nnue(&dir.path).is_none());
    }

    #[test]
    fn test_locate_runtime_pairs_binary_with_weight() {
        let dir = TempDir::new("locate_runtime");
        let binary = dir.touch(platform_binary_name());
        dir.touch(NNUE_FILE);
        dir.touch("Copying.txt");
        dir.touch("Pikafish-Android-arm64-universal");

        let (found_binary, found_nnue) = EngineInstaller::locate_runtime(&dir.path).unwrap();

        assert_eq!(found_binary, binary);
        assert_eq!(found_nnue.unwrap(), dir.path.join(NNUE_FILE));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&found_binary)
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "装配后应具备执行权限");
        }
    }

    #[test]
    fn test_resolve_download_url_rejects_non_builtin_profile() {
        let mut profile = EngineProfile::builtin_pikafish();
        profile.is_builtin = false;

        assert!(resolve_download_url(&profile).is_err());
    }

    #[test]
    fn test_resolve_download_url_rejects_missing_url() {
        let mut profile = EngineProfile::builtin_pikafish();
        profile.download_url = None;
        assert!(resolve_download_url(&profile).is_err());

        profile.download_url = Some("   ".to_string());
        assert!(resolve_download_url(&profile).is_err());
    }

    #[test]
    fn test_resolve_download_url_returns_registered_url() {
        let profile = EngineProfile::builtin_pikafish();
        let url = resolve_download_url(&profile).unwrap();

        assert!(
            url.starts_with("https://github.com/official-pikafish/Pikafish/releases/download/")
        );
        assert!(url.ends_with(".7z"));
    }

    #[test]
    fn commit_can_rollback_after_configuration_failure() {
        let root = TempDir::new("rollback");
        let target = root.path.join("pikafish");
        let staging = root.path.join(".pikafish.staging");
        let old_binary = target.join("old-engine");
        let new_binary = staging.join("new-engine");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(&old_binary, b"old").unwrap();
        std::fs::write(&new_binary, b"new").unwrap();

        let mut profile = EngineProfile::builtin_pikafish();
        profile.path = new_binary.to_string_lossy().into_owned();
        let prepared = PreparedInstallation {
            profile,
            staging_dir: staging,
            target_dir: target.clone(),
        };

        let committed = EngineInstaller::commit_prepared(prepared).unwrap();
        assert_eq!(std::fs::read(target.join("new-engine")).unwrap(), b"new");
        committed.rollback().unwrap();
        assert_eq!(std::fs::read(old_binary).unwrap(), b"old");
        assert!(!target.join("new-engine").exists());
    }

    #[test]
    fn commit_finalize_removes_backup_only_after_success() {
        let root = TempDir::new("finalize");
        let target = root.path.join("pikafish");
        let staging = root.path.join(".pikafish.staging");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(target.join("old-engine"), b"old").unwrap();
        let new_binary = staging.join("new-engine");
        std::fs::write(&new_binary, b"new").unwrap();

        let mut profile = EngineProfile::builtin_pikafish();
        profile.path = new_binary.to_string_lossy().into_owned();
        let committed = EngineInstaller::commit_prepared(PreparedInstallation {
            profile,
            staging_dir: staging,
            target_dir: target.clone(),
        })
        .unwrap();
        let backup = root.path.join(".pikafish-backup");
        assert!(backup.exists());
        committed.finalize().unwrap();
        assert!(!backup.exists());
        assert_eq!(std::fs::read(target.join("new-engine")).unwrap(), b"new");
    }

    #[test]
    fn test_percent_of_handles_unknown_total() {
        assert_eq!(percent_of(10, Some(20)), 50.0);
        assert_eq!(percent_of(30, Some(20)), 100.0);
        assert_eq!(percent_of(10, None), 0.0);
        assert_eq!(percent_of(10, Some(0)), 0.0);
    }
}
