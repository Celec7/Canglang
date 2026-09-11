---
title: Canglang 棋谱与开局库
doc_type: reference
status: current
authority: descriptive
audience: maintainers
canonical: true
summary: 描述 PGN、XQF、揭棋 .cjq、棋谱模型以及本地与云库协同的开局库处理边界。
---

# Canglang 棋谱与开局库

棋谱实现在 [`src-tauri/src/manual`](../src-tauri/src/manual)，开局库实现在 [`src-tauri/src/engine/book`](../src-tauri/src/engine/book)。文件字节与格式解析只在 Rust 服务处理；云库查询同样只在 Rust 完成，前端不直接访问 chessdb.cn。

## 棋谱模型

`ChessManual` 保存标题、日期、红黑双方、赛事、起始 FEN 与根节点。`ManualNode` 是多叉变例树：`children[0]` 是主线后继，后续子节点是变例分支；节点还保存走法、中文记谱、注释与评分。

节点注释是可选的 UTF-8 纯文本，允许换行。根节点注释表示起始局面说明；普通节点注释表示走完该节点着法后的说明，主线和变例使用相同语义且彼此独立。空白注释按无注释处理，非空注释保留换行和文字顺序。PGN 第一手之前的花括号注释归入根节点，着法后的注释归入对应节点；连续注释按出现顺序合并为多行文本。

## PGN

`PgnParser` 把文本解析为 `ChessManual`，`PgnExporter` 导出为 PGN。`ManualService` 对 `.pgn` 先做编码解码再调用解析器；保存输出 UTF-8 无 BOM 的 PGN。IPC 的 `manual_export_pgn` 只返回字符串，不写文件。

## XQF

`XqfParser` 解析二进制 XQF 文件，处理文件头、版本相关密钥、棋子位置、分支与 GBK 注释，并在建立节点前校验每步 ICCS 走法的格式和当前局面合法性。结果统一为 `ChessManual`，前端不需知道 XQF 内部布局。当前备注功能通过 PGN 和规范化 XQF v10 写出。XQF 写出不承诺保留源文件的版本、加密参数、未知头字段或字节级布局；它只生成新的、未加密的 canonical v10 文件。v10 写出只接受单主线树，遇到多分支变例返回明确错误，不扁平化或静默丢弃分支。XQF 的定长文本字段使用 GBK，标题、赛事、日期、双方和节点备注无法编码的 Unicode 字符会使导出失败。

## 揭棋文档

揭棋使用独立的 `.cjq` v1 容器，编码为 UTF-8 JSON 且不带 BOM。顶层记录固定格式与规则标识、对局方式、纯文本元数据、初始信息、ICCS 主线、按半回合定位的纯文本备注和终局。走法只保存 ICCS 与当步公开揭子结果；吃子、将军和自然胜负始终由 Rust 依据揭棋规则重放推导，不接受文件中的派生缓存。

`private_game` 保存标准初始棋位的完整固定身份，用于本地续局；该身份由 Rust 会话事实直接写入磁盘，不经过前端模型。`public_replay` 只保存已经发生的揭子事件，未揭身份保持未知，加载后只允许回看、编辑备注与再次公开导出。两种文件都恢复完整保留主线并把游标置于末端。

读取前先限制文件为 16 MiB，主线最多 10,000 个半回合，单个元数据字段最多 4 KiB，单条备注最多 64 KiB。解析拒绝未知格式、版本或规则、类型不匹配、重复备注键、越界备注、非法固定身份、超量公开棋种、缺失或多余揭子事件、非法 ICCS、自然终局后的走法以及冲突的人工终局。文件内容不触发外部资源读取。

`JieqiDocumentService` 在同目录写临时文件并同步数据，再替换目标；失败时保留旧文件并清理临时文件。揭棋文档不经 `ManualService` 的 PGN/XQF 分派，也不借用 `ConfigService`。

## 服务分派和错误

`ManualService` 按路径扩展名分派 `.pgn` 与 `.xqf`；`.cjq` 由独立的 `JieqiDocumentService` 处理。空路径、文件不存在、扩展名不支持、IO 失败与解析失败分别映射到 `ManualError`。当前普通棋谱导出支持 `.pgn` 和独立的 `.xqf` canonical v10 编码路径，不能改名把任意格式当 PGN 写入；XQF 路径不复用 PGN 文本写出，PGN/XQF 接口也不接受揭棋文档。

解析器保留能表达的棋谱信息；格式错误时返回明确错误或按解析器契约停止，不在 UI 层静默拼接丢失节点。

## 开局库

`OpeningBookService` 统一管理本地 `.bh` 库与象棋云库（chessdb.cn），对前端只暴露一条查询路径。文件实现 `BhOpenBook` 实现 `IOpeningBook` trait，可并发加载多个库；重复路径加载被忽略，查询对当前局面做普通与镜像 hash 匹配，并按评分与胜率降序排列。

云库由 `CloudBookClient` 访问，向 `chessdb.php` 发起 `queryall` 请求，解析 `move`、`egtb`、`score`、`winrate`、`note` 字段；残局库命中标记为「象棋云库 (残局)」。客户端带 10 分钟、最多 512 条的内存缓存，2 秒超时，网络异常或非 200 响应降级为未命中。

`CloudBookMode` 决定两源的协同方式：

| 模式 | 行为 |
| --- | --- |
| `hybrid`（默认） | 本地库有结果即返回；脱谱或无本地库时查询云库 |
| `cloud_only` | 只查询云库 |
| `local_only` | 只查询本地 `.bh` 库，完全离线 |
| `merge` | 同时查询两源，按 ICCS 去重后合并排序 |

`BookMove` 用 ICCS 走法与胜和负统计，同时保留中文记谱、备注与来源。`bookQuery` 在返回前为缺少记谱的候选补齐中文记谱，并过滤格式无效或不符合当前局面的云库候选。开局库查询依赖 `BoardState` 的当前走方与 Zobrist hash，不由前端计算；云库开关与模式属 `BookState` 运行状态，由 `bookSetCloudEnabled`、`bookSetCloudMode` 设置，并持久化在 `AppConfig` 的 `cloudBookEnabled` 与 `cloudBookMode`。

查询接口保持只读列表：单条记录因表不存在、行读取失败或走法编码无法转换而不可用时，按不可用候选处理，返回其余有效结果或空列表；加载路径与文件打开错误仍经 `EngineError` 返回。区分“无匹配”与“数据库损坏”属于 trait 接口语义，不应在 IPC 层由空列表猜测；云库不可用与“无匹配”同样不区分，两者都降级为空结果。
