# 73 ST 世界书注入流程

本文定义 ST 世界书从来源合并到 Prompt 落槽的运行时流程。数据模型见 [72_st_worldbook_model.md](72_st_worldbook_model.md)。实现依据 `E:\AIPlay\ST_latest\public\scripts\world-info.js`，详细判定链路参考 [reference/SillyTavernLorebook.md](reference/SillyTavernLorebook.md)。

## 1. 输入构造

`checkWorldInfo(chatForWI, maxContext, dryRun, globalScanData)` 的输入按 ST 运行时构造：

1. 从本轮 `coreChat` 构造 `chatForWI`：按最近消息在前的顺序反转；若 `world_info_include_names = true`，每条消息格式为 `${name}: ${mes}`，否则只用正文。
2. 构造 `globalScanData`：
   - `personaDescription`
   - `characterDescription`
   - `characterPersonality`
   - `characterDepthPrompt`
   - `scenario`
   - `creatorNotes`
   - `trigger`
3. 把允许参与扫描的扩展注入文本加入 `WorldInfoBuffer.injectBuffer`，例如 Author's Note / quiet prompt 等 `scan = true` 的 extension prompt。

## 2. 世界书来源

`getSortedEntries()` 合并四类来源：

| 来源 | ST 存储位置 | 去重规则 |
|---|---|---|
| Chat lore | 当前聊天 `chat_metadata['world_info']` | 如果同一世界书名称已在 global 中启用，跳过 |
| Persona lore | `power_user.persona_description_lorebook` | 如果同一世界书名称已在 chat 或 global 中启用，跳过 |
| Global lore | 全局设置 `world_info.globalSelect` / `selected_world_info` | 全局选择列表直接参与 |
| Character lore | 角色卡 `data.extensions.world` + `world_info.charLore[].extraBooks` | 如果同名书已在 global、chat 或 persona 中启用，跳过；剩余内容与 global 按 `world_info_character_strategy` 合并 |

`data.extensions.world` 在 ST 中是世界书名称字符串；`world_info.charLore[].extraBooks` 按角色文件名匹配，存放额外世界书名称列表。加载世界书后，ST 把 `data.entries` 对象转为 entry 数组，并给每个 entry 加上 `world: worldName`。

角色卡内嵌 `data.character_book` 不直接参与扫描；必须先通过 Import Card Lore 转换并保存为外部世界书。若未执行导入，运行时只保留该内嵌书作为角色卡数据。

RST 内部可用稳定 `lore_id` 管理资源，但 ST 兼容运行时和导出必须能映射回世界书名称字符串。来源合并不得读取 `active_api_config_id`。切换 API 配置后，Chat lore、Persona lore、Global lore、Character lore 的选择、去重、排序、递归和预算规则保持不变；只有最终 Provider 请求映射可能变化。

## 3. 排序与装饰器

排序规则：

1. `Chat lore` 永远最前。
2. `Persona lore` 次之。
3. `Global lore` 与 `Character lore` 依据 `world_info_character_strategy`。ST 默认值是 `character_first`：
   - `0 evenly`：两者合并后统一按 `order` 降序。
   - `1 character_first`：Character 内部按 `order` 降序，然后 Global 内部按 `order` 降序。
   - `2 global_first`：Global 内部按 `order` 降序，然后 Character 内部按 `order` 降序。
4. 每个 entry 解析内容前缀装饰器 `@@activate` / `@@dont_activate`，并计算 hash 供 timed effects 使用。

## 4. 扫描与激活

单轮扫描按排序后的 entry 依次判定：

1. 计算 token 预算：`round(world_info_budget * maxContext / 100)`，若 `world_info_budget_cap > 0` 则取 cap。
2. 初始化 timed effects（sticky / cooldown / delay）。
3. 跳过已失败概率 / 已激活去重的 entry。
4. 跳过 `disable = true` 的 entry。
5. 检查 `triggers` 是否包含本次 `globalScanData.trigger`。
6. 检查 `characterFilter.names / tags`。
7. 检查 delay / cooldown / sticky。
8. 检查 recursion gate：`delayUntilRecursion`、`excludeRecursion`。
9. 应用 decorators：`@@activate` 强制激活，`@@dont_activate` 禁止激活。
10. 应用外部强制激活。
11. `constant` 或 active sticky 直接进入候选。
12. 构造当前 entry 的扫描文本：`scanDepth ?? world_info_depth` 决定最多扫描多少条最近消息；若 `scanDepth` 小于等于当前递归 / min-activation 的起始深度，本轮扫描文本为空。
13. 匹配主关键词 `key`。
14. 若 `selective = true`，继续匹配 `keysecondary` + `selectiveLogic`。

候选产生后：

1. 对本轮候选执行 Inclusion Group 裁剪。
2. 执行概率判定；sticky entry 不重新掷概率。
3. 执行 token 预算；`ignoreBudget = true` 的 entry 不受预算限制。
4. 若 `world_info_recursive` 且本轮通过概率检查的 entry 中存在未 `preventRecursion` 的内容，把它们加入递归缓冲并继续扫描；ST 当前代码是在预算检查后继续用这些内容推进递归，因此 RST 若要严格复刻，应按代码行为而不是只按最终 prompt 落槽集合判断。
5. 若配置了 `world_info_min_activations` 且未达到，增加扫描深度继续；`world_info_max_recursion_steps` 与 min activations 互斥。

## 5. Prompt 落槽

扫描结束后，ST 先对每个最终激活 entry 的 `content` 执行 `WORLD_INFO` placement 的 prompt-only Regex；只有 `AT_DEPTH` 词条会把自身 `depth` 传给 Regex 深度过滤，其他位置传 `null`。随后按 `entry.position` 分流：

| position | 输出 |
|---|---|
| `BEFORE_CHAR` | `worldInfoBefore` |
| `AFTER_CHAR` | `worldInfoAfter` |
| `EM_TOP / EM_BOTTOM` | 示例消息前后插入 |
| `AN_TOP / AN_BOTTOM` | 作者注释上下拼接 |
| `AT_DEPTH` | `worldInfoDepth[]`，按 `depth + role` 聚合 |
| `OUTLET` | `outletEntries[outletName]` |

落槽后的继续组装：

1. `worldInfoBefore / worldInfoAfter` 进入 Context Template 的 `wiBefore / wiAfter / loreBefore / loreAfter`。
2. `EM_*` 在示例消息数组生成前插入。
3. `AT_DEPTH` 通过 extension prompt 写入聊天深度位置。
4. `OUTLET` 写入命名 outlet，供扩展或 prompt 片段引用。
5. OpenAI Chat Completion 路径还会在 OpenAI 消息组装阶段把 before / after 作为 system prompt 片段加入消息序列；非 OpenAI 路径通过文本补全 prompt slot 注入。
