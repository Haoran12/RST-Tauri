# RST-Tauri 文档总览

Ran's SmartTavern：基于 Tauri 的双模式 AI 聊天应用。

- **SillyTavern 模式**：复刻 SillyTavern 体验，角色卡 V3 + 世界书 + 预设，JSON 存储。
- **Agent 模式**：基于 RP Agent 架构的高级角色扮演系统，分层"客观世界 / 人物具身状态 / 主观认知与意图 / 结果规划与状态更新"，SQLite 存储。

## 文档结构

| 路径 | 内容 | 适用场景 |
|---|---|---|
| [implementation_plan.md](docs/implementation_plan.md) | 项目概述 + 技术栈选型 + 阶段路线图 + 里程碑 + 关键决策 | 想知道"项目到了哪一步、下一步做什么" |
| [01_architecture.md](docs/01_architecture.md) | 总体架构图 + 设计原则 + 前/后端模块结构 + LLM/程序边界总表 + 关键铁律 + 数据形态铁律 | 想理解整体架构与跨系统约束 |
| [70_st_mode.md](docs/70_st_mode.md) | SillyTavern 模式总览：兼容原则 + 文档导航 | 理解 ST 模式总体边界 |
| [71_st_character_cards.md](docs/71_st_character_cards.md) | ST 角色卡：TavernCard V3 + 角色卡导入导出边界 | 实现角色卡管理 |
| [72_st_worldbook_model.md](docs/72_st_worldbook_model.md) | ST 世界书数据模型：外部世界书 + CharacterBook + 转换规则 | 实现世界书存储和编辑 |
| [73_st_worldbook_injection.md](docs/73_st_worldbook_injection.md) | ST 世界书注入流程：来源合并、排序、扫描、递归、预算和落槽 | 实现世界书运行时 |
| [74_st_presets.md](docs/74_st_presets.md) | ST 预设系统：预设类型、导入导出、自动选择、Master Export | 实现预设管理 |
| [75_st_runtime_assembly.md](docs/75_st_runtime_assembly.md) | ST 运行时组装：全局状态、会话 metadata、Provider 参数适配 | 实现生成请求组装 |
| [76_st_regex.md](docs/76_st_regex.md) | ST 正则扩展：脚本数据模型、作用域、运行时替换和导入导出 | 实现 Regex 扩展兼容 |
| [10_agent_data_model.md](docs/10_agent_data_model.md) | Agent 三层语义 + Layer 1/2/3 数据模型 + KnowledgeEntry + CharacterRecord | Agent 数据契约 |
| [11_agent_runtime.md](docs/11_agent_runtime.md) | 三层运行时 + 主循环 + Active Set + 验证规则 + 调用预算 | Agent 运行时编排 |
| [12_agent_simulation.md](docs/12_agent_simulation.md) | 环境 / 灵力档位翻译 + Mana Combat Resolution + Skill Model 契约 | Agent 程序化派生与硬规则解算 |
| [13_agent_llm_io.md](docs/13_agent_llm_io.md) | PromptBuilder + CognitivePass / SceneStateExtractor / OutcomePlanner / SurfaceRealizer I/O + Dirty Flags | Agent LLM 节点提示词与结构化契约 |
| [14_agent_persistence.md](docs/14_agent_persistence.md) | Agent SQLite 表结构、索引与持久化边界 | Agent 存储实现 |
| [20_backend_contracts.md](docs/20_backend_contracts.md) | AIProvider trait + chat_structured + OpenAI/Gemini/Anthropic/DeepSeek/Claude Code Interface 适配范围 | 后端 AI 调用层 |
| [30_logging_and_observability.md](docs/30_logging_and_observability.md) | Agent Trace + 运行 Logs + LLM 请求响应还原 + 异常事件 + 定期清理 | 日志、调试与可观测性 |
| [90_pitfalls_and_tests.md](docs/90_pitfalls_and_tests.md) | 潜在坑点 + 测试用例 / 验证方案 | 风险登记与质量门禁 |

## 参考文档（实现外部依赖与历史参考，不在主文档树）

| 路径 | 内容 |
|---|---|
| [reference/RST_Sch.md](docs/reference/RST_Sch.md) | RST 会话调度模式：ST 模式 vs Agent 模式的目录结构 |
| [reference/SillyTavernLorebook.md](docs/reference/SillyTavernLorebook.md) | SillyTavern 世界书注入判定流程详细分析 |

外部依赖：

- `D:\Projects\RST-flutter\docs\rp_agent_*` — 源自 Flutter 项目的 RP Agent 架构文档，本项目数据模型的概念基础。
- `rp_cards\` — 灵力档位锚点参考（凡人 100 / 入门 500-800 / 大成 2400 / 神祇 8800 等）。
- `SillyTavern\` — SillyTavern 官方实现，角色卡 V3、世界书、预设与 Regex 逻辑的兼容目标。

## 文档维护原则

- 每份文档单一职责边界：路线图 / 架构 / 模式实现 / 数据 / 运行时 / 程序化解算 / LLM I/O / 持久化 / 后端 / 风险。
- **修改时直接更新最新版**，不保留历史对比、版本演进或"改进前后"标记。重大变更走 git commit 而非文档内嵌。
- 概念变更从架构层（`01_architecture.md`）开始，向下传递到具体实现文档。
- 跨文档共享的"铁律"集中在 `01_architecture.md`（数据形态铁律 + LLM/程序边界总表），其他文档引用即可，不重复定义。
