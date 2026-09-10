//! Tauri IPC 命令层
//!
//! 汇聚来自 `core`、`engine`、`manual` 模块的命令 handler。由此导出的注册表
//! 通过 `tauri_specta::collect_commands!` 传入 `crate::run()`

pub mod book;
pub mod config;
pub mod core;
pub mod engine;
pub mod manual;
