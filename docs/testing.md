---
title: Canglang 测试策略
doc_type: testing
status: current
authority: normative
audience: maintainers
canonical: true
summary: 规定 Canglang 各模块的测试层级以及变更到验证命令的映射。
---

# Canglang 测试策略

测试优先验证真实模块边界与公开行为。Rust 测试位于模块内的 `#[cfg(test)]` 单元测试与 [`src-tauri/tests`](../src-tauri/tests) 集成测试。前端行为回归位于 [`tests/frontend`](../tests/frontend)，由 Node 内置测试运行器执行，Vite SSR 加载实际源码；测试替换桌面 IPC 响应，通过 Pinia 和 Vue 自定义 renderer 验证状态及启动顺序，不替代 Rust 棋规验证或浏览器交互验收。

## 测试层级

| 变更范围 | 首选验证 |
| --- | --- |
| 棋盘、棋子、坐标和走法 | `board_tests.rs`、`move_tests.rs`、`rules_tests.rs` |
| 中文记谱和 hash | `notation_tests.rs`、`zobrist_tests.rs` |
| 规则档案与循环裁决 | `adjudication_tests.rs` |
| `GameState` 和对局命令 | `game_tests.rs`、`ipc_core_tests.rs` |
| 揭棋身份、规则、记谱和历史 | `jieqi_model_tests.rs`、`jieqi_rules_tests.rs`、`jieqi_notation_tests.rs`、`jieqi_game_tests.rs` |
| 统一会话版本与模式生命周期 | `session_tests.rs`、`session_lifecycle_tests.rs`、`jieqi_session.test.mjs` |
| UCI/UCCI 格式与解析 | `protocol_tests.rs`、`engine_tests.rs` |
| 外部引擎进程 | `engine_tests.rs` 的 fake UCI（握手、完整历史、会话日志、stderr、CRLF、停止/异常退出）；`engine_e2e_tests.rs`，需要可用引擎时运行 |
| 内置引擎下载与装配 | `engine::installer` 单元测试；`installer_e2e_tests.rs`，需要官方压缩包时运行 |
| 配置、便携模式与原子写入 | `engine::config` 单元测试 |
| PGN/XQF 和编码 | `pgn_parser_tests.rs`、`pgn_exporter_tests.rs`、`xqf_parser_tests.rs`、`xqf_exporter_tests.rs`、`manual_io_service_tests.rs` |
| 揭棋 `.cjq` 与私密边界 | `jieqi_manual_tests.rs`、`jieqi_document_ipc_tests.rs` |
| 开局库 | `book_tests.rs`；云库解析用 `engine::book::cloud_book` 单元测试，不依赖网络 |
| IPC 模型或 Specta 绑定 | `ipc_core_tests.rs`、`cargo test`、`pnpm typecheck` |
| Vue、Tailwind 或 Vite | `pnpm typecheck`、`pnpm build` |
| 前端棋谱同步、开局库查询/评价与启动规则 | `pnpm test`、`pnpm typecheck` |
| 棋谱保存竞争、窗口关闭保护与终局停止分析 | `pnpm test`，通过可控 IPC 响应及 Tauri 事件模拟验证真实 store/composable；桌面关闭另做运行时验收 |
| 揭棋文档编辑、保存竞争与加载回滚 | `jieqi-document.test.mjs`；配合 Rust `.cjq` 测试验证后端事实与公开返回 |
| 响应式与无障碍 UI | 浏览器 AX tree/键盘验收：`1280×800`、`1024×768`、`800×700`；检查 Dialog 焦点、棋盘 roving tabindex、ARIA 名称/状态和 live region |
| Tauri 配置或桌面集成 | `pnpm exec tauri build --debug --no-bundle` |

## 基线检查

提交前按变更范围运行：

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
pnpm test
pnpm typecheck
pnpm build
```

不必每次都运行外部引擎 E2E 或完整桌面打包；但只要变更影响对应路径，就必须运行该路径的专门验证并记录失败原因。UI 重构还必须记录实际视口、键盘/焦点路径和环境限制；浏览器 AX tree 不能替代 screen-reader speech 验证。

分支和 PR 的 CI 执行前端行为回归及将警告视为错误的 Clippy 检查。

需要真实引擎与官方压缩包的两个被忽略测试通过环境变量启用：

```sh
PIKAFISH_ENGINE=/path/to/engine PIKAFISH_NNUE=/path/to/pikafish.nnue \
  cargo test --test engine_e2e_tests -- --ignored
PIKAFISH_ONLINE=1 PIKAFISH_ARCHIVE=/path/to/Pikafish.7z \
  cargo test --test installer_e2e_tests -- --ignored

UCCI_ENGINE=/path/to/ucci-engine \
  cargo test --test ucci_e2e_tests -- --ignored
```

协议日志面板默认只订阅当前运行，展开后可查询最近一次分析的 `analysis_session_id`；导出文件包含握手前置流、原始收发文本、生命周期行、时间戳、序号和流方向。未设置 `PIKAFISH_ENGINE`、`PIKAFISH_ARCHIVE`、`PIKAFISH_ONLINE=1` 或 `UCCI_ENGINE` 时，真实引擎测试只输出跳过原因，不把环境缺失误报为兼容性通过。

## 测试不变量

- 非法走法不能改变对局局面、历史或结果。
- 揭棋合法性复用象棋移动几何并叠加明暗状态；候选高亮包含几何成立但会送将的位置，最终落子仍拒绝且不揭示真实身份。
- 揭棋公开快照、公开回放与文档 IPC 响应不得包含 Assigned、稳定棋子 ID 或未揭真实身份。
- 私有与公开 `.cjq` 必须逐步重放验证；文件失败、陈旧 token 或保存失败不得替换当前对局或损坏旧文件。
- 会话写操作必须校验 token；前端串行发送，`stale_session` 不自动重试。
- 悔棋和重做必须恢复相同的局面和走方。
- FEN 解析和序列化应保持规范化 round-trip。
- UCI/UCCI 格式化和解析必须分别符合各自协议。
- PGN/XQF 解析结果必须保持棋谱主线和变例结构。
- canonical XQF v10 导出必须能被内部解析器回读；不支持的变例或不可表示字符必须显式失败，不得静默丢失。
- IPC 模型变化必须同步生成绑定，不能只让 Rust 测试通过。
- 内置引擎装配必须选中当前平台的可执行文件并绑定同目录权重，且不得把 Android 构建或附属文档当成引擎。
- 配置写入必须原子替换，便携模式下路径跟随可执行文件所在目录。
- 云库超时或不可用时查询必须降级为未命中，不得使 `bookQuery` 失败或伪造来源。
- 引擎停止必须结束搜索和相关事件转发任务。
- 揭棋会话必须在前后端同时禁用普通象棋引擎和开局库，换局停止旧引擎生命周期。

## 失败处理

测试失败先判断是领域逻辑、边界转换、生命周期还是环境依赖问题。不通过放宽断言、吞掉错误或在消费者加兜底来隐藏生产方错误；修复后重跑最接近失败行为的专门测试，再跑受影响的基线检查。

## 棋谱备注验收记录

- Rust 语义验收覆盖根节点、主线节点、变例节点、嵌套变例、连续多行备注、空白备注和 PGN 特殊字符；备注被清空时不改变走法应用后的 FEN。
- 前端 smoke test 在窄视口 `314×822` 完成：展开着法列表后可见备注编辑器，编辑器具备可访问名称，可输入多行文本并通过“保存备注”提交；Vite 浏览器环境没有 Tauri runtime，因此不把 `invoke` 缺失误判为桌面 IPC 缺陷。
- canonical XQF v10 已完成内部生成后回读和失败边界验证。当前验证环境未安装可用的外部 XQF 阅读器，外部打开结果不作兼容性承诺。
