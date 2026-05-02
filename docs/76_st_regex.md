# 76 ST 正则扩展

本文定义 ST 模式的 Regex 扩展兼容目标：数据模型、存储位置、运行时过滤条件、替换语义、UI / 导入导出行为和测试边界。运行时组装见 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)，角色卡扩展字段见 [71_st_character_cards.md](71_st_character_cards.md)，预设结构见 [74_st_presets.md](74_st_presets.md)。

## 1. 参考依据

- `SillyTavern\public\scripts\extensions\regex\engine.js`
- `SillyTavern\public\scripts\extensions\regex\index.js`
- `SillyTavern\public\scripts\extensions\regex\editor.html`
- `SillyTavern\public\scripts\extensions\regex\dropdown.html`
- `SillyTavern\public\scripts\char-data.js`
- `SillyTavern\public\scripts\extensions.js`
- `SillyTavern\public\script.js`
- `SillyTavern\public\scripts\world-info.js`
- `SillyTavern\public\scripts\slash-commands.js`

## 2. 核心语义

Regex 扩展是一组按顺序执行的 find / replace 脚本。脚本可以直接改写聊天文本，也可以只影响显示层或出站 prompt。

脚本来源分三类：

| 类型 | ST 名称 | 存储位置 | 启用门槛 |
|---|---|---|---|
| `global` | Global Scripts | 全局扩展设置 `regex` | 只受脚本自身 `disabled` 与扩展开关控制 |
| `preset` | Preset Scripts | 当前生成预设的扩展字段 `regex_scripts` | 预设名必须在 `preset_allowed_regex[preset_key]` 中 |
| `scoped` | Scoped Scripts | 当前角色卡 `data.extensions.regex_scripts` | 角色头像文件名必须在 `character_allowed_regex` 中 |

运行时合并顺序必须复刻 ST：`global -> preset -> scoped`。每一类内部按数组顺序执行。不要按 `SCRIPT_TYPES` 数值排序；ST 的数值是 `GLOBAL = 0`、`SCOPED = 1`、`PRESET = 2`，但实际合并顺序来自对象声明顺序。

## 3. 数据模型

```typescript
type RegexPlacement = 1 | 2 | 3 | 5 | 6;

interface RegexScriptData {
  id: string;
  scriptName: string;
  findRegex: string;
  replaceString: string;
  trimStrings: string[];
  placement: RegexPlacement[];
  disabled: boolean;
  markdownOnly: boolean;
  promptOnly: boolean;
  runOnEdit: boolean;
  substituteRegex: 0 | 1 | 2;
  minDepth: number | null;
  maxDepth: number | null;
}
```

字段语义：

| 字段 | 语义 |
|---|---|
| `id` | 脚本 UUID。缺失时补新 UUID；导入时强制换新 UUID |
| `scriptName` | UI、导出文件名和 slash command 查找名称 |
| `findRegex` | JS 风格正则字符串，可为 `/pattern/flags`，也可为普通 pattern |
| `replaceString` | 替换模板，支持 `{{match}}` / `$0` / `$1` / `$<name>` |
| `trimStrings` | 对被引用的匹配文本执行精确字符串移除；每项先跑宏替换 |
| `placement` | 脚本作用点，见下表 |
| `disabled` | 禁用脚本；禁用脚本保留但不参与自动运行 |
| `markdownOnly` | 只改聊天显示，不写回聊天文件 |
| `promptOnly` | 只改出站 prompt，不写回聊天文件 |
| `runOnEdit` | 编辑已有消息时是否运行 |
| `substituteRegex` | `findRegex` 中宏替换策略：`0` 不替换、`1` 原样替换、`2` 转义后替换 |
| `minDepth` / `maxDepth` | 深度过滤。无效、空值或调用点未传 depth 时不限制 |

`placement` 枚举：

| 值 | 名称 | 作用点 |
|---|---|---|
| `1` | `USER_INPUT` | 用户消息、impersonate 输出 |
| `2` | `AI_OUTPUT` | AI 输出、角色开场白、备用开场白 |
| `3` | `SLASH_COMMAND` | STscript / narrator 等 slash command 文本 |
| `5` | `WORLD_INFO` | 世界书词条内容，实际只在 `promptOnly` 时生效 |
| `6` | `REASONING` | reasoning block 显示与 prompt 内容 |

兼容迁移：

- 旧 `MD_DISPLAY = 0` 已废弃。若旧脚本只含 `0`，ST 会改为所有非废弃 placement，并设置 `markdownOnly = true`、`promptOnly = true`。
- 旧 `sendAs = 4` 会迁移为 `SLASH_COMMAND = 3`。
- `placement` 缺失或非数组时回填为空数组；脚本会保存但不会自动生效。

## 4. 全局扩展设置

```typescript
interface RegexExtensionSettings {
  regex: RegexScriptData[];
  regex_presets: RegexPreset[];
  character_allowed_regex: string[];
  preset_allowed_regex: Record<string, string[]>; // RST 内部 key 为 preset_key；ST 导入/导出时再映射 apiId
}

interface RegexPreset {
  id: string;
  name: string;
  isSelected: boolean;
  global: RegexPresetItem[];
  scoped: RegexPresetItem[];
  preset: RegexPresetItem[];
}

interface RegexPresetItem {
  id: string;
}
```

`regex_presets` 不是生成参数预设。它只保存三类脚本中“当前启用脚本 ID 列表及顺序”。应用某个 Regex Preset 时：

1. 对每一类目标脚本列表，只有 preset 中列出的脚本会被启用，其余脚本设为 `disabled = true`。
2. 目标脚本列表按 preset 中 ID 顺序重排；未列入 preset 的脚本保留在列表中但禁用。
3. 保存后刷新 Regex UI，并在有当前聊天时重新加载聊天显示。

## 5. 运行时算法

入口函数等价于：

```typescript
function getRegexedString(raw, placement, options): string
```

执行流程：

1. 非字符串输入返回空字符串；空字符串、缺少 placement、扩展被禁用时直接返回原文本。
2. 合并允许运行的脚本：global 全部可见，preset / scoped 必须通过对应 allow list。
3. 对每个脚本依次判断：
   - `disabled` 为真则跳过。
   - `findRegex` 为空或无效则跳过。
   - `placement` 不包含当前作用点则跳过。
   - `isEdit = true` 且 `runOnEdit = false` 时跳过。
   - 调用点传入 `depth` 时，执行 `minDepth` / `maxDepth` 过滤。
   - `markdownOnly` 只在 `isMarkdown = true` 时运行。
   - `promptOnly` 只在 `isPrompt = true` 时运行。
   - 两者都为 false 时，只在非 markdown、非 prompt 的源文本阶段运行。
4. 通过 `runRegexScript()` 对当前累积文本执行替换，后一条脚本读取前一条脚本的输出。

正则编译使用 LRU 缓存，容量 1000。全局或 sticky 正则每次运行前重置 `lastIndex = 0`。

## 6. 替换语义

`findRegex` 先按 `substituteRegex` 处理：

| 值 | 名称 | 行为 |
|---|---|---|
| `0` | `NONE` | 不替换宏 |
| `1` | `RAW` | 对 `findRegex` 执行宏替换，替换结果原样进入正则 |
| `2` | `ESCAPED` | 对宏值做正则转义后再替换 |

替换执行规则：

- `replaceString` 中的 `{{match}}` 先转换为 `$0`。
- `$0` / `$1` / `$2` 等数字引用，以及 `$<name>` 命名捕获引用，会被对应匹配值替换。
- 未匹配到的捕获组替换为空字符串。
- `trimStrings` 只作用于被引用的匹配值，不作用于替换模板中的普通字面量。
- `trimStrings` 每一项先执行宏替换，再从匹配值中 `replaceAll(..., '')`。
- 最终替换结果再执行一次普通宏替换。
- 当前 ST 实现不支持 overlay 策略；RST 第一版也不实现 overlay。

注意：由于 ST 使用 replacement callback 返回最终字符串，`$&`、``$` ``、`$'` 等 JS 原生替换模板能力不应作为兼容承诺；实现应优先兼容 `{{match}}`、数字捕获和命名捕获。

## 7. 注入点

RST 运行时必须在以下位置接入 Regex：

| 阶段 | placement | options | 是否写回聊天文件 |
|---|---|---|---|
| 用户发送消息 | `USER_INPUT` | 默认源文本阶段 | 是 |
| AI 输出清理后入库 | `AI_OUTPUT`，impersonate 用 `USER_INPUT` | 默认源文本阶段 | 是 |
| 新聊天首条角色消息 / 备用开场白 | `AI_OUTPUT` | 默认源文本阶段 | 是 |
| 编辑已有消息 | 按用户 / AI / narrator 判定 | `isEdit = true` | 是，仅运行 `runOnEdit` |
| 聊天消息显示格式化 | 用户 / AI / slash / reasoning | `isMarkdown = true`，带 depth | 否 |
| 出站 prompt 历史消息 | `USER_INPUT` / `AI_OUTPUT` | `isPrompt = true`，带 depth | 否 |
| reasoning 加入 prompt | `REASONING` | `isPrompt = true`，带 depth | 否 |
| reasoning 显示 / 编辑 | `REASONING` | 显示或编辑参数 | 视具体调用点 |
| 世界书词条落槽前 | `WORLD_INFO` | `isPrompt = true`，传 atDepth depth | 否 |
| slash command 文本 | `SLASH_COMMAND` | 默认源文本阶段，必要时传 characterOverride | 取决于命令是否写消息 |

深度语义：

- 聊天显示和 prompt 历史中，`depth = 0` 表示最后一条可用非系统消息，`1` 表示倒数第二条，以此类推。
- continue 生成时，ST 对 prompt 历史 depth 有偏移，避免把待续写前缀当作普通最后消息。
- 世界书只有 `atDepth` 类型词条传入自身 depth；其他世界书位置传 `null`，不触发深度过滤。

## 8. 安全与权限

角色卡和预设可以内嵌 Regex 脚本，因此默认不能静默运行：

- 角色卡内嵌脚本存放在 `data.extensions.regex_scripts`。首次遇到时提示用户允许；允许后把角色 `avatar` 写入 `character_allowed_regex`。
- 预设内嵌脚本存放在 preset 扩展字段 `regex_scripts`。首次遇到时提示用户允许；允许后把当前 preset 名写入 `preset_allowed_regex[preset_key]`。`preset_key` 使用 RST 的预设稳定 ID 或规范化路径，不随当前 API 配置切换而变化；导入 ST 设置时保留原始 `apiId` 分组用于导出兼容，但运行时授权先归一化到 `preset_key`。
- 切换 API 配置不得改变 `character_allowed_regex`、`preset_allowed_regex`、Regex Preset 的选择或脚本启用顺序。
- 删除角色或预设时，应清理对应 allow list 与提示记录。
- 角色 scoped 脚本在群聊中不可编辑；运行时仍应以当前 ST 兼容上下文为准，避免凭空选择某个群成员角色卡脚本。

RST 需要保留这个确认机制。导入角色卡、导入预设或切换预设时，不应自动执行新出现的内嵌脚本。

## 9. UI 与导入导出

第一版需要支持：

- 新建、编辑、删除三类脚本。
- 启用 / 禁用脚本。
- 拖拽或按钮调整脚本顺序。
- 在 global / preset / scoped 之间移动脚本；移动保留同一个 `id`。
- 单脚本导出为 `regex-<scriptName>.json`，内容为单个 `RegexScriptData`。
- 批量导出为 JSON 数组。
- 导入单个对象或数组，用户选择导入目标类型，导入时为每个脚本重新生成 `id`。
- Regex Preset 的创建、更新、应用、删除。
- `/regex`、`/regex-toggle`、`/regex-preset` 三个 slash command。

调试器可后置，但数据模型应预留按当前活动规则链逐步展示转换结果的能力。

Regex 脚本编辑中的大文本字段使用 [42_structured_text_editor.md](42_structured_text_editor.md) 定义的 Structured Text Editor：

- `findRegex`、`replaceString` 和多行 `trimStrings` 编辑区默认 Plain。
- Plain 模式的括号 / 引号诊断只提示文本错配，不替代 Regex 编译校验。
- `findRegex` 是否为合法 JS 风格正则、flags 是否可用、捕获引用是否存在，仍由 Regex 模块 validator 判断。
- 脚本 `extensions`、导入对象和批量导出预览使用 JSON 模式；保存前必须保持 ST 兼容字段与未知字段。

## 10. 测试要求

- `global -> preset -> scoped` 顺序稳定，且每类内部顺序稳定。
- `markdownOnly` 只影响显示，不写回聊天 JSON。
- `promptOnly` 只影响出站 prompt 和世界书注入内容，不写回聊天 JSON。
- 默认脚本会改写用户输入、AI 输出、开场白和编辑保存后的消息。
- `runOnEdit = false` 的脚本不会影响编辑保存。
- `placement` 过滤正确覆盖 user / AI / slash / world info / reasoning。
- `minDepth` / `maxDepth` 在显示和 prompt 组装中生效。
- `substituteRegex = ESCAPED` 会转义宏值，避免宏值被当作正则语法。
- `{{match}}`、`$1`、`$<name>` 与 `trimStrings` 组合行为与 ST 一致。
- 内嵌角色脚本和预设脚本未被允许前不会运行。
- 导入脚本重新生成 `id`，移动脚本不改变 `id`。
- Regex Preset 只保存启用脚本 ID 与顺序；应用 preset 会禁用未列入脚本。
