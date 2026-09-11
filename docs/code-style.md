---
title: Canglang 代码风格
doc_type: reference
status: current
authority: normative
audience: maintainers
canonical: true
summary: 统一 Canglang 的 Rust、TypeScript、Vue、Tailwind 和跨边界代码风格。
---

# Canglang 代码风格

代码风格服务于可读性、边界清晰与稳定验证；采用语言与现有工具链的默认约定，不为形式一致引入多余抽象或依赖。

## Rust

- 使用 `cargo fmt` 格式化，提交前运行 `cargo fmt --check`。
- 类型和 trait 使用 PascalCase；函数、方法、模块和字段使用 snake_case；常量使用 SCREAMING_SNAKE_CASE。
- 类型、函数和模块名使用英文；源代码中的注释和 doc comment 使用简洁中文，协议名称、文件格式、代码符号和领域文字面量保留原文。
- 领域错误使用枚举和结构化字段表达。调用方不得依赖错误显示文本来判断错误类别。
- `unwrap` 和 `expect` 只用于已经由类型或明确不变量保证成功的位置；外部文件、进程、协议和用户输入使用显式错误处理。
- `core` 保持纯领域逻辑；Tauri 命令、文件 IO、进程生命周期和事件转发放在对应边界模块。
- 公共 Rust 类型和方法写清楚输入、输出、失败条件和生命周期语义；不要把实现过程写进 doc comment。

## 注释与文档语言

- `src/`、`src-tauri/` 和测试源码中的解释性注释统一使用中文；Rust 的 `///`、模块级 `//!`、TypeScript JSDoc 和 Vue 模板注释都遵循此规则。
- 注释优先说明原因、约束、生命周期、跨边界语义和测试意图，不重复代码已经直接表达的操作过程。
- 短的单行实现注释、步骤注释和标签不加句号；需要分隔多个完整句子时使用中文标点。
- 中文棋谱、开局名称和测试 fixture 属于被描述的数据时直接保留；协议名称、文件格式和代码符号保留原文。
- `docs/` 下的正式文档继续使用中文；它们描述当前项目行为，不属于源码注释迁移范围。
- `src/bindings/generated.ts` 由 Rust 的 Specta 注释生成，不手工修改生成内容。
- 新行为和回归修复必须在最接近行为的 Rust 测试中覆盖。

## TypeScript 和 Vue

- 使用现有格式：双引号、分号、2 空格缩进和尾随逗号。
- 保持 `tsconfig.json` 的 `strict`、`noUnusedLocals`、`noUnusedParameters` 等检查通过。
- 类型只在类型位置使用时采用 `import type`；避免无必要的 `any`。
- Vue 组件使用 `<script setup lang="ts">`，组件名和文件名使用 PascalCase。
- Pinia store 管理跨组件的后端状态和请求协调；composable 管理交互状态和可复用行为；组件负责展示和用户输入。
- 前端不复制 Rust 棋规、引擎协议解析或棋谱格式解析；需要这些能力时通过 IPC 请求 Rust。
- Tauri 命令统一经过 [`src/lib/ipc.ts`](../src/lib/ipc.ts) 访问；组件不直接手写 `invoke`。
- `src/bindings/generated.ts` 是生成文件，只修改 Rust 命令或 Specta 类型后重新生成。
- 模板中的列表提供稳定 `key`，异步事件处理保持错误可见，不用空 catch 隐藏失败。

## Tailwind 和 UI

- 优先使用语义 token，例如 `bg-background`、`text-foreground`、`border-border` 和 `text-muted-foreground`。
- 条件或合并 class 使用 [`cn`](../src/lib/utils.ts)，不要在模板中重复拼接 class 字符串。
- 布局优先使用 `flex`、`grid` 和 `gap-*`；等宽高元素使用 `size-*`，避免无意义的重复样式。
- 优先组合 `src/components/ui` 中已有的 Button、Card、Tabs、Badge 等组件。
- 棋盘 SVG 的棋盘线、棋子和状态标记属于视觉绘制数据，可以使用明确的原始颜色；普通应用 UI 使用主题变量。
- 组件样式不承载棋规、协议或文件格式逻辑。

## 跨边界命名

- Rust 内部使用 Rust 命名约定，Specta 生成的 TypeScript API 使用 camelCase 命令函数名和 Rust 序列化字段名。
- FEN 用于局面，ICCS 用于走法；不要让同一个字段在不同边界隐含不同编码。
- `SessionSnapshot` 是普通象棋/揭棋统一对局视图、`ThinkData` 是引擎实时输出、`BookMove` 是开局库查询结果；前端类型直接来自生成绑定或与事件契约一致的本地类型。
- 错误在 Rust 侧分类，在前端通过 `unwrap` 统一抛出；不要用错误字符串驱动正常 UI 分支。

## 工具和验证

不强制引入 ESLint、Prettier 或额外格式化插件；确有需求时先评估对 Vue、Tailwind class 排序与生成文件的影响，再单独引入配置与迁移。

```sh
cargo fmt --check
cargo clippy --all-targets --all-features
pnpm typecheck
pnpm build
```

代码风格规则发生变化时，同步更新本页和根目录 [AGENTS.md](../AGENTS.md) 中的入口规则。
