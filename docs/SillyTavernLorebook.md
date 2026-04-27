# SillyTavern 世界书（Lore / World Info）注入判定流程

本文整理 `D:\AI\SillyTavern` 当前实现中，Lore 条目从“候选”到“真正注入 Prompt”的完整判定链路。重点是**执行顺序**与**分支条件**，避免只看单个开关导致误判。

## 1. 入口与总体链路

核心调用关系：

1. 在生成前构造扫描输入（聊天文本 + 全局扫描数据）。
2. 调用 `getWorldInfoPrompt(...)`。
3. 内部调用 `checkWorldInfo(...)` 执行主判定循环。
4. 返回 `worldInfoBefore/worldInfoAfter/worldInfoDepth/...`，再按不同 API 路径注入。

关键位置：

- `D:\AI\SillyTavern\public\script.js:4535`  
  构造 `chatForWI`，并调用 `getWorldInfoPrompt`。
- `D:\AI\SillyTavern\public\scripts\world-info.js:892`  
  `getWorldInfoPrompt(...)` 包装层。
- `D:\AI\SillyTavern\public\scripts\world-info.js:4579`  
  `checkWorldInfo(...)` 主扫描与判定逻辑。
- `D:\AI\SillyTavern\public\scripts\openai.js:1338`  
  OpenAI 聊天补全路径将 `worldInfoBefore/After` 作为 system prompt 片段。

---

## 2. 扫描输入如何构成

### 2.1 聊天扫描串 `chatForWI`

在 `script.js:4535`：

- 原始源：`coreChat`。
- 每条消息可选是否带名字（`world_info_include_names`）：
  - 开：`"${x.name}: ${x.mes}"`。
  - 关：仅 `x.mes`。
- 最后 `reverse()`，即按“近到远”顺序供 WI 扫描。

### 2.2 全局扫描字段 `globalScanData`

在 `script.js:4537` 附近传入：

- `personaDescription`
- `characterDescription`
- `characterPersonality`
- `characterDepthPrompt`
- `scenario`
- `creatorNotes`
- `trigger`（本次生成触发类型）

这些字段**是否参与匹配**由每个条目的 `matchPersonaDescription / matchCharacterDescription / ...` 控制。

---

## 3. 候选条目集合如何得到

`getSortedEntries()`（`world-info.js:4460`）会合并四类来源：

1. Global lore（已选全局世界书）。
2. Character lore（角色绑定世界书 + 角色额外书）。
3. Chat lore（聊天绑定世界书）。
4. Persona lore（人设绑定世界书）。

顺序规则：

1. Chat lore 永远最前。
2. Persona lore 次之。
3. Global 与 Character 按 `world_info_character_strategy` 合并：
   - `evenly`
   - `character_first`
   - `global_first`

去重/避重逻辑：若同一本书已在更高优先来源激活，会被跳过（例如 chat lore 已激活后，global 不再重复加载该书）。

额外预处理：

- 解析内容前缀装饰器：`@@activate` / `@@dont_activate`（`parseDecorators`）。
- 为条目计算 hash（用于 timed effects 追踪）。

---

## 4. 扫描缓冲区（WorldInfoBuffer）怎么拼

类：`WorldInfoBuffer`（`world-info.js:199`）。

`buffer.get(entry, scanState)`（`world-info.js:279`）拼接顺序：

1. 聊天深度窗口：
   - 使用 `entry.scanDepth`，否则用全局 `world_info_depth + skew`。
2. 条目启用的全局字段（persona/角色描述等）。
3. 允许被扫描的扩展注入文本（`context.extensionPrompts[key]?.scan`）。
4. 递归缓冲 `#recurseBuffer`：
   - 仅在非 `MIN_ACTIVATIONS` 状态下参与匹配。

注意：

- 深度非法会被截断或直接返回空。
- 大小写与整词匹配支持“条目级覆盖全局级”。

---

## 5. 单条目匹配规则（关键词层）

函数：`matchKeys(...)`（`world-info.js:337`）。

优先级：

1. 若 key 是 `/.../flags` 形式，按正则匹配（`parseRegexFromString`）。
2. 否则普通文本匹配：
   - `caseSensitive` 生效后决定是否统一转小写。
   - `matchWholeWords` 打开时：
     - 多词短语走 `includes`。
     - 单词走边界正则（避免子串误命中）。
   - 关闭时直接 `includes`。

---

## 6. 主判定循环（最关键）

`checkWorldInfo(...)` 的 `while (scanState)` 循环，每轮都按**固定顺序**对每条 entry 处理。

### 6.1 扫描前全局初始化

在 `world-info.js:4579` 后：

- 计算预算：
  - `budget = round(world_info_budget * maxContext / 100)`，至少 1。
  - 若设置 `world_info_budget_cap`，再取上限。
- 创建 `WorldInfoTimedEffects` 并先执行 `checkTimedEffects()`。
- 计算可用的 `delayUntilRecursion` 层级队列。

### 6.2 每条 entry 的门控顺序（严格按代码先后）

出现一下任何一种情况即跳过当前条目：

1. 已在 `failedProbabilityChecks` 中，或已在 `allActivatedEntries` 中。
2. `disable == true`。
3. `triggers` 不包含当前 `globalScanData.trigger`。
4. `characterFilter`（名字/标签，含 include/exclude 模式）不通过。
5. timed effects 抑制：
   - `delay` 激活中：抑制。
   - `cooldown` 激活且`not sticky`：抑制。
6. 递归相关门控：
   - 当前非 `RECURSION`，且 `delayUntilRecursion` 为真：抑制。
   - 当前是 `RECURSION`，但条目要求更高延迟层级：抑制。
   - 当前是 `RECURSION` 且条目 `excludeRecursion`：抑制。
7. 装饰器强制：
   - `@@activate`：直接激活。
   - `@@dont_activate`：直接抑制。
8. 外部强制激活命中（`WORLDINFO_FORCE_ACTIVATE`）：直接激活。
9. `constant`：直接激活。
10. sticky 当前活跃：直接激活。
11. 无主关键词 `key`：跳过。
12. 主关键词匹配失败：跳过。
13. 无副关键词或 selective 不成立：主关键词命中即激活。
14. 有副关键词时按 `selectiveLogic` 判定：
    - `AND_ANY`：任一副关键词命中。
    - `NOT_ALL`：存在至少一个副关键词不命中。
    - `NOT_ANY`：全部副关键词都不命中。
    - `AND_ALL`：全部副关键词都命中。

通过后进入 `activatedNow`（“候选激活集”）。

---

## 7. 组竞争（Inclusion Group）裁剪

函数：`filterByInclusionGroups(...)`（`world-info.js:5251`）。

仅对 `group` 非空条目生效，步骤如下：

1. 先按 group 名分桶（支持 `a,b,c` 多组归属）。
2. timed effects 先裁：
   - 有 sticky 的组，仅保留 sticky 条目。
   - 组内 cooldown/delay 条目移除。
3. 可选 group scoring（全局 `world_info_use_group_scoring` 或条目 `useGroupScoring`）：
   - 计算 `buffer.getScore(entry, scanState)`。
   - 分数低于组内最高分者淘汰。
4. 若该 group 以前轮已激活过，当前轮同组新条目全部淘汰。
5. 若有 `groupOverride`，按 `order` 最高者胜。
6. 否则按 `groupWeight` 做加权随机，仅保留 1 条。

---

## 8. 概率与预算（进入最终激活前最后两关）

位置：`world-info.js:4880` 起。

### 8.1 概率

- `useProbability == false` 或 `probability == 100`：直接通过。
- sticky 活跃条目不重掷概率。
- 否则 `Math.random()*100 <= probability` 才通过。
- 概率失败条目加入 `failedProbabilityChecks`，后续轮次直接跳过。

### 8.2 预算

- 预算按 token 计，不是按条目数。
- 本轮累积内容若使 token 达到/超过预算：
  - 非 `ignoreBudget` 条目不加入最终激活。
  - 首次溢出会标记 `token_budget_overflowed = true`。
- `ignoreBudget == true` 条目可在预算溢出后继续通过（代码专门保留该通道）。

---

## 9. 递归、最小激活与停止条件

### 9.1 递归触发

满足以下条件会进入下一轮 `RECURSION`：

1. 开启 `world_info_recursive`。
2. 未预算溢出。
3. 本轮有“成功激活且未 `preventRecursion`”的新条目。

进入递归时，会把这些条目内容加入 `#recurseBuffer`，使后续条目可被“条目内容”再次触发。

### 9.2 最小激活补扫（MIN_ACTIVATIONS）

当准备停止但 `allActivatedEntries.size < world_info_min_activations` 且未溢出：

1. 若未超过 `world_info_min_activations_depth_max`（或聊天长度上限），则：
   - `buffer.advanceScan()`（增加全局扫描深度）。
   - 下一轮状态设为 `MIN_ACTIVATIONS`。
2. 否则停止。

注意：`MIN_ACTIVATIONS` 状态扫描时，不包含递归缓冲；若该轮后存在递归缓冲，会先补一轮 `RECURSION`。

### 9.3 递归层级延迟（delayUntilRecursion）

存在未处理层级时，即使本轮原本要停，也会继续 `RECURSION` 并切到下一 delay level。

### 9.4 最大递归步数

若 `world_info_max_recursion_steps > 0` 且循环次数达到上限，直接停止。  
UI 层还保证它与 `min_activations` 互斥（一个非 0 时另一个会被置 0）。

---

## 10. 激活后如何落入 Prompt

构建阶段在 `world-info.js:5057` 起，按 `entry.position` 分流：

- `before` -> `worldInfoBefore`
- `after` -> `worldInfoAfter`
- `EMTop/EMBottom` -> 示例消息注入
- `ANTop/ANBottom` -> 作者注释上下拼接
- `atDepth` -> 深度注入（按 depth + role 聚合）
- `outlet` -> outlet 名称分桶

随后：

1. `getWorldInfoPrompt()` 返回这些分流结果。
2. 非 OpenAI 路径在 `script.js` 中写入对应扩展注入槽位。
3. OpenAI 路径在 `openai.js` 中把 `worldInfoBefore/After` 作为 system prompt 片段加入消息序列。

---

## 11. Timed Effects（sticky/cooldown/delay）对判定的真实影响

类：`WorldInfoTimedEffects`（`world-info.js:479`）。

### 11.1 sticky

- 条目激活后可写入 sticky 区间（按聊天长度计时）。
- sticky 活跃时：
  - 条目会被直接激活（不依赖关键词）。
  - 概率检查不重掷。
  - 在 group 冲突中优先保留 sticky。

### 11.2 cooldown

- cooldown 活跃且条目非 sticky：直接抑制，不进入激活。
- sticky 结束时若配置 cooldown，会立刻登记 cooldown（含保护标记）。

### 11.3 delay

- `chat.length < entry.delay` 时，条目处于 delay 活跃状态并被抑制。

---

## 12. 一页式伪代码（便于快速定位问题）

```text
build chatForWI + globalScanData
entries = getSortedEntries()
init timedEffects, budget, scanState=INITIAL

while scanState != NONE:
  activatedNow = []
  for entry in entries:
    if failedProb or alreadyActivated: continue
    if disabled/trigger/charFilter/timed/delayRecursionGate fail: continue
    if @@dont_activate: continue
    if @@activate or externalActivate or constant or sticky: activatedNow += entry; continue
    if no primary key: continue
    if primary not match: continue
    if has secondary:
      if selectiveLogic fail: continue
    activatedNow += entry

  apply inclusion-group filtering (timed -> scoring -> priority/weighted winner)
  for entry in activatedNow:
    if probability fail: mark failed; continue
    if budget overflow and not ignoreBudget: continue/break
    allActivated += entry

  decide next scan state:
    recursion? min-activations? delayed-recursion-level?
    else stop

build worldInfoBefore/After/Depth/AN/EM/outlet from allActivated
persist timedEffects
return prompt parts
```

---

## 13. 排障建议（最常见“为什么没注入”）

按以下顺序排查最快：

1. 条目是否被 `disable` / `triggers` / `characterFilter` 卡住。
2. 是否处在 `delay` 或 `cooldown`。
3. 是否被 `delayUntilRecursion` 卡在非递归轮。
4. 关键词是否因大小写、整词、regex 语法导致不命中。
5. 副关键词逻辑（`selectiveLogic`）是否与预期相反。
6. 是否在 inclusion group 中被淘汰（sticky、score、override、weight）。
7. 是否概率失败并进入 `failedProbabilityChecks`。
8. 是否命中预算上限且非 `ignoreBudget`。
9. 是否被 `preventRecursion` 影响后续连锁触发。

