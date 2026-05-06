# 78 ST 扩展系统

本文记录 `E:\AIPlay\SillyTavern` 当前 Extension 的启用方式、对外接口，以及它与世界书条目注入、Regex、预设提示词组装和消息记录提取过滤之间的交互方式。运行时组装主链见 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)，世界书注入见 [73_st_worldbook_injection.md](73_st_worldbook_injection.md)，Regex 兼容见 [76_st_regex.md](76_st_regex.md)。

## 1. 参考依据

- `SillyTavern\public\scripts\extensions.js`
- `SillyTavern\public\scripts\st-context.js`
- `SillyTavern\public\scripts\events.js`
- `SillyTavern\public\lib\eventemitter.js`
- `SillyTavern\public\script.js`
- `SillyTavern\public\scripts\world-info.js`
- `SillyTavern\public\scripts\openai.js`
- `SillyTavern\src\endpoints\extensions.js`
- `SillyTavern\src\plugin-loader.js`

## 2. 名词边界

SillyTavern 中有两套容易混淆的扩展机制：

| 名称 | 位置 | 启用默认值 | 能力边界 |
|---|---|---|---|
| Frontend Extension | `public/scripts/extensions/*` 与用户 / 全局 `third-party/*` | 随 `enable_extensions` / `extensions.enabled` 配置开启 | 在浏览器端以 ES module 加载，可监听事件、改 UI、注册 slash command、注入 prompt、改生成请求数据 |
| Server Plugin | 根目录 `plugins/*`，由 `src/plugin-loader.js` 加载 | `enableServerPlugins = false` | 在 Node 后端注册 `/api/plugins/<id>` 路由；不自动接入聊天主流程 |

本文的 Extension 指第一类 Frontend Extension。Server Plugin 只在第 12 节说明差异。

## 3. Extension 启用总流程

ST 的 Extension 启用不是静态打包注册，而是运行时发现、读取 manifest、再把 JS/CSS 注入页面：

```
settings.enable_extensions / config extensions.enabled
       ↓
loadExtensionSettings(settings, versionChanged, enableAutoUpdate)
       ↓
eventSource.emit(EXTENSIONS_FIRST_LOAD)
       ↓
GET /api/extensions/discover
       ↓
读取每个扩展 /scripts/extensions/<name>/manifest.json
       ↓
按 loading_order 排序
       ↓
检查 disabledExtensions / requires / dependencies / minimum_client_version
       ↓
加载 i18n、<script type="module">、<link rel="stylesheet">
       ↓
activeExtensions.add(name)
       ↓
调用 manifest.hooks.activate 指向的导出函数
```

后端 `/api/extensions/discover` 返回三类扩展：

| type | ST 加载名 | 来源 | 优先级 |
|---|---|---|---|
| `system` | `<folder>` | `public/scripts/extensions/<folder>`，排除 `third-party` | 内置 |
| `local` | `third-party/<folder>` | 当前用户目录的扩展目录 | 高于同名 global |
| `global` | `third-party/<folder>` | 全局扩展目录 | 同名 local 存在时被过滤 |

禁用状态保存在 `extension_settings.disabledExtensions`。启用 / 禁用扩展时，ST 先调用 `hooks.enable` 或 `hooks.disable`，再更新设置；默认会 reload 页面，也支持 `reload=false` 延后生效。

## 4. manifest 契约

典型 manifest：

```json
{
  "display_name": "Regex",
  "loading_order": 1,
  "requires": [],
  "optional": [],
  "js": "index.js",
  "css": "style.css",
  "author": "kingbri",
  "version": "1.0.0",
  "homePage": "https://github.com/SillyTavern/SillyTavern",
  "hooks": {
    "activate": "init"
  }
}
```

核心字段：

| 字段 | 语义 |
|---|---|
| `display_name` | UI 展示名 |
| `loading_order` | 激活排序；相同顺序按 `display_name` 排 |
| `requires` | 依赖 Extras API 模块；不是 JS 模块依赖 |
| `optional` | 可选 Extras 模块，只用于 UI 展示 |
| `dependencies` | 依赖其他 Extension 的内部名；依赖缺失或被禁用则不加载 |
| `minimum_client_version` | 最低 ST client 版本 |
| `js` | ES module 入口，注入 `<script type="module">` |
| `css` | CSS 文件，注入 `<link rel="stylesheet">` |
| `i18n` | locale 到 JSON 文件的映射 |
| `auto_update` | 第三方扩展自动更新标志 |
| `hooks` | 生命周期钩子函数名 |
| `generate_interceptor` | 生成前拦截器的 `globalThis` 函数名 |

支持的 `hooks`：

- `install`
- `update`
- `delete`
- `clean`
- `enable`
- `disable`
- `activate`

Hook 函数必须从扩展 JS 入口模块导出。ST 会 `import()` 入口模块，按 manifest 中的函数名调用；函数可返回 Promise，ST 最多等待 5 秒，超时只写警告。

## 5. 扩展能拿到的接口

Extension 有两种接入方式。

第一种是直接 import ST 前端模块，这是内置扩展最常见方式：

```javascript
import { eventSource, event_types, setExtensionPrompt } from '../../../script.js';
import { extension_settings, getContext } from '../../extensions.js';
```

第二种是调用 `getContext()`。`st-context.js` 为扩展暴露兼容对象，重要能力包括：

| 能力 | 代表接口 |
|---|---|
| 当前会话状态 | `chat`、`characters`、`groups`、`characterId`、`groupId`、`chatMetadata` |
| 事件总线 | `eventSource`、`eventTypes` |
| 生成调用 | `generate`、`generateQuietPrompt`、`generateRaw`、`generateRawData`、`stopGeneration` |
| Prompt 注入 | `extensionPrompts`、`setExtensionPrompt` |
| 持久化 | `saveChat`、`saveMetadata`、`saveSettingsDebounced`、`updateChatMetadata` |
| Slash command | `SlashCommandParser`、`SlashCommand`、`SlashCommandArgument`、`executeSlashCommandsWithOptions` |
| 模板与弹窗 | `renderExtensionTemplateAsync`、`Popup`、`callGenericPopup` |
| 工具调用 | `registerFunctionTool`、`unregisterFunctionTool`、`ToolManager` |
| 世界书 | `loadWorldInfo`、`saveWorldInfo`、`getWorldInfoPrompt`、`updateWorldInfoList` |
| 变量 / 宏 | `variables.local/global`、`macros` |
| 媒体和消息 UI | `appendMediaToMessage`、`updateMessageBlock`、`messageFormatting` |
| 角色字段扩展 | `writeExtensionField`、`writeExtensionFieldBulk`、`UNSET_VALUE` |

这个接口不是沙箱。Extension 实际运行在 ST 前端同一 JS 环境中，可直接修改全局状态、DOM、聊天数组和设置对象。

## 6. 事件总线语义

`eventSource` 是一个自定义 EventEmitter。`emit()` 会按 listener 顺序串行 `await`，所以事件监听器可以阻塞主流程。`APP_READY` 和 `APP_INITIALIZED` 是 auto-fire 事件：事件已经发过以后再注册 listener，也会立刻用上次参数调用。

常用事件：

| 阶段 | 事件 |
|---|---|
| 应用生命周期 | `APP_INITIALIZED`、`APP_READY`、`SETTINGS_LOADED`、`SETTINGS_UPDATED` |
| 扩展加载 | `EXTENSIONS_FIRST_LOAD`、`EXTENSION_SETTINGS_LOADED`、`EXTRAS_CONNECTED` |
| 会话切换 | `CHAT_CHANGED`、`CHAT_LOADED`、`CHAT_CREATED`、`CHAT_DELETED`、`CHAT_RENAMED` |
| 消息变更 | `MESSAGE_SENT`、`MESSAGE_RECEIVED`、`MESSAGE_EDITED`、`MESSAGE_DELETED`、`MESSAGE_UPDATED`、`USER_MESSAGE_RENDERED`、`CHARACTER_MESSAGE_RENDERED` |
| 生成流程 | `GENERATION_STARTED`、`GENERATION_AFTER_COMMANDS`、`GENERATION_STOPPED`、`GENERATION_ENDED` |
| Prompt 组装 | `GENERATE_BEFORE_COMBINE_PROMPTS`、`GENERATE_AFTER_COMBINE_PROMPTS`、`GENERATE_AFTER_DATA` |
| Chat Completion | `CHAT_COMPLETION_PROMPT_READY`、`CHAT_COMPLETION_SETTINGS_READY` |
| 世界书 | `WORLD_INFO_ACTIVATED`、`WORLDINFO_SCAN_DONE`、`WORLDINFO_UPDATED` |

事件参数多数是可变对象。ST 在若干位置明确依赖 listener 修改参数，例如 `GENERATE_AFTER_COMBINE_PROMPTS` 可改 `eventData.prompt`，`WORLDINFO_SCAN_DONE` 可改下一轮扫描状态、预算和激活文本。

## 7. 与生成主流程的交互

`Generate()` 中 Extension 相关主链如下：

```
GENERATION_STARTED
       ↓
执行聊天输入中的 slash command
       ↓
GENERATION_AFTER_COMMANDS
       ↓
必要时 sendMessageAsUser()
       ↓
提取 coreChat
       ↓
Regex prompt-only 处理历史消息 / reasoning
       ↓
runGenerationInterceptors(coreChat, maxContext, type)
       ↓
getWorldInfoPrompt(chatForWI, maxContext, ...)
       ↓
读取 extension_prompts 的 BEFORE_PROMPT / IN_PROMPT
       ↓
IN_CHAT extension prompt 按 depth / role 注入 coreChat
       ↓
预设 / Prompt Manager 组装最终 prompt 或 messages
       ↓
GENERATE_BEFORE_COMBINE_PROMPTS
       ↓
GENERATE_AFTER_COMBINE_PROMPTS
       ↓
Provider generate_data 构建
       ↓
GENERATE_AFTER_DATA
       ↓
调用 API
       ↓
Regex AI_OUTPUT / USER_INPUT 处理返回文本
       ↓
MESSAGE_RECEIVED / CHARACTER_MESSAGE_RENDERED
       ↓
GENERATION_ENDED
```

Extension 影响主流程的硬入口有四个：

1. 事件 listener：可观察并在部分事件中修改参数。
2. `setExtensionPrompt()`：把扩展文本写入 `extension_prompts`，由世界书扫描、prompt 组装和 OpenAI Prompt Manager 读取。
3. `generate_interceptor`：在世界书扫描前拿到 `coreChat`，可修改消息数组或中止生成。
4. 直接操作 ST 状态：通过 `getContext()` 或直接 import 修改聊天、metadata、设置、DOM。

## 8. 与世界书条目注入的交互

世界书扫描入口是 `getWorldInfoPrompt()` / `checkWorldInfo()`。它与 Extension 有三类交互。

第一，Extension prompt 可以参与世界书关键词扫描。`setExtensionPrompt(key, value, position, depth, scan = true, role)` 中 `scan = true` 时，`checkWorldInfo()` 会通过 `getExtensionPromptByName(key)` 取出文本并加入 `WorldInfoBuffer.#injectBuffer`。这些文本不一定已经注入最终 prompt，但会成为世界书关键词匹配的扫描上下文。Author's Note、Persona Description、Memory、Vectors 等扩展都可能通过这个路径影响世界书激活。

第二，世界书扫描循环本身暴露 `WORLDINFO_SCAN_DONE`。事件参数包含当前 / 下一扫描状态、新激活条目、全部激活条目文本、排序条目、递归延迟层级、预算和 timed effects。Listener 可以改变：

- `args.state.next`
- `args.activated.text`
- `args.recursionDelay.currentLevel`
- `args.budget.current`
- `args.budget.overflowed`

这意味着 Extension 不只是被世界书读取，也能改变世界书递归扫描是否继续、预算是否溢出以及激活文本。

第三，世界书输出会回写到 prompt 注入体系。ST 把深度注入、Author's Note 增补、Example Message、outlet 等结果转换为 `extension_prompts` 或最终组装字段。例如 `CUSTOM_WI_DEPTH_ROLE(depth, role)` 会写成 `IN_CHAT` depth prompt；Author's Note 会合并进 `NOTE_MODULE_NAME` 对应 prompt。后续预设组装阶段再读取这些结果。

RST 兼容要求：

- `scan` 标志必须保留，且世界书扫描必须读取可注入扩展文本。
- `WORLDINFO_SCAN_DONE` 若实现第三方扩展兼容，必须明确哪些字段允许被 listener 修改。
- 世界书生成的 depth / AN / outlet 结果仍应走统一 Prompt 注入表，不能绕过 `RequestAssembler` 直接拼 provider 请求。

## 9. 与 Regex 的交互

Regex 在 ST 中表现为 Extension，但主流程把 Regex engine 当核心文本处理层直接 import。因此 Regex 既受 Extension 启用状态管理，又在以下位置影响主流程：

| 阶段 | Regex placement | 影响 |
|---|---|---|
| 用户发送消息 | `USER_INPUT` | 写回聊天记录 |
| AI 输出入库 | `AI_OUTPUT`，impersonate 用 `USER_INPUT` | 写回聊天记录 |
| 聊天历史进入 prompt | `USER_INPUT` / `AI_OUTPUT` + `isPrompt = true` + depth | 不写回聊天记录，只影响请求 |
| reasoning 进入 prompt | `REASONING` + `isPrompt = true` | 不写回聊天记录 |
| 世界书词条内容落槽 | `WORLD_INFO` + `isPrompt = true` | 不改世界书文件，只影响请求 |
| slash command 文本 | `SLASH_COMMAND` | 取决于命令是否写消息 |
| Markdown 显示 | 对应 placement + `isMarkdown = true` | 只影响显示 |

这和普通 Extension prompt 是两层机制：

- `setExtensionPrompt()` 增加或替换 prompt 片段。
- Regex 对已有文本做 find / replace。

顺序上，聊天历史先被抽为 `coreChat`，再以 `isPrompt = true` 跑 Regex；世界书激活内容在落槽前跑 `WORLD_INFO` prompt-only；最终 AI 输出在写回聊天前跑输出 Regex。

RST 兼容要求：

- Regex Extension 可以在 UI / 设置层以 Extension 形式呈现，但 runtime 必须把 Regex engine 作为请求组装链路中的固定阶段。
- Regex prompt-only 不得写回聊天、角色卡、世界书或预设文件。
- 切换 API 配置不得改变 Regex allow list、Regex Preset 或脚本启用顺序。

## 10. 与预设提示词组装的交互

ST 的非 OpenAI 路径和 OpenAI 路径处理 Extension prompt 的方式不同。

### 10.1 非 OpenAI / text completion 路径

`Generate()` 会读取：

- `getExtensionPrompt(BEFORE_PROMPT)`
- `getExtensionPrompt(IN_PROMPT)`
- `getExtensionPrompt(IN_CHAT, depth, role)`

`BEFORE_PROMPT`、`IN_PROMPT` 作为 scenario / story string 附近的锚点参与最终字符串 prompt 组合。`IN_CHAT` 通过 `doChatInject()` 按 depth 插入 `coreChat`：

- depth `0` 是最新消息附近。
- continue 生成时 depth `0` 插入位置偏移到 `1`，避免压到待续写前缀。
- role 顺序为 `system -> user -> assistant`，越后越贴近高优先级位置。

随后 ST 把 `coreChat` 格式化为 `mesSend`，执行上下文裁剪，再与 worldInfo、scenario、examples、jailbreak 等预设字段合并。

### 10.2 OpenAI / chat completion 路径

OpenAI 路径由 `prepareOpenAIMessages()` 和 Prompt Manager 处理：

- 已知扩展 prompt 有特殊 identifier：`1_memory -> summary`、`2_floating_prompt -> authorsNote`、`3_vectors -> vectorsMemory`、`4_vectors_data_bank -> vectorsDataBank`、`chromadb -> smartContext`。
- 任意未知 extension prompt 只要 position 是 `BEFORE_PROMPT` 或 `IN_PROMPT`，会以清理后的 key 作为 identifier 加入 `systemPrompts`。
- Prompt Manager 可覆盖这些 prompt 的 role、injection position、injection depth、injection order。
- In-chat prompt 注入由 `populationInjectionPrompts()` 处理，固定把 Extension prompt 放入 order `100` 桶，与 Prompt Manager 的 in-chat prompt 一起按 depth / role 插入 messages。

RST 兼容要求：

- 中立 `RequestAssembler` 必须把 Extension prompt 看作 Prompt Assembly 输入，而不是 Provider 层能力。
- 对 OpenAI-compatible provider，仍应复刻 ST 的 Prompt Manager marker / injection 语义，再映射到 provider messages。
- `GENERATE_BEFORE_COMBINE_PROMPTS`、`GENERATE_AFTER_COMBINE_PROMPTS`、`GENERATE_AFTER_DATA` 如果开放给扩展，必须在 Trace / Logs 中记录扩展修改后的 prompt 或请求摘要。

## 11. 与消息记录提取过滤的交互

ST 生成前不会把完整聊天数组原样送入 prompt，而是先提取 `coreChat`：

```javascript
let coreChat = chat.filter(x => !x.is_system || (canUseTools && Array.isArray(x.extra?.tool_invocations)));
if (type === 'swipe') {
  coreChat.pop();
}
```

含义：

- 普通 system message 不进入 prompt 历史。
- 带 tool invocation 的 system message 可以保留，前提是当前 provider 支持 tool calling。
- swipe 生成会移除最后一条候选回复。
- 用户消息发送阶段可能先写入聊天数组，再参与本次 `coreChat`。
- 文件附件内容会在 `coreChat` 映射阶段追加到消息文本。
- reasoning 会按当前角色 / 群聊成员过滤后拼进消息文本。
- Regex prompt-only 在 `coreChat` 上运行，不写回 `chat`。

Extension 与这一步的交互点：

| 交互点 | 结果 |
|---|---|
| `MESSAGE_SENT` / `MESSAGE_RECEIVED` listener | 可以在下一次生成前改变真实聊天记录 |
| `generate_interceptor` | 可以直接修改本次 `coreChat`，例如 Vector Storage 重新排列或裁剪消息 |
| `setExtensionPrompt(IN_CHAT)` | 不改真实聊天记录，只向本次 prompt 插入虚拟消息 |
| `setExtensionPrompt(scan = true)` | 不改真实聊天记录，但影响世界书扫描 buffer |
| Regex prompt-only | 不改真实聊天记录，只影响本次请求文本 |

RST 兼容要求：

- 真实聊天记录、prompt-only 变换、虚拟注入消息必须分层保存，避免把临时 prompt 产物写回会话文件。
- 消息提取过滤应发生在世界书扫描前，因为世界书扫描使用的是 `coreChat` 派生的 `chatForWI`。
- `generate_interceptor` 若支持第三方扩展，必须在 Trace 中记录前后消息数量、是否 abort、扩展名和上下文预算。

## 12. Server Plugin 差异

`src/plugin-loader.js` 加载的是后端 Server Plugin：

```javascript
export const info = {
  id: 'plugin_id',
  name: 'Plugin Name',
  description: '...'
};

export async function init(router) {
  router.get('/route', handler);
}

export async function exit() {}
```

约束：

- 只有 `enableServerPlugins = true` 时加载。
- 目录可以是 npm package，也可以有 `index.js` / `index.cjs` / `index.mjs`。
- `info.id` 只能是小写字母、数字、下划线、短横线。
- 插件只能通过 `init(router)` 注册 `/api/plugins/<id>` 下的 API route。
- Server Plugin 不自动获得前端 `eventSource`、`extension_prompts` 或 `Generate()` 钩子。

RST 若后续支持 Server Plugin，应把它作为后端 API 扩展机制，不得把它等同于 ST Frontend Extension。

## 13. RST 落地边界

RST 复刻 ST Extension 时必须先保留运行时安全边界：

- Provider-bound prompt 仍只能通过 `RequestAssembler` 单一闸口输出。Extension prompt、Regex、世界书和附件都必须在闸口前变成可记录的中立输入。
- 第三方任意 JS 不应默认运行在 Tauri WebView 的高权限上下文中。若支持 third-party Extension，应默认关闭，启用前显示权限和风险，并禁止直接获得文件系统 / shell / opener 能力。
- 内置兼容扩展可先以 typed module 实现：Regex、Memory、Vectors、Quick Reply、Attachments 等按 ST 行为暴露配置和事件，不要求第一版执行外部仓库 JS。
- 所有 Extension 对 prompt、世界书扫描状态、生成数据的修改都应进入 Trace / Logs：记录扩展名、钩子、输入摘要、输出摘要、是否 abort、耗时和错误。
- API 配置切换不得触发 Extension enable/disable、世界书选择、Regex allow list、Prompt Manager marker、聊天 metadata 或角色卡字段的自动改写。
