use crate::core::board::BoardState;
use crate::core::position::Move;
use crate::core::rules::MoveValidator;
use crate::engine::diagnostics::EngineRawLine;
use crate::engine::error::EngineError;
use crate::engine::models::{
    AnalysisEvent, AnalysisRequest, AnalysisStartResult, ConstraintApplicationStatus,
    EngineCapabilities, EngineConfig, EngineInfo, EngineOptionDescriptor, EngineOptionType,
    EnginePositionContext, RootMoveConstraint,
};
use crate::engine::probe;
use crate::engine::process::EngineProcess;
use crate::engine::protocol::{
    HandshakeInfo, Protocol, capabilities_for_protocol, parse_handshake_line,
};
use crate::engine::protocols::{UcciProtocol, UciProtocol};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{Notify, broadcast, oneshot};
use tokio::task::JoinHandle;

const PROTOCOL_TIMEOUT: Duration = Duration::from_secs(5);

/// 统一调度与分析会话，屏蔽 UCI/UCCI 协议差异，向调用方暴露 ThinkData 与 BestMove 流
pub struct EngineSession {
    process: Option<EngineProcess>,
    protocol: Option<Arc<dyn Protocol>>,
    think_tx: broadcast::Sender<AnalysisEvent>,
    best_move_tx: broadcast::Sender<AnalysisEvent>,
    stopped_tx: broadcast::Sender<()>,
    ready_tx: broadcast::Sender<()>,
    raw_tx: broadcast::Sender<EngineRawLine>,
    board_state: Arc<Mutex<Option<BoardState>>>,
    parse_handle: Option<JoinHandle<()>>,
    raw_handle: Option<JoinHandle<()>>,
    raw_stop_tx: Option<oneshot::Sender<()>>,
    raw_log_path: Option<PathBuf>,
    raw_log_writer: Option<Arc<Mutex<BufWriter<File>>>>,
    raw_processed_sequence: Arc<std::sync::atomic::AtomicU32>,
    raw_drain_notify: Arc<Notify>,
    is_analyzing: Arc<AtomicBool>,
    analysis_needs_ready: Arc<AtomicBool>,
    analysis_session_id: Arc<Mutex<Option<String>>>,
    capabilities: Option<EngineCapabilities>,
    option_descriptors: Vec<EngineOptionDescriptor>,
}

impl EngineSession {
    pub fn new() -> Self {
        let (think_tx, _) = broadcast::channel(1024);
        let (best_move_tx, _) = broadcast::channel(64);
        let (stopped_tx, _) = broadcast::channel(8);
        let (ready_tx, _) = broadcast::channel(8);
        let (raw_tx, _) = broadcast::channel(4096);
        Self {
            process: None,
            protocol: None,
            think_tx,
            best_move_tx,
            stopped_tx,
            ready_tx,
            raw_tx,
            board_state: Arc::new(Mutex::new(None)),
            parse_handle: None,
            raw_handle: None,
            raw_stop_tx: None,
            raw_log_path: None,
            raw_log_writer: None,
            raw_processed_sequence: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            raw_drain_notify: Arc::new(Notify::new()),
            is_analyzing: Arc::new(AtomicBool::new(false)),
            analysis_needs_ready: Arc::new(AtomicBool::new(false)),
            analysis_session_id: Arc::new(Mutex::new(None)),
            capabilities: None,
            option_descriptors: Vec::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.process.as_ref().is_some_and(|p| p.is_running())
    }

    pub fn is_analyzing(&self) -> bool {
        self.is_analyzing.load(Ordering::Acquire)
    }

    pub fn think_stream(&self) -> broadcast::Receiver<AnalysisEvent> {
        self.think_tx.subscribe()
    }

    pub fn best_move_stream(&self) -> broadcast::Receiver<AnalysisEvent> {
        self.best_move_tx.subscribe()
    }

    pub fn stopped_stream(&self) -> broadcast::Receiver<()> {
        self.stopped_tx.subscribe()
    }

    pub fn raw_line_stream(&self) -> broadcast::Receiver<EngineRawLine> {
        self.raw_tx.subscribe()
    }

    /// 当前分析局面的线程安全引用
    pub fn board_state(&self) -> Arc<Mutex<Option<BoardState>>> {
        self.board_state.clone()
    }

    pub async fn protocol_log(&self) -> Vec<EngineRawLine> {
        self.wait_for_log_drain().await;
        flush_log_writer(self.raw_log_writer.as_ref());
        read_protocol_log(self.raw_log_path.as_deref())
    }

    pub async fn protocol_log_for_session(&self, analysis_session_id: &str) -> Vec<EngineRawLine> {
        self.wait_for_log_drain().await;
        flush_log_writer(self.raw_log_writer.as_ref());
        let all_lines = read_protocol_log(self.raw_log_path.as_deref());
        let first_analysis_sequence = all_lines
            .iter()
            .filter(|line| line.analysis_session_id.is_some())
            .map(|line| line.sequence)
            .min();
        let mut lines = all_lines
            .into_iter()
            .filter(|line| match line.analysis_session_id.as_deref() {
                Some(session_id) => session_id == analysis_session_id,
                None => first_analysis_sequence.is_none_or(|sequence| line.sequence < sequence),
            })
            .collect::<Vec<_>>();
        lines.sort_by_key(|line| line.sequence);
        lines
    }

    async fn wait_for_log_drain(&self) {
        let target = self
            .process
            .as_ref()
            .map(EngineProcess::current_sequence)
            .unwrap_or(0);
        loop {
            if self.raw_processed_sequence.load(Ordering::Acquire) >= target {
                return;
            }
            let notified = self.raw_drain_notify.notified();
            if self.raw_processed_sequence.load(Ordering::Acquire) >= target {
                return;
            }
            notified.await;
        }
    }

    /// 启动引擎子进程并完成握手与就绪检查
    pub async fn start(&mut self, config: &EngineConfig) -> Result<EngineInfo, EngineError> {
        if self.process.is_some() || self.protocol.is_some() {
            self.stop().await;
        }

        let detected = if config.protocol.trim().eq_ignore_ascii_case("auto") {
            Some(probe::detect(&config.path).await?)
        } else {
            None
        };
        let protocol_name = detected
            .as_ref()
            .map(|info| info.protocol.as_str())
            .unwrap_or_else(|| config.protocol.trim());
        let protocol: Arc<dyn Protocol> = match protocol_name.to_ascii_lowercase().as_str() {
            "ucci" => Arc::new(UcciProtocol::new()),
            "uci" => Arc::new(UciProtocol::new()),
            protocol => {
                return Err(EngineError::InvalidConfig(format!(
                    "unsupported engine protocol '{protocol}'"
                )));
            }
        };

        let mut resolved_config = config.clone();
        resolved_config.protocol = protocol_name.to_string();
        let mut process = self.process.take().unwrap_or_default();
        let line_rx = process.line_stream();
        let mut raw_rx = process.raw_line_stream();
        let exit_rx = process.exit_stream();
        let log_path = std::env::temp_dir().join(format!(
            "canglang-engine-log-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let log_writer = Arc::new(Mutex::new(BufWriter::new(File::create(&log_path)?)));
        if let Err(error) = process.start(&resolved_config) {
            let _ = std::fs::remove_file(&log_path);
            return Err(error);
        }
        self.raw_log_path = Some(log_path.clone());
        self.raw_log_writer = Some(log_writer.clone());
        self.raw_processed_sequence.store(0, Ordering::Release);
        self.is_analyzing.store(false, Ordering::Release);
        self.analysis_needs_ready.store(false, Ordering::Release);
        if let Ok(mut guard) = self.board_state.lock() {
            *guard = None;
        }

        let raw_tx = self.raw_tx.clone();
        let raw_processed_sequence = self.raw_processed_sequence.clone();
        let raw_drain_notify = self.raw_drain_notify.clone();
        let log_writer = log_writer.clone();
        let (raw_stop_tx, mut raw_stop_rx) = oneshot::channel();
        let raw_handle = tokio::spawn(async move {
            let record_line = |line: EngineRawLine| {
                if let Ok(mut writer) = log_writer.lock()
                    && serde_json::to_writer(&mut *writer, &line).is_ok()
                {
                    let _ = writer.write_all(b"\n");
                    let _ = writer.flush();
                }
                let sequence = line.sequence;
                let _ = raw_tx.send(line);
                raw_processed_sequence.fetch_max(sequence, Ordering::AcqRel);
                raw_drain_notify.notify_waiters();
            };
            loop {
                tokio::select! {
                    result = raw_rx.recv() => match result {
                        Some(line) => record_line(line),
                        None => break,
                    },
                    _ = &mut raw_stop_rx => {
                        while let Ok(line) = raw_rx.try_recv() {
                            record_line(line);
                        }
                        break;
                    }
                }
            }
        });

        let (handshake_tx, handshake_rx) = oneshot::channel::<()>();
        let (ready_tx, ready_rx) = oneshot::channel::<()>();

        let think_tx = self.think_tx.clone();
        let best_move_tx = self.best_move_tx.clone();
        let proto = protocol.clone();
        let is_analyzing = self.is_analyzing.clone();
        let analysis_needs_ready = self.analysis_needs_ready.clone();
        let analysis_session_id = self.analysis_session_id.clone();
        let stopped_tx = self.stopped_tx.clone();
        let ready_broadcast_tx = self.ready_tx.clone();
        let handshake_state = Arc::new(Mutex::new(HandshakeInfo::default()));
        let handshake_state_for_task = handshake_state.clone();
        let handshake_finished = Arc::new(AtomicBool::new(false));
        let handshake_finished_for_task = handshake_finished.clone();
        let handshake_ok = protocol.handshake_ok_marker().to_string();
        let ready_ok = protocol.ready_ok_marker().to_string();

        let mut handshake_tx = Some(handshake_tx);
        let mut ready_tx = Some(ready_tx);

        let parse_handle = tokio::spawn(async move {
            let mut rx = line_rx;
            let mut exit_rx = exit_rx;
            loop {
                tokio::select! {
                    result = rx.recv() => match result {
                        Ok(line) => {
                        if !handshake_finished_for_task.load(Ordering::Acquire)
                            && let Ok(mut guard) = handshake_state_for_task.lock()
                        {
                            parse_handshake_line(&mut guard, &line);
                        }
                        if line.eq_ignore_ascii_case(&handshake_ok)
                            && let Some(tx) = handshake_tx.take()
                        {
                            let _ = tx.send(());
                        }
                        if line.eq_ignore_ascii_case(&ready_ok)
                            && let Some(tx) = ready_tx.take()
                        {
                            handshake_finished_for_task.store(true, Ordering::Release);
                            let _ = tx.send(());
                        }
                        if line.eq_ignore_ascii_case(&ready_ok) {
                            let _ = ready_broadcast_tx.send(());
                        }

                        if let Some(td) = proto.parse_info(&line)
                            && let Ok(guard) = analysis_session_id.lock()
                            && let Some(session_id) = guard.clone()
                        {
                            let _ = think_tx.send(AnalysisEvent::Think {
                                analysis_session_id: session_id,
                                data: td,
                            });
                        }

                        if let Some(iccs) = proto.parse_best_move(&line)
                            && let Ok(mv) = Move::from_iccs(&iccs)
                        {
                            is_analyzing.store(false, Ordering::Release);
                            analysis_needs_ready.store(true, Ordering::Release);
                            if let Ok(guard) = analysis_session_id.lock()
                                && let Some(session_id) = guard.clone()
                            {
                                let _ = best_move_tx.send(AnalysisEvent::BestMove {
                                    analysis_session_id: session_id,
                                    move_iccs: mv.to_iccs(),
                                });
                            }
                        }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            is_analyzing.store(false, Ordering::Release);
                            break;
                        }
                    },
                    result = exit_rx.recv() => {
                        if result.is_ok() {
                            is_analyzing.store(false, Ordering::Release);
                            let _ = stopped_tx.send(());
                        }
                        break;
                    }
                }
            }
        });

        let startup_result = async {
            // 1. 发送握手并等待确认
            process.send(protocol.handshake_command()).await?;
            wait_for_protocol_ack(handshake_rx, "handshake", PROTOCOL_TIMEOUT).await?;

            // 2. 发送自定义 Option 与 NNUE 权重配置
            if let Some(nnue) = &resolved_config.nnue_path {
                let trimmed = nnue.trim();
                if !trimmed.is_empty() {
                    let cmd = protocol.format_set_option("EvalFile", trimmed);
                    process.send(&cmd).await?;
                }
            }

            if let Some(options) = &resolved_config.options {
                let handshake_options = handshake_state
                    .lock()
                    .map(|guard| guard.options.clone())
                    .unwrap_or_default();
                let descriptors = if handshake_options.is_empty() {
                    detected
                        .as_ref()
                        .map(|info| info.option_descriptors.clone())
                        .unwrap_or_default()
                } else {
                    handshake_options
                };
                let validated_options = validate_option_overrides(options, &descriptors)?;
                for (key, value) in validated_options {
                    let cmd = protocol.format_set_option(key, value);
                    process.send(&cmd).await?;
                }
            }

            // 3. 确认就绪并等待确认
            process.send(protocol.ready_command()).await?;
            wait_for_protocol_ack(ready_rx, "ready", PROTOCOL_TIMEOUT).await?;

            Ok::<(), EngineError>(())
        }
        .await;

        if let Err(error) = startup_result {
            process.stop(None).await;
            parse_handle.abort();
            raw_handle.abort();
            let _ = std::fs::remove_file(&log_path);
            self.raw_log_path = None;
            self.raw_log_writer = None;
            self.analysis_needs_ready.store(false, Ordering::Release);
            return Err(error);
        }

        self.process = Some(process);
        self.protocol = Some(protocol);
        self.parse_handle = Some(parse_handle);
        self.raw_handle = Some(raw_handle);
        self.raw_stop_tx = Some(raw_stop_tx);
        let handshake = handshake_state
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        let option_descriptors = if handshake.options.is_empty() {
            detected
                .as_ref()
                .map(|info| info.option_descriptors.clone())
                .unwrap_or_default()
        } else {
            handshake.options.clone()
        };
        let capabilities = capabilities_for_protocol(protocol_name, &handshake);
        self.capabilities = Some(capabilities.clone());
        self.option_descriptors = option_descriptors.clone();
        Ok(EngineInfo {
            name: handshake
                .name
                .clone()
                .or_else(|| detected.as_ref().and_then(|info| info.name.clone())),
            protocol: protocol_name.to_string(),
            ready: true,
            diagnostic: if handshake.raw_lines.is_empty() {
                detected.as_ref().and_then(|info| info.diagnostic.clone())
            } else {
                Some(handshake.raw_lines.join(" | "))
            },
            option_descriptors,
            capabilities,
        })
    }

    /// 在一次搜索完成或被停止后，通过 readyok 建立下一条命令的协议边界
    async fn synchronize_after_analysis(&mut self) -> Result<(), EngineError> {
        let mut ready_rx = self.ready_tx.subscribe();
        if let (Some(process), Some(protocol)) = (self.process.as_mut(), self.protocol.as_ref()) {
            process.send(protocol.ready_command()).await?;
            let ready = tokio::time::timeout(Duration::from_millis(500), ready_rx.recv())
                .await
                .is_ok_and(|result| result.is_ok());
            if !ready {
                return Err(EngineError::ProtocolTimeout {
                    stage: "stop-ready".to_string(),
                });
            }
            self.is_analyzing.store(false, Ordering::Release);
            self.analysis_needs_ready.store(false, Ordering::Release);
            if let Ok(mut guard) = self.analysis_session_id.lock() {
                *guard = None;
            }
            process.set_analysis_session_id(None);
        }
        Ok(())
    }

    /// 内部中止当前正在进行的分析，并等待引擎响应 bestmove 以清空状态
    async fn stop_current_analysis(&mut self) -> Result<(), EngineError> {
        if !self.is_analyzing() {
            return Ok(());
        }
        if let (Some(process), Some(protocol)) = (self.process.as_mut(), self.protocol.as_ref()) {
            let mut best_rx = self.best_move_tx.subscribe();
            let cmd = protocol.format_stop();
            process.send(cmd).await?;
            let stopped = tokio::time::timeout(Duration::from_millis(500), async {
                while let Ok(event) = best_rx.recv().await {
                    if matches!(event, AnalysisEvent::BestMove { .. }) {
                        return true;
                    }
                }
                false
            })
            .await
            .unwrap_or(false);
            if !stopped {
                // 不要把仍在运行的搜索重新标记为下一会话
                // 调用方必须重试或停止引擎，之后到达的 bestmove 仍归属于旧分析会话
                return Err(EngineError::ProtocolTimeout {
                    stage: "stop".to_string(),
                });
            }
        }
        self.synchronize_after_analysis().await
    }

    /// 按请求中的起始局面和完整历史发起一次分析
    pub async fn analyze(
        &mut self,
        request: AnalysisRequest,
    ) -> Result<AnalysisStartResult, EngineError> {
        let board = replay_position(&request.position)?;
        let capabilities = self.capabilities.clone().ok_or(EngineError::NotRunning)?;
        let resolved = resolve_root_constraint(&board, &request.constraint, &capabilities)?;

        if resolved.status == ConstraintApplicationStatus::Unsupported {
            return Ok(AnalysisStartResult {
                constraint_status: resolved.status,
                started: false,
            });
        }

        if self.is_analyzing() {
            self.stop_current_analysis().await?;
        }
        if self.analysis_needs_ready.load(Ordering::Acquire) {
            self.synchronize_after_analysis().await?;
        }

        let multi_pv_value = if capabilities.multi_pv {
            let requested = request.config.multi_pv.max(1).min(i32::MAX as u32) as i32;
            let descriptor = self
                .option_descriptors
                .iter()
                .find(|descriptor| descriptor.name.eq_ignore_ascii_case("MultiPV"));
            let min = descriptor.and_then(|item| item.min).unwrap_or(1).max(1);
            let max = descriptor
                .and_then(|item| item.max)
                .unwrap_or(requested)
                .max(min);
            Some(requested.clamp(min, max).to_string())
        } else {
            None
        };

        let process = self.process.as_mut().ok_or(EngineError::NotRunning)?;
        let protocol = self.protocol.as_ref().ok_or(EngineError::NotRunning)?;

        if let Ok(mut guard) = self.board_state.lock() {
            *guard = Some(board);
        }

        if let Some(value) = multi_pv_value {
            let cmd = protocol.format_set_option("MultiPV", &value);
            let _ = process
                .send_with_session(&cmd, Some(&request.analysis_session_id))
                .await;
        }

        let history = request.position.history.clone();
        let pos = protocol.format_position(Some(&request.position.start_fen), Some(&history));
        // 安装新局面期间保持进程输出不带标签
        // 停止搜索后仍在排空的字节不能被误认为本会话输出；先显式为 position 指令标记会话，再在 go 前启用常规输出标签
        process
            .send_with_session(&pos, Some(&request.analysis_session_id))
            .await?;
        process.set_analysis_session_id(Some(request.analysis_session_id.clone()));

        let go = if capabilities.searchmoves {
            protocol.format_go(&request.config, resolved.search_moves.as_deref())
        } else if capabilities.banmoves {
            protocol.format_go_with_banmoves(
                &request.config,
                resolved.ban_moves.as_deref().unwrap_or_default(),
            )
        } else {
            protocol.format_go(&request.config, None)
        };
        if let Ok(mut guard) = self.analysis_session_id.lock() {
            *guard = Some(request.analysis_session_id.clone());
        }
        process.send(&go).await?;
        self.is_analyzing.store(true, Ordering::Release);

        Ok(AnalysisStartResult {
            constraint_status: resolved.status,
            started: true,
        })
    }

    /// 触发握手声明的 button 类型动态选项，例如 Clear Hash
    pub async fn trigger_button_option(&mut self, name: &str) -> Result<(), EngineError> {
        if self.is_analyzing() {
            return Err(EngineError::InvalidConfig(
                "cannot trigger an engine button while analysis is running".to_string(),
            ));
        }
        if self.analysis_needs_ready.load(Ordering::Acquire) {
            self.synchronize_after_analysis().await?;
        }
        let descriptor = self
            .option_descriptors
            .iter()
            .find(|descriptor| descriptor.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| EngineError::InvalidConfig(format!("unknown engine option: {name}")))?;
        if descriptor.option_type != EngineOptionType::Button {
            return Err(EngineError::InvalidConfig(format!(
                "engine option is not a button: {name}"
            )));
        }
        let protocol = self.protocol.as_ref().ok_or(EngineError::NotRunning)?;
        let process = self.process.as_mut().ok_or(EngineError::NotRunning)?;
        process
            .send(&protocol.format_button_option(&descriptor.name))
            .await
    }

    /// 立即出招：中断当前思考并等待输出 bestmove
    pub async fn move_now(&mut self) -> Result<(), EngineError> {
        self.stop_current_analysis().await
    }

    /// 停止思考并优雅退出引擎子进程
    pub async fn stop(&mut self) {
        let mut process = self.process.take();
        if let Some(process_ref) = process.as_mut() {
            if let Some(protocol) = self.protocol.as_ref() {
                let _ = process_ref.send(protocol.format_quit()).await;
            }
            process_ref.stop(None).await;
            process_ref.set_analysis_session_id(None);
        }
        drop(process);

        // 进程读取任务已经排空管道，通知原始日志收集器排空无界队列并等待明确边界
        if let Some(stop_tx) = self.raw_stop_tx.take() {
            let _ = stop_tx.send(());
        }

        if let Some(handle) = self.parse_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        if let Some(handle) = self.raw_handle.take() {
            let _ = handle.await;
        }

        self.is_analyzing.store(false, Ordering::Release);
        self.analysis_needs_ready.store(false, Ordering::Release);
        if let Ok(mut guard) = self.analysis_session_id.lock() {
            *guard = None;
        }
        self.protocol = None;
        self.capabilities = None;
        self.option_descriptors.clear();
        flush_log_writer(self.raw_log_writer.as_ref());
        self.raw_log_writer = None;
        if let Some(path) = self.raw_log_path.take() {
            let _ = std::fs::remove_file(path);
        }
        if let Ok(mut guard) = self.board_state.lock() {
            *guard = None;
        }
    }
}

fn read_protocol_log(path: Option<&std::path::Path>) -> Vec<EngineRawLine> {
    let Some(path) = path else {
        return Vec::new();
    };
    let Ok(file) = File::open(path) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if let Ok(parsed) = serde_json::from_str::<EngineRawLine>(&line) {
            lines.push(parsed);
        }
    }
    lines.sort_by_key(|line| line.sequence);
    lines
}

fn flush_log_writer(writer: Option<&Arc<Mutex<BufWriter<File>>>>) {
    if let Some(writer) = writer
        && let Ok(mut writer) = writer.lock()
    {
        let _ = writer.flush();
    }
}

async fn wait_for_protocol_ack(
    receiver: oneshot::Receiver<()>,
    stage: &str,
    timeout: Duration,
) -> Result<(), EngineError> {
    tokio::time::timeout(timeout, receiver)
        .await
        .map_err(|_| EngineError::ProtocolTimeout {
            stage: stage.to_string(),
        })?
        .map_err(|_| EngineError::StreamClosed)
}

impl Default for EngineSession {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct ResolvedConstraint {
    status: ConstraintApplicationStatus,
    search_moves: Option<Vec<String>>,
    ban_moves: Option<Vec<String>>,
}

/// 从起始 FEN 重放完整 ICCS 历史，并校验请求携带的当前 FEN
fn replay_position(position: &EnginePositionContext) -> Result<BoardState, EngineError> {
    let mut board = if position.start_fen.trim().eq_ignore_ascii_case("startpos") {
        BoardState::initial()
    } else {
        BoardState::from_fen(&position.start_fen)
            .map_err(|error| EngineError::InvalidConfig(format!("invalid start FEN: {error}")))?
    };

    for iccs in &position.history {
        let mv = Move::from_iccs(iccs).map_err(|error| {
            EngineError::InvalidConfig(format!("invalid history move '{iccs}': {error}"))
        })?;
        if !MoveValidator::can_move(&board, mv) {
            return Err(EngineError::InvalidConfig(format!(
                "illegal history move '{iccs}'"
            )));
        }
        board = board.apply_move(mv).0;
    }

    let current = BoardState::from_fen(&position.current_fen)
        .map_err(|error| EngineError::InvalidConfig(format!("invalid current FEN: {error}")))?;
    if board != current {
        return Err(EngineError::InvalidConfig(
            "analysis position does not match start FEN plus history".to_string(),
        ));
    }
    Ok(board)
}

fn resolve_root_constraint(
    board: &BoardState,
    constraint: &RootMoveConstraint,
    capabilities: &EngineCapabilities,
) -> Result<ResolvedConstraint, EngineError> {
    let all_legal = MoveValidator::get_all_legal_moves(board)
        .into_iter()
        .map(|mv| mv.to_iccs())
        .collect::<Vec<_>>();
    let legal_by_lower = all_legal
        .iter()
        .map(|iccs| (iccs.to_ascii_lowercase(), iccs.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let requested = [
        &constraint.candidates,
        &constraint.banned,
        &constraint.temporary_excluded,
    ]
    .into_iter()
    .any(|moves| moves.iter().any(|mv| !mv.trim().is_empty()));
    if !requested {
        return Ok(ResolvedConstraint {
            status: ConstraintApplicationStatus::NotApplied,
            search_moves: None,
            ban_moves: None,
        });
    }

    let normalize = |moves: &[String]| -> Result<Vec<String>, EngineError> {
        let mut normalized = Vec::new();
        for move_text in moves.iter().map(String::as_str).map(str::trim) {
            if move_text.is_empty() {
                continue;
            }
            let key = move_text.to_ascii_lowercase();
            let canonical = legal_by_lower.get(&key).ok_or_else(|| {
                EngineError::InvalidConfig(format!(
                    "root constraint move is not legal: {move_text}"
                ))
            })?;
            if !normalized.iter().any(|item| item == canonical) {
                normalized.push(canonical.clone());
            }
        }
        Ok(normalized)
    };

    let candidates = normalize(&constraint.candidates)?;
    let banned = normalize(&constraint.banned)?;
    let temporary_excluded = normalize(&constraint.temporary_excluded)?;
    let mut allowed = if candidates.is_empty() {
        all_legal.clone()
    } else {
        candidates
    };
    allowed.retain(|iccs| !banned.iter().any(|item| item == iccs));
    allowed.retain(|iccs| !temporary_excluded.iter().any(|item| item == iccs));
    if allowed.is_empty() {
        return Err(EngineError::NoSearchMoves);
    }

    if capabilities.searchmoves {
        return Ok(ResolvedConstraint {
            status: ConstraintApplicationStatus::Applied,
            search_moves: Some(allowed),
            ban_moves: None,
        });
    }
    if capabilities.banmoves {
        let ban_moves = all_legal
            .into_iter()
            .filter(|iccs| !allowed.iter().any(|item| item == iccs))
            .collect();
        return Ok(ResolvedConstraint {
            status: ConstraintApplicationStatus::Applied,
            search_moves: None,
            ban_moves: Some(ban_moves),
        });
    }

    Ok(ResolvedConstraint {
        status: ConstraintApplicationStatus::Unsupported,
        search_moves: None,
        ban_moves: None,
    })
}

fn validate_option_overrides<'a>(
    options: &'a std::collections::HashMap<String, String>,
    descriptors: &[crate::engine::models::EngineOptionDescriptor],
) -> Result<Vec<(&'a str, &'a str)>, EngineError> {
    let mut validated = Vec::with_capacity(options.len());
    for (name, value) in options {
        let Some(descriptor) = descriptors
            .iter()
            .find(|descriptor| descriptor.name.eq_ignore_ascii_case(name))
        else {
            // Threads/Hash 是常见的 Profile 字段，但并非所有 UCI/UCCI
            // 引擎都会声明这两个选项，它们只是尽力设置的默认值
            // 其它未知覆盖值仍视为配置错误，避免拼写错误悄悄改变引擎行为
            if name.eq_ignore_ascii_case("Threads") || name.eq_ignore_ascii_case("Hash") {
                continue;
            }
            return Err(EngineError::InvalidConfig(format!(
                "unknown engine option: {name}"
            )));
        };
        let valid = match descriptor.option_type {
            crate::engine::models::EngineOptionType::Check => {
                value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false")
            }
            crate::engine::models::EngineOptionType::Spin => {
                value.parse::<i32>().ok().is_some_and(|number| {
                    descriptor.min.is_none_or(|min| number >= min)
                        && descriptor.max.is_none_or(|max| number <= max)
                })
            }
            crate::engine::models::EngineOptionType::Combo => {
                descriptor.vars.iter().any(|item| item == value)
            }
            crate::engine::models::EngineOptionType::String
            | crate::engine::models::EngineOptionType::File => true,
            crate::engine::models::EngineOptionType::Button => false,
        };
        if !valid {
            return Err(EngineError::InvalidConfig(format!(
                "invalid value for engine option '{name}'"
            )));
        }
        validated.push((name.as_str(), value.as_str()));
    }
    Ok(validated)
}

/// 从全部合法走法中排除指定走法，返回其 ICCS 序列（大小写不敏感匹配）
pub fn compute_search_moves(all_legal: &[Move], excluded: &[String]) -> Vec<String> {
    let excluded_lower: HashSet<String> = excluded
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    all_legal
        .iter()
        .map(|mv| mv.to_iccs())
        .filter(|iccs| !excluded_lower.contains(&iccs.to_lowercase()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::models::RootMoveConstraint;

    #[tokio::test]
    async fn closed_protocol_ack_is_reported() {
        let (sender, receiver) = oneshot::channel();
        drop(sender);

        assert!(matches!(
            wait_for_protocol_ack(receiver, "handshake", PROTOCOL_TIMEOUT).await,
            Err(EngineError::StreamClosed)
        ));
    }

    #[tokio::test]
    async fn protocol_ack_timeout_is_reported() {
        let (_sender, receiver) = oneshot::channel();

        assert!(matches!(
            wait_for_protocol_ack(receiver, "ready", Duration::ZERO).await,
            Err(EngineError::ProtocolTimeout { stage }) if stage == "ready"
        ));
    }

    #[tokio::test]
    async fn unsupported_protocol_is_rejected_before_process_start() {
        let mut session = EngineSession::new();
        let config = EngineConfig::new("/does/not/exist", "xboard");

        assert!(matches!(
            session.start(&config).await,
            Err(EngineError::InvalidConfig(message))
                if message.contains("unsupported engine protocol")
        ));
    }

    #[test]
    fn replays_full_history_and_rejects_stale_current_fen() {
        let start = BoardState::initial();
        let mv = Move::from_iccs("h2e2").unwrap();
        let current = start.apply_move(mv).0;
        let position = EnginePositionContext {
            start_fen: start.to_fen(),
            history: vec![mv.to_iccs()],
            current_fen: current.to_fen(),
        };

        assert_eq!(replay_position(&position).unwrap(), current);
        let mut stale = position;
        stale.current_fen = start.to_fen();
        assert!(matches!(
            replay_position(&stale),
            Err(EngineError::InvalidConfig(message)) if message.contains("does not match")
        ));
    }

    #[test]
    fn root_constraints_use_searchmoves_and_report_unsupported() {
        let board = BoardState::initial();
        let first = MoveValidator::get_all_legal_moves(&board)[0].to_iccs();
        let constraint = RootMoveConstraint {
            candidates: Vec::new(),
            banned: vec![first.clone()],
            temporary_excluded: Vec::new(),
        };
        let search_capabilities = EngineCapabilities {
            protocol: "uci".to_string(),
            searchmoves: true,
            banmoves: false,
            ponder: false,
            multi_pv: false,
        };
        let resolved = resolve_root_constraint(&board, &constraint, &search_capabilities).unwrap();
        assert_eq!(resolved.status, ConstraintApplicationStatus::Applied);
        assert!(!resolved.search_moves.unwrap().contains(&first));

        let unsupported = EngineCapabilities {
            searchmoves: false,
            ..search_capabilities
        };
        let resolved = resolve_root_constraint(&board, &constraint, &unsupported).unwrap();
        assert_eq!(resolved.status, ConstraintApplicationStatus::Unsupported);
    }

    #[test]
    fn validates_string_engine_option_overrides_against_handshake_descriptors() {
        let descriptors = vec![crate::engine::models::EngineOptionDescriptor {
            name: "Hash".to_string(),
            option_type: crate::engine::models::EngineOptionType::Spin,
            default: None,
            min: Some(1),
            max: Some(1024),
            step: None,
            vars: Vec::new(),
        }];
        let mut valid = std::collections::HashMap::new();
        valid.insert("Hash".to_string(), "256".to_string());
        assert_eq!(
            validate_option_overrides(&valid, &descriptors)
                .unwrap()
                .len(),
            1
        );

        valid.insert("Hash".to_string(), "2048".to_string());
        assert!(matches!(
            validate_option_overrides(&valid, &descriptors),
            Err(EngineError::InvalidConfig(message)) if message.contains("invalid value")
        ));
    }

    #[test]
    fn ignores_undeclared_common_profile_options_but_rejects_other_unknown_options() {
        let mut options = std::collections::HashMap::new();
        options.insert("Threads".to_string(), "8".to_string());
        options.insert("Hash".to_string(), "256".to_string());
        assert!(validate_option_overrides(&options, &[]).unwrap().is_empty());

        options.insert("TypoOption".to_string(), "1".to_string());
        assert!(matches!(
            validate_option_overrides(&options, &[]),
            Err(EngineError::InvalidConfig(message)) if message.contains("TypoOption")
        ));
    }
}
