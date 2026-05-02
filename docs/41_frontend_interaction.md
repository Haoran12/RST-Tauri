# 41 前端交互主框架

本文档承载 RST 前端 UI 的主框架、一级页面、导航、布局、主题和交互状态边界。Agent World Editor 的业务规则、校验与提交细节见 [40_agent_world_editor.md](40_agent_world_editor.md)；结构化文本编辑器的 Plain / JSON / YAML 模式与诊断规则见 [42_structured_text_editor.md](42_structured_text_editor.md)。本文只定义它们在全应用 Shell 中的入口和衔接方式。

相关基础文档：

- 应用数据目录、配置分层与前后端模块结构见 [02_app_data_and_modules.md](02_app_data_and_modules.md)。
- ST 模式兼容原则见 [70_st_mode.md](70_st_mode.md)。
- ST 运行时全局状态与 API / 预设解耦规则见 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)。
- Agent 世界编辑器见 [40_agent_world_editor.md](40_agent_world_editor.md)。
- 结构化文本编辑器见 [42_structured_text_editor.md](42_structured_text_editor.md)。
- 日志与 Trace 边界见 [30_logging_and_observability.md](30_logging_and_observability.md)。

---

## 1. 设计定位

RST 首版 UI 是桌面 Tauri 工具型应用，而不是网站首页或移动优先聊天壳。默认首屏是资源工作台，用于管理角色卡、世界书、Agent World、预设、API 配置和最近会话；聊天页和 Agent World Editor 是从资源工作台进入的高频工作流。

默认交互取向：

- **资源管理首屏**：启动后进入 `/library`，显示最近资源、最近会话、配置健康状态和 Agent World 状态摘要。
- **工具型紧凑布局**：信息密度高于沉浸式聊天 UI，优先支持长期编辑、切换、比对和调试。
- **主题跟随系统**：首次启动跟随系统亮 / 暗主题，用户可在设置中手动固定主题。
- **双模式清晰但不断裂**：ST 模式和 Agent 模式在导航与状态标签上清楚区分，但共享 API 配置池、资源库入口和日志查看体验。

---

## 2. App Shell

首版使用四区结构：

1. **一级导航栏**：固定在左侧，承载资源工作台、ST 聊天、Agent、资源库、API 配置、日志、设置等入口。
2. **上下文列表区**：显示当前页面的资源列表、会话列表、World 列表或筛选结果；可折叠。
3. **主工作区**：承载聊天、编辑器、详情页、表单或日志查看器。
4. **右侧检查面板**：承载摘要、校验、影响分析、运行状态、请求详情或 Trace 片段；默认可折叠。

布局规则：

- 桌面宽屏默认显示一级导航、上下文列表区和主工作区；右侧检查面板按页面需要打开。
- 窄屏下上下文列表区与右侧检查面板改为抽屉，主工作区不得被永久遮挡。
- 一级导航项使用图标 + 短文本；常用动作按钮优先使用图标按钮并配 tooltip。
- 禁止用首页大 hero、营销卡片或说明性大段文案替代真实工作入口。
- 不使用卡片套卡片；页面区块使用全宽分区或无框布局，卡片仅用于重复资源项、弹窗和明确的工具面板。

### 2.1 Shell 尺寸与区域职责

默认桌面布局采用固定导航 + 可调侧栏 + 弹性主区：

| 区域 | 默认尺寸 | 最小 / 最大 | 职责 |
|---|---:|---:|---|
| 一级导航栏 | 72px | 折叠 52px | 顶层路由、全局状态徽标、设置入口 |
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

### 2.2 响应式断点

- `>= 1280px`：四区可同时显示，右侧检查面板按页面需要默认展开。
- `960px - 1279px`：一级导航、上下文列表区、主工作区常驻；右侧检查面板默认抽屉。
- `< 960px`：上下文列表区和右侧检查面板都进入抽屉；主工作区独占宽度。
- `< 720px`：动作栏按钮优先折叠为图标或 overflow menu；主操作仍必须一键可达。

窄屏抽屉打开时不得触发路由变化；关闭抽屉后必须保留列表筛选、当前选中项和未提交 draft。

---

## 3. 一级页面

### 3.1 资源工作台

路由：`/library`

资源工作台是默认首页，回答“我现在可以继续哪里、哪些资源需要处理、配置是否可用”。

必须展示：

- 最近 ST 会话与 Agent 会话。
- 最近编辑的角色卡、世界书、预设和 Agent World。
- 当前 `active_api_config_id` 的健康状态：未配置、可用、连接失败、缺少 key、Provider 能力受限。
- Agent World 摘要：running / paused、主线光标、最近 editor commit、待处理冲突或校验警告。
- 快捷入口：新建 ST 会话、新建 / 打开 Agent World、导入角色卡、导入世界书、管理 API 配置。

资源工作台不得直接执行复杂提交；破坏性操作必须跳转到对应资源页或弹出确认流程。

### 3.2 ST 聊天

路由：`/chat/st/:sessionId?`

ST 聊天页优先复刻 SillyTavern 文本聊天体验，同时保留 RST 的 Provider / 预设解耦规则。

页面结构：

- 上下文列表区：ST 会话列表、角色筛选、最近聊天。
- 主工作区：消息列表、输入框、生成 / 停止 / 重试 / 编辑消息等操作。
- 右侧检查面板：当前角色卡、Chat lore、Character lore、Global lore、预设、Regex、API 配置摘要。

交互边界：

- 切换 API 配置只影响下一次请求的连接配置和 Provider 参数映射。
- 切换 API 配置不得改变当前预设、世界书选择、Regex 授权状态、聊天 metadata 或角色卡扩展字段。
- ST 模式只写全局运行 Logs，不写 Agent Trace。

### 3.3 Agent 工作区

路由：`/agent/worlds/:worldId?`

Agent 工作区面向同一 World 下的多时期、多人物会话入口和世界状态摘要。

必须展示：

- World 列表与状态：running、paused、has active turn、has pending LLM call、needs rollback attention。
- 当前 World 的 `WorldMainlineCursor` 摘要。
- 当前主线、过去线、未来预演会话入口。
- 角色、地点、最近 turn、最近 Trace 和冲突提示。
- 进入 World Editor、Trace Viewer、日志详情的入口。

Agent 工作区不直接编辑结构化 Truth；结构化编辑必须进入 World Editor。

### 3.4 资源库

路由：

- `/resources/characters`
- `/resources/worldbooks`
- `/resources/presets`
- `/resources/regex`

资源库统一管理 ST 资源和可复用配置资源。列表页必须支持搜索、标签 / 类型筛选、导入、导出、复制、删除影响预览和详情编辑入口。

资源类型边界：

- 角色卡页负责 TavernCard V3 创建、编辑、导入、导出和未知字段保留提示。
- 世界书页负责外部世界书与角色卡内嵌 CharacterBook 的查看和编辑入口；词条 `content` 使用 Structured Text Editor，默认 Plain，可切 JSON / YAML 作为 LLM 可读的结构化正文，但仍保存为 string。
- 预设页负责 sampler、instruct、context、sysprompt、reasoning、prompt 预设；大段模板正文使用 Structured Text Editor，允许 Plain / JSON / YAML 组织 prompt 文本。
- Regex 页负责 global / preset / scoped 脚本、授权状态和作用点提示；`findRegex` / `replaceString` 使用 Structured Text Editor 的 Plain 诊断，Regex 编译合法性仍由 Regex 模块判断。

资源库中的删除、重命名、导出和批量操作必须显示影响摘要；与会话绑定的资源不得静默断链。

### 3.5 Agent World Editor

路由：`/agent/worlds/:worldId/editor`

Agent World Editor 在主 Shell 内打开，但内部遵循 [40_agent_world_editor.md](40_agent_world_editor.md) 的四区结构：

- World 导航。
- 实体导航。
- 编辑区。
- 校验与影响面板。

Shell 级规则：

- 从 Agent 工作区进入时必须保留当前 World 上下文。
- World running、active turn、pending LLM call 或 unresolved rollback task 存在时，保存按钮禁用。
- 即使保存禁用，用户仍可编辑 draft、运行 validation、查看 impact summary。
- GodOnly、private、apparent、self_belief 字段必须有稳定标识，避免作者误以为这是角色可见内容。

### 3.6 日志与调试

路由：`/logs`

日志页用于观察系统，不驱动业务状态。首版提供基础查看与跳转能力；高级 Trace 可视化、批量清理、导出和容量管理属于阶段八增强。

必须区分：

- 全局运行 Logs：应用启动、ST LLM 调用、Provider 错误、设置变更、清理任务。
- World 内 Logs：Agent 回合相关 LLM 调用、Provider 错误、异常事件。
- Agent Trace：回合决策、Active Set、Layer 2 派生、CognitivePass、验证、OutcomePlanner、SurfaceRealizer 和提交记录。

首版基础查看器要求：

- 支持按 request_id、world_id、scene_turn_id、trace_id、provider、model、status、时间范围筛选。
- LLM 日志可查看脱敏后的 request / response / schema / stream chunks / assembled_text / readable_text。
- 从 Agent Trace 可跳转到关联 LLM request；从日志也可跳回相关 World / turn。
- 日志清理入口只提供安全入口或跳转；阶段八实现手动清理 / 导出 / 容量统计时，必须遵守 [30_logging_and_observability.md](30_logging_and_observability.md) 的自动清理与用户确认边界。

### 3.7 API 配置与设置

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

---

## 5. 路由与状态边界

### 5.1 路由约定

```text
/library
/chat/st/:sessionId?
/agent/worlds/:worldId?
/agent/worlds/:worldId/editor
/resources/characters
/resources/worldbooks
/resources/presets
/resources/regex
/api-configs
/logs
/settings
```

路由只表达用户当前位置和可分享的页面上下文。抽屉展开、列表筛选、临时选中项、未提交草稿等 UI 临时状态不强制写入路由。

### 5.2 Pinia store 边界

建议新增或保留以下状态职责：

| Store | 职责 |
|---|---|
| `appShell` | 当前路由上下文、导航折叠、右侧面板、主题、最近访问、全局 toast / modal 状态 |
| `resourceLibrary` | 资源索引、搜索、筛选、导入导出任务、批量选择 |
| `chat` | ST 会话、消息、生成状态、输入草稿 |
| `characters` | 角色卡索引、详情、导入导出 |
| `worldbook` | 世界书索引、条目编辑、注入配置摘要 |
| `agent` | Agent World、会话、运行状态、主线光标、Trace 入口摘要 |
| `agentWorldEditor` | editor snapshot、draft、validation result、impact summary、commit result |
| `settings` | API 配置、预设选择、主题偏好、运行配置草稿 |

Shell UI 状态不得写入业务 store；业务 store 不应控制导航展开、抽屉开关或主题。

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

1. 进入 `/library`。
2. 检查数据目录、配置目录和全局 Logs 数据库是否可读写。
3. 若没有 API 配置，在资源工作台显示 API 配置缺失状态和创建入口。
4. 若存在最近资源，按最近使用时间展示 ST 会话、Agent World 和资源编辑入口。
5. 不弹出阻断式向导；只有数据目录不可用或配置损坏时使用 modal。

### 7.2 导入 ST 资源

1. 用户从资源工作台或资源库选择导入。
2. 前端展示文件解析摘要：资源类型、名称、版本、未知字段、内嵌 CharacterBook / Regex。
3. 如会覆盖现有资源，进入覆盖确认并显示引用会话数量。
4. 保存后保留原始未知字段，并在详情页显示兼容性提示。
5. 导入失败时保留错误详情和重试入口，不生成半成品资源。

### 7.3 打开 Agent World 并进入编辑器

1. 用户从 `/library` 或 `/agent/worlds/:worldId?` 打开 World。
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

后续实现验收：

- 启动后默认进入 `/library`。
- 用户可从资源工作台跳转到最近 ST 会话、Agent World、角色卡、世界书、API 配置。
- ST 聊天中切换 API 配置后，预设、世界书、Regex 授权和聊天 metadata 不变化。
- Agent World running 或存在 active turn 时，World Editor 保存禁用，但 validation 可运行。
- 日志页能区分全局 Logs、World Logs 与 Agent Trace，并支持 request / trace 双向跳转。
- 主题默认跟随系统，用户可手动固定亮色或暗色。
- 窄屏下上下文列表区和右侧检查面板可折叠，主工作区不被遮挡。
- 资源列表搜索无结果、资源为空、数据目录不可读时都有明确空状态和直接动作。
- 未保存 draft 在切换资源、离开页面或覆盖导入前会提示保留 / 丢弃 / 取消。
- Structured Text Editor 的 dirty 状态会并入父级 draft；JSON / YAML blocker 存在时保存被阻止并定位到对应行列。
- 破坏性操作必须显示影响摘要；运行中被锁定的操作必须显示 disabled 原因。
- 亮色 / 暗色主题下状态徽标不只依赖颜色表达，窄屏动作栏不会出现文本重叠。
