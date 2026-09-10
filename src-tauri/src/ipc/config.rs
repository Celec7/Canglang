//! 全局持久化配置与便携模式 IPC 命令

use crate::AppError;
use crate::engine::config::{AppConfig, ConfigLocationInfo, ConfigService};
use std::path::PathBuf;
use tauri::State;
use tokio::sync::Mutex;

/// 受管的配置服务状态
pub struct ConfigState(pub Mutex<ConfigService>);

impl ConfigState {
    pub fn new(app_config_dir: Option<PathBuf>) -> Self {
        Self(Mutex::new(ConfigService::new(app_config_dir)))
    }
}

impl Default for ConfigState {
    fn default() -> Self {
        Self::new(None)
    }
}

/// 读取当前持久化配置。若文件尚不存在则返回标准初始默认配置
#[tauri::command]
#[specta::specta]
pub async fn config_load(state: State<'_, ConfigState>) -> Result<AppConfig, AppError> {
    let service = state.0.lock().await;
    Ok(service.load()?)
}

/// 保存配置（采用文件系统原子替换，杜绝异常断电导致配置损坏为 0 字节）
#[tauri::command]
#[specta::specta]
pub async fn config_save(config: AppConfig, state: State<'_, ConfigState>) -> Result<(), AppError> {
    let service = state.0.lock().await;
    service.save(&config)?;
    Ok(())
}

/// 获取当前配置文件的物理存储路径及是否处于便携模式
#[tauri::command]
#[specta::specta]
pub async fn config_get_location(
    state: State<'_, ConfigState>,
) -> Result<ConfigLocationInfo, AppError> {
    let service = state.0.lock().await;
    Ok(service.location_info())
}
