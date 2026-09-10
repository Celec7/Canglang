/**
 * Rust 事件载荷在前端的镜像类型
 *
 * 权威定义仍位于 `engine/models.rs` 和 `engine/installer.rs`，修改字段时需保持同步
 */
export interface ThinkData {
  depth: number;
  score: number;
  mate_in: number | null;
  multi_pv: number;
  nps: number;
  time_ms: number;
  pv: string[];
  pv_chinese: string[];
}

export type AnalysisEvent =
  | {
      kind: "think";
      payload: { analysis_session_id: string; data: ThinkData };
    }
  | {
      kind: "best_move";
      payload: { analysis_session_id: string; move_iccs: string };
    };

export interface DownloadProgressPayload {
  profileId: string;
  stage: string;
  downloadedBytes: number;
  totalBytes: number | null;
  percent: number;
  speedBytesPerSec: number;
  message: string | null;
}

export interface EngineRawLine {
  run_id: string;
  analysis_session_id: string | null;
  timestamp_ms: number;
  sequence: number;
  stream: "stdin" | "stdout" | "stderr" | "lifecycle";
  raw: string;
}

export const IPC_EVENTS = {
  think: "think://",
  bestMove: "bestmove://",
  engineStopped: "engine://stopped",
  downloadProgress: "engine://download-progress",
  protocol: "engine://protocol",
} as const;
