# RST-Tauri 开发规范

本文档定义 RST-Tauri 项目的开发规范、代码风格和工作流程。

## 项目概述

RST-Tauri 是基于 Tauri + Vue 3 + Rust 的双模式 AI 聊天应用：
- **SillyTavern 模式 (ST)**：复刻 SillyTavern 体验，JSON 文件存储
- **Agent 模式**：高级角色扮演系统，SQLite 存储，三层语义隔离

技术栈：
- 前端：Vue 3 + TypeScript + Naive UI + Pinia + Vue Router
- 后端：Tauri + Rust
- 存储：JSON (ST) / SQLite (Agent)

## 开发工作流

### 任务跟踪

**强制要求**：

1. **开发前**：阅读 `docs/Tasks_list.md`，了解当前进度和下一步任务
2. **开发中**：使用 Task 工具记录已完成和待完成的事项
3. **开发后**：更新 `docs/Tasks_list.md`，标记完成的任务并记录完成日期

#### Task 工具使用

1. 开始新功能/任务前，使用 `TaskCreate` 创建任务
2. 开始工作时，使用 `TaskUpdate` 将状态设为 `in_progress`
3. 完成后立即使用 `TaskUpdate` 将状态设为 `completed`
4. 定期查看 `TaskList` 了解整体进度

#### Tasks_list.md 维护

- 任务完成后，更新对应行的状态为 `✅`，填写完成日期
- 新增任务时，添加到对应阶段，保持编号连续
- 定期更新统计表和更新日志

### Git 提交规范

提交信息格式：
```
<type>: <subject>

<body>
```

类型：
- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档更新
- `style`: 代码格式（不影响功能）
- `refactor`: 重构
- `test`: 测试相关
- `chore`: 构建/工具相关

示例：
```
feat: 实现 AI Provider 抽象层

- 定义 AIProvider trait
- 实现 chat/chat_structured/chat_stream 三种调用模式
- 添加 LoggingAIProvider wrapper
```

### 分支策略

- `main`: 主分支，保持稳定
- `feat/<feature-name>`: 功能分支
- `fix/<bug-name>`: 修复分支

## 代码规范

### TypeScript/Vue 前端

1. **文件命名**：
   - Vue 组件：PascalCase，如 `WorldEditorShell.vue`
   - TypeScript 文件：camelCase，如 `api.ts`
   - 类型定义：camelCase，如 `character.ts`

2. **组件结构**：
   ```vue
   <script setup lang="ts">
   // 1. imports
   // 2. props/emits
   // 3. composables
   // 4. reactive state
   // 5. computed
   // 6. methods
   // 7. lifecycle hooks
   </script>

   <template>
   <!-- 模板内容 -->
   </template>

   <style scoped>
   /* 样式 */
   </style>
   ```

3. **Pinia Store 规范**：
   - 使用 Composition API 风格
   - Store 文件放在 `src/stores/`
   - 每个 store 职责单一，参考 `docs/41_frontend_interaction.md` 的边界定义

4. **类型定义**：
   - 所有 API 响应必须有类型定义
   - 与 Rust 端对应的类型放在 `src/types/agent/`
   - 避免使用 `any`，必须使用时添加注释说明原因

### Rust 后端

1. **模块结构**：
   - 每个 major 模块有 `mod.rs`
   - 子模块按职责划分，参考 `docs/02_app_data_and_modules.md`

2. **错误处理**：
   - 使用 `Result<T, String>` 作为 Tauri 命令返回类型
   - 错误信息要具体，便于调试

3. **异步处理**：
   - 使用 `async_trait` for trait 方法
   - 避免在等待远程 LLM 时持有数据库写事务

4. **日志规范**：
   - 所有 LLM 调用必须经过 `LoggingAIProvider`
   - API Key 等敏感信息必须在落库前脱敏

### 安全开发规范

1. **数据目录边界**：
   - 默认运行数据必须位于应用目录旁的 `./data/`
   - 只有用户显式选择或设置覆盖路径时，才允许写入其他数据根
   - 不得默认写入 AppData、Application Support、`~/.config` 等系统用户数据目录

2. **路径安全**：
   - 禁止把前端传入的 `id`、`name`、`filename`、导入 metadata 直接拼成路径
   - 所有 JSON / PNG / 资源文件访问必须通过 `storage::paths::safe_join` 或等价安全 helper
   - 必须拒绝绝对路径、`..`、Windows prefix、控制字符、平台保留字符和保留设备名
   - 导入文件名只能作为显示提示；持久化文件名优先使用内部稳定 ID

3. **日志与密钥**：
   - API Key、Authorization、`x-api-key`、Provider secret/token、代理用户名和密码必须在写入 SQLite 前脱敏
   - 脱敏必须发生在后端日志写入层，不能只依赖前端隐藏
   - Provider 原始错误、URL query、schema、request/response JSON 都属于脱敏范围

4. **Tauri 能力最小化**：
   - 默认关闭 `withGlobalTauri`
   - 不得添加 shell、filesystem、opener 等高权限插件或 broad capability，除非有明确命令边界和威胁模型说明
   - 新增 Tauri command 时必须说明输入校验、路径边界和日志脱敏行为

5. **运行时组装边界**：
   - ST 请求必须经过统一后端组装入口：加载 API 配置和预设，运行世界书注入，应用允许的 Regex prompt 变换，生成中立请求，再映射 Provider
   - 前端不得绕过组装入口直接拼 provider-bound prompt
   - `AIProvider` 只负责网络请求和格式映射，不参与世界书扫描、预设选择、资源保存或日志落库

## 架构约束

### 数据形态铁律

自由文本仅在三处出现：
1. 用户输入
2. SceneStateExtractor 输入
3. SurfaceRealizer 输出

其他中间节点必须为严格 schema JSON。

### 三层语义隔离

- **Layer 1 (Truth)**: 客观真相，仅编排器与结果规划/验证层访问
- **Layer 2 (Access)**: 逐角色可触及视图，每回合重建，无持久化
- **Layer 3 (Subjective)**: 主观心智，每回合 cognitive pass 后更新

**禁止跨层直接读写**。

### LLM 节点权限

| 节点 | 权限 |
|---|---|
| SceneInitializer | 公开上下文 + 场景相关私有约束 |
| SceneStateExtractor | 场景域 God-read |
| CharacterCognitivePass | 只读 L2 + prior L3 |
| OutcomePlanner | God-read，输出候选，不直接提交 |
| SurfaceRealizer | 只读 NarrationScope 派生输入 |

### 关键铁律

1. **KnowledgeAccessResolver 永不调 LLM**
2. **God 读取 ≠ 提交权限**
3. **日志不驱动业务**
4. **配置不在热路径做 IO**

## 文档索引

| 文档 | 内容 |
|---|---|
| `docs/01_architecture.md` | 总体架构、设计原则、LLM/程序边界 |
| `docs/02_app_data_and_modules.md` | 数据目录、模块结构、职责边界 |
| `docs/10_agent_data_model.md` | Agent 三层数据模型 |
| `docs/11_agent_runtime.md` | Agent 运行时主循环 |
| `docs/14_agent_persistence.md` | SQLite 表结构 |
| `docs/20_backend_contracts.md` | AI Provider 抽象 |
| `docs/41_frontend_interaction.md` | 前端 UI 主框架 |
| `docs/implementation_plan.md` | 实现阶段与里程碑 |
| `docs/Tasks_list.md` | 任务清单与进度跟踪 |
| `docs/91_test_matrix.md` | 测试矩阵 |

## 验收标准

每个阶段完成后需验证 `docs/91_test_matrix.md` 中对应的测试用例。

代码提交前：
1. 确保类型检查通过
2. 确保构建成功
3. 更新相关任务状态
