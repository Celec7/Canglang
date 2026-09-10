// 在 Windows 发布构建中阻止额外的控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    canglang_app::run()
}
