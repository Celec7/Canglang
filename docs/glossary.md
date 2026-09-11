---
title: Canglang 术语表
doc_type: glossary
status: current
authority: normative
audience: maintainers
canonical: true
summary: 定义 Canglang 文档和代码中使用的核心术语。
---

# Canglang 术语表

| 术语 | 含义 |
| --- | --- |
| BoardState | Rust 中表示中国象棋 10×9 局面的值类型，包含棋子数组和当前走方。 |
| GameState | 通过 `ActiveGame` 维护普通象棋或揭棋，并拥有统一游戏 ID、修订、历史与结果的会话状态。 |
| SessionToken | 写操作携带的 `game_id + expected_revision` 乐观并发凭据。 |
| SessionSnapshot | 普通象棋与揭棋共用的公开会话视图，包含版本、位置分支、历史、结果和能力。 |
| PositionView | 会话位置的带标签联合：普通象棋为 FEN，揭棋为不含秘密的结构化公开视图。 |
| PlyRecord | 跨边界的主线历史条目，含步号、ICCS、中文记谱、走方与局面 FEN。 |
| RuleProfile | 对局采用的竞赛规则档案：`china2020` 或 `asian2017`。 |
| RuleAssessment | 规则服务对重复局面的机器可读判定与解释；只作参考提示，不改写对局结果。 |
| FEN | 跨边界表示普通中国象棋局面的字符串格式，包含棋盘布局和走方；不承载揭棋隐藏身份。 |
| ICCS | 四字符起止坐标走法格式，例如 `h2e2`。 |
| UCI | 通用国际象棋引擎通信协议；用于兼容 UCI 的引擎。 |
| UCCI | 中国象棋引擎通信协议；用于兼容 UCCI 的引擎。 |
| IPC | 前端与 Tauri Rust 宿主之间的进程间通信边界。 |
| Tauri Specta | 从 Rust 命令和类型生成 TypeScript IPC 绑定的工具链。 |
| EngineProcess | 管理外部引擎子进程及其标准输入输出的模块。 |
| EngineSession | 管理协议、搜索和引擎实时输出的会话对象。 |
| EngineProfile | 持久化在 `config.json` 的引擎档案，含路径、协议、线程、hash、权重与内置标记。 |
| ThinkData | 引擎一次 `info` 搜索更新的结构化数据。 |
| MultiPV | 引擎同时返回多个候选主变化的分析模式。 |
| ConfigService | 解析 `config.json` 位置（含便携模式）并以原子替换写入的服务。 |
| Portable Mode | 配置与引擎安装目录跟随可执行文件所在目录的部署模式。 |
| Opening Book | 按局面返回候选开局走法及统计信息的数据源，可为本地 `.bh` 库或象棋云库。 |
| CloudBookMode | 本地库与云库的协同策略：`hybrid`、`cloud_only`、`local_only`、`merge`。 |
| ChessManual | 棋谱元数据和变例树组成的结构化模型。 |
| ManualNode | 棋谱变例树中的一个节点，保存走法、记谱、注释、评分和子分支。 |
| Main Line | 棋谱变例树中每个节点的第一个子节点组成的主线。 |
| Variation | 棋谱主线之外的分支。 |
| 揭棋（Jieqi） | 建立在中国象棋移动几何和王安全规则之上，并增加随机身份、明暗状态与揭子事件的棋种。 |
| 首步角色（move_as） | 暗子在首次移动前使用的原始棋位角色，只决定公开移动几何，不代表真实身份。 |
| Assigned | 本地揭棋完整固定身份来源，仅存在于 Rust 私有状态与 `.cjq` 私有续局。 |
| RecordedReveals | 公开回放的身份来源，只记录主线已经发生的揭子证据。 |
| PublicPly | 揭棋公开半回合事件，包含 ICCS、记谱、揭子、公开吃子和将军信息。 |
| `.cjq` | Canglang 揭棋 UTF-8 JSON 文档；分为私有续局与公开回放。 |
