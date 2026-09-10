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
| `core` | 棋盘、棋子、走法、棋规、规则档案、记谱、hash、对局状态 | Rust 标准库、serde、内部 core 模块 | Tauri、Vue、文件 IO、外部进程 |
| `engine` | 引擎模型、协议、进程、会话、安装器、配置服务和开局库（本地 `.bh` 与云库） | `core`、tokio、rusqlite、reqwest、serde | Vue 组件、前端状态、Tauri 命令注册 |
| `manual` | PGN/XQF、编码、棋谱模型和导出 | `core`、serde、文件 IO | UI 状态、引擎进程、Tauri 权限 |
| `ipc` | 命令、受管状态、序列化边界和事件转发 | `core`、`engine`、`manual`、Tauri | 重复实现棋规或文件格式 |
| `bindings` | 从 Rust 命令生成前端类型和调用函数 | Tauri API | 手工业务逻辑和手工维护的 API 类型 |
| `lib/ipc.ts` | 统一命令、错误解包和 typed 事件订阅入口 | generated bindings、`lib/events.ts`、Tauri event API | 领域校验和 UI 状态 |
| `lib/events.ts` | Rust `ThinkData`、最佳着法和下载进度事件的前端契约镜像 | Rust `specta::Type` 模型（权威） | 生成第二套领域模型或改变事件字段 |
| `stores` | 前端可见状态、请求协调和快照投影 | `lib/ipc.ts`、Vue、绑定类型 | 复制 Rust 棋局或协议状态机 |
| `composables` | 组件交互状态和展示辅助 | stores、前端纯函数、IPC 入口 | 成为后端事实源 |
| `components` | 展示、用户输入和组件组合 | stores、composables、UI components | 直接调用命令、订阅 Tauri 事件、访问 Rust 内部或实现棋规 |

`core` 内部按领域职责分层：`board` 负责 FEN 与棋盘值语义，`rules::attacks` 负责攻击/路径查询，`rules::generation` 负责候选目标与合法走法枚举，`rules::legality` 负责伪合法走法、完整合法性、棋局结果与棋盘完整校验，`rules::profile` 定义规则档案，`rules::repetition` 与 `rules::adjudicator` 负责循环识别与裁决解释；`rules::MoveValidator` 是对外稳定门面。`game::GameState` 独占历史状态转移，调用方只能经只读快照与走法操作访问。

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

- 局面跨边界用 FEN，走法用 ICCS；Rust 内部可用 `BoardState` 与 `Move`。
- 公开 IPC 模型用 serde 与 Specta；前端只用生成类型，不手写同名接口。
- 后端错误在 `AppError` 汇总，以字符串错误 payload 暴露给前端；前端经 `unwrap` 统一转为异常，不据错误字符串判断业务状态。
- 需要解析输入的 IPC 命令不得把无效 FEN、ICCS、颜色或历史静默降级为空结果。
- 实时引擎信息走事件，不把高频 `ThinkData` 塞进命令返回值或全局快照。
- 文件路径只出现在文件服务与对应输入边界；文件内容解析在 Rust 完成。
- 配置只经 `ConfigService` 读写；前端不自行拼装配置文件路径，也不直接写磁盘。
- 外部网络访问只允许出现在 `engine/book`（云库）与 `engine/installer`（引擎下载）；`core`、`manual` 与前端组件保持无网络依赖。
- `BoardState::from_fen` 只保证结构完整、可表示；需要王宫与照面王等完整棋规时显式调用 `MoveValidator::validate_board`，不混用两层校验。
