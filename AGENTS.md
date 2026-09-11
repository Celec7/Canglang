# Canglang 开发指南

Canglang 是一个基于 Tauri v2 的中国象棋桌面应用。Vue 3、Vite、TypeScript 和 Pinia 组成前端，Rust 负责棋规、对局状态、引擎、棋谱和开局库。

修改代码前，先按变更范围阅读：

- [架构总览](docs/architecture.md)
- [模块边界](docs/module-boundaries.md)
- [IPC 契约](docs/ipc-contract.md)
- [象棋与揭棋领域](docs/chess-domain.md)
- [引擎系统](docs/engine.md)
- [棋谱与开局库](docs/manual-formats.md)
- [前端开发](docs/frontend.md)
- [代码风格](docs/code-style.md)
- [开发指南](docs/development.md)
- [测试策略](docs/testing.md)
- [术语表](docs/glossary.md)
- [文档维护规则](docs/AGENTS.md)

## 长期约束

- Rust `core` 是棋盘、棋子、走法、合法性、将军和胜负判定的事实来源；前端不得复制棋规。
- `GameState` 负责对局状态和历史，`EngineState` 负责外部引擎会话，`BookState` 负责开局库，`ConfigState` 负责持久化配置；四者不得互相承载无关状态。
- `ipc/` 只负责 Tauri 命令、受管状态和数据转换；领域逻辑应放在对应的 Rust 模块中。
- 前端通过 `src/lib/ipc.ts` 和 `src/bindings/generated.ts` 访问 IPC；不得在组件中直接手写 `invoke`，不得手工修改生成绑定。
- `src/bindings/generated.ts` 由 `tauri-specta` 从带 `#[specta::specta]` 的 Rust 命令和类型生成；命令或公开数据结构变化时必须重新生成并检查消费者。
- `ThinkData` 通过 `think://` 事件发送，最佳着法通过 `bestmove://` 事件发送；事件 payload 必须与 Rust 模型保持一致。
- ICCS 是两种棋局的走法跨边界表示；普通象棋局面用 FEN，揭棋局面用不含隐藏身份的公开结构化视图，具体契约见[象棋与揭棋领域](docs/chess-domain.md)。
- 引擎子进程必须通过 `EngineProcess`、`EngineSession` 和协议抽象管理；停止、重启和进程退出必须清理对应任务。
- PGN、XQF 和 `.bh` 文件由 Rust 服务读取和解析；前端只消费结构化结果，不直接解析文件格式。
- 配置只经 `ConfigService` 读写，保持原子写入；外部网络访问只允许出现在云库查询、内置引擎下载和系统浏览器打开链接。
- Tauri capability 只授予应用实际使用的权限；修改 `src-tauri/capabilities/default.json` 时同步说明权限用途。
- 源代码注释和 doc comment 使用简洁中文；句末不保留不必要的句号，棋谱、中文记谱、用户可见文案和其它领域文字面量保持原文。

## 仓库布局

```text
src/                         Vue 前端、Pinia stores、IPC 适配和 UI 组件
src/bindings/generated.ts    tauri-specta 生成的 TypeScript 命令和类型
src-tauri/src/core/           棋盘、棋子、走法、棋规、记谱和 hash
src-tauri/src/engine/         UCI/UCCI、引擎进程、会话、安装器、配置服务和开局库
src-tauri/src/manual/         PGN/XQF 解析、导出、编码和棋谱模型
src-tauri/src/ipc/            Tauri 命令、应用状态和事件转发
src-tauri/tests/              Rust 集成测试
src-tauri/tauri.conf.json     Tauri 构建和窗口配置
```

## 常用命令

```sh
pnpm install
pnpm dev
pnpm typecheck
pnpm build
pnpm exec tauri dev
pnpm exec tauri build
pnpm package
pnpm release [patch|minor|major]
cargo fmt --check
cargo check
cargo test
```

前端依赖使用 pnpm 管理。生产构建由 Tauri 的 `beforeBuildCommand` 自动调用 `pnpm build`；验证桌面集成时优先使用 `pnpm exec tauri build --debug --no-bundle`。

## 变更要求

- 先检查 `git status --short`，保留与当前任务无关的用户改动。
- 修改 Rust 公开命令或 `specta::Type` 类型后，运行 `cargo test` 以重新生成并检查 TypeScript 绑定。
- 修改领域规则、协议解析、棋谱格式或异步生命周期时，同时更新对应测试和正式文档。
- 新增前端状态时先判断它属于现有 store、composable 还是组件局部状态，避免建立第二套状态源。
- 使用现在时描述正式文档，不把临时计划、调试过程或提交历史写进 `docs/`。
- 提交信息使用 Conventional Commits，例如 `feat: add manual loading`、`fix: stop engine task`、`docs: document IPC contract`。

代码风格的完整约定见 [代码风格](docs/code-style.md)。Rust 格式以 `rustfmt` 为准，前端保持现有 TypeScript/Vue 格式；除非单独确认，不为格式规则新增工具依赖。
