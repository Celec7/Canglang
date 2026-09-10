use crate::engine::book::CloudBookMode;
use crate::engine::error::EngineError;
use crate::engine::models::{AnalysisMode, EngineOptionValue};
use serde::{Deserialize, Serialize};
use specta_typescript::Number;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 外部引擎配置档案项
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EngineProfile {
    pub id: String,
    pub name: String,
    pub path: String,
    pub protocol: String,
    pub threads: Option<u32>,
    pub hash_mb: Option<u32>,
    pub nnue_path: Option<String>,
    #[serde(default)]
    pub option_overrides: HashMap<String, EngineOptionValue>,
    /// 内置发行注册表中的可比较修订；自定义引擎为空字符串
    #[serde(default)]
    pub release_revision: String,
    /// 最近一次成功安装并探活的修订；旧配置缺失时为空字符串
    #[serde(default)]
    pub installed_revision: String,
    pub is_builtin: bool,
    pub download_url: Option<String>,
    pub description: Option<String>,
}

impl EngineProfile {
    pub fn builtin_pikafish() -> Self {
        Self {
            id: "builtin-pikafish".to_string(),
            name: "Pikafish".to_string(),
            path: "".to_string(),
            protocol: "uci".to_string(),
            threads: None,
            hash_mb: None,
            nnue_path: None,
            option_overrides: HashMap::new(),
            release_revision: "2026-09-06".to_string(),
            installed_revision: String::new(),
            is_builtin: true,
            download_url: Some(
                "https://github.com/official-pikafish/Pikafish/releases/download/Pikafish-2026-09-06/Pikafish.2026-09-06.7z"
                    .to_string(),
            ),
            description: Some(
                "当前世界顶尖的中国象棋开源 NNUE 深度神经网络引擎，支持残局与高深度多路分析。".to_string(),
            ),
        }
    }

    /// 在预制注册中心中按 id 查找内置引擎定义
    pub fn find_builtin(id: &str) -> Option<Self> {
        default_builtin_profiles().into_iter().find(|p| p.id == id)
    }

    /// 该档案在引擎安装根目录下使用的子目录名，例如 `builtin-pikafish` → `pikafish`
    pub fn install_key(&self) -> &str {
        self.id
            .rsplit('-')
            .next()
            .filter(|segment| !segment.is_empty())
            .unwrap_or(self.id.as_str())
    }
}

/// 预制（内置）引擎注册中心
///
/// 新增内置在线引擎时只需在此追加定义：`AppConfig::default` 与引擎安装 IPC
/// 都从注册中心取初始档案，避免在多个位置重复硬编码同一份定义
pub fn default_builtin_profiles() -> Vec<EngineProfile> {
    vec![EngineProfile::builtin_pikafish()]
}

/// 沧浪象棋全局持久化配置模型
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub theme: String,
    pub board_orientation: String,
    pub show_coordinates: bool,
    pub show_engine_arrow: bool,
    pub animations: bool,
    pub move_animation_seconds: f64,
    pub sound_enabled: bool,
    pub default_rule_profile: String,
    pub opening_book_paths: Vec<String>,
    pub cloud_book_enabled: bool,
    pub cloud_book_mode: CloudBookMode,
    pub engine_profiles: Vec<EngineProfile>,
    pub active_engine_id: Option<String>,
    pub analysis_mode: AnalysisMode,
    #[specta(type = Number)]
    pub analysis_limit_value: u64,
    pub multi_pv: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        let engine_profiles = default_builtin_profiles();
        let active_engine_id = engine_profiles.first().map(|p| p.id.clone());
        Self {
            theme: "light".to_string(),
            board_orientation: "red".to_string(),
            show_coordinates: true,
            show_engine_arrow: true,
            animations: true,
            move_animation_seconds: 0.2,
            sound_enabled: true,
            default_rule_profile: "china2020".to_string(),
            opening_book_paths: Vec::new(),
            cloud_book_enabled: true,
            cloud_book_mode: CloudBookMode::Hybrid,
            engine_profiles,
            active_engine_id,
            analysis_mode: AnalysisMode::FixedTime,
            analysis_limit_value: 1000,
            multi_pv: 1,
        }
    }
}

/// 配置文件位置与便携模式状态
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConfigLocationInfo {
    pub file_path: String,
    pub is_portable: bool,
    /// 配置文件当前是否已存在。前端据此判断是否为首次运行：默认配置自带
    /// 内置引擎，不能再用“档案列表为空”推断磁盘上有没有用户数据
    pub exists: bool,
}

/// 配置管理服务，支持便携模式检测与崩溃保护原子写入
pub struct ConfigService {
    config_path: PathBuf,
    is_portable: bool,
}

impl ConfigService {
    pub fn new(app_config_dir: Option<PathBuf>) -> Self {
        let (config_path, is_portable) = Self::resolve_config_path(None, app_config_dir);
        Self {
            config_path,
            is_portable,
        }
    }

    #[cfg(test)]
    pub fn with_custom_path(path: PathBuf, is_portable: bool) -> Self {
        Self {
            config_path: path,
            is_portable,
        }
    }

    pub fn location_info(&self) -> ConfigLocationInfo {
        ConfigLocationInfo {
            file_path: self.config_path.to_string_lossy().to_string(),
            is_portable: self.is_portable,
            exists: self.config_path.is_file(),
        }
    }

    /// 引擎安装根目录
    ///
    /// 便携模式下位于可执行文件（即 `config.json`）同级的 `engines/`，
    /// 标准模式下位于系统用户配置目录下的 `engines/`；两种模式都由
    /// `config_path` 的父目录决定，因此无需再区分便携标志
    pub fn engines_dir(&self) -> PathBuf {
        self.config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("engines")
    }

    /// 指定内置引擎的安装目录，例如 `engines/pikafish`
    pub fn engine_install_dir(&self, install_key: &str) -> PathBuf {
        self.engines_dir().join(install_key)
    }

    /// 解析配置文件的目标物理路径
    /// 优先级：
    /// 1. 若可执行文件目录 (exe_dir) 存在 `config.json` 或 `.portable` 标记文件，锁定为便携模式，使用 `exe_dir/config.json`；
    /// 2. 否则，回退到操作系统标准用户配置目录 (`app_config_dir`)
    pub fn resolve_config_path(
        custom_exe_dir: Option<&Path>,
        app_config_dir: Option<PathBuf>,
    ) -> (PathBuf, bool) {
        let exe_dir = custom_exe_dir.map(Path::to_path_buf).or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf))
        });

        if let Some(dir) = exe_dir {
            let portable_config = dir.join("config.json");
            let portable_flag = dir.join(".portable");

            if portable_config.is_file() || portable_flag.exists() {
                return (portable_config, true);
            }
        }

        let config_dir = app_config_dir.unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".canglang")
        });

        (config_dir.join("config.json"), false)
    }

    /// 读取持久化配置。若文件不存在或破损，返回默认配置
    pub fn load(&self) -> Result<AppConfig, EngineError> {
        if !self.config_path.exists() {
            return Ok(AppConfig::default());
        }

        let content = std::fs::read_to_string(&self.config_path)?;
        let config = serde_json::from_str::<AppConfig>(&content).unwrap_or_default();
        Ok(config)
    }

    /// 将配置格式化为 Pretty JSON，并通过临时文件原子替换保存（崩溃安全）
    pub fn save(&self, config: &AppConfig) -> Result<(), EngineError> {
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| EngineError::InvalidConfig(e.to_string()))?;

        if let Some(parent) = self.config_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }

        // 原子替换：先写同级 .tmp 临时文件，再做文件系统原子 rename
        let tmp_path = self.config_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, content.as_bytes())?;
        std::fs::rename(&tmp_path, &self.config_path)?;

        Ok(())
    }
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
                "canglang_cfg_test_{}_{}_{n}",
                std::process::id(),
                tag
            ));
            let _ = std::fs::create_dir_all(&path);
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_resolve_portable_when_config_exists() {
        let dir = TempDir::new("portable_exist");
        let cfg_file = dir.path.join("config.json");
        std::fs::write(&cfg_file, b"{}").unwrap();

        let (resolved, is_portable) = ConfigService::resolve_config_path(
            Some(&dir.path),
            Some(PathBuf::from("/some/system/dir")),
        );

        assert!(is_portable);
        assert_eq!(resolved, cfg_file);
    }

    #[test]
    fn test_resolve_portable_when_flag_exists() {
        let dir = TempDir::new("portable_flag");
        let flag_file = dir.path.join(".portable");
        std::fs::write(&flag_file, b"").unwrap();

        let (resolved, is_portable) = ConfigService::resolve_config_path(
            Some(&dir.path),
            Some(PathBuf::from("/some/system/dir")),
        );

        assert!(is_portable);
        assert_eq!(resolved, dir.path.join("config.json"));
    }

    #[test]
    fn test_resolve_system_fallback() {
        let dir = TempDir::new("standard_fallback");
        let sys_dir = PathBuf::from("/system/app_config");

        let (resolved, is_portable) =
            ConfigService::resolve_config_path(Some(&dir.path), Some(sys_dir.clone()));

        assert!(!is_portable);
        assert_eq!(resolved, sys_dir.join("config.json"));
    }

    #[test]
    fn test_atomic_save_and_load_roundtrip() {
        let dir = TempDir::new("save_load");
        let target_cfg = dir.path.join("config.json");
        let service = ConfigService::with_custom_path(target_cfg.clone(), true);

        let config = AppConfig {
            theme: "dark".to_string(),
            cloud_book_enabled: false,
            opening_book_paths: vec!["/path/to/test.bh".to_string()],
            ..AppConfig::default()
        };

        service.save(&config).unwrap();
        assert!(target_cfg.exists());

        // 临时文件应已被自动原子重命名清理
        assert!(!dir.path.join("config.json.tmp").exists());

        let loaded = service.load().unwrap();
        assert_eq!(loaded.theme, "dark");
        assert!(!loaded.cloud_book_enabled);
        assert_eq!(loaded.opening_book_paths, vec!["/path/to/test.bh"]);
    }

    #[test]
    fn test_default_config_registers_builtin_engine() {
        let config = AppConfig::default();

        assert_eq!(config.engine_profiles, default_builtin_profiles());
        assert_eq!(config.active_engine_id.as_deref(), Some("builtin-pikafish"));

        let builtin = &config.engine_profiles[0];
        assert!(builtin.is_builtin);
        assert!(builtin.path.is_empty(), "内置引擎初始状态不应有本地路径");
        assert_eq!(builtin.protocol, "uci");
        assert!(builtin.download_url.is_some());
    }

    #[test]
    fn test_find_builtin_resolves_registered_profile() {
        let found = EngineProfile::find_builtin("builtin-pikafish").expect("内置皮卡鱼应已注册");
        assert!(found.is_builtin);
        assert_eq!(found.install_key(), "pikafish");
        assert!(EngineProfile::find_builtin("builtin-unknown").is_none());
    }

    #[test]
    fn test_engine_install_dir_follows_config_location() {
        let portable_dir = TempDir::new("engines_portable");
        let portable = ConfigService::with_custom_path(portable_dir.path.join("config.json"), true);
        assert_eq!(
            portable.engine_install_dir("pikafish"),
            portable_dir.path.join("engines").join("pikafish")
        );

        let standard_dir = TempDir::new("engines_standard");
        let standard =
            ConfigService::with_custom_path(standard_dir.path.join("config.json"), false);
        assert_eq!(
            standard.engines_dir(),
            standard_dir.path.join("engines"),
            "标准模式同样把 engines/ 放在配置目录下"
        );
    }

    #[test]
    fn test_location_info_reports_config_existence() {
        let dir = TempDir::new("location_exists");
        let config_path = dir.path.join("config.json");
        let service = ConfigService::with_custom_path(config_path.clone(), true);

        assert!(
            !service.location_info().exists,
            "首次运行不应报告配置已存在"
        );

        service.save(&AppConfig::default()).unwrap();
        let info = service.location_info();
        assert!(info.exists);
        assert!(info.is_portable);
        assert_eq!(info.file_path, config_path.to_string_lossy());
    }

    #[test]
    fn test_engine_profile_serializes_camel_case_contract() {
        let profile = EngineProfile::builtin_pikafish();
        let value = serde_json::to_value(&profile).unwrap();

        for key in [
            "id",
            "name",
            "path",
            "protocol",
            "threads",
            "hashMb",
            "nnuePath",
            "optionOverrides",
            "releaseRevision",
            "installedRevision",
            "isBuiltin",
            "downloadUrl",
            "description",
        ] {
            assert!(value.get(key).is_some(), "缺少字段 {key}");
        }

        let roundtrip: EngineProfile = serde_json::from_value(value).unwrap();
        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn legacy_profile_without_revision_fields_loads_as_unrecorded() {
        let mut value = serde_json::to_value(EngineProfile::builtin_pikafish()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("optionOverrides");
        object.remove("releaseRevision");
        object.remove("installedRevision");

        let profile: EngineProfile = serde_json::from_value(value).unwrap();
        assert!(profile.option_overrides.is_empty());
        assert!(profile.release_revision.is_empty());
        assert!(profile.installed_revision.is_empty());
    }
}
