# 77 ST 多模态附件

本文定义 ST 模式下图片 / PDF 附件的本地存储、聊天记录引用方式、运行时请求组装、Provider 差异适配与降级边界。

角色卡、世界书、预设、Regex 与基础 ST 组装流程分别见 [71_st_character_cards.md](71_st_character_cards.md)、[72_st_worldbook_model.md](72_st_worldbook_model.md)、[74_st_presets.md](74_st_presets.md)、[76_st_regex.md](76_st_regex.md)、[75_st_runtime_assembly.md](75_st_runtime_assembly.md)。后端中立请求与 Provider 抽象见 [20_backend_contracts.md](20_backend_contracts.md)。

---

## 1. 目标与边界

第一版只把两类二进制输入当作一等聊天附件：

- `image`：PNG / JPEG / WEBP / GIF 等视觉输入。
- `pdf`：`application/pdf` 文档输入。

第一版不做的事：

- 不把任意 `docx/xlsx/html/md` 都当作“视觉文档”等价物。
- 不把 Provider 远端 `file_id` / `file_uri` 当作用户数据源。
- 不对不支持多模态的 Provider 静默做 OCR / 文本提取降级。

若后续支持其他文件类型，必须在 [20_backend_contracts.md](20_backend_contracts.md) 扩展中立 `ContentPart`，并补测试矩阵。

---

## 2. 三层分离：源文件 / 聊天记录 / Provider 传输

多模态附件必须严格分成三层：

1. **源文件层**：应用在 `./data/` 下保存的本地二进制文件，是唯一持久化真源。
2. **聊天记录层**：消息只保存 `attachment_id` 引用与展示元数据，不内嵌 base64。
3. **Provider 传输层**：按当前 Provider / model 能力，把同一附件映射为 inline bytes、Provider Files API 句柄或兼容内容块。

这三层不得混用：

- 聊天 JSON 不能保存整段 base64。
- Provider 返回的 `file_id` / `file_uri` 只是缓存，不可替代本地源文件。
- 运行时不得直接依赖用户输入的远程 URL 作为唯一可发送副本。

---

## 3. 本地存储布局

ST 模式在应用数据根目录新增统一附件库：

```text
./data/
├── chats/
│   ├── <session_id>.json
│   └── ...
└── chat_attachments/
    └── <attachment_id>/
        ├── source.bin
        ├── meta.json
        └── derived/
```

约束：

- `attachment_id` 使用生成的稳定 ID，不能直接使用用户文件名。
- `source.bin` 是唯一持久化真源；扩展名只保存在 `meta.json`。
- `derived/` 只放派生内容，例如缩略图、PDF 页预览、局部文本抽取缓存；删除派生内容不应破坏原会话。
- 用户粘贴或导入远程 URL 时，应用必须先把文件下载 / 镜像到 `./data/chat_attachments/`，再允许它进入聊天记录。

建议元数据：

```typescript
interface ChatAttachmentMeta {
  attachment_id: string;
  kind: 'image' | 'pdf';
  mime_type: string;
  original_filename?: string;
  sha256: string;
  size_bytes: number;
  created_at: string;
  source_origin: 'local_file' | 'clipboard' | 'imported_url';
  original_source_url?: string;   // 仅 provenance，不是发送契约
  width?: number;
  height?: number;
  page_count?: number;
}
```

`original_filename` 只是显示提示。任何本地路径都必须由 `storage::paths::safe_join` 或等价 helper 生成。

---

## 4. 聊天记录表示

ST 模式的消息内容从“纯文本字符串”扩展为有序 `parts`。第一版只允许 `user` 消息携带附件；`assistant` 输出仍以文本为主，不把模型返回的二进制结果纳入本设计。

```typescript
type ChatMessagePart =
  | { type: 'text'; text: string }
  | {
      type: 'attachment_ref';
      attachment_id: string;
      kind: 'image' | 'pdf';
      mime_type: string;
      display_name?: string;
      vision_detail?: 'auto' | 'low' | 'high' | 'original';
    };
```

规则：

- 文本与附件保持原始顺序，便于映射到各 Provider 的 `content[] / parts[]`。
- 聊天 JSON 不重复保存 `sha256`、原始字节或远端 `file_id`；这些属于附件元数据或运行时缓存。
- 老的纯文本聊天可在读取时视为单个 `{ type: 'text' }` part。
- 带附件的会话导出回纯 ST 聊天文件时，默认不承诺 100% 兼容；需要显式提示“导出为 RST 扩展格式”或“移除附件后导出文本版”。

---

## 5. Provider 上传缓存

为了避免多轮会话中重复发送同一大文件，RST 允许为附件维护“远端句柄缓存”：

```typescript
interface AttachmentUploadCacheEntry {
  attachment_id: string;
  api_config_id: string;
  provider_kind: string;
  remote_handle: string;      // file_id / file_uri / 兼容后端句柄
  transport: 'provider_file';
  created_at: string;
  last_verified_at?: string;
}
```

第一版实现建议把这类缓存与附件真源放在同一目录，例如：

```text
./data/chat_attachments/<attachment_id>/upload_cache.json
```

规则：

- 该缓存只是优化，不是数据源。
- 缓存 key 至少包含 `attachment_id + api_config_id + provider_kind`；必要时再加 `base_url` / tenant / account 维度。
- Provider 返回“文件不存在 / 已过期 / 无权限”时，运行时可删除当前连接对应的缓存句柄，自动重新上传一次并刷新缓存。
- 该“删缓存并重传一次”的恢复逻辑应同时覆盖普通 `chat`、结构化 `chat_structured` 与流式 `chat_stream` 路径，避免只有文本主路径具备恢复能力。
- 切换 API 配置、endpoint 或账号后，不得复用旧缓存句柄。

---

## 6. 运行时组装

### 6.1 总流程

```text
用户附加图片 / PDF
       ↓
Storage 导入到 ./data/chat_attachments/<attachment_id>/
       ↓
消息 parts 只写 attachment_ref
       ↓
RequestAssembler 读取消息 parts
       ↓
AttachmentResolver 读取 meta + source.bin
       ↓
CapabilityResolver 判断当前 Provider / model 是否支持 image / pdf
       ↓
ProviderRequestMapper 选择 inline / provider_file / 兼容内容块
       ↓
AIProvider.chat() / chat_stream()
```

### 6.2 选择原则

- **优先本地真源**：无论最初来自文件、剪贴板还是 URL，发送时都从本地 `source.bin` 出发。
- **优先原生多模态**：Provider 支持图片 / PDF 原生输入时，直接发送原生多模态；不做“先 OCR 再假装是同等输入”的静默降级。
- **优先可复用句柄**：对多轮会话、重复引用或较大附件，优先走 Provider Files API / file handle。
- **inline 只作受控路径**：适合小文件、一次性请求，或 Provider 某协议不支持可复用文件句柄时。
- **URL 不是默认传输**：即使 Provider 支持 URL 引用，RST 默认仍以本地镜像 + inline / 上传文件为主，避免第三方 URL 失效、权限漂移或泄露访问轨迹。

---

## 7. Provider 差异与默认映射

### 7.1 OpenAI Responses API

- 图片：映射为 `input_image`。
- PDF：映射为 `input_file`。
- 支持三种输入来源：inline base64、Files API `file_id`、外部 URL。
- 默认策略：
  - 图片：小图可 inline；多轮或大图优先上传 Files API，再发 `input_image.file_id`。
  - PDF：优先上传 Files API，再发 `input_file.file_id`；一次性小 PDF 可 inline。
- 当前实现要求：
  - RST 发送前可按当前 OpenAI 连接把本地 PDF 上传到 `/files`，并在该连接维度缓存 `file_id`，避免同一附件在多轮中重复上传。
  - 该缓存只对当前 `base_url + account/api_key` 生效，不能跨账号或跨 endpoint 复用。

### 7.2 OpenAI Chat Completions API

- 图片：映射为 `content[].image_url.url`，可用远程 URL 或 `data:` URL。
- PDF：映射为 `content[].file`，可用 `file_id` 或 base64 `file_data`。
- 不支持 file URL 形式的 PDF 输入。
- 默认策略：
  - 图片：使用本地文件转 `data:` URL；不依赖未明确承诺的 image `file_id` 路径。
  - PDF：优先上传 Files API，用 `type=file + file_id`；小 PDF 可 base64 `file_data`。
- 当前实现要求：
  - 与 OpenAI Responses 共用同一份 PDF 上传 / `file_id` 复用逻辑，但缓存命中范围仍限定在当前 OpenAI 连接。

### 7.3 Anthropic Messages API

- 图片：`content[].type=image`，`source.type` 可为 `base64` / `url` / `file`。
- PDF：`content[].type=document`，`source.type` 可为 `url` / `base64` / `file`。
- 多轮含大附件时，Anthropic 官方明确建议使用 Files API 减少 payload。
- 默认策略：
  - 图片 / PDF：小文件可 base64；多轮或较大文件优先 Files API `file_id`。
- 当前实现要求：
  - RST 发送前可按当前 Anthropic 连接把本地 PDF 上传到 `/files`，并在该连接维度缓存 `file_id`。
  - 该路径依赖 Anthropic Files API beta；若兼容后端不支持，应回退到 inline base64，而不是改写本地源文件或静默删附件。

### 7.4 Gemini `generateContent`

- 图片：支持 inline bytes，也支持先上传到 Files API，再在 `parts` 中引用 `file_uri`。
- PDF：支持 inline `application/pdf`，也支持 Files API `file_uri`。
- Gemini 不以“让模型自己拉第三方 URL”作为主文档化输入路径；官方示例是先下载 / 读字节再发送，或先上传到 Files API。
- 默认策略：
  - 图片：小图 inline；大图或重复使用走 Files API。
  - PDF：默认优先 Files API；小 PDF 可 inline。
- 当前实现要求：
  - RST 发送前可按当前 Gemini 连接把本地 PDF 上传到 Files API，并在该连接维度缓存 `file_uri`。
  - `file_uri` 只作运行时复用句柄；切换 API key、endpoint 或项目后不得复用旧句柄。

### 7.5 DeepSeek Chat Completions

- 当前官方聊天接口 `messages[].content` 仍是字符串文本。
- 结论：第一版视为 **不支持原生图片 / PDF 输入**。
- 默认策略：发现消息中含 `attachment_ref` 时，在发请求前直接报能力错误；不得静默做 OCR / 文本抽取替代。

### 7.6 Claude Code Interface

- 该适配目标不是单一官方稳定网络协议，而是“Claude Code 风格消息 / 工具 / 环境接口”兼容面。
- 本仓库参考文档已记录 `image` / `document` 等内容块，但也明确“最小复刻可先只实现 text”。
- 默认策略：
  - 运行时必须先看后端 capability 或兼容层声明。
  - 若后端声明支持 `image` / `document` block，则按声明映射。
  - 若未声明，则与 DeepSeek 一样 fail fast，不做静默降级。

---

## 8. 能力探测与错误边界

API 配置除了 Provider 类型外，还必须暴露或缓存最少能力快照：

```typescript
interface ModelCapabilitySnapshot {
  supports_image_input: boolean;
  supports_pdf_input: boolean;
  preferred_transport: 'inline' | 'provider_file' | 'mixed';
}
```

规则：

- “Provider 支持多模态”不等于“当前 model 支持多模态”。
- 发送前必须同时校验 `provider_kind + model + protocol_variant`。
- 能力不满足时，错误发生在 `RequestAssembler / ProviderRequestMapper`，而不是把无效载荷发到远端后再等 4xx。
- 对 `DeepSeek` 和未声明多模态能力的 `Claude Code Interface`，错误消息应明确指出“当前 API 配置不支持图片/PDF 输入”。

---

## 9. 明确禁止的静默降级

以下做法第一版禁止：

- 用户发 PDF，程序自动抽纯文本后当作“等价 PDF”发给文本模型。
- 用户发图片，程序自动 OCR / caption 后再无提示地改发文本模型。
- 用户切到不支持多模态的 Provider 时，自动删除消息里的附件 part。
- 为了迁就某个 Provider，把本地聊天记录里的附件替换成远端 URL。

若未来需要“提取文本后再发送”的工作流，必须作为显式工具或用户确认操作，生成新的文本消息 / tool result，而不是伪装成原附件仍被原生发送。

---

## 10. 第一版实现建议

第一版建议按以下优先级实现：

1. 本地附件库：`chat_attachments/ + attachment_ref`
2. `OpenAI Responses` 图片 / PDF
3. `Anthropic Messages` 图片 / PDF
4. `Gemini` 图片 / PDF
5. `OpenAI Chat Completions` 图片 / PDF
6. `DeepSeek` 的 fail-fast 校验
7. `Claude Code Interface` 的 capability-negotiated path

理由：

- `OpenAI Responses / Anthropic / Gemini` 都提供清晰的一等多模态路径，适合作为统一抽象的主验证面。
- `OpenAI Chat Completions` 能做，但图片 / PDF 的传输形态比 Responses 更碎。
- `DeepSeek` 当前更适合明确阻止，而不是发明非官方语义。
- `Claude Code Interface` 天然依赖兼容后端能力，不应反过来驱动本地源模型。
