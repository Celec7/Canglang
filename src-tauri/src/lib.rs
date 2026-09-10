pub mod core;
pub mod engine;
pub mod ipc;
pub mod manual;
pub mod services;

use crate::core::game::GameState;
use crate::ipc::book::BookState;
use crate::ipc::config::ConfigState;
use crate::ipc::engine::EngineState;
use specta::Type;
use std::sync::Mutex;
use tauri::Manager;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Core error: {0}")]
    Core(#[from] core::CoreError),
    #[error("Engine error: {0}")]
    Engine(#[from] engine::EngineError),
    #[error("Manual error: {0}")]
    Manual(#[from] manual::ManualError),
}

// Tauri 命令错误会序列化进 IPC 错误载荷，这里统一以字符串形式暴露
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// tauri-specta 把命令错误在 TS 绑定中表示为普通的 `string`
impl Type for AppError {
    fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::Primitive::str)
    }
}

/// 收集全部 IPC 命令，构造一个 tauri-specta builder：
/// 既生成类型安全的 TypeScript 绑定，也提供由 specta 托管的 invoke handler
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        ipc::core::get_initial_board,
        ipc::core::parse_fen,
        ipc::core::validate_position,
        ipc::core::to_fen,
        ipc::core::make_move,
        ipc::core::get_legal_moves,
        ipc::core::get_candidate_moves,
        ipc::core::to_chinese_notation,
        ipc::core::is_in_check,
        ipc::core::preview_line,
        ipc::core::apply_move_line,
        ipc::core::new_game,
        ipc::core::set_rule_profile,
        ipc::core::game_result,
        ipc::core::undo_move,
        ipc::core::redo_move,
        ipc::core::jump_to,
        ipc::core::resign,
        ipc::core::ping,
        ipc::session::session_get,
        ipc::session::session_new,
        ipc::session::session_targets,
        ipc::session::session_move,
        ipc::session::session_undo,
        ipc::session::session_redo,
        ipc::session::session_jump,
        ipc::session::session_resign,
        ipc::session::session_offer_draw,
        ipc::session::session_respond_draw,
        ipc::session::session_cancel_draw,
        ipc::engine::engine_start,
        ipc::engine::engine_analyze,
        ipc::engine::engine_move_now,
        ipc::engine::engine_trigger_button_option,
        ipc::engine::engine_change_tactic,
        ipc::engine::engine_stop,
        ipc::engine::engine_status,
        ipc::engine::engine_protocol_log,
        ipc::engine::engine_export_protocol_log,
        ipc::engine::engine_pick_file,
        ipc::engine::engine_download_builtin,
        ipc::engine::engine_remove_builtin,
        ipc::manual::manual_load,
        ipc::manual::manual_pick_file,
        ipc::manual::manual_parse_text,
        ipc::manual::manual_save,
        ipc::manual::manual_save_xqf,
        ipc::manual::manual_export_pgn,
        ipc::book::book_load,
        ipc::book::book_unload,
        ipc::book::book_clear,
        ipc::book::book_loaded,
        ipc::book::book_query,
        ipc::book::book_set_cloud_enabled,
        ipc::book::book_set_cloud_mode,
        ipc::book::book_get_cloud_status,
        ipc::config::config_load,
        ipc::config::config_save,
        ipc::config::config_get_location,
    ])
}

/// 启动 Tauri v2 宿主
///
/// 通过 tauri-specta builder 注册全部 IPC 命令 handler（同时向前端暴露类型
/// 安全的命令绑定），并依据 `tauri.conf.json` 生成的应用上下文运行。受管的
/// 状态分别承载对局会话（[`GameState`]）、引擎会话（[`EngineState`]）、
/// 开局库服务（[`BookState`]）与全局配置服务（[`ConfigState`]）
pub fn run() {
    let builder = specta_builder();
    #[cfg(debug_assertions)]
    {
        builder
            .export(
                specta_typescript::Typescript::default(),
                "../src/bindings/generated.ts",
            )
            .expect("导出类型安全命令绑定失败");
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(GameState::default()))
        .manage(EngineState::default())
        .manage(BookState::default())
        .setup(|app| {
            let app_config_dir = app.path().app_config_dir().ok();
            app.manage(ConfigState::new(app_config_dir));
            Ok(())
        })
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("运行 Canglang Tauri 应用时出错");
}

#[cfg(test)]
mod tests {
    use specta_typescript::Typescript;

    #[test]
    fn generates_specta_bindings() {
        super::specta_builder()
            .export(Typescript::default(), "../src/bindings/generated.ts")
            .expect("导出类型安全命令绑定失败");
    }
}
