# 70 SillyTavern 模式总览

ST 模式的目标是复刻 SillyTavern 的聊天体验：角色卡 V3、世界书、预设、聊天元数据与 Prompt 注入流程尽量兼容 SillyTavern 当前实现；RST 只在 ST 资源与 API Provider 绑定关系上做有意偏离。

## 1. 文档边界

| 文档 | 职责 |
|---|---|
| [71_st_character_cards.md](71_st_character_cards.md) | TavernCard V3 角色卡结构、导入导出边界、角色卡与世界书的引用关系 |
| [72_st_worldbook_model.md](72_st_worldbook_model.md) | 外部世界书、角色卡内嵌 CharacterBook、字段默认值与双向转换 |
| [73_st_worldbook_injection.md](73_st_worldbook_injection.md) | 世界书来源合并、排序、扫描、递归、预算和 Prompt 落槽 |
| [74_st_presets.md](74_st_presets.md) | ST 预设类型、导入导出、自动选择，以及与 API Provider 解耦的规则 |
| [75_st_runtime_assembly.md](75_st_runtime_assembly.md) | 全局状态、会话 metadata、运行时请求组装和 Provider 参数适配边界 |
| [76_st_regex.md](76_st_regex.md) | Regex 扩展的数据模型、作用域、替换语义、运行时挂点和导入导出 |

本文只保留跨文档原则和导航，不承载具体数据结构。

## 2. 兼容原则

- **数据模型优先兼容 SillyTavern 当前实现**：外部世界书 `entries: { [uid]: entry }`、角色卡内嵌 `character_book.entries[]`、聊天文件头部 `chat_metadata`、角色卡 `data.extensions.world`、全局 `settings.world_info.globalSelect` 等约定都按 ST 现状处理。
- **运行时流程优先复刻 SillyTavern**：世界书来源、去重、排序、扫描缓冲、递归、预算与落槽顺序均以 `SillyTavern` 的实际代码为准。
- **未知字段必须保留**：导入角色卡、世界书、预设和聊天 metadata 时，未被 RST 显式理解的字段不得丢弃，保存 / 导出时应写回原结构。
- **有意偏离点只有一类**：RST 解绑 Preset / 世界书运行时选择与 API Provider。导入 / 导出尽量保留 ST 预设 JSON 原字段，但运行时不把 preset、世界书、角色卡世界书绑定、聊天 world_info metadata 或 Regex 授权状态绑定到 provider endpoint、key、model 或 connection profile。
- **API 切换无 ST 资源副作用**：切换 `active_api_config_id` 只改变下一次请求使用的连接配置和 Provider 参数映射，不触发自动选择预设、切换世界书、重跑世界书绑定迁移、改变 Regex allow list、改写聊天 metadata 或保存资源文件。

## 3. 参考依据

- `SillyTavern\public\scripts\character-card-parser.js`
- `SillyTavern\src\types\spec-v2.d.ts`
- `SillyTavern\public\scripts\world-info.js`
- `SillyTavern\public\script.js`
- `SillyTavern\public\scripts\preset-manager.js`
- `SillyTavern\public\scripts\extensions\regex\engine.js`
- `SillyTavern\public\scripts\extensions\regex\index.js`
- `SillyTavern\src\endpoints\presets.js`
- `SillyTavern\src\endpoints\characters.js`
- 世界书注入完整链路详见 [reference/SillyTavernLorebook.md](reference/SillyTavernLorebook.md)

## 4. 数据流总览

```
角色卡 V3 + 聊天记录 + chat_metadata
       ↓
加载 Character lore / Chat lore / Persona lore / Global lore
       ↓
世界书转换、去重、排序、扫描、递归、预算裁剪
       ↓
生成 before / after / depth / AN / EM / outlet 注入结果
       ↓
Regex prompt-only 处理聊天历史 / 世界书 / reasoning
       ↓
Context Template + Instruct Template + Prompt Preset 组装 Prompt
       ↓
当前 API 配置 + 当前预设参数映射到 Provider 请求
       ↓
AIProvider.chat() 或 chat_stream()
```

ST 模式只做文本聊天和 Prompt 组装，不承担 Agent 模式的世界状态演化、认知节点调度或结构化 LLM I/O 职责。
