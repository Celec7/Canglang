//! 引擎 IPC 命令与事件转发
//!
//! 把 `engine` 模块（`EngineSession`）桥接到前端。会话保存在受管的
//! [`EngineState`]；后台 tokio 任务订阅会话的 `think_stream`/`best_move_stream`
//! 广播并重发为 Tauri 事件：
//!
//! - `think://`   —— 最新 [`ThinkData`]，约 50ms 节流（latest-wins）
//! - `bestmove://` —— 引擎最佳着法的 ICCS 字符串
//! - `engine://stopped` —— 外部引擎异常退出时的空 payload 通知
//!
//! 所有命令为 `async`，并对会话使用 `tokio::sync::Mutex`，以便在 `.await`
//! 点之间驱动受保护的会话

use crate::AppError;
use crate::core::board::BoardState;
use crate::core::notation::NotationConverter;
use crate::engine::EngineRawLine;
use crate::engine::config::EngineProfile;
use crate::engine::installer::EngineInstaller;
use crate::engine::models::{
    AnalysisEvent, AnalysisRequest, AnalysisStartResult, EngineConfig, EngineInfo,
};
use crate::engine::{EngineError, EngineSession};
use crate::ipc::config::ConfigState;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{Mutex, broadcast};
use tokio::task::JoinHandle;

/// 受管的引擎状态：运行中的会话 + 后台事件转发任务
#[derive(Default)]
pub struct EngineState {
    lifecycle: Mutex<()>,
    session: Arc<Mutex<Option<EngineSession>>>,
    forward_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl EngineState {
    async fn stop_forward_task(&self) {
        let handle = self.forward_task.lock().await.take();
        if let Some(handle) = handle {
            handle.abort();
            let _ = handle.await;
        }
    }

    async fn stop_session(&self) {
        let session = self.session.lock().await.take();
        if let Some(mut session) = session {
            session.stop().await;
        }
    }

    async fn stop_all(&self) {
        // 先停止事件转发，避免被替换的会话在进程退出期间发布旧事件
        self.stop_forward_task().await;
        self.stop_session().await;
    }
}

/// 启动（或重启）引擎子进程并开始转发其事件
#[tauri::command]
#[specta::specta]
pub async fn engine_start(
    config: EngineConfig,
    state: State<'_, EngineState>,
    app: AppHandle,
) -> Result<EngineInfo, AppError> {
    let _lifecycle = state.lifecycle.lock().await;
    // 先启动候选会话，再停止旧会话：切换失败时原活动引擎仍保持可用
    let mut session = EngineSession::new();

    // 启动前先订阅，确保不遗漏分析事件（广播接收端会在转发任务开始前缓存）
    let think_rx = session.think_stream();
    let best_rx = session.best_move_stream();
    let stopped_rx = session.stopped_stream();
    let raw_rx = session.raw_line_stream();

    let info = match session.start(&config).await {
        Ok(info) => info,
        Err(error) => {
            session.stop().await;
            return Err(error.into());
        }
    };

    let board_state = session.board_state();

    state.stop_all().await;

    *state.session.lock().await = Some(session);
    let session_ref = state.session.clone();
    let fhandle = tokio::spawn(forward_events(
        app.clone(),
        think_rx,
        best_rx,
        stopped_rx,
        raw_rx,
        session_ref,
        board_state,
    ));

    *state.forward_task.lock().await = Some(fhandle);

    Ok(info)
}

/// 开始对给定局面进行分析
#[tauri::command]
#[specta::specta]
pub async fn engine_analyze(
    request: AnalysisRequest,
    state: State<'_, EngineState>,
) -> Result<AnalysisStartResult, AppError> {
    let mut guard = state.session.lock().await;
    let session = guard
        .as_mut()
        .ok_or_else(|| AppError::Engine(EngineError::NotRunning))?;
    Ok(session.analyze(request).await?)
}

/// 中断当前搜索并发出引擎的最佳着法
#[tauri::command]
#[specta::specta]
pub async fn engine_move_now(state: State<'_, EngineState>) -> Result<(), AppError> {
    let mut guard = state.session.lock().await;
    let session = guard
        .as_mut()
        .ok_or_else(|| AppError::Engine(EngineError::NotRunning))?;
    if !session.is_running() {
        return Err(AppError::Engine(EngineError::NotRunning));
    }
    session.move_now().await?;
    Ok(())
}

/// 触发当前引擎握手声明的 button 类型动态选项
#[tauri::command]
#[specta::specta]
pub async fn engine_trigger_button_option(
    name: String,
    state: State<'_, EngineState>,
) -> Result<(), AppError> {
    let mut guard = state.session.lock().await;
    let session = guard
        .as_mut()
        .ok_or_else(|| AppError::Engine(EngineError::NotRunning))?;
    session.trigger_button_option(&name).await?;
    Ok(())
}

/// 排除给定 ICCS 走法后重新搜索
#[tauri::command]
#[specta::specta]
pub async fn engine_change_tactic(
    request: AnalysisRequest,
    state: State<'_, EngineState>,
) -> Result<AnalysisStartResult, AppError> {
    let mut guard = state.session.lock().await;
    let session = guard
        .as_mut()
        .ok_or_else(|| AppError::Engine(EngineError::NotRunning))?;
    Ok(session.analyze(request).await?)
}

/// 停止引擎子进程及其事件转发任务
#[tauri::command]
#[specta::specta]
pub async fn engine_stop(state: State<'_, EngineState>) -> Result<(), AppError> {
    let _lifecycle = state.lifecycle.lock().await;
    state.stop_all().await;
    Ok(())
}

/// 报告引擎会话是否在运行或正在分析
#[tauri::command]
#[specta::specta]
pub async fn engine_status(state: State<'_, EngineState>) -> Result<bool, AppError> {
    let guard = state.session.lock().await;
    let active = match guard.as_ref() {
        Some(session) => session.is_running() || session.is_analyzing(),
        None => false,
    };
    Ok(active)
}

/// 查询当前引擎运行中的原始协议日志
#[tauri::command]
#[specta::specta]
pub async fn engine_protocol_log(
    analysis_session_id: Option<String>,
    state: State<'_, EngineState>,
) -> Result<Vec<EngineRawLine>, AppError> {
    let guard = state.session.lock().await;
    let session = guard
        .as_ref()
        .ok_or_else(|| AppError::Engine(EngineError::NotRunning))?;
    Ok(match analysis_session_id.as_deref() {
        Some(id) => session.protocol_log_for_session(id).await,
        None => session.protocol_log().await,
    })
}

/// 将当前运行中的原始协议日志导出为文本，不改变后端日志缓冲
#[tauri::command]
#[specta::specta]
pub async fn engine_export_protocol_log(
    path: String,
    analysis_session_id: Option<String>,
    state: State<'_, EngineState>,
) -> Result<(), AppError> {
    let lines = engine_protocol_log(analysis_session_id, state).await?;
    let mut file = std::fs::File::create(path).map_err(EngineError::Io)?;
    for line in lines {
        writeln!(
            file,
            "# {} {} {:?} {:?}",
            line.sequence, line.run_id, line.analysis_session_id, line.stream
        )
        .map_err(EngineError::Io)?;
        file.write_all(line.raw.as_bytes())
            .map_err(EngineError::Io)?;
        if !line.raw.ends_with('\n') {
            file.write_all(b"\n").map_err(EngineError::Io)?;
        }
    }
    Ok(())
}

/// 弹出系统原生文件对话框，按 `kind` 过滤可选文件类型
/// `nnue` 过滤权重文件，`book` 过滤 `.bh` 开局库，`folder` 选择目录；其它取值按引擎可执行文件处理，
/// 不做扩展名过滤，以兼容 Unix 上无扩展名的引擎
#[tauri::command]
#[specta::specta]
pub async fn engine_pick_file(kind: Option<String>) -> Result<Option<String>, AppError> {
    let dialog = match kind.as_deref() {
        Some("nnue") => rfd::AsyncFileDialog::new()
            .set_title("选择 NNUE 神经网络权重文件")
            .add_filter("NNUE 权重", &["nnue"]),
        Some("book") => rfd::AsyncFileDialog::new()
            .set_title("选择开局库文件")
            .add_filter("开局库", &["bh"]),
        Some("folder") => {
            let folder = rfd::AsyncFileDialog::new()
                .set_title("选择引擎目录")
                .pick_folder()
                .await;
            return Ok(folder.map(|f| f.path().to_string_lossy().into_owned()));
        }
        Some("protocol-log") => {
            let file = rfd::AsyncFileDialog::new()
                .set_title("导出引擎协议日志")
                .add_filter("文本日志", &["log", "txt"])
                .save_file()
                .await;
            return Ok(file.map(|f| f.path().to_string_lossy().into_owned()));
        }
        _ => rfd::AsyncFileDialog::new().set_title("选择象棋引擎可执行文件"),
    };

    let file = dialog.pick_file().await;
    Ok(file.map(|f| f.path().to_string_lossy().into_owned()))
}

/// 最终向前端发射思考数据前，推导并补全中文走法序列（延迟求值，节省中间节流帧的开销）
fn enrich_think(
    board_state: &Arc<std::sync::Mutex<Option<BoardState>>>,
    mut event: AnalysisEvent,
) -> AnalysisEvent {
    if let AnalysisEvent::Think { data, .. } = &mut event
        && !data.pv.is_empty()
        && data.pv_chinese.is_empty()
        && let Ok(guard) = board_state.lock()
        && let Some(board) = guard.as_ref()
    {
        data.pv_chinese = NotationConverter::deduce_pv_chinese(board, &data.pv);
    }
    event
}

/// 将会话的事件流转发为 Tauri 事件，并对思考数据按 multi_pv 分路做节流
async fn forward_events(
    app: AppHandle,
    mut think_rx: broadcast::Receiver<AnalysisEvent>,
    mut best_rx: broadcast::Receiver<AnalysisEvent>,
    mut stopped_rx: broadcast::Receiver<()>,
    mut raw_rx: broadcast::Receiver<EngineRawLine>,
    session_ref: Arc<Mutex<Option<EngineSession>>>,
    board_state: Arc<std::sync::Mutex<Option<BoardState>>>,
) {
    let latest_thinks = Arc::new(std::sync::Mutex::new(std::collections::HashMap::<
        u32,
        AnalysisEvent,
    >::new()));
    let mut interval = tokio::time::interval(Duration::from_millis(50));
    let mut think_closed = false;
    let mut best_closed = false;

    loop {
        tokio::select! {
            r = best_rx.recv() => match r {
                Ok(event @ AnalysisEvent::BestMove { .. }) => {
                    let batch = {
                        let mut guard = latest_thinks.lock().unwrap();
                        std::mem::take(&mut *guard)
                    };
                    for (_idx, td) in batch {
                        let td = enrich_think(&board_state, td);
                        let _ = app.emit("think://", td);
                    }
                    let _ = app.emit("bestmove://", event);
                }
                Ok(AnalysisEvent::Think { .. }) => {}
                Err(broadcast::error::RecvError::Closed) => best_closed = true,
                Err(broadcast::error::RecvError::Lagged(_)) => {}
            },
            r = think_rx.recv() => match r {
                Ok(event @ AnalysisEvent::Think { .. }) => {
                    let multi_pv = match &event {
                        AnalysisEvent::Think { data, .. } => data.multi_pv,
                        AnalysisEvent::BestMove { .. } => unreachable!(),
                    };
                    latest_thinks.lock().unwrap().insert(multi_pv, event);
                }
                Ok(AnalysisEvent::BestMove { .. }) => {}
                Err(broadcast::error::RecvError::Closed) => think_closed = true,
                Err(broadcast::error::RecvError::Lagged(_)) => {}
            },
            r = stopped_rx.recv() => match r {
                Ok(()) => {
                    let _ = app.emit("engine://stopped", ());
                    if let Some(mut session) = session_ref.lock().await.take() {
                        session.stop().await;
                    }
                    break;
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => {}
            },
            r = raw_rx.recv() => match r {
                Ok(line) => {
                    let _ = app.emit("engine://protocol", line);
                }
                Err(broadcast::error::RecvError::Closed) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => {}
            },
            _ = interval.tick() => {
                let batch = {
                    let mut guard = latest_thinks.lock().unwrap();
                    std::mem::take(&mut *guard)
                };
                for (_idx, td) in batch {
                    let td = enrich_think(&board_state, td);
                    let _ = app.emit("think://", td);
                }
            }
        }

        if think_closed && best_closed {
            break;
        }
    }
}

/// 在线下载、解压并装配内置引擎（如皮卡鱼 Pikafish）
/// 下载与解压期间不持有配置锁，避免长时间阻塞其它配置读写；装配完成后先对
/// staging 产物做一次协议探活，再替换正式目录并写入 `config.json`。探活或配置
/// 写入失败都不会破坏现有安装
#[tauri::command]
#[specta::specta]
pub async fn engine_download_builtin(
    profile_id: String,
    config_state: State<'_, ConfigState>,
    state: State<'_, EngineState>,
    app: AppHandle,
) -> Result<EngineProfile, AppError> {
    let _lifecycle = state.lifecycle.lock().await;
    if state
        .session
        .lock()
        .await
        .as_ref()
        .is_some_and(|session| session.is_running())
    {
        return Err(AppError::Engine(EngineError::InvalidConfig(
            "请先停止当前运行中的引擎，再安装或升级内置引擎".to_string(),
        )));
    }
    // 1. 解析档案与安装目录：只在读取配置时短暂持锁
    let (profile, install_dir) = {
        let service = config_state.0.lock().await;
        let config = service.load()?;
        let profile = config
            .engine_profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .or_else(|| EngineProfile::find_builtin(&profile_id))
            .ok_or_else(|| AppError::InvalidArgument(format!("未找到引擎档案: {profile_id}")))?;

        let install_dir = service.engine_install_dir(profile.install_key());
        (profile, install_dir)
    };

    if !profile.is_builtin {
        return Err(AppError::InvalidArgument(format!(
            "引擎档案 {profile_id} 不是内置在线引擎，请改为手动选择可执行文件"
        )));
    }

    // 2. 下载并装配到 staging，正式目录仍保持可用
    let prepared = EngineInstaller::prepare_with_progress(&profile, &install_dir, |payload| {
        let _ = app.emit(crate::engine::DOWNLOAD_PROGRESS_EVENT, payload);
    })
    .await?;

    // 3. 先对 staging 产物做探活，失败时只清理 staging
    let mut probe = EngineSession::new();
    let mut probe_config = EngineConfig::new(
        prepared.profile.path.clone(),
        prepared.profile.protocol.clone(),
    );
    if let Some(nnue_path) = prepared.profile.nnue_path.as_deref() {
        probe_config = probe_config.with_nnue(nnue_path.to_string());
    }
    let probe_result = probe.start(&probe_config).await;
    probe.stop().await;

    if let Err(error) = probe_result {
        prepared.cleanup();
        return Err(AppError::Engine(EngineError::ProbeFailed {
            details: format!("新引擎探活失败，已保留现有安装: {error}"),
        }));
    }

    // 4. 探活成功后才替换正式目录并原子写入 Profile
    let committed = EngineInstaller::commit_prepared(prepared)?;
    let installed = committed.profile.clone();
    let save_result = {
        let service = config_state.0.lock().await;
        (|| -> Result<(), AppError> {
            let mut config = service.load()?;
            match config
                .engine_profiles
                .iter()
                .position(|p| p.id == profile_id)
            {
                Some(index) => config.engine_profiles[index] = installed.clone(),
                None => config.engine_profiles.push(installed.clone()),
            }
            service.save(&config)?;
            Ok(())
        })()
    };

    if let Err(error) = save_result {
        if let Err(rollback_error) = committed.rollback() {
            return Err(AppError::Engine(EngineError::InvalidConfig(format!(
                "引擎配置保存失败，且旧版本恢复失败: {error}; {rollback_error}"
            ))));
        }
        return Err(error);
    }

    committed.finalize()?;

    Ok(installed)
}

/// 清除内置引擎已下载的本地文件，并把它重置为未安装状态
/// 只删除本应用在 `engines/<install_key>` 下创建的内容；自定义本地引擎不经过
/// 该目录，因此不受影响
#[tauri::command]
#[specta::specta]
pub async fn engine_remove_builtin(
    profile_id: String,
    config_state: State<'_, ConfigState>,
    state: State<'_, EngineState>,
) -> Result<EngineProfile, AppError> {
    let _lifecycle = state.lifecycle.lock().await;
    let service = config_state.0.lock().await;
    let mut config = service.load()?;
    let index = config
        .engine_profiles
        .iter()
        .position(|profile| profile.id == profile_id)
        .ok_or_else(|| AppError::InvalidArgument(format!("未找到引擎档案: {profile_id}")))?;

    if !config.engine_profiles[index].is_builtin {
        return Err(AppError::InvalidArgument(format!(
            "引擎档案 {profile_id} 不是内置在线引擎"
        )));
    }

    let active_profile = config.active_engine_id.as_deref() == Some(profile_id.as_str());

    // 只有活动 Profile 拥有运行中的 EngineSession
    // 删除未活动的内置 Profile 不能中断其它引擎，删除活动 Profile 则必须先停止会话再移动可执行文件
    if active_profile
        && state
            .session
            .lock()
            .await
            .as_ref()
            .is_some_and(|session| session.is_running())
    {
        state.stop_all().await;
    }

    let install_dir = service.engine_install_dir(config.engine_profiles[index].install_key());
    let backup_dir = install_dir
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(format!(
            ".{}-remove-backup",
            install_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ));
    let had_install = install_dir.exists();
    let _ = std::fs::remove_dir_all(&backup_dir);
    if had_install {
        std::fs::rename(&install_dir, &backup_dir).map_err(|error| {
            EngineError::InvalidConfig(format!(
                "准备删除引擎目录 {} 失败: {error}",
                install_dir.display()
            ))
        })?;
    }

    let profile = &mut config.engine_profiles[index];
    profile.path = String::new();
    profile.nnue_path = None;
    profile.installed_revision = String::new();
    let reset = profile.clone();

    if let Err(error) = service.save(&config) {
        if had_install && let Err(rollback_error) = std::fs::rename(&backup_dir, &install_dir) {
            return Err(AppError::Engine(EngineError::InvalidConfig(format!(
                "卸载配置保存失败，且旧引擎恢复失败: {error}; {rollback_error}"
            ))));
        }
        return Err(AppError::Engine(error));
    }
    if had_install {
        std::fs::remove_dir_all(&backup_dir).map_err(|error| {
            EngineError::InvalidConfig(format!(
                "清理旧引擎目录备份失败 {}: {error}",
                backup_dir.display()
            ))
        })?;
    }

    Ok(reset)
}
