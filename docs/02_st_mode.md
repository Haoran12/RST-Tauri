# 02 SillyTavern 模式规格

复刻 SillyTavern 体验：角色卡 V3 + 世界书 + 注入流程，JSON 文件存储。

参考依据：
- `D:\AI\SillyTavern\public\scripts\character-card-parser.js`
- `D:\AI\SillyTavern\public\scripts\spec-v2.d.ts`
- 世界书注入完整链路详见 [reference/SillyTavernLorebook.md](reference/SillyTavernLorebook.md)

---

## 1. 角色卡 (TavernCard V3)

```typescript
interface TavernCardV3 {
  spec: 'chara_card_v3';
  spec_version: string;  // >= 3.0 and < 4.0
  data: {
    name: string;
    description: string;
    personality: string;
    scenario: string;
    first_mes: string;
    mes_example: string;
    creator_notes: string;
    system_prompt: string;
    post_history_instructions: string;
    alternate_greetings: string[];
    tags: string[];
    creator: string;
    character_version: string;
    extensions: Record<string, any>;
    character_book?: CharacterBook;
  };
}
```

---

## 2. 世界书 (CharacterBook)

完整字段（覆盖 SillyTavern 全部能力）：

- **匹配**：主关键词 / 次关键词 + 触发逻辑（AND_ANY / NOT_ALL / NOT_ANY / AND_ALL）+ 大小写 / 全词匹配 / 正则
- **插入**：position（7 种枚举：BEFORE_CHAR / AFTER_CHAR / AN_TOP / AN_BOTTOM / AT_DEPTH / EM_TOP / EM_BOTTOM）+ depth + order
- **概率**：probability(0-100) + useProbability
- **递归**：excludeRecursion / preventRecursion / delayUntilRecursion
- **分组**：group / groupOverride / groupWeight / useGroupScoring
- **时间**：sticky / cooldown / delay
- **匹配目标扩展**：matchPersonaDescription / matchCharacterDescription / matchScenario / matchCreatorNotes 等
- **其他**：constant（常驻）/ vectorized / automationId / role

---

## 3. 注入流程

详细判定链路见 [reference/SillyTavernLorebook.md](reference/SillyTavernLorebook.md)。核心要点：

1. 构造 `chatForWI`（聊天上下文）+ `globalScanData`（全局扫描字段）。
2. `getSortedEntries()` 合并 chat / persona / global / character lore，按 `world_info_character_strategy` 排序与去重。
3. `WorldInfoBuffer` 拼接扫描缓冲（聊天深度 + 全局字段 + 扩展注入 + 递归缓冲）。
4. 主循环 `checkWorldInfo()`：每条 entry 顺序判定（disabled → trigger → characterFilter → timed effects → recursion gate → decorators → constant/sticky → 关键词匹配 → selectiveLogic）。
5. Inclusion Group 裁剪（timed → scoring → priority/weighted）。
6. 概率与预算（按 token，非按条目数）。
7. 递归与 MIN_ACTIVATIONS 决定是否再扫一轮。
8. 按 `entry.position` 分流到 `worldInfoBefore / After / Depth / AN / EM / outlet`。
9. 落入最终 Prompt（OpenAI 路径在 `openai.js` 把 before/after 作为 system prompt 片段）。

## 4. 预设
预设是SillyTavern模式Prompt Builder的核心功能. Prompt将按照预设条目的顺序和内容要求, 组建提示词准备发送给LLM.

### 预设条目的字段