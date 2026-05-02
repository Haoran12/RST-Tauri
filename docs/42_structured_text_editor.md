# 42 结构化文本编辑器

本文档定义 RST 通用 Structured Text Editor：在 ST 模式和 Agent 模式中编辑 `content`、模板正文、扩展字段和配置片段时共享的文本编辑能力。首版基于 CodeMirror 6 实现，RST 只封装业务绑定、模式切换、diagnostics 映射和少量输入辅助扩展，不自研编辑器核心。它只负责文本形态、格式化、括号 / 引号诊断和前端交互，不替代各业务模块的 schema 校验、权限校验、注入逻辑或 Agent World Editor 提交门禁。

相关基础文档：

- 总体架构、数据形态铁律与关键边界见 [01_architecture.md](01_architecture.md)。
- 前后端模块结构见 [02_app_data_and_modules.md](02_app_data_and_modules.md)。
- 前端交互主框架见 [41_frontend_interaction.md](41_frontend_interaction.md)。
- ST 世界书、预设和 Regex 见 [72_st_worldbook_model.md](72_st_worldbook_model.md)、[74_st_presets.md](74_st_presets.md)、[76_st_regex.md](76_st_regex.md)。
- Agent World Editor 见 [40_agent_world_editor.md](40_agent_world_editor.md)。

---

## 1. 目标与边界

### 1.1 首版目标

Structured Text Editor v1 支持：

1. **CodeMirror 6 编辑器底座**：使用 CM6 的 `EditorView`、undo history、bracket matching / closing、language support、lint diagnostics 和 keymap，不用 textarea 自研核心编辑体验。
2. **三种编辑模式**：`Plain`、`JSON`、`YAML`。
3. **语言包可扩展**：首版内置 Plain / JSON / YAML；后续可通过受控语言注册表启用 Markdown、TOML、XML、JavaScript 等 CM6 language support。
4. **编辑时自动缩进**：JSON / YAML 模式按语言规则和常见写法在换行、输入冒号 / 逗号 / 括号时调整缩进，保留 undo 历史。
5. **括号 / 引号匹配检查**：实时标出未闭合、错配、非法转义和不可解析位置。
6. **保守自动修正**：JSON 模式可把确定属于对象键的未加引号键名修正为英文双引号形式；不确定时只给 quick fix。
7. **保存前诊断**：父级表单保存前必须读取 editor diagnostics；存在 blocker 时禁止保存。
8. **跨模式复用**：ST 世界书 / 预设 / Regex 与 Agent World Editor 使用同一组件和诊断模型。

### 1.2 明确不做

- 不把 Plain 文本自动理解为世界事实、Regex、prompt 规则或 Agent Truth。
- 不 fork 或重写 CodeMirror 的核心编辑状态、选择、undo、光标移动、基础缩进与括号匹配机制。
- 不从普通用户配置中执行任意 JavaScript 语言包；第三方语言包必须来自应用预装、受信插件或显式允许的扩展机制。
- 不用 LLM 修复用户内容。
- 不在编辑器内部执行世界书注入、Regex 替换、Provider 参数映射或 Agent Knowledge 访问判断。
- 不绕过父级资源的保存流程；编辑器只修改 parent draft，不直接写 JSON 文件或 SQLite。
- 不把格式化失败的内容静默改写为“看起来合法”的另一种语义。

---

## 2. 适用场景

### 2.1 ST 模式

ST 资源中的大文本字段默认接入 Structured Text Editor：

| 资源 | 字段 | 默认模式 | 保存形态 |
|---|---|---|---|
| 世界书 | `WorldInfoEntry.content` | Plain | string |
| CharacterBook | `CharacterBookEntry.content` | Plain | string |
| 预设 | `SystemPrompt.content`、`PromptItem.content`、`story_string`、`wi_format` 等模板正文 | Plain | string |
| Regex | `findRegex`、`replaceString`、多行 `trimStrings` 编辑区 | Plain | string / string[] |
| 兼容扩展 | `extensions`、未知字段查看 / 编辑 | JSON | object |

ST 的 `content`、prompt 模板和其他 string 字段可以选择 `Plain`、`JSON` 或 `YAML` 任一模式。选择 JSON / YAML 时，用户是在用结构化文本组织 prompt 内容，方便 LLM 阅读和遵循；保存后仍是格式化后的字符串，ST 运行时不把它当作业务对象解析，也不改变 SillyTavern 兼容文件形态。

### 2.2 Agent 模式

Agent World Editor 中的结构化内容字段接入 Structured Text Editor：

| 资源 | 字段 | 默认模式 | 保存形态 |
|---|---|---|---|
| KnowledgeEntry | `content`、`apparent_content`、`self_belief` | JSON | `serde_json::Value` |
| World rules | `world_base.yaml` 高级编辑视图 | YAML | YAML 文件，经 ConfigValidator 编译 |
| 扩展字段 | `extensions`、`metadata`、低频 Provider / world settings 扩展 | JSON | object |
| LLM-readable 叶子字段 | `summary_text`、`descriptors`、`notes` | Plain | string |

Agent 顶层 `KnowledgeEntry.content` 必须保持结构化。Plain 模式只允许用于 schema 声明为 string 的叶子字段；若用户在需要 object / array 的字段上切到 Plain，UI 可展示文本但保存前必须给出 blocker。

---

## 3. 模式语义

编辑模式描述的是“当前文本采用哪种书写语法”，不是“字段属于哪种业务数据”。同一个 ST 世界书 `content`、预设 prompt 正文或 Agent 的 LLM-readable 字符串，都可以用 Plain、JSON、YAML 或后续注册的其他语言包组织给 LLM 阅读的内容；只有绑定声明为 `storageKind = json_value` 的字段，才会在保存时被程序解析成结构化业务值。

### 3.1 Plain

Plain 模式面向自由排版文本、普通 prompt 正文、世界书正文、Regex 字符串和 LLM-readable 文本：

- 不解析语义，不改变换行内容。
- 支持括号、方括号、花括号、单双引号、反引号的平衡诊断。
- 不做缩进。
- 不因为括号 / 引号 warning 阻止保存，除非父级字段声明为必须结构化。

### 3.2 JSON

JSON 模式面向采用严格 JSON 语法书写的文本。它既可用于真正保存为 `serde_json::Value` 的 Agent structured content，也可用于 ST 世界书、预设 prompt、系统提示词等 string 字段，把内容组织成 LLM 可读的 JSON 文本：

- 实时解析 JSON；不可解析时给出 blocker 诊断。
- 格式化使用 2 spaces 缩进，不重排 object key，不删除未知字段。
- 输入 `{`、`[`、`,` 或按 Enter 时，根据当前对象 / 数组层级自动缩进；输入 `}`、`]` 时自动回退到匹配层级。
- 在对象上下文中输入常见未加引号键名时自动修正为英文双引号键名，例如 `name:` → `"name":`、`display_name:` → `"display_name":`。该修正只在当前行能确定为 object member key 时触发，且必须能一键 undo。
- 单引号包裹的对象键可在安全时修正为双引号键，例如 `'name':` → `"name":`；若键名内含转义、换行或歧义字符，只提示 quick fix。
- 对尾随逗号、缺少逗号、缺少冒号、字符串使用中文引号等常见问题给 diagnostics 和 quick fix；自动改写只限确定不改变语义的局部文本。
- 保存到 string 字段时写入格式化后的 JSON 文本；后续运行时仍把它作为 prompt 字符串注入，程序不读取其中 key 作为业务规则。
- 保存到 structured binding 时写入解析后的 `serde_json::Value`。
- 不支持注释；需要注释的配置片段应使用 YAML 模式或业务字段。

### 3.3 YAML

YAML 模式面向采用 YAML 语法书写的文本。它既可用于 `world_base.yaml` 等真实 YAML 配置，也可用于 ST content、prompt 模板和 LLM-readable 字符串，把长文本组织成层级清晰、便于 LLM 理解的 YAML 文本：

- 实时解析 YAML；不可解析时给出 blocker 诊断。
- 格式化使用 2 spaces 缩进，禁止 tab 缩进。
- 用户输入冒号并换行时，若当前行形如 `key:` 且没有同行值，下一行自动缩进 2 spaces。
- 在 list item 后换行时保持列表缩进；当上一行是 `- key:` 或 `-` 后续块时，下一行按 YAML 嵌套规则增加 2 spaces。
- 在 block scalar `|` / `>` 后换行时自动进入内容缩进；后续内容行保持同一缩进，直到用户手动退格或输入父级键。
- Tab 输入自动转换为 2 spaces；已存在 tab 缩进显示 warning，保存到 YAML / structured binding 前必须修正。
- 对冒号后缺少空格、list item 后缺少空格、混合缩进、重复 key 等常见问题给 diagnostics；只有空白和缩进类问题允许自动修正。
- 保存到 string 字段时写入格式化后的 YAML 文本；注释、键名和层级会作为 prompt 文本的一部分被 LLM 读取，程序不读取其中 key 作为业务规则。
- 保存到 Agent structured binding 时先转换为 JSON-compatible value，再进入业务 schema 校验。
- YAML anchors、aliases、tags 若无法稳定转换为 JSON-compatible value，保存到 structured binding 前必须报 blocker；保存到普通 string 字段时可作为文本保留，但需要 warning 提示其不会被程序展开为业务结构。
- 注释可在 string 字段和纯 YAML 文件视图中保留；转换为 `serde_json::Value` 的字段不承诺保留注释。

---

## 4. 诊断与格式化

Structured Text Editor 的前端编辑能力基于 CodeMirror 6 extension 组合实现。CM6 负责编辑状态、选择、undo history、括号匹配、基础补全、语言包缩进和 lint UI；RST 负责把 CM6 diagnostics 映射为 `StructuredTextDiagnostic`，并补充项目特有的 JSON key 修正、YAML 常见写法缩进、字段绑定和保存前后端复检。

### 4.1 Diagnostic 模型

```typescript
type BuiltinStructuredTextMode = 'plain' | 'json' | 'yaml';
type StructuredTextLanguageId = string;
type StructuredTextSeverity = 'info' | 'warning' | 'blocker';

interface StructuredTextDiagnostic {
  severity: StructuredTextSeverity;
  code:
    | 'unmatched_bracket'
    | 'unclosed_quote'
    | 'invalid_escape'
    | 'parse_error'
    | 'unsupported_yaml_feature'
    | 'auto_fix_available'
    | 'auto_fix_applied'
    | 'schema_type_mismatch';
  message: string;
  line: number;
  column: number;
  length?: number;
}
```

父级表单保存规则：

- `blocker`：禁止保存，焦点跳到第一个 blocker。
- `warning`：允许保存，但在检查面板展示。
- `info`：只在状态栏或 tooltip 展示。

业务 schema 校验生成的错误可以映射为 `schema_type_mismatch`，但编辑器自身不理解业务字段含义。

### 4.2 编辑时输入辅助

输入辅助必须是局部、确定、可撤销的操作：

- 所有自动缩进和自动修正都必须作为单次 undo step 进入编辑器历史。
- 输入辅助只能根据当前模式、当前行、邻近括号 / 缩进栈和解析器 diagnostics 判断，不做业务字段推断。
- 当自动修正可能改变文本含义时，只显示 quick fix，不直接改写。
- 用户可在编辑器设置中关闭“自动修正”，但当前语言包 diagnostics 仍然保留。

JSON 输入辅助：

- Enter 后根据最近未闭合 `{` / `[` 增加 2 spaces 缩进。
- 在对象 member 行输入 `:` 时，若冒号前是未加引号的普通键名，自动包成英文双引号。
- 普通键名范围为字母、数字、下划线、短横线、点号和常见 Unicode 标识符；含空格、冒号、引号或模板宏时不自动修正，只提供 quick fix。
- 输入 `{` / `[` 自动补 `}` / `]`；输入 `"` 自动补闭合双引号；选中文本时用双引号包裹。
- 输入 `,` 后换行保持当前数组 / 对象层级缩进。

YAML 输入辅助：

- 当前行以 `key:` 结束并按 Enter 时，下一行自动缩进 2 spaces。
- 当前行以 `-` 或 `- key:` 开始并按 Enter 时，下一行保持 list 对齐；若进入子块则再增加 2 spaces。
- 当前行以 `|` 或 `>` 结束并按 Enter 时，下一行自动进入 block scalar 内容缩进。
- Backspace 在纯缩进行上按 2 spaces 为单位退格。
- 粘贴文本时可提示“Normalize indentation”，但不得在未确认时重排整段内容。

### 4.3 格式化时机

- 用户点击 Format 时立即格式化。
- JSON / YAML 模式在保存前自动格式化；若解析失败，不改写文本并阻止保存。
- 切换模式时先保留原文本；只有目标模式解析成功且用户确认应用格式化时才改写文本。

### 4.4 括号与引号匹配

匹配检查必须覆盖：

- `()`、`[]`、`{}`。
- 单引号、双引号、反引号。
- JSON 字符串中的转义序列。
- Regex 字符串中的 `/pattern/flags` 只做文本层括号 / 引号提示；正则语义是否合法仍由 Regex 模块校验。

---

## 5. 绑定模型

```typescript
interface StructuredTextBinding {
  resourceKind:
    | 'st_worldbook_entry'
    | 'st_characterbook_entry'
    | 'st_preset'
    | 'st_regex_script'
    | 'agent_knowledge_entry'
    | 'agent_world_rules'
    | 'generic_extensions';
  fieldPath: string;
  allowedModes: StructuredTextLanguageId[];
  defaultMode: StructuredTextLanguageId;
  storageKind: 'string' | 'json_value' | 'yaml_file';
  requiredValueShape?: 'string' | 'object' | 'array' | 'any';
}

interface StructuredTextDraft {
  binding: StructuredTextBinding;
  mode: StructuredTextLanguageId;
  text: string;
  diagnostics: StructuredTextDiagnostic[];
  isDirty: boolean;
  lastFormattedAt?: string;
}
```

绑定规则：

- `storageKind = string`：保存格式化后的文本，不额外解析为业务对象；JSON / YAML 的结构只服务可读性和 LLM 理解。
- `storageKind = json_value`：当前语言包必须声明 `canParseToJsonValue = true`，保存前解析成功，并满足 `requiredValueShape` 后才可保存；首版只允许 JSON / YAML。
- `storageKind = yaml_file`：保存 YAML 文本，但必须先通过 ConfigValidator 或业务 validator。

所有 parent draft 持有原始业务字段；Structured Text Editor 只持有当前字段的局部草稿。切换资源、离开页面或覆盖导入时，仍使用 [41_frontend_interaction.md](41_frontend_interaction.md) 的未保存 draft 提示流程。

---

## 6. UI 交互

编辑器顶部工具栏固定包含：

- 语言 / 模式选择：首版显示 Plain / JSON / YAML；后续可显示已安装且当前字段允许的 language pack。
- Format 按钮。
- Diagnostics 状态：passed / warnings / blockers。
- 行列位置与缩进设置摘要。

编辑区规则：

- 使用等宽字体。
- 错误行显示 gutter marker；hover 展示诊断。
- 当前括号 / 引号配对高亮。
- 自动补全限于当前语言包提供的括号 / 引号闭合、JSON 对象键引号修正、YAML 缩进和常见标点空白修正；不自动补业务字段。
- quick fix 可用于当前语言包声明的局部修复；首版包括“给 JSON key 加双引号”“移除 JSON 尾随逗号”“YAML 缩进转 2 spaces”“冒号后补空格”。
- 大字段懒加载或虚拟滚动，避免打开大型世界书词条时阻塞 UI。

检查面板规则：

- 诊断按 blocker / warning / info 分组。
- 点击诊断跳到对应行列。
- 对 Agent Knowledge 字段，检查面板同时展示业务 validator 结果，但必须和文本诊断分区显示。

---

## 7. 实现边界

### 7.1 CodeMirror 6 依赖

首版前端依赖：

```json
{
  "dependencies": {
    "codemirror": "^6",
    "@codemirror/lang-json": "^6",
    "@codemirror/lang-yaml": "^6",
    "@codemirror/lint": "^6"
  }
}
```

实现原则：

- `StructuredTextEditor.vue` 直接管理 CM6 `EditorView` 生命周期，不强依赖第三方 Vue wrapper。
- 使用 CM6 `Compartment` 动态切换 Plain / JSON / YAML language extensions、theme、readonly 和 lint 配置。
- 使用 CM6 `linter` 输出文本 diagnostics，并通过 adapter 转换为 RST 的 `StructuredTextDiagnostic`。
- 使用 CM6 keymap / transaction extension 实现 JSON key 自动修正、YAML Enter 缩进和 quick fix actions。
- 使用 CM6 theme extension 映射 Naive UI theme token，避免单独维护一套视觉体系。
- 不直接使用不可定制的 `basicSetup` 作为长期配置；可以在原型期使用，正式实现应按需组合 history、line numbers、fold gutter、bracket matching、close brackets、search、lint gutter 等 extensions。

### 7.2 语言包注册表

语言支持通过注册表管理，而不是在业务组件中硬编码：

```typescript
interface StructuredTextLanguagePack {
  languageId: string;              // plain / json / yaml / markdown / toml / ...
  label: string;
  source: 'builtin' | 'bundled' | 'trusted_plugin';
  storageKinds: ('string' | 'json_value' | 'yaml_file')[];
  load: () => Promise<Extension[]>; // 返回 CM6 language / lint / keymap / helper extensions
  canParseToJsonValue?: boolean;
  supportsFormat?: boolean;
  supportsLint?: boolean;
  supportsAutoIndent?: boolean;
}
```

首版注册：

- `plain`：无结构解析，只启用基础编辑、括号 / 引号提示和搜索。
- `json`：`@codemirror/lang-json` + RST JSON lint / key 修正 / 保存前 JSON parse。
- `yaml`：`@codemirror/lang-yaml` + RST YAML lint / 缩进辅助 / 保存前 YAML parse。

后续扩展策略：

- 应用可预装更多 CM6 官方语言包，并通过动态 import 注册，例如 Markdown、XML、JavaScript、TOML。
- 用户自定义语言包不能只是写进普通 JSON/YAML 配置后直接执行；因为 CM6 language support 是 JavaScript 代码，加载任意包等价于执行任意前端代码。
- 第三方语言包必须走受信插件机制：插件 manifest 声明 `structured_text_languages`，应用显示来源、权限和版本，用户确认后才启用。
- 普通用户配置只能选择“已安装 / 已信任”的 `languageId`、默认模式和字段绑定，不能提供任意 JS loader。
- `storageKind = json_value` 默认只允许 `json` / `yaml` 这类可稳定转换到 JSON-compatible value 的语言包；其他语言包只能用于 `storageKind = string`，除非其 pack 明确声明 `canParseToJsonValue = true` 并通过后端复检。

建议前端模块：

```text
src/components/shared/structured-text-editor/
├── StructuredTextEditor.vue
├── StructuredTextToolbar.vue
├── StructuredTextDiagnostics.vue
├── cm6Setup.ts
├── languageRegistry.ts
└── modeAdapters.ts

src/composables/
└── useStructuredTextDraft.ts

src/types/
└── structuredText.ts
```

### 7.3 后端复检

建议后端模块：

```text
src-tauri/src/text_format/
├── mod.rs
├── json.rs
└── yaml.rs
```

前端负责即时体验，后端负责保存前最终解析与格式化一致性。任何后端解析失败都必须返回结构化 diagnostics，不能只返回字符串错误。后端不参与光标级编辑体验，不复刻 CM6 parser；只做保存前 parse、schema shape 检查、YAML 到 JSON-compatible value 转换和最终格式化。

---

## 8. 测试与验收

首版必须覆盖：

- Structured Text Editor 使用 CodeMirror 6 `EditorView` 封装，切换资源 / 销毁组件时释放 view，不泄漏 listener。
- Plain / JSON / YAML 模式切换通过 CM6 `Compartment` 完成，不重建父级 draft，不丢失 undo 之外的业务状态。
- 语言包注册表能注册 / 查询 / 动态加载 builtin language pack；未注册 `languageId` 不会进入编辑器。
- 普通用户配置只能选择已安装 / 已信任 languageId，不能加载任意 JavaScript language support。
- `storageKind = json_value` 字段拒绝切换到不能稳定 parse 为 JSON-compatible value 的语言包。
- Plain / JSON / YAML 模式可切换，切换失败不丢失原文本。
- JSON 格式化保持 key 顺序并使用 2 spaces。
- JSON 模式在对象上下文输入 `name:` 会自动修正为 `"name":`；含空格、模板宏或歧义字符的 key 只给 quick fix，不自动改写。
- JSON 模式输入 `{` / `[` / `,` / Enter 时自动维护 2 spaces 缩进，输入 `}` / `]` 时回退到匹配层级。
- JSON 单引号 key 可安全转换为双引号；字符串值中的单引号不被误改。
- JSON 尾随逗号、缺少逗号、缺少冒号能给 diagnostics 和局部 quick fix。
- YAML tab 缩进、无法转换的 anchor / tag 保存前报 blocker。
- YAML 模式输入 `key:` 后按 Enter 自动缩进 2 spaces。
- YAML list item、嵌套 map、block scalar `|` / `>` 换行缩进符合 YAML 常见写法。
- YAML 粘贴混合缩进时提示 normalize，不未经确认重排整段内容。
- Plain 模式括号 / 引号不匹配显示 warning，但不强制阻止普通字符串字段保存。
- Agent `KnowledgeEntry.content` 在 Plain 顶层保存时报 `schema_type_mismatch` blocker。
- ST 世界书 / 预设 prompt 的 string 字段可用 JSON / YAML 组织给 LLM 阅读的结构化文本，保存后仍为 string，不改变 ST 文件 schema。
- Regex `findRegex` 的括号提示不替代 Regex 编译校验。
- 父级 draft 未保存提示与 editor dirty 状态一致。
- 后端保存前复跑解析，前端绕过 diagnostics 时仍不能提交非法 JSON / YAML。

文档阶段验证：

- `git diff -- README.md docs AGENTS.md`
- `git status --short`
