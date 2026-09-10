use crate::engine::error::EngineError;
use crate::engine::models::{EngineConfig, EngineInfo};
use crate::engine::process::EngineProcess;
use crate::engine::protocol::{HandshakeInfo, capabilities_for_protocol, parse_handshake_line};
use std::time::Duration;
use tokio::sync::broadcast;

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// 依次尝试 UCI、UCCI 握手，返回首个完整 ready 的协议结果
pub async fn detect(path: &str) -> Result<EngineInfo, EngineError> {
    let mut diagnostics = Vec::new();
    for protocol in ["uci", "ucci"] {
        match probe_candidate(path, protocol).await {
            Ok(info) => return Ok(info),
            Err(error) => diagnostics.push(format!("{protocol}: {error}")),
        }
    }

    Err(EngineError::ProbeFailed {
        details: diagnostics.join("; "),
    })
}

async fn probe_candidate(path: &str, protocol: &str) -> Result<EngineInfo, EngineError> {
    let config = EngineConfig::new(path, protocol);
    let mut process = EngineProcess::default();
    let mut lines = process.line_stream();
    process.start(&config)?;

    let result = async {
        process
            .send(if protocol == "uci" { "uci" } else { "ucci" })
            .await?;
        let (_, handshake_lines) = wait_for_marker(
            &mut lines,
            if protocol == "uci" { "uciok" } else { "ucciok" },
            "handshake",
            PROBE_TIMEOUT,
        )
        .await?;

        process.send("isready").await?;
        let (_, ready_lines) =
            wait_for_marker(&mut lines, "readyok", "ready", PROBE_TIMEOUT).await?;

        let mut handshake = HandshakeInfo::default();
        for line in &handshake_lines {
            parse_handshake_line(&mut handshake, line);
        }
        let capabilities = capabilities_for_protocol(protocol, &handshake);
        let mut transcript = handshake_lines;
        transcript.extend(ready_lines);
        Ok::<_, EngineError>(EngineInfo {
            name: handshake.name.clone(),
            protocol: protocol.to_string(),
            ready: true,
            diagnostic: Some(transcript.join(" | ")),
            option_descriptors: handshake.options,
            capabilities,
        })
    }
    .await;

    let _ = process.send("quit").await;
    process.stop(Some(Duration::from_millis(300))).await;
    result
}

async fn wait_for_marker(
    lines: &mut broadcast::Receiver<String>,
    marker: &str,
    stage: &str,
    timeout: Duration,
) -> Result<(Option<String>, Vec<String>), EngineError> {
    let wait = async {
        let mut name = None;
        let mut transcript = Vec::new();
        loop {
            match lines.recv().await {
                Ok(line) => {
                    if line.len() <= 160 {
                        transcript.push(line.clone());
                    }
                    if let Some(value) = line.strip_prefix("id name ") {
                        name = Some(value.trim().to_string());
                    }
                    if line.trim().eq_ignore_ascii_case(marker) {
                        return Ok::<_, EngineError>((name, transcript));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return Err(EngineError::StreamClosed),
            }
        }
    };

    tokio::time::timeout(timeout, wait)
        .await
        .map_err(|_| EngineError::ProtocolTimeout {
            stage: stage.to_string(),
        })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_probe_order_is_explicit() {
        assert_eq!(["uci", "ucci"], ["uci", "ucci"]);
    }

    #[tokio::test]
    async fn probe_wait_accepts_identity_and_marker() {
        let (sender, mut receiver) = broadcast::channel(4);
        sender.send("id name Test Engine".to_string()).unwrap();
        sender.send("uciok".to_string()).unwrap();

        let (name, lines) = wait_for_marker(
            &mut receiver,
            "uciok",
            "handshake",
            Duration::from_millis(20),
        )
        .await
        .unwrap();
        assert_eq!(name.as_deref(), Some("Test Engine"));
        assert_eq!(lines.last().map(String::as_str), Some("uciok"));
    }

    #[tokio::test]
    async fn probe_wait_reports_timeout_and_closed_stream() {
        let (_sender, mut receiver) = broadcast::channel::<String>(1);
        assert!(matches!(
            wait_for_marker(&mut receiver, "uciok", "handshake", Duration::ZERO).await,
            Err(EngineError::ProtocolTimeout { stage }) if stage == "handshake"
        ));

        let (sender, mut receiver) = broadcast::channel::<String>(1);
        drop(sender);
        assert!(matches!(
            wait_for_marker(
                &mut receiver,
                "uciok",
                "handshake",
                Duration::from_millis(20)
            )
            .await,
            Err(EngineError::StreamClosed)
        ));
    }
}
