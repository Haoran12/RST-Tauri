# 72 ST 世界书数据模型

本文定义 SillyTavern 世界书的持久化形态、角色卡内嵌 CharacterBook 形态，以及二者之间的转换规则。实现依据 `E:\AIPlay\ST_latest\public\scripts\world-info.js` 与 `E:\AIPlay\ST_latest\src\endpoints\worldinfo.js`。注入执行流程见 [73_st_worldbook_injection.md](73_st_worldbook_injection.md)。

## 1. 两种数据形态

SillyTavern 有两套相关但不同的数据形态：

1. **外部世界书文件**：ST 路径为 `data/<user>/worlds/<name>.json`。文件名 `<name>` 是 UI 选择、角色绑定和聊天 metadata 中使用的世界书名称。文件内容至少必须包含 `entries`。
2. **角色卡内嵌 CharacterBook**：角色卡 spec 的 `data.character_book`，`entries` 是数组。它不会被 ST 运行时直接扫描；必须先由 Import Card Lore 转换为外部世界书。

RST 内部可以使用稳定 `lore_id`，但 ST 兼容导入 / 导出必须保留世界书名称字符串。角色卡 `data.extensions.world`、聊天 `chat_metadata.world_info`、全局 `selected_world_info` / `world_info.globalSelect` 都使用 ST 世界书名称，而不是 provider、model 或 API 配置 ID。

## 2. 外部世界书文件

ST 后端导入和保存世界书时只强制检查对象中存在 `entries` 字段；运行时假设 `entries` 是以 UID 为 key 的对象。

```typescript
interface WorldInfoFile {
  // ST 兼容文件只要求 entries；name 可由文件名决定。
  entries: Record<string, WorldInfoEntry>;

  // 从角色卡内嵌书导入时，ST convertCharacterBook 会保留原始 CharacterBook。
  originalData?: CharacterBook;

  // RST 可增加内部字段，但导出 ST 兼容文件时应按目标决定是否剥离。
  rst_lore_id?: string;
  name?: string;
  description?: string;
  extensions?: Record<string, any>;

  [key: string]: any;
}
```

导入规则：

- 若 `entries` 是对象，key 通常与 `entry.uid` 相同；运行时以对象值为 entry。
- 若为了兼容旧数据遇到数组，RST 可以归一化为对象，但导出 ST 兼容文件时应写回对象。
- 任何 `WorldInfoEntry[]` / `WorldbookEntry[]` 只能作为编辑器、排序、过滤或运行时扫描的临时视图；外部世界书文件、导入导出数据和 ST 兼容持久化格式必须使用 `Record<string, WorldInfoEntry>`。
- 未知字段必须保留。

## 3. WorldInfoEntry

ST 的新建 entry 模板来自 `newWorldInfoEntryTemplate`，字段名使用 camelCase：

```typescript
interface WorldInfoEntry {
  uid: number;

  key: string[];
  keysecondary: string[];
  comment: string;
  content: string;

  constant: boolean;
  vectorized: boolean;
  selective: boolean;
  selectiveLogic: WorldInfoLogic;
  addMemo: boolean;

  order: number;
  position: WorldInfoPosition;
  disable: boolean;
  ignoreBudget: boolean;

  excludeRecursion: boolean;
  preventRecursion: boolean;
  delayUntilRecursion: number | boolean;

  probability: number;
  useProbability: boolean;

  depth: number;
  role: ExtensionPromptRole;
  outletName: string;

  group: string;
  groupOverride: boolean;
  groupWeight: number;
  useGroupScoring: boolean | null;

  scanDepth: number | null;
  caseSensitive: boolean | null;
  matchWholeWords: boolean | null;

  sticky: number | null;
  cooldown: number | null;
  delay: number | null;

  matchPersonaDescription: boolean;
  matchCharacterDescription: boolean;
  matchCharacterPersonality: boolean;
  matchCharacterDepthPrompt: boolean;
  matchScenario: boolean;
  matchCreatorNotes: boolean;

  automationId: string;
  triggers: string[];
  displayIndex?: number;

  characterFilter?: {
    names: string[];
    tags: string[];
    isExclude: boolean;
  };

  extensions?: Record<string, any>;
  [key: string]: any;
}
```

### 默认值

| 字段 | ST 默认值 |
|---|---|
| `key`, `keysecondary`, `triggers` | `[]` |
| `comment`, `content`, `outletName`, `group`, `automationId` | `''` |
| `constant`, `vectorized`, `addMemo`, `disable`, `ignoreBudget`, `excludeRecursion`, `preventRecursion`, `match*`, `groupOverride` | `false` |
| `selective` | `true` |
| `selectiveLogic` | `WorldInfoLogic.AND_ANY` |
| `order` | `100` |
| `position` | `WorldInfoPosition.BEFORE_CHAR` |
| `delayUntilRecursion` | `0` |
| `probability` | `100` |
| `useProbability` | `true` |
| `depth` | `4` |
| `role` | `ExtensionPromptRole.SYSTEM` |
| `groupWeight` | `100` |
| `scanDepth`, `caseSensitive`, `matchWholeWords`, `useGroupScoring`, `sticky`, `cooldown`, `delay` | `null` |

`characterFilter` 不在新建模板中默认写入；UI 需要时才创建 `{ isExclude: false, names: [], tags: [] }`。`names` 存的是角色文件 key / avatar stem，`tags` 存的是 tag id。RST 不能把缺失的 `characterFilter` 和空对象混为一谈，因为 ST 会在清空筛选且未勾选 exclude 时删除该字段。

`displayIndex` 不是 `newWorldInfoEntryTemplate` 的默认字段，但 ST 的编辑器、移动词条和 CharacterBook 转换会写入它；加载旧词条时会用 `uid` 兜底。RST 应保留并导出该字段，不能把它当成纯 UI 临时值丢弃。

## 4. 枚举

```typescript
enum WorldInfoLogic {
  AND_ANY = 0,
  NOT_ALL = 1,
  NOT_ANY = 2,
  AND_ALL = 3,
}

enum WorldInfoPosition {
  BEFORE_CHAR = 0,
  AFTER_CHAR = 1,
  AN_TOP = 2,
  AN_BOTTOM = 3,
  AT_DEPTH = 4,
  EM_TOP = 5,
  EM_BOTTOM = 6,
  OUTLET = 7,
}

enum ExtensionPromptRole {
  SYSTEM = 0,
  USER = 1,
  ASSISTANT = 2,
}
```

## 5. CharacterBook

CharacterBook 结构来自 TavernCard V2/V3 卡数据：

```typescript
interface CharacterBook {
  name?: string;
  description?: string;
  scan_depth?: number;
  token_budget?: number;
  recursive_scanning?: boolean;
  extensions: Record<string, any>;
  entries: CharacterBookEntry[];
}

interface CharacterBookEntry {
  keys: string[];
  content: string;
  extensions: Record<string, any>;
  enabled: boolean;
  insertion_order: number;
  case_sensitive?: boolean;

  name?: string;
  priority?: number;
  id?: number;
  comment?: string;
  selective?: boolean;
  secondary_keys?: string[];
  constant?: boolean;
  position?: 'before_char' | 'after_char';
}
```

## 6. CharacterBook 导入转换

ST `convertCharacterBook(characterBook)` 返回：

```typescript
{
  entries: Record<string, WorldInfoEntry>,
  originalData: characterBook,
}
```

转换规则必须按 ST 当前逻辑实现：

- 若 `CharacterBookEntry.id` 缺失，ST 会直接把数组 index 写回原 entry 的 `id`。
- `id` → `uid`，并作为 `entries[id]` 的 key。
- `keys` → `key`。
- `secondary_keys || []` → `keysecondary`。
- `comment || ''` → `comment`。
- `content` → `content`。
- `constant || false` → `constant`。
- `selective || false` → `selective`。注意这与新建 WorldInfoEntry 默认 `selective = true` 不同。
- `insertion_order` → `order`。
- `enabled` → `disable = !enabled`。
- `!!comment` → `addMemo`。
- `extensions.position ?? (position === 'before_char' ? BEFORE_CHAR : AFTER_CHAR)` → `position`。
- `extensions.exclude_recursion ?? false` → `excludeRecursion`。
- `extensions.prevent_recursion ?? false` → `preventRecursion`。
- `extensions.delay_until_recursion ?? false` → `delayUntilRecursion`。注意 CharacterBook 转换缺省值是 `false`，而新建外部词条模板缺省值是 `0`。
- `extensions.display_index ?? index` → `displayIndex`。
- `extensions.probability ?? 100` → `probability`。
- `extensions.useProbability ?? true` → `useProbability`。这是 ST 当前代码中的 camelCase 字段，不是 `use_probability`。
- `extensions.depth ?? 4` → `depth`。
- `extensions.selectiveLogic ?? AND_ANY` → `selectiveLogic`。
- `extensions.outlet_name ?? ''` → `outletName`。
- `extensions.group ?? ''`、`group_override ?? false`、`group_weight ?? 100` → 分组字段。
- `extensions.scan_depth / case_sensitive / match_whole_words / use_group_scoring` → 对应 camelCase 字段，缺失为 `null`。
- `extensions.automation_id ?? ''` → `automationId`。
- `extensions.role ?? SYSTEM` → `role`。
- `extensions.vectorized ?? false` → `vectorized`。
- `extensions.sticky / cooldown / delay` → timed effects，缺失为 `null`。
- `extensions.match_persona_description / match_character_description / match_character_personality / match_character_depth_prompt / match_scenario / match_creator_notes` → 对应匹配目标布尔值。
- `extensions.triggers || []` → `triggers`。
- `extensions.ignore_budget ?? false` → `ignoreBudget`。
- `extensions` 原样保留到 `WorldInfoEntry.extensions`。

导入内嵌书后，ST 会把转换结果保存为外部世界书文件，并把角色世界书选择切到该世界书名称。未执行导入时，`data.character_book` 只是角色卡数据，不参与扫描。

## 7. 反向导出原则

外部世界书导出为 ST 兼容文件时，默认写 `entries` 对象和 ST 能理解的字段。若该世界书来自 CharacterBook 导入并保留了 `originalData`，导出角色卡时应优先使用 `originalData` 或按上述映射反向构造 `data.character_book`。

RST 不理解的原始字段继续保留在原对象、`extensions` 或 `originalData` 中；不得因为编辑基础字段而丢弃。

## 8. Content 编辑器

`WorldInfoEntry.content` 与 `CharacterBookEntry.content` 使用 [42_structured_text_editor.md](42_structured_text_editor.md) 定义的 Structured Text Editor。

编辑规则：

- 默认模式为 Plain。
- 用户可切换 JSON / YAML，把词条正文组织成 LLM 可读的结构化文本、列表、配置样式文本或示例数据。
- 无论选择哪种模式，ST 世界书 content 保存形态仍是 string，不写成 object / array。
- JSON / YAML 模式保存前自动格式化；解析失败时阻止保存该字段，但不改写原文本。
- Plain 模式只做保守缩进矫正和括号 / 引号 warning；JSON / YAML 格式化只改变注入文本的排版，不改变世界书文件 schema 或运行时注入路径。
- `extensions` 与未知兼容字段可用 JSON 模式编辑，保存时必须保持对象形态并保留未知字段。
