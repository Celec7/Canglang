---
title: Canglang 前端开发
doc_type: reference
status: current
authority: descriptive
audience: maintainers
canonical: true
summary: 描述 Vue 前端的状态、交互、IPC 访问和 UI 组件约定。
---

# Canglang 前端开发

前端位于 [`src`](../src)，使用 Vue 3 `<script setup lang="ts">`、Pinia、Vite、Tailwind CSS 与本地 shadcn-vue 风格组件。

## 状态层

| 层 | 当前职责 |
| --- | --- |
| `GameStore` | 对局快照、FEN、结果、历史、悔棋/重做、跳转和走子请求 |
| `EngineStore` | 引擎运行状态、分析会话、根节点约束、实时思考数据、PV 预览和最佳着法事件 |
| `BookStore` | 开局库路径、加载/卸载、云库开关与模式、查询结果 |
| `ManualStore` | 棋谱载入、当前节点、变例导航与导出 |
| `usePreferencesStore` | 界面与规则偏好、引擎档案、分析配置，并负责落盘 |
| `useBoard` | 棋盘选择、合法落点高亮和点击到 ICCS 的转换 |
| `usePositionEditor` | 摆子编辑器的棋子拾取、放置与局面组装 |
| `useReplay` | 复盘播放状态与自动步进 |
| `useShortcuts` | 全局快捷键绑定与输入焦点判断 |
| `useA11yAnnouncer` | 无障碍播报通道（polite / assertive） |
| `useAppLifecycle` | 偏好、棋局、开局库初始化与跨 store watcher |
| `useWorkspaceLayout` | 响应式面板默认值与用户显式切换覆盖 |
| `useEngineSettings` | 引擎档案、文件选择、内置引擎下载与分析参数 |
| Vue components | 展示、输入、按钮动作和组件组合 |

Store 经 `src/lib/ipc.ts` 调用命令。`GameStore` 用 `GameSnapshot` 覆盖可见状态；`EngineStore` 订阅带 `analysis_session_id` 的 `think://`、`bestmove://` 和 `engine://stopped`，显式停止或外部引擎异常退出时都清理展示状态。`usePreferencesStore` 以 localStorage 为备份、防抖后经 `configSave` 原子写入 `config.json`，启动时按 `configGetLocation().exists` 决定是否迁移旧配置。

`useAppLifecycle` 在配置加载、对局初始化后，通过 `setRuleProfile` 将持久化的默认规则应用到后端，再启动引擎。对局初始化或规则恢复失败时，分析区显示错误，引擎不启动。

对局结果也是分析监听的依赖：认输或规则裁决进入终局时，取消自动走子并停止当前搜索，清除活动分析上下文；引擎进程保持就绪。恢复到未终局的局面时按当前控制模式重新分析。

`BookStore` 用递增查询序号隔离异步响应：新查询立即移除旧候选，只有最新请求可更新候选列表和加载状态；清空查询同时使在途响应失效。开局库评价由 `lib/book-evaluation.ts` 展示，W/D/L 分别表示胜、和、负，数据来源不改变评价含义。

## 组件关系

`App.vue` 组合 `AppToolbar`、`ChessBoard`、`BoardControls`、`AnalysisWorkspace`、`MoveList`、`ReplayControls` 与 `SettingsView`。面板与按钮用 `src/components/ui` 的 Button、Card、Tabs、Badge 等组件。新增 UI 前先复用已有组件与语义颜色。

棋盘负责 SVG 绘制与点击目标，`useBoard` 负责交互状态，Rust 负责最终合法性判断；引擎分析输出由 `MultiPvList` 展示，点击 PV 后由 Rust 纯预览快照驱动棋盘渲染，明确应用才写入 `GameStore`。`BookTable` 展示 Rust 返回的开局库候选。`SettingsView` 只提供 Dialog 外壳与导航，领域内容位于 `components/settings/*`，引擎档案管理与内置引擎在线获取由 `useEngineSettings` 协调。设置页中的 selected Profile 只是待编辑项，只有“保存并应用当前引擎”成功后才更新 `activeEngineId`；Profile 到 `EngineConfig` 的转换集中在 `src/lib/engine-profile.ts`。

普通走子或应用 PV 成功后，`GameStore` 将后端快照中游标以内的历史交给 `ManualStore.recordHistory`，同步当前棋谱路径。清空棋谱后继续走子同样恢复完整的当前历史。已有节点、备注与分支保留，新节点使棋谱进入待保存状态并使旧导出文本失效；后端拒绝整条 PV 时不更新棋谱。

PGN/XQF 保存记录发起请求时的棋谱内容版本，只在成功且版本未改变时清除待保存标记。保存期间新增走子、修改备注或替换棋谱，不会被旧请求误标为已保存。

`useWindowCloseGuard` 在应用挂载时注册 Tauri 窗口关闭监听，关闭按钮和系统关闭请求统一检查棋谱待保存状态。取消放弃修改时阻止关闭，确认后由 Tauri SDK 销毁窗口；卸载时取消监听，包括注册尚未完成的情况。

`src/lib/presentation.ts` 负责结果、规则状态与回合的中文文案，`engine-evaluation.ts` 负责分数到胜率的换算与格式化，`sound.ts` 提供程序化音效，`window.ts` 封装无边框窗口控制。这些模块只做展示换算，不重新计算领域结论。

## 无障碍与快捷键

- 棋盘焦点采用 roving tabindex：方向键在格点间移动，只保留一个可聚焦单元，选中与合法落点通过 `aria-*` 与无障碍名称暴露。
- `src/lib/a11y.ts` 生成传统称谓的局面标签（如「红方巡河线」「黑方二路」），`useA11yAnnouncer` 提供 polite / assertive 两级播报通道。
- `src/lib/shortcuts.ts` 是快捷键定义与展示的唯一来源，`useShortcuts` 负责绑定；输入框聚焦时不拦截按键。
- 新增交互控件必须提供可访问名称与键盘操作路径；纯视觉装饰不进入无障碍树。

## IPC 访问

组件不直接导入 Tauri `invoke`、`commands` 或原始事件订阅。命令由 store/composable 经 `commands` 与 `unwrap` 调用；`listenThink`、`listenBestMove`、`listenEngineStopped`、`listenDownloadProgress` 是 `src/lib/ipc.ts` 的 typed gateway，payload 镜像集中在 `src/lib/events.ts`。新增边界能力时先改 Rust 命令与 Specta 模型，再更新绑定、事件镜像与 feature owner。

响应式验收至少覆盖 `1280×800`、`1024×768` 和 `800×700`；设置 Dialog 在窄宽度下采用可滚动 feature 导航与内容区。验收同时检查键盘操作、Dialog 焦点恢复、棋盘 roving tabindex、可访问名称/状态以及 polite/assertive live region；若没有 screen-reader runtime，必须标记为环境限制。

外部链接用 `@tauri-apps/plugin-opener` 的 `openUrl` 在系统浏览器打开，不用 `window.open`；可用 URL 范围由 `src-tauri/capabilities/default.json` 的 scope 限定。

前端模型字段直接使用生成绑定中的名称（如 `red_to_move`、`multi_pv`、`win_rate`；配置类模型为 `cloudBookMode`、`isPortable`）；不在组件中定义与生成模型同名但含义不同的接口。

## 样式和主题

全局样式在 [`src/assets/main.css`](../src/assets/main.css)，Tailwind 4 经 PostCSS 与现有主题加载。主题色用 CSS 变量与语义类，如 `bg-background`、`text-foreground`、`border-border`、`text-muted-foreground`。

- 阵营色：红黑以 `--side-red` / `--side-black` 为唯一来源（恒色），`--board-red` / `--board-black` 转发到它们。可读前景 `--side-red-fg` / `--side-black-fg` 在暗色下提亮。语义类：`bg-side-red`、`text-side-red-fg`、`bg-side-black`、`text-side-black-fg`；棋盘提示色 `bg-board-hint`。
- 评估主色：胜率条与评估曲线统一用红方视角的 `--side-red-fg`；MultiPvList 领先方为红用 `text-side-red-fg`、为黑用 `text-side-black-fg`。
- 棋子面：用独立 `--piece-face`（暖象牙骨白）叠一圈侧色环（红 `--board-red`、黑 `--board-black`）与 `--board-line` 雕边，让平面棋子从同色棋盘底浮现。
- 高度：三档语义刻度 24(`h-6`)/32(`h-8`)/40(`h-10`)；`Button` 映射 default=32、sm=24、lg=40、icon=32。行级交互与表单行归 32，工具栏/复盘/记谱紧凑控件归 24，头部与强调动作归 40。
- 字号：四档 token `text-caption`(10)/`text-body-sm`(11)/`text-body`(12)/`text-title`(14)。`text-body` 与 `text-xs` 等价、`text-title` 与 `text-sm` 等价；散落的 `text-[10px]`/`text-[11px]` 统一为 `text-caption`/`text-body-sm`。

UI 组件经 `cn` 合并类；布局优先 `flex`、`grid`、`gap`。新增组件保持既有 `class` 覆盖、可访问名称与键盘可操作性。
