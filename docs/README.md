---
title: Canglang 文档
doc_type: reference
status: current
authority: descriptive
audience: maintainers
canonical: true
summary: Canglang 正式文档入口和推荐阅读路径。
---

# Canglang 文档

Canglang 是基于 Tauri v2 的中国象棋桌面应用。先读架构与模块边界，再按修改范围读领域、IPC、引擎、棋谱或前端文档。

## 推荐阅读路径

1. [架构总览](architecture.md)：了解 Vue 前端、Tauri IPC 和 Rust 服务如何组合。
2. [模块边界](module-boundaries.md)：确认新逻辑的归属和允许的依赖方向。
3. [开发指南](development.md)：搭建环境并运行常用检查。
4. 按任务阅读 [象棋与揭棋领域](chess-domain.md)、[IPC 契约](ipc-contract.md)、[引擎系统](engine.md)、[棋谱与开局库](manual-formats.md) 或 [前端开发](frontend.md)。
5. 修改完成后参考 [测试策略](testing.md) 选择验证范围。

## 文档地图

| 主题 | 文档 |
| --- | --- |
| 架构和所有权 | [architecture.md](architecture.md)、[module-boundaries.md](module-boundaries.md) |
| 跨边界数据 | [ipc-contract.md](ipc-contract.md)、[chess-domain.md](chess-domain.md) |
| 外部能力 | [engine.md](engine.md)、[manual-formats.md](manual-formats.md) |
| 前端和代码风格 | [frontend.md](frontend.md)、[code-style.md](code-style.md) |
| 工程流程 | [development.md](development.md)、[testing.md](testing.md) |
| 术语 | [glossary.md](glossary.md) |

文档维护规则见 [docs/AGENTS.md](AGENTS.md)，仓库级开发约束见 [AGENTS.md](../AGENTS.md)。
