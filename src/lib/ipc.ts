// 类型安全 IPC 桥。命令来自 tauri-specta 生成的 `@/bindings`；带结果形状的命令
// 被包装为 `{ status, data | error }` 联合类型，`unwrap` 将其规范化为出错时抛
// 异常的 Promise。后端事件（`think://` / `bestmove://`）仍通过 @tauri-apps/api/event 订阅

import { commands } from "@/bindings";
import {
  IPC_EVENTS,
  type AnalysisEvent,
  type DownloadProgressPayload,
  type EngineRawLine,
} from "@/lib/events";
import { listen as listenRaw, type UnlistenFn } from "@tauri-apps/api/event";

export { commands };

export type Result<T> = { status: "ok"; data: T } | { status: "error"; error: string };

export async function unwrap<T>(res: Result<T>): Promise<T> {
  if (res.status === "error") throw new Error(res.error);
  return res.data;
}

function listen<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return listenRaw<T>(event, (e) => cb(e.payload));
}

export function listenThink(cb: (payload: AnalysisEvent) => void): Promise<UnlistenFn> {
  return listen(IPC_EVENTS.think, cb);
}

export function listenBestMove(cb: (payload: AnalysisEvent) => void): Promise<UnlistenFn> {
  return listen(IPC_EVENTS.bestMove, cb);
}

export function listenEngineStopped(cb: () => void): Promise<UnlistenFn> {
  return listen(IPC_EVENTS.engineStopped, cb);
}

export function listenDownloadProgress(
  cb: (payload: DownloadProgressPayload) => void
): Promise<UnlistenFn> {
  return listen(IPC_EVENTS.downloadProgress, cb);
}

export function listenEngineProtocol(cb: (payload: EngineRawLine) => void): Promise<UnlistenFn> {
  return listen(IPC_EVENTS.protocol, cb);
}

export type { UnlistenFn };
