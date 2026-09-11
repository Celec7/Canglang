---
title: Canglang 正式文档维护规则
doc_type: maintenance-rules
status: current
authority: maintenance
audience: maintainers
canonical: true
summary: 规定 Canglang 正式文档的职责、事实归属、链接和变更检查规则。
---

# 正式文档维护规则

`docs/` 保存 Canglang 当前的架构、领域契约、API 边界、开发流程和测试策略。正式文档不是实施计划、交接记录或提交历史。

## 文档分层

| 文档 | 唯一职责 |
| --- | --- |
| `architecture.md` | 应用组成、主数据流和运行时关系 |
| `module-boundaries.md` | 模块所有权、依赖方向和禁止的跨层依赖 |
| `ipc-contract.md` | Tauri 命令、事件、序列化和生成绑定契约 |
| `chess-domain.md` | 象棋及其上层揭棋的棋盘、走法、身份、棋规和统一对局状态契约 |
| `engine.md` | 引擎协议、进程、会话、分析和事件流 |
| `manual-formats.md` | PGN、XQF、揭棋 `.cjq`、编码、棋谱模型和开局库 |
| `frontend.md` | Vue、Pinia、组件、样式和前端数据流 |
| `code-style.md` | Rust、TypeScript、Vue、Tailwind 和跨边界代码风格 |
| `development.md` | 环境搭建、日常命令和桌面开发流程 |
| `testing.md` | 测试层级、验证命令和变更到检查的映射 |
| `glossary.md` | 术语的统一含义 |

每个事实只有一个权威归属。其他文档通过相对链接引用，不复制完整定义。

## Metadata 规则

每份正式 Markdown 文档都在开头使用 YAML front matter：

```yaml
title: 文档标题
doc_type: architecture | module-boundaries | contract | api | reference | development | testing | glossary | maintenance-rules
status: current
authority: normative | descriptive | maintenance
audience: maintainers
canonical: true
summary: 一句话说明本文唯一职责。
```

Metadata 只说明文档身份和职责，不记录作者、更新时间、进度、版本历史或临时实现状态。

## 写作规则

- 使用中文；代码符号、协议名称、命令名和必要的英文术语保留原文。
- 架构文档写组件地图和主流程；字段、错误和状态细节归入对应契约文档。
- 描述揭棋时明确其复用象棋棋盘与移动几何，再说明身份和明暗叠加规则；不得写成互不相关的平行棋规。
- 隐藏身份的公开范围归 `chess-domain.md`，`.cjq` 字段和限额归 `manual-formats.md`，命令和 token 归 `ipc-contract.md`，不要跨文档复制完整定义。
- 文档描述当前代码行为，不写“即将”“未来实现”或迁移过程。
- 使用相对 Markdown 链接，移动或删除文件后搜索并修复所有引用。
- Mermaid 只用于确实能表达模块关系、顺序或分支的地方；表格用于所有权和 API 映射。
- 段落保持单一主题，不在多份文档中重复同一条规则。
- 正式文档不复制源码、测试清单或生成文件的完整内容；应链接到权威实现。

## 变更检查

文档变更至少检查：

```sh
rg -n "旧文件名|旧术语|旧模块路径" docs AGENTS.md
git diff --check
```

文档和代码冲突时，以当前代码和已确认的产品行为为准，更新文档；未落地的想法不写成当前契约。
