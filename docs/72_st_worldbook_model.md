# 72 ST 世界书数据模型

本文定义 SillyTavern 世界书的持久化形态、角色卡内嵌 CharacterBook 形态，以及二者之间的转换规则。注入执行流程见 [73_st_worldbook_injection.md](73_st_worldbook_injection.md)。

## 1. 两种数据形态

SillyTavern 有两套相关但不同的数据形态：

1. **外部世界书文件**：`data/<user>/worlds/<name>.json`，运行时原生读取，`entries` 是以 UID 为 key 的对象。
2. **角色卡内嵌 CharacterBook**：角色卡 spec 的 `data.character_book`，`entries` 是数组；ST 运行时会先转换成外部世界书 entry 形态再参与扫描。

RST 必须同时支持两者，并在导入 / 导出时保留未知字段和 `extensions`。

世界书身份与 API 配置无关。外部世界书文件名 / 内部稳定 ID、角色卡 `data.extensions.world` 绑定、聊天 `chat_metadata.world_info` 绑定和全局 `world_info.globalSelect` 都不得以 `apiId`、Provider 类型、model 或 connection profile 作为命名空间。切换 API 配置不会复制、重命名、迁移、启用、禁用或重新选择任何世界书。

## 2. 外部世界书文件

```typescript
interface WorldInfoFile {
  name?: string;
  description?: string;
  extensions?: Record<string, any>;

  // SillyTavern 当前实现使用对象而不是数组。
  // key 通常与 entry.uid 相同；导入时以 entry.uid 为准，缺失时可用对象 key 补齐。
  entries: Record<string, WorldInfoEntry>;

  // 从角色卡内嵌书导入时，ST 会保留原始 CharacterBook。
  originalData?: CharacterBook;
}
```

## 3. WorldInfoEntry

```typescript
interface WorldInfoEntry {
  uid: number;

  // 匹配
  key: string[];
  keysecondary: string[];
  selective: boolean;
  selectiveLogic: WorldInfoLogic;
  caseSensitive: boolean | null;
  matchWholeWords: boolean | null;
  scanDepth: number | null;

  // 内容与排序
  comment: string;
  content: string;
  constant: boolean;
  vectorized: boolean;
  order: number;                 // ST sortFn 为 b.order - a.order
  position: WorldInfoPosition;
  depth: number;                 // atDepth 使用，默认 4
  role: ExtensionPromptRole;     // atDepth role，0=system, 1=user, 2=assistant
  outletName: string;

  // 状态与预算
  disable: boolean;
  ignoreBudget: boolean;
  probability: number;           // 0-100
  useProbability: boolean;

  // 递归
  excludeRecursion: boolean;
  preventRecursion: boolean;
  delayUntilRecursion: boolean | number;

  // 分组
  group: string;
  groupOverride: boolean;
  groupWeight: number;           // 默认 100
  useGroupScoring: boolean | null;

  // timed effects
  sticky: number | null;
  cooldown: number | null;
  delay: number | null;

  // 扫描目标扩展
  matchPersonaDescription: boolean;
  matchCharacterDescription: boolean;
  matchCharacterPersonality: boolean;
  matchCharacterDepthPrompt: boolean;
  matchScenario: boolean;
  matchCreatorNotes: boolean;

  // 角色过滤与触发类型
  characterFilter?: {
    names?: string[];
    tags?: string[];
    isExclude?: boolean;
  };
  triggers: string[];

  // UI / 自动化 / 扩展
  addMemo: boolean;
  displayIndex?: number;
  automationId: string;
  extensions?: Record<string, any>;
}
```

默认值按 ST `newWorldInfoEntryTemplate` 回填。导入缺字段的旧文件时，必须补齐默认值；未知字段不得丢弃。

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

## 6. 转换规则

CharacterBook 到 WorldInfoEntry：

- `id` 缺失时用数组 index 补齐，并映射为 `WorldInfoEntry.uid`。
- `keys` → `key`。
- `secondary_keys` → `keysecondary`。
- `insertion_order` → `order`。
- `enabled` → `!disable`。
- `position: before_char / after_char` 只表达基础 before / after。
- ST 扩展位置（AN / depth / EM / outlet）从 `entry.extensions.position` 读取。
- `extensions` 中的 `exclude_recursion / prevent_recursion / delay_until_recursion / probability / depth / selectiveLogic / group / role / sticky / cooldown / delay / match_* / triggers / ignore_budget` 等字段转换到运行时 entry 对应字段。

WorldInfoEntry 到 CharacterBook：

- `uid` → `id`。
- `key` → `keys`。
- `keysecondary` → `secondary_keys`。
- `order` → `insertion_order`。
- `!disable` → `enabled`。
- 基础 before / after 可写入 `position`。
- ST 扩展字段写入 `entry.extensions`。
- RST 不理解的原始字段继续保留在 `extensions` 或原始对象中。
