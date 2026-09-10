import { getCurrentWindow } from "@tauri-apps/api/window";

function appWindow() {
  return getCurrentWindow();
}

export function startWindowDragging(): Promise<void> {
  return appWindow().startDragging();
}

export function minimizeWindow(): Promise<void> {
  return appWindow().minimize();
}

export function toggleWindowMaximize(): Promise<void> {
  return appWindow().toggleMaximize();
}

export function closeWindow(): Promise<void> {
  return appWindow().close();
}

export function isWindowMaximized(): Promise<boolean> {
  return appWindow().isMaximized();
}
