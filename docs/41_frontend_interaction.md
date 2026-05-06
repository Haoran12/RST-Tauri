# 41 前端交互主框架

本文档承载 RST 前端 UI 的主框架、一级页面、导航、布局、主题和交互状态边界。Agent World Editor 的业务规则、校验与提交细节见 [40_agent_world_editor.md](40_agent_world_editor.md)；结构化文本编辑器的 Plain / JSON / YAML 模式与诊断规则见 [42_structured_text_editor.md](42_structured_text_editor.md)。本文只定义它们在全应用 Shell 中的入口和衔接方式。

> 当前实现状态（2026-05-05）：仓库已落地基础 App Shell、一级路由、左侧导航、上下文列表区、主工作区，以及 ST / 资源 / API 配置 / 日志等页面入口；`ContextList`、`InspectPanel`、资源工作台和 API 配置页已接入真实 store 数据并提供只读摘要 / 基础 CRUD 交互，Agent 工作区与 Agent World Editor 仍有部分原型化区域，日志页仍以占位入口为主。本文多数内容依然属于目标 spec，而非全部已实现现状。

相关基础文档：

- 应用数据目录、配置分层与前后端模块结构见 [02_app_data_and_modules.md](02_app_data_and_modules.md)。
- ST 模式兼容原则见 [70_st_mode.md](70_st_mode.md)。
- ST 运行时全局状态与 API / 预设解耦规则见 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)。
- Agent 世界编辑器见 [40_agent_world_editor.md](40_agent_world_editor.md)。
- 结构化文本编辑器见 [42_structured_text_editor.md](42_structured_text_editor.md)。
- 日志与 Trace 边界见 [30_logging_and_observability.md](30_logging_and_observability.md)。

---

## 1. 设计定位

RST 首版 UI 是桌面 Tauri 工具型应用，而不是网站首页或移动优先聊天壳。ST 模式与 Agent 模式在产品语义、任务密度、资源组织和主工作流上都应视为两套一等体验，而不是一套共享导航下的不同页面分支。

前端必须显式提供**全局模式选择器**，让用户先进入 `ST Workspace` 或 `Agent Workspace`，再在各自模式内完成高频工作流。共享能力只保留在 API 配置、日志、设置、主题 token 和少量基础组件层；模式内的首页、主导航、上下文列表、检查面板、最近记录和默认路由都要分离。

默认交互取向：

- **模式优先而非混合工作台**：启动后先进入模式选择页；若已记录上次模式且用户未要求切换，则直接恢复到该模式的默认首页。
- **ST / Agent 各自有独立首页**：ST 首页聚合最近 ST 会话、角色卡、世界书、预设和当前连接；Agent 首页聚合 World、主线状态、最近会话和运行风险。
- **工具型紧凑布局**：信息密度高于沉浸式聊天 UI，优先支持长期编辑、切换、比对和调试。
- **主题跟随系统**：目标为首次启动跟随系统亮 / 暗主题，用户可在设置中手动固定主题；当前代码中 `system` 仍暂时回退到亮色主题。
- **双模式前端完全分区**：ST 模式和 Agent 模式共享底层基础设施，但不共享一级业务导航、首页信息架构或上下文侧栏。

---

## 2. App Shell

前端 Shell 拆成三层：

1. **Mode Switch Layer**：全局模式选择器与模式恢复逻辑；决定当前进入 ST Shell、Agent Shell 还是共享页。
2. **Mode Shell**：`STShell` 与 `AgentShell` 各自承载一级导航、上下文列表区、主工作区和右侧检查面板。
3. **Shared Shell**：`/api-configs`、`/logs`、`/settings` 等跨模式页面；允许保留更轻的共享导航框架，但必须持续显示当前来源模式或“共享页”标识。

每个 Mode Shell 内部继续使用四区结构：

1. **一级导航栏**：固定在左侧，只承载当前模式的业务入口；不得再把 ST 与 Agent 的业务入口混排在同一列。
2. **上下文列表区**：显示当前页面的资源列表、会话列表、World 列表或筛选结果；可折叠。
3. **主工作区**：承载聊天、编辑器、详情页、表单或日志查看器。
4. **右侧检查面板**：承载摘要、校验、影响分析、运行状态、请求详情或 Trace 片段；默认可折叠。

当前代码对应关系：

- 当前代码中的 `src/components/layout/AppLayout.vue`、`AppNav.vue`、`ContextList.vue`、`InspectPanel.vue` 仍是混合式共享壳层；后续要拆为 `ModeSelector` + `STShell` + `AgentShell` + `SharedShell`。

布局规则：

- 桌面宽屏默认显示当前模式的一级导航、上下文列表区和主工作区；右侧检查面板按页面需要打开。
- 窄屏下上下文列表区与右侧检查面板改为抽屉，主工作区不得被永久遮挡。
- 模式切换器在桌面宽屏优先放在导航顶端，使用 Segmented Control 或双按钮 pill；窄屏下可折叠进顶部栏或抽屉头部，但不能隐藏到设置页深层。
- 一级导航项使用图标 + 短文本；常用动作按钮优先使用图标按钮并配 tooltip。
- 禁止用首页大 hero、营销卡片或说明性大段文案替代真实工作入口。
- 不使用卡片套卡片；页面区块使用全宽分区或无框布局，卡片仅用于重复资源项、弹窗和明确的工具面板。

### 2.1 全局模式选择器

全局模式选择器是一级信息架构，不是普通筛选控件。它至少承担以下职责：

- 当前模式展示：`ST Workspace` / `Agent Workspace` / `Shared`
- 切换入口：一键切换模式，不要求先回到首页
- 最近模式恢复：启动时读取上次模式并恢复到该模式默认首页或上次业务路由
- 跨模式跳转确认：若当前页存在未保存 draft、进行中的生成或运行中编辑锁，先提示保留 / 丢弃 / 取消
- 共享页回跳：从 API 配置、日志、设置返回时，能回到切换前的模式上下文

推荐交互：

- 首次启动：显示 `Mode Select` 页，两个主卡片分别进入 ST 或 Agent
- 非首次启动：直接恢复上次模式，并在导航顶端持续显示模式切换器
- 当前模式高亮必须稳定可见，不依赖 tooltip
- 切换模式不改变底层数据，只改变前端路由与 Shell 上下文

### 2.2 Shell 尺寸与区域职责

### 2.1 Shell 尺寸与区域职责

默认桌面布局采用固定导航 + 可调侧栏 + 弹性主区：

| 区域 | 默认尺寸 | 最小 / 最大 | 职责 |
|---|---:|---:|---|
| 一级导航栏 | 72px | 折叠 52px | 当前模式顶层路由、模式徽标、设置入口 |
| 上下文列表区 | 280px | 220px / 420px | 当前页面资源列表、搜索、筛选、批量选择 |
| 主工作区 | 弹性 | 640px 起 | 聊天、详情编辑、表格、编辑器画布或日志正文 |
| 右侧检查面板 | 340px | 280px / 480px | 当前对象摘要、校验、影响分析、运行状态、请求详情 |

主工作区顶部保留页面动作栏：

- 左侧显示当前位置标题、资源名、状态标签和未保存标记。
- 中间可放页面级 tabs、segmented control 或视图切换。
- 右侧放主要动作：新建、导入、保存、运行 validation、提交、停止生成等。
- 动作栏高度保持稳定；长标题截断并在 tooltip 中显示完整名称。

底部状态栏只显示跨页面且需要持续可见的信息：

- 当前 API 配置健康摘要。
- Agent World running / paused / pending LLM / needs rollback 状态。
- 最近一次后台任务状态。
- 离线、权限不足或数据目录不可写等全局阻断状态。

### 2.3 响应式断点

- `>= 1280px`：四区可同时显示，右侧检查面板按页面需要默认展开。
- `960px - 1279px`：一级导航、上下文列表区、主工作区常驻；右侧检查面板默认抽屉。
- `< 960px`：上下文列表区和右侧检查面板都进入抽屉；主工作区独占宽度。
- `< 720px`：动作栏按钮优先折叠为图标或 overflow menu；主操作仍必须一键可达。

窄屏抽屉打开时不得触发路由变化；关闭抽屉后必须保留列表筛选、当前选中项和未提交 draft。

---

## 3. 一级页面

### 3.1 模式选择页

路由：`/mode-select`

模式选择页是新的默认入口，用于回答“我现在要进入哪套工作流”。它不是营销首页，而是工作台分流页。

必须展示：

- `ST Workspace` 入口：最近 ST 会话数、角色卡 / 世界书 / 预设数量、当前连接状态
- `Agent Workspace` 入口：World 数量、最近会话、运行中 World、待处理风险
- 最近访问模式和最近访问时间
- 共享入口：API 配置、日志、设置

模式选择页不展示混合资源列表，也不允许在这里直接编辑复杂业务对象。

### 3.2 ST 首页

路由：`/st`

ST 首页回答“我在 ST 侧现在可以继续哪里、哪些资源可直接进入”。它替代旧的混合 `/library` 工作台。

必须展示：

- 最近 ST 会话。
- 最近编辑的角色卡、世界书、预设、Regex 资源。
- 当前 `active_api_config_id` 的健康状态：未配置、可用、连接失败、缺少 key、Provider 能力受限。
- 当前 `active_preset`、已选角色卡 / 世界书资源摘要和快速跳转。
- 快捷入口：新建 ST 会话、导入角色卡、导入世界书、管理预设、管理 API 配置。

ST 首页不得混入 Agent World 状态摘要或 Agent 会话入口。

### 3.3 ST 聊天

路由：`/st/chat/:sessionId?`

ST 聊天页优先复刻 SillyTavern 文本聊天体验，同时保留 RST 的 Provider / 预设解耦规则。

页面结构：

- 上下文列表区：ST 会话列表、角色筛选、最近聊天。
- 主工作区：页头、消息列表、输入框、生成 / 停止 / 重试 / 编辑消息等操作。
- 页头在会话标题附近提供紧凑选择器：当前 ST 预设与当前主 API 配置。二者写入全局 ST runtime state，只影响下一次请求；选择器不得把 API 配置或预设固化进会话文件。
- 右侧检查面板：当前角色卡、Chat lore、Character lore、Global lore、预设、Regex、API 配置摘要。

交互边界：

- 切换 API 配置只影响下一次请求的连接配置和 Provider 参数映射。
- 切换 API 配置不得改变当前预设、世界书选择、Regex 授权状态、聊天 metadata 或角色卡扩展字段。
- ST 模式只写全局运行 Logs，不写 Agent Trace。

### 3.4 Agent 首页 / 工作区入口

路由：`/agent`

Agent 首页用于回答“当前有哪些 World 值得进入”。它与 ST 首页分离，不再展示角色卡 / 世界书 / 预设资源池。

必须展示：

- 最近 Agent World
- running / paused / pending LLM / needs rollback World 摘要
- 最近 Agent 会话
- 快捷入口：新建 World、打开最近 World、查看 Trace / 日志

### 3.5 Agent 工作区

路由：`/agent/worlds/:worldId?`

Agent 工作区面向同一 World 下的多时期、多人物会话入口和世界状态摘要。

必须展示：

- World 列表与状态：running、paused、has active turn、has pending LLM call、needs rollback attention。
- 当前 World 的 `WorldMainlineCursor` 摘要。
- 当前主线、过去线、未来预演会话入口。
- Session Launcher：创建会话时显式选择 `Character` / `Director` 视角；`Character` 模式从当前 World 角色列表中选一个扮演对象，`Director` 模式不绑定角色。
- 角色、地点、最近 turn、最近 Trace 和冲突提示。
- 进入 World Editor、Trace Viewer、日志详情的入口。

Agent 工作区不直接编辑结构化 Truth；结构化编辑必须进入 World Editor。Agent 工作区也不得混入 ST 资源入口。

Agent 会话入口的交互边界：

- 会话创建时必须选择 `period_anchor`；`session_kind` 由后端根据 `WorldMainlineCursor` 自动推导，前端不让用户手动指定 mainline / retrospective / future_preview。
- `Character` 模式与 `Director` 模式使用不同的输入权限模型：前者默认把普通文本优先解释为角色动作 / 发言 / 内心，后者默认把普通文本优先解释为场景候选与导演偏置。
- 聊天页页头必须持续显示当前会话的 `session_kind`、`player_mode` 和扮演对象标签，如“扮演：林清寒”或“视角：导演”。

### 3.6 ST 资源库

路由：

- `/st/resources/characters`
- `/st/resources/worldbooks`
- `/st/resources/presets`
- `/st/resources/regex`

资源库统一管理 ST 资源和可复用配置资源。Agent 领域对象不进入这组页面。列表页必须支持搜索、标签 / 类型筛选、导入、导出、复制、删除影响预览和详情编辑入口。

资源类型边界：

- 角色卡页负责 TavernCard V3 创建、编辑、导入、导出和未知字段保留提示。
- 世界书页负责外部世界书与角色卡内嵌 CharacterBook 的查看和编辑入口；词条 `content` 使用 Structured Text Editor，默认 Plain，可切 JSON / YAML 作为 LLM 可读的结构化正文，但仍保存为 string。
- 预设页负责 sampler、instruct、context、sysprompt、reasoning、prompt 预设；大段模板正文使用 Structured Text Editor，允许 Plain / JSON / YAML 组织 prompt 文本。
- Regex 页负责 global / preset / scoped 脚本、授权状态和作用点提示；`findRegex` / `replaceString` 使用 Structured Text Editor 的 Plain 诊断，Regex 编译合法性仍由 Regex 模块判断。

资源库中的删除、重命名、导出和批量操作必须显示影响摘要；与会话绑定的资源不得静默断链。

### 3.7 Agent World Editor

路由：`/agent/worlds/:worldId/editor`

Agent World Editor 在主 Shell 内打开，但内部遵循 [40_agent_world_editor.md](40_agent_world_editor.md) 的四区结构：

- World 导航。
- 实体导航。
- 编辑区。
- 校验与影响面板。

Shell 级目标规则：

- 从 Agent 工作区进入时必须保留当前 World 上下文。
- World running、active turn、pending LLM call 或 unresolved rollback task 存在时，保存按钮禁用。
- 即使保存禁用，用户仍可编辑 draft、运行 validation、查看 impact summary。
- GodOnly、private、apparent、self_belief 字段必须有稳定标识，避免作者误以为这是角色可见内容。

### 3.8 日志与调试

路由：`/logs`

日志页用于观察系统，不驱动业务状态。页面详细数据边界、命令边界、清理与导出规则见 [30_logging_and_observability.md](30_logging_and_observability.md) 第 8 节。首版应优先完成元数据列表、LLM 请求详情、stream chunk 查看和 Trace 双向跳转；容量统计、长期未更新 World 提示、导出和手动清理属于日志管理增强。

必须区分：

- 全局运行 Logs：应用启动、ST LLM 调用、Provider 错误、设置变更、清理任务。
- World 内 Logs：Agent 回合相关 LLM 调用、Provider 错误、异常事件。
- Agent Trace：回合决策、Active Set、Layer 2 派生、CognitivePass、验证、OutcomePlanner、SurfaceRealizer 和提交记录。

页面布局：

- 左侧筛选栏：来源、类型、级别、状态、时间范围、Provider、模型、World、Turn、Trace、request 精确搜索。
- 中央列表：按时间倒序展示日志元数据，默认不加载大 JSON 正文。
- 右侧检查面板：按需展示脱敏 request / response / schema / stream chunks / readable text / Trace step / 异常事件。
- 顶部动作栏：范围切换、关键词搜索、时间范围、刷新、导出和清理管理入口。

首版 MVP 要求：

- 支持按 request_id、world_id、scene_turn_id、trace_id、provider、model、status、时间范围筛选。
- LLM 日志可查看脱敏后的 request / response / schema / stream chunks / assembled_text / readable_text。
- 从 Agent Trace 可跳转到关联 LLM request；从日志也可跳回相关 World / turn。
- ST 模式日志只能出现在全局运行 Logs；Agent 回合相关日志必须能在对应 World 范围查到。
- 日志清理入口只提供安全入口或跳转；实现手动清理 / 导出 / 容量统计时，必须先预览影响并遵守 [30_logging_and_observability.md](30_logging_and_observability.md) 的自动清理与用户确认边界。

交互约束：

- 点击 World / Turn / Trace 链接只改变查看位置或跳转上下文，不触发回放、回滚、重新生成或状态修复。
- 展开原始 JSON 前提示其中可能包含本地调试资料；复制按钮只复制已显示的脱敏内容。
- 列表必须分页或虚拟化，避免一次性读取 request / response / stream chunk 正文。
- 任何清理 World 内日志或 Trace 的动作都必须进入确认流程，并明确展示不会自动删除关键 Agent Trace 与被提交记录引用的日志。

### 3.9 API 配置与设置

路由：

- `/api-configs`
- `/settings`

API 配置页管理全应用共享的 `api_configs/` 配置池。第一版必须把 OpenAI Responses、OpenAI Chat Completions、Gemini、Anthropic、DeepSeek、Claude Code Interface 作为一等适配目标展示。

设置页承载：

- 主题：跟随系统、亮色、暗色。
- 数据目录查看与后续迁移入口。
- 全局运行配置安全子集。
- 日志保留策略安全子集。
- 默认 ST API 配置与默认 Agent LLM Profile。

可能改变旧世界语义的 World 规则配置不得混入普通全局设置页；应从对应 Agent World 的设置或 World Editor 进入，并显示影响警告。

---

## 4. 通用交互模式

### 4.1 资源列表

角色卡、世界书、预设、Regex、Agent World、会话和日志列表共享同一套列表语义：

- 列表顶部固定搜索框、类型筛选、排序和批量选择入口。
- 资源项必须展示名称、类型 / 模式、最近更新时间、绑定状态和风险徽标。
- ST 资源显示兼容性徽标：TavernCard V3、CharacterBook、ST 世界书、Preset、Regex scope。
- Agent 资源显示运行状态徽标：running、paused、active turn、pending LLM、needs rollback、validation warning。
- 选中项只改变主工作区详情，不自动执行打开会话、提交、删除或导出等动作。
- 删除、批量移动、导出和覆盖导入必须进入确认流程，并显示影响摘要。

列表空状态必须给出直接动作，但不使用大段说明：

- 资源为空：显示导入、新建或打开目录入口。
- 搜索无结果：显示清除筛选入口。
- 数据目录不可读：显示错误摘要和打开设置入口。

### 4.2 详情编辑

编辑详情页统一使用“摘要头 + 分组表单 + 原始扩展字段”的结构：

- 摘要头展示资源名、来源、保存状态、兼容性 / 校验状态和最近修改时间。
- 高频字段使用类型化表单；低频兼容字段放入折叠区或 JSON editor。
- 未保存 draft 与已保存 snapshot 必须分离；离开页面、切换资源或执行导入覆盖前提示保留 / 丢弃 / 取消。
- 保存前先做本地 schema 校验；涉及引用、访问权限、Provider 能力或 Agent World 状态时再调用后端 validation。
- 成功保存后只刷新受影响资源；不得因为列表刷新丢失当前滚动位置和选中项。

大文本字段统一接入 Structured Text Editor：

- 用户可在 Plain / JSON / YAML 中选择字段允许的模式。
- JSON / YAML 保存前自动格式化并复跑解析；解析失败时不改写 draft。
- Plain 保存前只做保守缩进矫正，不改变文本语义。
- 编辑器 diagnostics 与父级 schema validation 分区显示；任一 blocker 都阻止保存。
- ST 字符串字段选择 JSON / YAML 时，是把 prompt/content 写成结构化文本供 LLM 阅读，保存形态仍是 string；Agent structured 字段必须解析为 `json_value` 后再进入业务 validator。

### 4.3 检查面板

右侧检查面板用于解释当前对象和当前动作，不作为主要编辑入口：

- 默认展示摘要、绑定关系、运行状态、校验结果和最近事件。
- validation blocker 使用错误色，warning 使用警告色，普通建议使用中性色。
- ImpactSummary 必须按“阻断 / 警告 / 将变更 / 可回滚信息”分组。
- LLM request 详情默认脱敏；展开原始 request / response 前显示数据敏感提示。
- Trace 片段只读；跳转按钮只能改变视图，不得重放、提交或修正状态。

### 4.4 确认与反馈

- 普通保存成功使用 toast；批量导入、重建索引、日志清理等长任务使用任务进度条。
- 破坏性操作使用 modal，并要求用户能看到对象名、引用数量、不可逆风险和 rollback 能力。
- 运行中被锁定的操作不隐藏入口，显示 disabled 原因和可行下一步。
- 后端 validation 失败时保留 draft，并把焦点定位到第一个 blocker 对应字段或检查面板条目。
- 用户主动取消 LLM 生成、导入或 validation 时，UI 必须进入稳定的 canceled 状态，不显示为失败。

### 4.5 Agent 输入框与命令面板

Agent 会话输入框以“自然 RP 优先，显式标记增强”为原则：

- 普通文字：动作 / 旁白候选
- `*...*`：内心活动
- `"..."` / `“...”`：引号片段；前端只做轻量高亮，不硬判是否一定是对白
- `[[...]]`：导演块
- `/command ...`：元命令

命令模式规则：

- 单次发送内容在 `trim_start()` 后只要以 `/` 开头，输入框立即进入 Command 模式。
- Command 模式下展开命令候选面板，不再做普通 RP 高亮或结构提示。
- 未知命令或参数错误只显示轻量 toast warning，并忽略本次发送；不进入剧情回合，不写聊天消息。

第一版命令面板至少支持：

- `/scene`
  - 输入后展开场景 / 地点候选列表
  - 默认展示最近场景、附近地点、常用地点
  - 支持继续输入关键词过滤
  - 选中后形成可发送的命令草稿，并向后端提交稳定地点引用，而不是要求用户手输内部 ID
- `/back`
  - 输入后展开当前 session 历史轮次列表
  - 每项显示轮次摘要、故事时间、正史状态和回退风险
  - 选中后形成可发送的命令草稿，并向后端提交稳定 turn 引用
  - UI 文案使用“回退到此轮 / 截断此后会话”，避免误导成“只删聊天消息”
- `/fork`
  - 输入后展开 World 复制确认面板
  - 展示源 World 名称、当前会话 / 当前轮次摘要，以及建议的新 World 标题
  - 发送时提交源 World 稳定引用与可选源 turn 引用
  - 成功后切换到新 World 工作区或新 Session Launcher，而不是留在当前 World 内开启平行主线

命令面板交互要求：

- 键盘优先：方向键切换、`Tab` 补全、`Enter` 发送、`Esc` 关闭面板但保留输入
- 不使用阻断式 modal 作为常规命令确认；`/back` 与 `/fork` 的风险 / 结果提示以内嵌说明或状态标签展示
- 清空 `/` 前缀后自动退出 Command 模式并恢复普通输入态

---

## 5. 路由与状态边界

### 5.1 路由约定

```text
/mode-select
/st
/st/chat/:sessionId?
/st/resources/characters
/st/resources/worldbooks
/st/resources/presets
/st/resources/regex
/agent
/agent/worlds/:worldId?
/agent/worlds/:worldId/sessions/:sessionId
/agent/worlds/:worldId/editor
/api-configs
/logs
/settings
```

路由只表达用户当前位置和可分享的页面上下文。抽屉展开、列表筛选、临时选中项、未提交草稿等 UI 临时状态不强制写入路由。

新增路由边界规则：

- 模式切换必须通过顶层路由分区表达，不允许继续依赖 page name 前缀在同一 Shell 内做大规模条件分支。
- `STShell` 只匹配 `/st/**`；`AgentShell` 只匹配 `/agent/**`；共享页不应挂在任一模式壳层内部。
- 若要兼容旧路由 `/library`、`/chat/st/*`、`/resources/*`，必须先做到新路由的显式 redirect，并在过渡期保留历史入口。

### 5.2 Pinia store 边界

当前 store 现状与目标职责：

| Store | 职责 |
|---|---|
| `appShell` | 当前模式、模式切换上下文、共享导航状态、主题、最近访问、全局 toast / modal 状态 |
| `chat` | ST 会话、消息、生成状态、输入草稿 |
| `characters` | 角色卡索引、详情、导入导出 |
| `worldbooks` | 世界书索引、条目编辑、导入导出 |
| `runtime` | ST 运行时组装预览、世界书注入和 Provider 参数映射相关前端状态 |
| `agent` | Agent World、会话、运行状态、主线光标、Trace 入口摘要 |
| `agentWorldEditor` | editor snapshot、draft、validation result、impact summary、commit result |
| `settings` | API 配置、预设选择、主题偏好、运行配置草稿 |

Shell UI 状态不得写入业务 store；业务 store 不应控制导航展开、抽屉开关或主题。

模式分离后的额外规则：

- `appShell` 只管理模式与共享 UI 状态，不直接承载 ST / Agent 业务列表。
- ST 与 Agent 的最近记录建议拆分为 `recentStSessions`、`recentAgentSessions`、`recentStResources`、`recentAgentWorlds`，不要继续复用模糊的混合数组。
- `ContextList` 与 `InspectPanel` 不再做单组件 route-aware 大分支，改为 `STContextList` / `AgentContextList` / `SharedContextList` 与对应检查面板。

---

## 6. 视觉与组件规则

- 使用 Vue 3 + TypeScript + Naive UI + Pinia + Vue Router。
- 页面主色不得形成单一色相统治；暗色主题也必须保留清晰的层级、警告色、成功色和中性色。
- 表单、表格、树、tabs、segmented control、select、switch、slider、popover、drawer 优先使用 Naive UI 原生组件。
- 工具按钮优先使用图标；不熟悉的图标必须有 tooltip。
- 固定格式元素要有稳定尺寸，避免 hover、动态标签或加载文案导致布局跳动。
- 紧凑界面内的标题字号必须克制；只有真正的页面级标题可使用较大字号。
- 所有按钮、标签、列表项文本必须在窄宽度下换行或截断，不得与相邻控件重叠。

### 6.1 主题 token

首版只定义少量稳定 token，具体值由 Naive UI theme override 承载：

| Token | 用途 |
|---|---|
| `color-bg-app` | 应用最底层背景 |
| `color-bg-surface` | 主工作区、列表和检查面板背景 |
| `color-bg-subtle` | 表格 header、列表 hover、只读字段 |
| `color-border-subtle` | 区域分隔线、列表分割线 |
| `color-text-primary` | 标题、主要正文 |
| `color-text-secondary` | 摘要、metadata、辅助说明 |
| `color-status-success` | 可用、已保存、validation passed |
| `color-status-warning` | 有警告、Provider 能力受限、影响需确认 |
| `color-status-danger` | blocker、删除、不可恢复错误 |
| `color-status-info` | running、pending、同步中 |

主题要求：

- 亮色主题使用高可读中性色作为主背景，不用大面积纯白卡片堆叠。
- 暗色主题避免整页深蓝 / 紫蓝单色倾向；列表、主区和检查面板必须有可辨别层级。
- 状态色不只依赖颜色表达，必须同时有 icon、文本或 tooltip。
- ST 模式和 Agent 模式可以有轻微 accent 差异，但不得改变组件结构或交互语义。

### 6.2 密度与排版

- 默认字号 14px；密集表格和 metadata 可用 12px；页面标题不超过 24px。
- 行高保持稳定：列表项 40px / 56px 两档，表格行 36px / 44px 两档。
- 表单 label 左对齐或顶部对齐按页面密度选择；同一表单内不混用。
- 长资源名、Provider model 名、Knowledge summary 必须支持中部截断或多行显示。
- 编辑器、日志和 JSON viewer 使用等宽字体，并保留复制按钮和横向滚动。
- Structured Text Editor 顶部固定显示 Plain / JSON / YAML segmented control、Format 按钮、diagnostics 状态和行列位置。

---

## 7. 关键工作流

### 7.1 首次启动

1. 进入 `/mode-select`。
2. 检查数据目录、配置目录和全局 Logs 数据库是否可读写。
3. 若没有 API 配置，在模式选择页和对应模式首页都显示 API 配置缺失状态和创建入口。
4. 若已存在上次访问模式且用户未要求切换，可自动跳转到 `/st` 或 `/agent`。
5. 不弹出阻断式向导；只有数据目录不可用或配置损坏时使用 modal。

### 7.2 导入 ST 资源

1. 用户从 ST 首页或 ST 资源库选择导入。
2. 前端展示文件解析摘要：资源类型、名称、版本、未知字段、内嵌 CharacterBook / Regex。
3. 如会覆盖现有资源，进入覆盖确认并显示引用会话数量。
4. 保存后保留原始未知字段，并在详情页显示兼容性提示。
5. 导入失败时保留错误详情和重试入口，不生成半成品资源。

### 7.3 打开 Agent World 并进入编辑器

1. 用户从 `/agent` 或 `/agent/worlds/:worldId?` 打开 World。
2. Agent 工作区先展示运行状态、主线光标、最近会话和风险提示。
3. 点击 World Editor 进入 `/agent/worlds/:worldId/editor`。
4. 若 World 不满足 paused-only 提交条件，编辑器仍可编辑 draft 和运行 validation，但提交按钮禁用。
5. 提交成功后返回 editor snapshot，Agent 工作区摘要同步刷新。

### 7.4 查看日志与 Trace

1. 用户从日志页筛选 request、world、turn、trace 或 provider。
2. 点击日志项打开右侧检查面板，展示脱敏 request / response 摘要。
3. 若有关联 Trace，提供跳转到 Trace 片段；若有关联 World / turn，提供返回上下文入口。
4. 日志页不得提供任何修改业务状态的动作。

---

## 8. 验收场景

文档阶段验证：

- `git diff -- README.md docs AGENTS.md`
- `git status --short`

目标实现验收：

以下条目用于后续前端完整实现后的验收，不代表当前仓库已经全部满足。

- 启动后默认进入 `/mode-select`；存在上次模式时允许恢复到 `/st` 或 `/agent`。
- 用户可从 ST 首页跳转到最近 ST 会话、角色卡、世界书、预设、API 配置。
- 用户可从 Agent 首页跳转到最近 World、Agent 会话、World Editor、日志。
- ST 聊天中切换 API 配置后，预设、世界书、Regex 授权和聊天 metadata 不变化。
- Agent World running 或存在 active turn 时，World Editor 保存禁用，但 validation 可运行。
- 日志页能区分全局 Logs、World Logs 与 Agent Trace，并支持 request / trace 双向跳转。
- 主题默认跟随系统，用户可手动固定亮色或暗色。
- 窄屏下上下文列表区和右侧检查面板可折叠，主工作区不被遮挡。
- 在 ST / Agent 间切换时，一级导航、上下文列表和检查面板会整体切换，不残留对方模式入口。
- 资源列表搜索无结果、资源为空、数据目录不可读时都有明确空状态和直接动作。
- 未保存 draft 在切换资源、离开页面或覆盖导入前会提示保留 / 丢弃 / 取消。
- Structured Text Editor 的 dirty 状态会并入父级 draft；JSON / YAML blocker 存在时保存被阻止并定位到对应行列。
- 破坏性操作必须显示影响摘要；运行中被锁定的操作必须显示 disabled 原因。
- 亮色 / 暗色主题下状态徽标不只依赖颜色表达，窄屏动作栏不会出现文本重叠。
