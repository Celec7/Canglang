---
title: Canglang 引擎系统
doc_type: reference
status: current
authority: descriptive
audience: maintainers
canonical: true
summary: 描述外部象棋引擎的协议抽象、进程生命周期、分析会话和实时事件。
---

# Canglang 引擎系统

引擎实现在 [`src-tauri/src/engine`](../src-tauri/src/engine)，经 `ipc/engine.rs` 暴露给前端。系统将协议格式化、进程 IO、搜索会话与 Tauri 事件转发分离。

## 配置和模型

`EngineConfig` 含可执行文件路径、协议名称、可选引擎选项与可选的 `nnue_path` 权重路径。`AnalysisConfig` 支持限时、限深、限节点与无限搜索，并携带 MultiPV 及引擎/开局库延迟字段。

`EngineProfile` 是持久化在 `config.json` 的引擎档案：`id`、`name`、`path`、`protocol`、`threads`、`hash_mb`、`nnue_path`、类型化 `option_overrides`，以及内置引擎的 `release_revision`、`installed_revision`、`is_builtin`、`download_url` 与 `description`。旧配置缺少新字段时按空覆盖和未记录修订读取。引擎自己的开局库、残局库路径属于对应动态选项；Canglang 的 `.bh`/云库仍由 `BookState` 管理。前端只通过统一的 Profile → `EngineConfig` 转换入口生成协议字符串。内置档案未下载时 `path` 为空；用户自建档案的 `is_builtin` 为 `false`。

`threads` 与 `hash_mb` 是跨引擎的常用档案字段，但只在握手声明了对应 Option 时下发；未声明时按 best-effort 忽略。`option_overrides` 的其它未知名称仍会拒绝启动，避免拼写错误静默失效；已声明的选项按握手类型和范围校验。

协议名称必须为 `ucci`、`uci` 或 `auto`；`auto` 先由 `engine::probe` 依次尝试两种握手，再按探测结果启动正式会话。其它取值返回 `EngineError::InvalidConfig`，不静默回落到某协议实现。

`ThinkData` 表示一次搜索更新，含深度、分数、将杀距离、MultiPV 序号、NPS、耗时、ICCS PV 与中文 PV。`EngineInfo` 还保留握手声明的动态选项及协议能力；未确认的根节点约束不会被当作已应用。分数与胜率的解释由前端展示层完成，Rust 保持原始分析数据。

协议诊断在进程 IO 边界采集 stdin、stdout、stderr 和生命周期原始行，保留空行、CRLF、长行和未知文本，并附 `run_id`、可选 `analysis_session_id`、Unix 毫秒时间戳和单调序号。`engine://protocol` 事件供面板实时显示，`engineProtocolLog(analysisSessionId)`/`engineExportProtocolLog(path, analysisSessionId)` 可以查询或导出当前运行的整段日志，或导出指定分析会话并自动带上握手前置流；解析后的 ThinkData 与最佳着法事件不替代原始诊断流。前端清空只是清空展示窗口，不删除后端缓冲。

## 协议抽象

`Protocol` 统一以下能力：

- 握手与 ready 命令及完成标记；
- FEN、历史走法与 `position` 命令格式化；
- `go`、`stop`、`quit` 与选项命令格式化；
- `info` 与最佳着法输出解析。

`UciProtocol` 用 UCI 的 `uciok`、`movetime` 等格式，`UcciProtocol` 用 UCCI 的 `ucciok`、`time` 等格式。协议差异封装在协议实现内，不散落到进程或 UI 代码。

## 生命周期

`EngineProcess` 管理外部子进程的 stdin/stdout 与行流，并在 stdout 关闭时报告异常退出；`EngineSession` 管理协议、搜索状态与 `ThinkData`/最佳着法/退出广播。退出命令由 `EngineSession` 按当前协议发送，进程层只负责等待退出或超时强制终止。

启动时，`EngineState` 的生命周期锁让启动/重启串行执行；IPC 层先创建并探活候选 `EngineSession`，成功后才取消旧的事件转发任务、停止旧会话并切换到新会话，因此候选启动失败不会破坏当前活动引擎。停止时同一清理路径结束搜索与子进程并取消转发任务。启动、分析、停止间的状态由 `EngineState` 异步锁保护。

握手与 `isready` 都须在超时内收到协议确认；启动阶段超时或 stdout 通道提前关闭返回引擎错误，并清理已创建的子进程与解析任务。运行阶段若 stdout 异常关闭，会话发出 `engine://stopped`，前端清理运行、分析和自动走子状态。停止清除会话中的进程、协议与局面引用，重启从干净状态开始。收到 `bestmove`、成功停止或分析命令失败后，会话分析状态恢复为空闲。

引擎未运行时，分析命令返回 `EngineError::NotRunning`。前端据命令结果更新状态，不凭按钮点击推断引擎已启动。

## 分析流

```text
engine_analyze(AnalysisRequest)
  → 起始 FEN + 完整 history ICCS + 当前 FEN 校验
  → EngineSession
  → Protocol.format_position / format_go
  → EngineProcess stdout
  → Protocol.parse_info / parse_best_move
  → broadcast channels
  → ipc/engine.rs
  → 带 analysis_session_id 的 think:// / bestmove://
  → EngineStore
```

思考事件在 IPC 转发层按 50ms 节流，避免高频 stdout 压垮前端。最佳着法作为独立但同样带会话 ID 的事件发送。Session 在发送 `position` 前从起始 FEN 重放完整历史并校验当前 FEN；根节点约束只作用于当前搜索根。接收端滞后时，转发器丢弃过期思考更新，不伪装成新分析结果；前端 `EngineStore` 还会同时校验会话 ID 与当前 FEN/ICCS 快照，拒绝停止、重置或跳转后迟到的事件。

## 内置在线引擎

`engine::config::default_builtin_profiles` 是预制引擎注册中心：`AppConfig::default` 的初始档案与安装命令都从这里取定义，新增内置引擎只需追加一项。内置档案 `is_builtin` 为 `true`，携带官方 `download_url` 与说明，未下载时 `path` 为空。

`engine_download_builtin` 执行安装流水线：

1. 按 `ConfigService::engine_install_dir` 解析目标目录——便携模式位于可执行文件（即 `config.json`）同级的 `engines/<key>`，标准模式位于系统用户配置目录下的 `engines/<key>`；读取档案后立即释放配置锁，下载期间不阻塞其它配置读写。
2. `EngineInstaller` 用 reqwest 流式下载官方 `.7z`，按 150ms 节流经 `engine://download-progress` 上报阶段、已下载字节、总字节、百分比与速率；下载不完整或传输中断即失败并清理临时压缩包。
3. `sevenz-rust` 解压到安装目录；`locate_runtime` 按当前操作系统挑可执行文件（先精确匹配官方文件名，再按平台特征递归兜底，排除 Android 构建与 `.nnue`/文档），类 Unix 平台补 `0o755`，并绑定同目录的 `pikafish.nnue`。
4. 先用 staging 目录中的产物通过 `EngineSession` 探针，再原子替换正式目录并写入 `config.json`。配置写入失败会恢复旧目录；探活失败则只清理 staging，现有安装保持可用。

`engine_remove_builtin` 删除 `engines/<key>` 目录并把档案重置为未下载状态；用户自建的本地档案不经过该目录，不受影响。

## NNUE 权重挂载

`EngineConfig::nnue_path` 在握手完成后、其它自定义选项和 `isready` 之前下发 `setoption name EvalFile value <path>`。为空时不发送该命令：`EngineProcess` 把子进程工作目录设为可执行文件所在目录，引擎据此加载同名的自带权重。

前端在引擎面板的“高级设置”里为已装配档案选择外置权重，留空即恢复引擎自带权重。

## 修改引擎代码

新增协议应实现 `Protocol` 并补充协议测试。修改进程生命周期时同时验证启动、分析、立即出招、停止、重启与异常退出。修改 `ThinkData` 或配置模型时重新生成 Specta 绑定，并检查 `EngineStore`、`EnginePanel`、`MultiPvList`。
