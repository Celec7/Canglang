<div align="center">

# 沧浪象棋 · Canglang

中国象棋桌面应用：对局、复盘、棋谱与引擎分析。

![license](https://img.shields.io/badge/license-MIT-green)
![rust](https://img.shields.io/badge/Rust-2024-orange)
![tauri](https://img.shields.io/badge/Tauri-v2-8a2be2)
![vue](https://img.shields.io/badge/Vue-3.5-42b883)
![vite](https://img.shields.io/badge/Vite-8-646cff)
![typescript](https://img.shields.io/badge/TypeScript-strict-3178c6)

</div>

## 功能

- **规则**：Rust 负责走法、合法性、将军和胜负判定；支持国标 2020 与亚规 2017。
- **引擎**：内置 Pikafish，可接入 UCI/UCCI 引擎并进行实时分析。
- **棋谱**：导入、导出 PGN/XQF 棋谱，保留变例。
- **开局库**：支持本地 `.bh` 库与象棋云库（chessdb.cn），可离线使用本地库。
- **局面编辑**：用摆子编辑器创建自定义局面。
- **工作区**：深浅主题、快捷键、音效、键盘棋盘和无障碍语音标注。

## 技术栈

| 层 | 技术 |
| --- | --- |
| 规则与领域 | Rust 2024（`src-tauri` → `canglang_app`） |
| 桌面运行时 | Tauri v2 |
| 前端 | Vue 3.5 · TypeScript · Vite 8 |
| 状态 / 样式 | Pinia · Tailwind CSS 4 · Shadcn-Vue（radix-vue、@lucide/vue） |

## 开始

前置：Node.js + pnpm、Rust toolchain，以及 [Tauri 系统依赖](https://tauri.app/start/prerequisites/)。

```sh
pnpm install
pnpm exec tauri dev       # 开发运行（前后端）
pnpm exec tauri build     # 打包桌面应用
```

仅前端调试：

```sh
pnpm dev                  # Vite 开发服务器
pnpm typecheck            # vue-tsc 类型检查
pnpm build                # 构建客户端产物
```

引擎分析在「设置 → 引擎与分析」中配置：可下载 Pikafish，也可添加本地 UCI/UCCI 引擎。

## TODO

- [x] 揭棋模式
- [ ] 实现截图识别局面
- [ ] 实时抓取棋局自动分析
- [ ] 本地联机对战（？）
- [ ] 多语言本地化

## 文档

完整正式文档见 [`docs/`](docs)：

- [架构总览](docs/architecture.md) · [模块边界](docs/module-boundaries.md)
- [IPC 契约](docs/ipc-contract.md) · [中国象棋领域](docs/chess-domain.md)
- [引擎系统](docs/engine.md) · [棋谱与开局库](docs/manual-formats.md)
- [前端开发](docs/frontend.md) · [代码风格](docs/code-style.md)
- [开发指南](docs/development.md) · [测试策略](docs/testing.md) · [术语表](docs/glossary.md)

## 贡献

先读 [AGENTS.md](AGENTS.md) 与 [`docs/`](docs) 中的模块边界与文档维护规则。跨边界模型、生命周期或权限变化需同步更新文档与测试；提交遵循 Conventional Commits。

## 许可证

[MIT](./LICENSE)。
