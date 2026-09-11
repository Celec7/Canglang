---
title: Canglang 开发指南
doc_type: development
status: current
authority: normative
audience: maintainers
canonical: true
summary: 说明 Canglang 的环境准备、开发启动、构建和日常验证命令。
---

# Canglang 开发指南

## 环境准备

项目包含 pnpm 管理的 Vue 前端与 Cargo workspace 管理的 Tauri Rust 宿主。准备 Node.js、pnpm、Rust toolchain 与 Tauri 系统依赖后，在仓库根目录安装前端依赖：

```sh
pnpm install
```

根目录 `Cargo.toml` 将 `src-tauri` 作为 workspace member，Rust 命令也从仓库根目录执行。

## 日常命令

```sh
pnpm dev                         # 仅启动 Vite 前端开发服务器
pnpm test                        # 前端状态与展示行为回归
pnpm typecheck                   # vue-tsc 类型检查
pnpm build                       # 构建 dist 前端产物
pnpm exec tauri dev              # 启动 Tauri 开发应用
pnpm exec tauri build            # 构建桌面应用及安装包
pnpm package                     # 快捷全量构建前端及桌面安装包
pnpm release [patch|minor|0.2.0] # 自动同步版本号、生成 Changelog 并打 Tag
cargo fmt --check                # 检查 Rust 格式
cargo check                      # 检查 Rust 编译
cargo test                       # 运行 Rust 测试并检查 Specta 绑定
```

只验证桌面编译而不生成安装包时：

```sh
pnpm exec tauri build --debug --no-bundle
```

## 前端构建

Vite 根目录是仓库根目录，源码入口 `src/main.ts`，输出目录 `dist`。Tauri 的 `beforeBuildCommand` 先执行 `pnpm build`，`frontendDist` 指向 `../dist`。

修改 Tailwind、Vite、TypeScript 或 Tauri CLI 版本后，至少运行 `pnpm typecheck`、`pnpm build` 与一次 Tauri 编译。

## IPC 绑定

调试构建或 Rust 测试经 `specta_builder` 导出 [`src/bindings/generated.ts`](../src/bindings/generated.ts)。绑定属生成产物；修改源命令或模型后运行 `cargo test`，再检查生成文件与前端类型检查。

## 引擎开发

引擎分析需本地可执行引擎路径，并在 UI 选对应协议。协议测试不依赖真实引擎；涉及进程启动、事件转发或最佳着法的改动，再运行引擎相关集成测试或 Tauri smoke test。内置引擎可在「设置 → 引擎与对弈」一键下载装配，开发时无需手工准备；`engineDownloadBuiltin` 需要网络访问。

## 配置与便携模式

配置写入 `config.json`：可执行文件同级目录存在时按便携模式解析，否则使用系统用户配置目录。前端偏好先在 localStorage 备份，再防抖经 `configSave` 原子写入。排查配置问题时先用 `configGetLocation` 确认实际路径与便携标记，不要假定固定目录。

云库（chessdb.cn）与引擎下载都依赖网络；离线开发时把 `CloudBookMode` 设为 `local_only` 可避免查询等待。

## 版本发布与更新日志

版本号保持 `package.json`、`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 三处强一致，前端通过 Vite `__APP_VERSION__` 动态绑定。

发版流程支持双通道自动化：

- **本地命令发版**：运行 `pnpm release [patch|minor|major|<版本号>]`。脚本自动同步版本配置、更新 `Cargo.lock`、根据 Conventional Commits 生成 `CHANGELOG.md`、提交 Git 并创建 `v*` 标签。执行 `git push && git push origin <tag>` 即可触发远端自动化打包。使用 `--dry-run` 预览变更。
- **GitHub 网页一键发版**：在 GitHub Actions 的 `release` 工作流选择升级级别（或输入指定版本号）直接触发。工作流自动更新版本、打标签、提取 Release Notes、执行跨平台矩阵打包，并上传发布。
- **分发产物与便携版**：全平台对外分发产物统一命名为 `Canglang`；Linux 发布流水线除 `deb` 和 AppImage 外，还上传 `Canglang-v*-linux-x86_64` 原生单二进制，可直接运行以避开 AppImage 的运行开销，但依赖目标系统提供 GTK/WebKit 等运行库；Windows 发布流水线除标准安装包外，自动将可执行文件与 `.portable` 标记打包为 `Canglang-v*-windows-x64-portable.zip` 免安装便携版。

## 工作流

开始修改前运行 `git status --short`。完成后按变更范围运行最小相关测试，再运行前端类型检查与构建。涉及跨边界模型、生命周期或权限变化时，同步更新对应正式文档。
