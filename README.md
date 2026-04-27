# RST-Tauri 文档总览

Ran's SmartTavern：基于 Tauri 的双模式 AI 聊天应用。

- **SillyTavern 模式**：复刻 SillyTavern 体验，角色卡 V3 + 世界书 + 预设，JSON 存储。
- **Agent 模式**：基于 RP Agent 架构的高级角色扮演系统，分层"客观世界 / 人物具身状态 / 主观认知与意图 / 仲裁"，SQLite 存储。

## 文档结构

| 路径 | 内容 | 适用场景 |
|---|---|---|
| [implementation_plan.md](docs/implementation_plan.md) | 项目概述 + 技术栈选型 + 阶段路线图 + 里程碑 + 关键决策 | 想知道"项目到了哪一步、下一步做什么" |
| [01_architecture.md](docs/01_architecture.md) | 总体架构图 + 设计原则 + 前/后端模块结构 + LLM/程序边界总表 + 8 大铁律 + 数据形态铁律 | 想理解整体架构与跨系统约束 |
| [02_st_mode.md](docs/02_st_mode.md) | SillyTavern 模式：角色卡 V3 + 世界书 + 注入流程 | 实现 ST 兼容功能 |
| [10_agent_data_and_simulation.md](docs/10_agent_data_and_simulation.md) | Agent 三层语义 + KnowledgeEntry + 全部 struct + 程序化档位翻译 + 仲裁公式 + SQLite | Agent 数据契约与程序化派生 |
| [11_agent_runtime.md](docs/11_agent_runtime.md) | CognitivePass + Prompt 指南 + 主循环 + Active Set + Dirty Flags + UserInput 解析 + Realizer + 9 条验证规则 + 调用预算 | Agent 运行时与 LLM 调用 |
| [20_backend_contracts.md](docs/20_backend_contracts.md) | AIProvider trait + chat_structured + 多 Provider 实现 | 后端 AI 调用层 |
| [30_logging_and_observability.md](docs/30_logging_and_observability.md) | Agent Trace + 运行 Logs + LLM 请求响应还原 + 异常事件 + 定期清理 | 日志、调试与可观测性 |
| [90_pitfalls_and_tests.md](docs/90_pitfalls_and_tests.md) | 潜在坑点 + 测试用例 / 验证方案 | 风险登记与质量门禁 |

## 参考文档（实现外部依赖与历史参考，不在主文档树）

| 路径 | 内容 |
|---|---|
| [reference/RST_Sch.md](docs/reference/RST_Sch.md) | RST 会话调度模式：ST 模式 vs Agent 模式的目录结构 |
| [reference/SillyTavernLorebook.md](docs/reference/SillyTavernLorebook.md) | SillyTavern 世界书注入判定流程详细分析 |

外部依赖：

- `D:\Projects\RST-flutter\docs\rp_agent_*` — 源自 Flutter 项目的 RP Agent 架构文档，本项目数据模型的概念基础。
- `D:\AI\rp_cards\` — 灵力档位锚点参考（凡人 100 / 入门 500-800 / 大成 2400 / 神祇 8800 等）。
- `D:\AI\SillyTavern\` — SillyTavern 官方实现，角色卡 V3 与世界书逻辑的兼容目标。

## 文档维护原则

- 每份文档单一职责边界：路线图 / 架构 / 模式实现 / 数据 / 运行时 / 后端 / 风险。
- **修改时直接更新最新版**，不保留历史对比、版本演进或"改进前后"标记。重大变更走 git commit 而非文档内嵌。
- 概念变更从架构层（`01_architecture.md`）开始，向下传递到具体实现文档。
- 跨文档共享的"铁律"集中在 `01_architecture.md`（数据形态铁律 + LLM/程序边界总表），其他文档引用即可，不重复定义。
