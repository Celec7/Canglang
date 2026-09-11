---
title: Canglang IPC 契约
doc_type: api
status: current
authority: normative
audience: maintainers
canonical: true
summary: 规定 Rust Tauri 命令、引擎事件和前端生成绑定之间的公开边界。
---

# Canglang IPC 契约

前后端经 Tauri IPC 通信。Rust 命令用 `#[tauri::command]` 与 `#[specta::specta]` 标记，`tauri-specta` 把命令与公开类型生成到 [`src/bindings/generated.ts`](../src/bindings/generated.ts)。

## 命令分组

### Core

`ping`、`getInitialBoard`、`parseFen`、`validatePosition`、`toFen`、`makeMove`、`getLegalMoves`、`getCandidateMoves`、`toChineseNotation`、`isInCheck`、`newGame`、`setRuleProfile`、`gameResult`、`undoMove`、`redoMove`、`jumpTo` 和 `resign`。

局面类命令以 FEN/ICCS 为输入输出；对局会话类命令访问 Tauri 管理的 `GameState`，返回 `GameSnapshot`。

`parseFen` 与 `validatePosition` 是两层校验：前者只要求结构完整、可表示（照面王等违规局面仍可解析），后者追加 `MoveValidator::validate_board` 的完整棋规校验。`newGame` 接收自定义 FEN 时同样执行完整棋规校验，避免正式对局建立在非法局面上。`getLegalMoves` 返回全部合法走法，`getCandidateMoves` 只返回指定棋子的候选目标；`jumpTo` 移动历史游标而不改动历史本身。

这些命令保留普通象棋兼容契约。当前应用主路径使用下述统一 Session 命令；揭棋不得调用只接受 FEN 的 Core 会话命令。

### Session

`sessionGet`、`sessionNew`、`sessionTargets`、`sessionMove`、`sessionUndo`、`sessionRedo`、`sessionJump`、`sessionResign`、`sessionOfferDraw`、`sessionRespondDraw` 和 `sessionCancelDraw`。

除只读的 `sessionGet` 外，命令接收 `SessionToken { game_id, expected_revision }`。成功变更增加 `revision`；改变保留主线或终局的操作同时增加 `content_revision`，只读跳转、撤销/重做游标与未接受的求和只增加呈现修订。token 不匹配返回 `stale_session`，服务端不自动套用或重试陈旧请求；前端刷新快照后由用户动作决定下一步。

`sessionNew` 以 `NewGameOptions` 选择普通象棋或揭棋。`SessionSnapshot.position` 与 `start_position` 是带标签的 `PositionView`：普通象棋携带 FEN，揭棋携带 `JieqiPositionViewV1`。`history` 对应 `SessionPly::Xiangqi(PlyRecord)` 或 `SessionPly::Jieqi(PublicPly)`；`capabilities` 是操作可用性的事实来源，禁用项同时给出原因。揭棋求和使用带 ID 的待回应请求，回应方、取消方和过期 ID 均由 Rust 校验。

`sessionTargets` 返回棋盘交互用的几何候选，而不是落子授权。普通象棋与揭棋都保留可能导致送将的候选位置；`sessionMove` 再执行完整王安全校验。两种棋局送将均返回 `illegal_move` 和统一说明“不能送将，请选择其他位置”，前端通过全局 toast 呈现，对局状态与暗子身份保持不变。

### Engine

`engineStart`、`engineAnalyze`、`engineMoveNow`、`engineChangeTactic`、`engineStop` 和 `engineStatus`。

引擎命令传递 `EngineConfig` 与 `AnalysisRequest`。请求包含分析会话 ID、起始 FEN、完整历史 ICCS、当前 FEN、根节点候选/禁着约束和 `AnalysisConfig`；分析命令返回 `AnalysisStartResult`，明确约束为 `applied`、`not_applied` 还是 `unsupported`，并标记是否真的启动。`engineStart` 返回 `EngineInfo`（引擎名、协议、就绪状态、动态选项、能力描述与诊断信息），`engineStatus` 返回当前是否运行或分析中。高频思考信息走事件，不经命令返回。

内置引擎另有三条命令：`enginePickFile` 按 `kind`（`engine`/`nnue`/`book`）过滤系统文件对话框；`engineDownloadBuiltin` 下载、解压、装配并探活内置引擎，返回已就绪的 `EngineProfile`；`engineRemoveBuiltin` 删除该引擎的本地文件并重置档案。

### Manual

`manualLoad`、`manualPickFile`、`manualParseText`、`manualSave`、`manualSaveXqf`、`manualExportPgn`、`jieqiDocumentSave` 和 `jieqiDocumentOpen`。

棋谱路径与 `ChessManual` 在边界传递，解析与导出在 Rust 完成。`manualPickFile(action)` 由 Rust 弹出原生文件选择器：`open` 过滤 `.pgn/.xqf`，`save` 保存 `.pgn`，`save_xqf` 保存 `.xqf`，`open_jieqi`、`save_jieqi_private`、`save_jieqi_public` 使用 `.cjq` 过滤器。`ChessManual.root` 及其后代节点的 `comment` 是可选多行纯文本：根节点表示起始局面说明，普通节点表示走完对应着法后的说明；备注编辑不新增对局快照字段，也不新增实时事件。`manualSaveXqf(path, manual, version)` 当前只接受版本 `10`，生成未加密 canonical XQF；多分支棋谱和无法用 GBK 表示的文本返回错误。前端不得解析 PGN、XQF 或 `.cjq`，也不得自行处理编码和转义。

`jieqiDocumentSave(token, path, kind, metadata, annotations, editRevision)` 只接受揭棋会话。后端在持锁期间校验版本 token、能力并克隆不可变对局事实，随后在锁外原子写入；回执原样携带所导出的 `gameId`、`contentRevision`、`editRevision` 与 `kind`，供前端判定保存结果是否仍然新鲜。`private_game` 的固定身份不会出现在参数或回执中。

`jieqiDocumentOpen(token, path)` 在锁外检查大小、解析并完整重放候选文档，成功后通过统一会话生命周期协调器停止旧引擎、再次检查 token 并替换对局。返回值只含 `SessionSnapshot` 和公开元数据/备注，永不返回 Assigned 身份；失败或过期不修改当前会话。

### Book

`bookLoad`、`bookUnload`、`bookClear`、`bookLoaded`、`bookQuery`、`bookSetCloudEnabled`、`bookSetCloudMode` 和 `bookGetCloudStatus`。

开局库以路径加载、以 FEN 查询，结果为 `BookMove[]`，并按当前 `CloudBookMode` 综合本地 `.bh` 库与象棋云库。云库开关与模式在 `BookState` 中管理，状态经 `bookGetCloudStatus` 返回 `CloudBookStatus`；模式语义与降级行为见[棋谱与开局库](manual-formats.md)。

### Config

`configLoad`、`configSave` 和 `configGetLocation`。

全局配置以 `AppConfig` 整体读写，字段覆盖主题与棋盘方向、动画与音效、默认规则档案、开局库路径与云库设置、引擎档案与当前分析配置。写入经 `ConfigService` 原子替换，避免异常中断损坏配置。

`configGetLocation` 返回 `ConfigLocationInfo`，其中 `filePath`、`isPortable` 说明物理位置，`exists` 说明配置文件是否已存在。前端据 `exists` 判断首次运行迁移，不用“档案列表为空”推断。便携模式判定与安装目录规则见[引擎系统](engine.md#内置在线引擎)。

`engineProtocolLog(analysisSessionId?)` 查询当前运行会话的原始 stdin/stdout/stderr/lifecycle 流；传入分析会话 ID 时只返回该分析段及握手前置流。`engineExportProtocolLog(path, analysisSessionId?)` 将同一范围导出；`enginePickFile("protocol-log")` 提供保存路径。每行包含运行 ID、分析会话 ID、时间戳、方向/流、序号和原始文本。诊断流独立于 ThinkData/bestmove，清空前端显示不会删除后端缓冲。

## 核心模型

| 模型 | 关键字段 | 用途 |
| --- | --- | --- |
| `MoveResult` | `fen`、`iccs`、`chinese_notation`、`legal`、`check`、`game_over` | 单次走子结果 |
| `PlyRecord` | `ply`、`iccs`、`notation`、`mover`、`is_capture`、`is_check`、`fen` | 主线历史条目 |
| `GameSnapshot` | `fen`、`start_fen`、`current_fen`、`current_ply`、`history`、`result`、`red_to_move`、`in_check`、`can_undo`、`can_redo`、`rule_profile`、`repetition_count`、`repetition_explanation`、`rule_status`、`rule_explanation` | 受管对局状态视图 |
| `SessionToken` | `game_id`、`expected_revision` | 统一会话乐观并发凭据 |
| `SessionSnapshot` | 游戏/内容修订、两种 `PositionView`、完整主线与游标、来源、规则、结果、能力和求和请求 | 普通象棋与揭棋的统一公开视图 |
| `PublicPly` | `ply`、`iccs`、`mover`、`notation`、`revealed`、公开吃子、`is_check` | 不含隐藏身份的揭棋历史事件 |
| `JieqiDocumentReceipt` | `game_id`、`content_revision`、`edit_revision`、`kind` | 判断异步保存结果是否仍匹配当前文档 |
| `EngineConfig` | `path`、`protocol`、`options`、`nnue_path` | 外部引擎配置 |
| `EngineInfo` | `name`、`protocol`、`ready`、`diagnostic` | 启动握手结果 |
| `AnalysisRequest` | `analysis_session_id`、`position`、`constraint`、`config` | 一次分析的完整位置与根节点约束 |
| `AnalysisStartResult` | `constraint_status`、`started` | 分析启动和约束实际应用状态 |
| `EngineProfile` | `id`、`name`、`path`、`protocol`、`threads`、`hashMb`、`nnuePath`、`optionOverrides`、`releaseRevision`、`installedRevision`、`isBuiltin`、`downloadUrl`、`description` | 持久化引擎档案，含内置在线引擎 |
| `AnalysisConfig` | `mode`、`value`、`multi_pv`、`engine_delay_ms`、`book_delay_ms` | 分析限制和显示配置 |
| `AppConfig` | 界面与规则偏好、`openingBookPaths`、`cloudBookEnabled`、`cloudBookMode`、`engineProfiles`、`activeEngineId`、分析配置 | 全局持久化配置 |
| `ConfigLocationInfo` | `filePath`、`isPortable`、`exists` | 配置文件位置与首次运行判定 |
| `CloudBookMode` | `hybrid`、`cloud_only`、`local_only`、`merge` | 本地库与云库的协同策略 |
| `CloudBookStatus` | `enabled`、`mode` | 云库开关与当前策略 |
| `ChessManual` | `title`、`date`、`red_player`、`black_player`、`event_name`、`start_fen`、`root`（节点含可选多行 `comment`） | 棋谱和变例树 |
| `BookMove` | `iccs`、`notation`、`score`、`win_count`、`draw_count`、`lose_count`、`win_rate`、`note`、`source` | 开局库候选走法 |

`GameSnapshot` 仅供旧普通象棋命令兼容；应用使用 `SessionSnapshot`。两种快照的 `history` 都是完整主线，`current_ply` 只表示呈现游标。绑定字段名沿用各模型的 `serde` 重命名策略：多数为 snake_case，配置类模型为 camelCase，以生成文件为准。本页只说明边界语义，完整字段以生成绑定与 Rust 的 `specta::Type` 为准。

## 错误语义

`CoreError`、`EngineError`、`ManualError` 统一转换为 `AppError`。可报告错误的命令返回 Specta typed-error 结果，前端经 [`unwrap`](../src/lib/ipc.ts) 转为异常；store 不据错误字符串判断业务状态。

Session 与揭棋文档命令返回结构化 `SessionError`，错误码为 `stale_session`、`invalid_input`、`illegal_move`、`operation_unavailable`、`game_finished` 或 `io_failure`。前端经 `unwrapSession` 保留错误码；`stale_session` 只触发刷新，不自动重试原写操作。

需解析 FEN/ICCS 的命令保留输入错误：`getLegalMoves`、`getCandidateMoves`、`validatePosition`、`isInCheck`、`toChineseNotation`、`makeMove`、`bookQuery` 及引擎分析命令不把无效输入降级为空结果。引擎历史中任一非法 ICCS 使命令失败；`resign` 只接受 `red` 或 `black`。IPC 适配层统一用 `unwrap` 处理 typed-error 结果。

`bookQuery` 只在输入非法或本地库读取失败时报错；云库网络超时与不可用按未命中降级为空列表，不伪装成错误。

Cargo package 与可执行文件名为 `canglang-app`，Rust library target 为 `canglang_app`；Rust 代码与集成测试统一用 `canglang_app` crate 名，Tauri command 名不随 crate 名改变。

## 引擎事件

| 事件 | payload | 生产方 | 消费方 |
| --- | --- | --- | --- |
| `think://` | `{ kind: "think", payload: { analysis_session_id, data: ThinkData } }` | `ipc/engine.rs` 节流转发 | `EngineStore` |
| `bestmove://` | `{ kind: "best_move", payload: { analysis_session_id, move_iccs } }` | `ipc/engine.rs` | `EngineStore` |
| `engine://stopped` | 空 payload；外部引擎异常退出 | `ipc/engine.rs` | `EngineStore` |
| `engine://download-progress` | `DownloadProgressPayload`（阶段、字节数、百分比、速率） | `engine/installer.rs` | 引擎面板下载卡片 |
| `engine://protocol` | `EngineRawLine`（run/session、时间戳、序号、流和原始文本） | `engine/EngineProcess` → `ipc/engine.rs` | 协议日志面板 |

前端经 `listen` 统一提取 event payload，并保存取消订阅函数；事件订阅不替代命令状态查询。

## 修改流程

1. 在对应 Rust 模块修改命令或 `specta::Type` 模型。
2. 保持 FEN、ICCS、字段命名与错误语义不变，除非产品契约明确改变。
3. 运行 `cargo test`，让 Specta 测试重新导出绑定。
4. 检查前端 store、composable 与组件的全部消费者。
5. 运行 `pnpm typecheck`、`pnpm build` 与相关 Rust 测试。

不要直接编辑生成的 `src/bindings/generated.ts`；它会被下一次生成覆盖。
