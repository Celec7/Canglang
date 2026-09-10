---
title: Canglang 中国象棋领域契约
doc_type: contract
status: current
authority: normative
audience: maintainers
canonical: true
summary: 描述棋盘表示、走法编码、合法性、记谱和对局状态的当前契约。
---

# Canglang 中国象棋领域契约

中国象棋领域实现在 [`src-tauri/src/core`](../src-tauri/src/core)，不依赖 Tauri、Vue、文件 IO 或外部引擎。

## 棋盘和坐标

棋盘为 10 行 9 列、共 90 个位置。Rust 行索引 `0..=9` 从黑方底线到红方底线，列索引 `0..=8` 从 `a` 列到 `i` 列。前端棋盘用同一方向，并在 `src/lib/chess.ts` 做数组与坐标的展示转换。

`BoardState` 以固定大小数组保存棋子与当前走方。标准初始局面由 `INITIAL_FEN` 定义，红方先行。

## FEN

FEN 使用十行棋盘布局与走方两段式表示，例如：

```text
rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w
```

`BoardState::from_fen` 校验行数、列数、棋子字符、走方与双方各一将/帅，只保证结构完整与可表示，不判断是否违反完整棋规（照面王仍可解析，以便规则诊断与错误恢复）。完整校验用 `MoveValidator::validate_board`，另查王宫位置与照面王。`to_fen` 生成规范化字符串。跨边界传 FEN，不传 Rust 内部棋盘数组。

## 走法和规则

`Move` 表示起始到目标位置；ICCS 为四字符，如 `h2e2`。`MoveValidator` 是稳定的规则公共门面，内部按职责位于 `core/rules/attacks.rs`、`generation.rs`、`legality.rs`，负责：

- 棋子伪合法移动与路径检查；
- 目标位置与双方棋子检查；
- 将军、将死与困毙判断；
- 指定棋子或一方合法走法枚举。

走法验证必须在 Rust 完成。前端可请求合法走法做选择高亮，但不得把高亮结果当作最终授权。

## 中文记谱

`NotationConverter` 依走前局面与 `Move` 生成繁体中文记谱，也能在明确局面下从中记谱反推走法。引擎 PV 的中文序列由会话层结合当前局面推导，前端不自行猜测。

## 对局状态

`GameState` 维护当前 `BoardState`、规则档案、`GameResult`、主线历史、悔棋栈与重做栈。

- 合法走子更新局面、交换走方、记录被吃棋子、中文记谱与 Zobrist hash。
- 非法走子不改变局面与历史。
- 新走子清空重做栈。
- 悔棋恢复前一局面，并将走法放入重做栈。
- 重做前重新验证走法，再恢复历史。
- `jump_to` 只移动呈现游标，不改写历史；游标可在起始局面与最新局面之间自由移动。
- 认输只在对局进行中改变结果。
- 绝杀或困毙将当前走方判负；长将、长捉等赛事循环裁决保留在 `rule_assessment` 作为参考提示，不直接改写自然对局结果。

`GameState` 只经只读访问器暴露当前局面、结果与历史；悔棋与重做栈属状态机内部，调用方不得直接修改。失败的走法、撤销或重做不改变状态。

IPC 用 `GameSnapshot` 暴露轻量视图，其中 `history` 是完整主线、`current_ply` 是呈现游标；前端不依赖 `GameState` 的内部栈结构。

## 规则档案

`RuleProfile` 表示对局采用的竞赛规则，当前为 `china2020`（中国象棋协会 2020 规则）与 `asian2017`（亚洲象棋联合会 2017 规则，兼作世界规则兼容档案）；默认 `china2020`。档案经 `set_rule_profile` 切换并随对局状态保存。

规则档案只影响循环裁判的判读，不改变走法生成与合法性。`rule_assessment` 输出机器可读的 `RuleStatus` 与人类可读解释：达到重复阈值时给出长将、长捉等判定提示，但 `result` 仍由绝杀、困毙、认输与自然和棋决定。规则档案语义变化时必须同时更新 `adjudication_tests.rs`。

## Hash

`ZobristHasher` 为局面、走方与镜像开局库查询提供稳定 hash。hash 只用于索引与比较，不是跨边界棋局格式；展示或传输局面仍用 FEN。
