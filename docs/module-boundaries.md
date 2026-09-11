---
title: Canglang 模块边界
doc_type: module-boundaries
status: current
authority: normative
audience: maintainers
canonical: true
summary: 规定 Canglang 各模块的所有权、依赖方向和跨层边界。
---

# Canglang 模块边界

模块边界由能力所有权决定。调用方可依赖模块公开的模型与函数，但不得复制被调用方负责的规则、解析或生命周期。

| 模块 | 负责 | 可以依赖 | 不应负责 |
| --- | --- | --- | --- |
| `core` | 象棋棋盘、棋子、走法与几何，普通象棋和揭棋规则、记谱、hash、统一对局状态 | Rust 标准库、serde、内部 core 模块 | Tauri、Vue、文件 IO、外部进程 |
| `engine` | 引擎模型、协议、进程、会话、安装器、配置服务和开局库（本地 `.bh` 与云库） | `core`、tokio、rusqlite、reqwest、serde | Vue 组件、前端状态、Tauri 命令注册 |
| `manual` | PGN/XQF、揭棋 `.cjq`、编码、棋谱模型、重放校验和原子导出 | `core`、serde、文件 IO | UI 状态、引擎进程、Tauri 权限 |
| `ipc` | 命令、受管状态、序列化边界和事件转发 | `core`、`engine`、`manual`、Tauri | 重复实现棋规或文件格式 |
| `bindings` | 从 Rust 命令生成前端类型和调用函数 | Tauri API | 手工业务逻辑和手工维护的 API 类型 |
| `lib/ipc.ts` | 统一命令、错误解包和 typed 事件订阅入口 | generated bindings、`lib/events.ts`、Tauri event API | 领域校验和 UI 状态 |
| `lib/events.ts` | Rust `ThinkData`、最佳着法和下载进度事件的前端契约镜像 | Rust `specta::Type` 模型（权威） | 生成第二套领域模型或改变事件字段 |
| `stores` | 前端可见状态、请求协调和快照投影 | `lib/ipc.ts`、Vue、绑定类型 | 复制 Rust 棋局或协议状态机 |
| `composables` | 组件交互状态和展示辅助 | stores、前端纯函数、IPC 入口 | 成为后端事实源 |
| `components` | 展示、用户输入和组件组合 | stores、composables、UI components | 直接调用命令、订阅 Tauri 事件、访问 Rust 内部或实现棋规 |

`core` 内部按领域职责分层：`board` 负责普通象棋 FEN 与棋盘值语义，`rules::legality` 的 `GeometryPolicy` 提供两种棋局共享的移动几何，`rules::attacks`、`generation` 与 `legality` 负责普通象棋攻击、生成和完整合法性，`jieqi` 在共享几何上增加首步角色、真实身份、明暗状态、揭子和休闲终局规则。`rules::profile`、`repetition` 与 `adjudicator` 只属于普通象棋竞赛裁决。`game::GameState` 通过 `ActiveGame` 独占两种对局的状态转移，调用方只能经公开快照与版本化操作访问。

## 依赖方向

```text
Vue Components
  → Stores / Composables
  → lib/ipc.ts (commands + typed event gateways)
  → generated.ts
  → Tauri IPC
  → ipc
  → core / engine / manual
```

`core` 不反向依赖 `ipc`。`engine` 可用 `core` 的局面与走法类型，但引擎协议不应进入 `core`；`manual` 可用 `core` 解析与生成走法，但棋谱文件格式不应污染棋规模块。

## 跨边界规则

- 走法跨边界统一用 ICCS。普通象棋局面跨边界用 FEN；揭棋因未揭身份不能由 FEN 表达，只通过 `JieqiPositionViewV1` 公开结构传递。Rust 内部使用对应的 `BoardState`、`JieqiPosition` 与 `Move`。
- 公开 IPC 模型用 serde 与 Specta；前端只用生成类型，不手写同名接口。
- 普通服务错误在 `AppError` 汇总，以字符串错误 payload 暴露；版本化会话命令使用带 `SessionErrorCode` 的 `SessionError`。前端分别经 `unwrap` 与 `unwrapSession` 解包，不据错误文案判断业务状态。
- 需要解析输入的 IPC 命令不得把无效 FEN、ICCS、颜色或历史静默降级为空结果。
- 实时引擎信息走事件，不把高频 `ThinkData` 塞进命令返回值或全局快照。
- 文件路径只出现在文件服务与对应输入边界；文件内容解析在 Rust 完成。
- 揭棋私有身份只存在于 `core` 与 `.cjq` 私有编码路径，不进入 IPC 参数、回执、公开快照或前端 store。
- 配置只经 `ConfigService` 读写；前端不自行拼装配置文件路径，也不直接写磁盘。
- 外部网络访问只允许出现在 `engine/book`（云库）与 `engine/installer`（引擎下载）；`core`、`manual` 与前端组件保持无网络依赖。
- `BoardState::from_fen` 只保证结构完整、可表示；需要王宫与照面王等完整棋规时显式调用 `MoveValidator::validate_board`，不混用两层校验。
- `EngineState` 的生命周期锁先于引擎会话异步锁；协调换局时不跨文件 I/O 持有 `GameState` 锁，提交候选前后均以 token 防止陈旧替换。
