use crate::engine::diagnostics::{EngineOutputStream, EngineRawLine};
use crate::engine::error::EngineError;
use crate::engine::models::EngineConfig;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{broadcast, mpsc};

fn timestamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// 负责外部引擎子进程的生命周期管理，提供 stdin/stdout 异步非阻塞管道读写与行流广播
pub struct EngineProcess {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    tx: broadcast::Sender<String>,
    raw_tx: mpsc::UnboundedSender<EngineRawLine>,
    raw_rx: Option<mpsc::UnboundedReceiver<EngineRawLine>>,
    sequence: Arc<AtomicU32>,
    run_id: String,
    analysis_session_id: Arc<Mutex<Option<String>>>,
    exit_tx: broadcast::Sender<()>,
    running: Arc<AtomicBool>,
    read_handle: Option<tokio::task::JoinHandle<()>>,
    stderr_handle: Option<tokio::task::JoinHandle<()>>,
}

impl EngineProcess {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(1024);
        let (raw_tx, raw_rx) = mpsc::unbounded_channel();
        let (exit_tx, _exit_rx) = broadcast::channel(8);
        Self {
            child: None,
            stdin: None,
            tx,
            raw_tx,
            raw_rx: Some(raw_rx),
            sequence: Arc::new(AtomicU32::new(0)),
            run_id: String::new(),
            analysis_session_id: Arc::new(Mutex::new(None)),
            exit_tx,
            running: Arc::new(AtomicBool::new(false)),
            read_handle: None,
            stderr_handle: None,
        }
    }

    /// 子进程是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire) && self.child.as_ref().and_then(|c| c.id()).is_some()
    }

    /// 启动引擎子进程并建立异步输入输出管道与 stdout 行流广播
    pub fn start(&mut self, config: &EngineConfig) -> Result<(), EngineError> {
        if config.path.trim().is_empty() {
            return Err(EngineError::InvalidConfig(
                "engine path cannot be empty".to_string(),
            ));
        }

        if !Path::new(&config.path).exists() {
            return Err(EngineError::ProcessStart {
                path: config.path.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "engine executable not found",
                ),
            });
        }

        if self.is_running() {
            return Err(EngineError::InvalidConfig(
                "engine process is already running".to_string(),
            ));
        }

        let working_dir = Path::new(&config.path)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let mut cmd = Command::new(&config.path);
        cmd.current_dir(&working_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| EngineError::ProcessStart {
            path: config.path.clone(),
            source: e,
        })?;
        self.run_id = format!(
            "run-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        self.sequence.store(0, Ordering::Release);
        if let Ok(mut guard) = self.analysis_session_id.lock() {
            *guard = None;
        }
        let sequence = self.sequence.fetch_add(1, Ordering::AcqRel) + 1;
        let _ = self.raw_tx.send(EngineRawLine {
            run_id: self.run_id.clone(),
            analysis_session_id: None,
            timestamp_ms: timestamp_ms(),
            sequence,
            stream: EngineOutputStream::Lifecycle,
            raw: "process started\n".to_string(),
        });

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| EngineError::ProcessStart {
                path: config.path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdin not piped"),
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EngineError::ProcessStart {
                path: config.path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdout not piped"),
            })?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| EngineError::ProcessStart {
                path: config.path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stderr not piped"),
            })?;

        let tx = self.tx.clone();
        let raw_tx = self.raw_tx.clone();
        let exit_tx = self.exit_tx.clone();
        let running = self.running.clone();
        let sequence = self.sequence.clone();
        let run_id = self.run_id.clone();
        let analysis_session_id = self.analysis_session_id.clone();
        let stderr_run_id = run_id.clone();
        let stderr_analysis_session_id = analysis_session_id.clone();
        let stderr_raw_tx = raw_tx.clone();
        let stderr_sequence = sequence.clone();
        let stderr_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut buffer = Vec::new();
            loop {
                buffer.clear();
                match reader.read_until(b'\n', &mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let raw = String::from_utf8_lossy(&buffer).into_owned();
                        let sequence = stderr_sequence.fetch_add(1, Ordering::AcqRel) + 1;
                        let _ = stderr_raw_tx.send(EngineRawLine {
                            run_id: stderr_run_id.clone(),
                            analysis_session_id: stderr_analysis_session_id
                                .lock()
                                .ok()
                                .and_then(|guard| guard.clone()),
                            timestamp_ms: timestamp_ms(),
                            sequence,
                            stream: EngineOutputStream::Stderr,
                            raw,
                        });
                    }
                }
            }
        });
        let read_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buffer = Vec::new();
            loop {
                buffer.clear();
                match reader.read_until(b'\n', &mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let raw = String::from_utf8_lossy(&buffer).into_owned();
                        let sequence = sequence.fetch_add(1, Ordering::AcqRel) + 1;
                        let _ = raw_tx.send(EngineRawLine {
                            run_id: run_id.clone(),
                            analysis_session_id: analysis_session_id
                                .lock()
                                .ok()
                                .and_then(|guard| guard.clone()),
                            timestamp_ms: timestamp_ms(),
                            sequence,
                            stream: EngineOutputStream::Stdout,
                            raw: raw.clone(),
                        });
                        let trimmed = raw.trim();
                        if !trimmed.is_empty() {
                            let _ = tx.send(trimmed.to_string());
                        }
                    }
                }
            }
            running.store(false, Ordering::Release);
            let sequence = sequence.fetch_add(1, Ordering::AcqRel) + 1;
            let _ = raw_tx.send(EngineRawLine {
                run_id: run_id.clone(),
                analysis_session_id: analysis_session_id
                    .lock()
                    .ok()
                    .and_then(|guard| guard.clone()),
                timestamp_ms: timestamp_ms(),
                sequence,
                stream: EngineOutputStream::Lifecycle,
                raw: "process exited\n".to_string(),
            });
            let _ = exit_tx.send(());
        });

        self.child = Some(child);
        self.stdin = Some(stdin);
        self.running.store(true, Ordering::Release);
        self.read_handle = Some(read_handle);
        self.stderr_handle = Some(stderr_handle);
        Ok(())
    }

    /// 向引擎标准输入异步发送一行指令并立即 Flush
    pub async fn send(&mut self, command: &str) -> Result<(), EngineError> {
        let analysis_session_id = self
            .analysis_session_id
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        self.send_with_session(command, analysis_session_id.as_deref())
            .await
    }

    /// 发送一条明确归属某个分析会话的协议指令，而不改变后续 stdout/stderr
    /// 的会话标签。用于在 `position` 阶段隔离停止搜索后可能迟到的输出
    pub async fn send_with_session(
        &mut self,
        command: &str,
        analysis_session_id: Option<&str>,
    ) -> Result<(), EngineError> {
        if !self.is_running() {
            return Err(EngineError::NotRunning);
        }

        let stdin = self.stdin.as_mut().ok_or(EngineError::NotRunning)?;
        stdin
            .write_all(command.as_bytes())
            .await
            .map_err(|e| EngineError::CommandSend { source: e })?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| EngineError::CommandSend { source: e })?;
        stdin
            .flush()
            .await
            .map_err(|e| EngineError::CommandSend { source: e })?;
        let sequence = self.sequence.fetch_add(1, Ordering::AcqRel) + 1;
        let _ = self.raw_tx.send(EngineRawLine {
            run_id: self.run_id.clone(),
            analysis_session_id: analysis_session_id.map(str::to_string),
            timestamp_ms: timestamp_ms(),
            sequence,
            stream: EngineOutputStream::Stdin,
            raw: format!("{command}\n"),
        });
        Ok(())
    }

    /// 将后续原始协议流归入指定分析会话；`None` 表示握手/生命周期前置流
    pub fn set_analysis_session_id(&self, analysis_session_id: Option<String>) {
        if let Ok(mut guard) = self.analysis_session_id.lock() {
            *guard = analysis_session_id;
        }
    }

    /// 停止引擎子进程（默认超时 1500 毫秒），若超时未优雅退出则强制 Kill
    pub async fn stop(&mut self, timeout: Option<Duration>) {
        self.running.store(false, Ordering::Release);
        let mut kill_requested = false;
        if let Some(child) = self.child.as_mut() {
            let effective = timeout.unwrap_or(Duration::from_millis(1500));
            if child.id().is_some() {
                match tokio::time::timeout(effective, child.wait()).await {
                    Ok(_) => {}
                    Err(_) => {
                        kill_requested = true;
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                    }
                }
            }
        }

        if kill_requested {
            self.emit_lifecycle("process kill requested\n");
        }

        if let Some(handle) = self.read_handle.take() {
            wait_or_abort(handle).await;
        }
        if let Some(handle) = self.stderr_handle.take() {
            wait_or_abort(handle).await;
        }

        self.stdin = None;
        self.child = None;
        self.emit_lifecycle("process stopped\n");
    }

    /// 标准输出行流（多播接收端）
    pub fn line_stream(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    /// 子进程 stdout 关闭时发出一次通知
    pub fn exit_stream(&self) -> broadcast::Receiver<()> {
        self.exit_tx.subscribe()
    }

    /// 原始 stdout/stderr 行流；与解析用 line_stream 相互独立
    pub fn raw_line_stream(&mut self) -> mpsc::UnboundedReceiver<EngineRawLine> {
        self.raw_rx
            .take()
            .expect("raw line receiver may only be taken once")
    }

    /// 当前已分配给原始协议日志的单调序号，用于等待日志收集器追平发送边界
    pub fn current_sequence(&self) -> u32 {
        self.sequence.load(Ordering::Acquire)
    }

    fn emit_lifecycle(&self, raw: &str) {
        let sequence = self.sequence.fetch_add(1, Ordering::AcqRel) + 1;
        let _ = self.raw_tx.send(EngineRawLine {
            run_id: self.run_id.clone(),
            analysis_session_id: self
                .analysis_session_id
                .lock()
                .ok()
                .and_then(|guard| guard.clone()),
            timestamp_ms: timestamp_ms(),
            sequence,
            stream: EngineOutputStream::Lifecycle,
            raw: raw.to_string(),
        });
    }
}

async fn wait_or_abort(handle: tokio::task::JoinHandle<()>) {
    let mut handle = handle;
    if tokio::time::timeout(Duration::from_millis(150), &mut handle)
        .await
        .is_err()
    {
        handle.abort();
        let _ = handle.await;
    }
}

impl Default for EngineProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EngineProcess {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        // `kill_on_drop(true)` 已确保子进程随 Child 释放而终结；此处仅作兜底
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
        if let Some(handle) = self.read_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.stderr_handle.take() {
            handle.abort();
        }
    }
}
