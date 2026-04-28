# LLM Request Format Replication Guide

本文档用于开发复刻本项目与 LLM 交互时的请求格式。结论基于源码静态调查，主请求路径位于 `services/api/claude.ts`，客户端构造位于 `services/api/client.ts`，内部消息到 API 消息的转换位于 `utils/messages.ts`。

## 1. 项目定位

该项目是 Claude Code CLI/Agent 的核心源码。它通过 Anthropic SDK 调用 Claude Messages API，提供交互式命令行 Agent、非交互式 `print` 模式、本地工具调用、文件读写、Shell 执行、MCP 工具、插件、技能、子 Agent 和远程会话能力。

LLM 主循环大致为：

1. UI/CLI/SDK 收集用户输入与会话历史。
2. `query()` 进入 Agent 循环。
3. `queryModelWithStreaming()` 或 `queryModelWithoutStreaming()` 发起模型请求。
4. `queryModel()` 组装 Anthropic Messages API 请求体。
5. 通过 `anthropic.beta.messages.create()` 发送请求。
6. 流式读取模型输出，遇到 `tool_use` 后执行本地工具，再把 `tool_result` 作为下一轮 `user` 消息发回模型。

## 2. API 形态

主路径使用 Anthropic SDK 的 Beta Messages API：

```ts
anthropic.beta.messages.create(
  {
    ...params,
    stream: true,
  },
  {
    signal,
    headers,
  },
)
```

非流式 fallback 也使用同一接口，但不设置 `stream: true`。

概念上等价于向 Anthropic Messages API 发送：

```http
POST /v1/messages
anthropic-version: 2023-06-01
content-type: application/json
```

实际请求由 Anthropic SDK、Bedrock SDK、Vertex SDK 或 Foundry SDK 封装，取决于运行时 Provider。

## 3. 顶层请求体

主请求体由 `paramsFromContext()` 构造。可复刻的核心结构如下：

```json
{
  "model": "claude-...",
  "messages": [],
  "system": [],
  "tools": [],
  "tool_choice": { "type": "auto" },
  "betas": [],
  "metadata": {
    "user_id": "{\"device_id\":\"...\",\"account_uuid\":\"...\",\"session_id\":\"...\"}"
  },
  "max_tokens": 8192,
  "thinking": {
    "type": "adaptive"
  },
  "temperature": 1,
  "context_management": {},
  "output_config": {},
  "speed": "fast"
}
```

字段说明：

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `model` | 是 | 规范化后的 Claude 模型 ID。源码通过 `normalizeModelStringForAPI()` 处理。 |
| `messages` | 是 | Anthropic Messages 格式的对话历史，由内部消息经 `normalizeMessagesForAPI()` 和 `addCacheBreakpoints()` 转换。 |
| `system` | 是 | 系统提示块数组，元素为 `{ type: "text", text, cache_control? }`。 |
| `tools` | 通常是 | 工具 schema 数组。无工具时可为空数组。 |
| `tool_choice` | 否 | 工具选择策略，通常为 auto 或指定工具。 |
| `betas` | 条件 | 需要启用 beta 能力时发送，如 thinking、tool search、prompt cache scope、structured outputs。 |
| `metadata` | 是 | Anthropic metadata，`user_id` 是 JSON 字符串，包含设备、账号、会话信息。 |
| `max_tokens` | 是 | 本轮最大输出 token。 |
| `thinking` | 条件 | 模型支持 thinking 且未禁用时发送。 |
| `temperature` | 条件 | thinking 禁用时发送；thinking 启用时通常不发送。 |
| `context_management` | 条件 | 相关 beta 启用时发送。 |
| `output_config` | 条件 | 用于 effort、structured output、task budget 等。 |
| `speed` | 条件 | fast mode 可用时为 `"fast"`。 |

## 4. Messages 格式

API 层的 `messages` 只保留 `user` 与 `assistant` 两类角色：

```ts
type MessageParam =
  | {
      role: 'user'
      content: string | ContentBlockParam[]
    }
  | {
      role: 'assistant'
      content: ContentBlockParam[]
    }
```

### 4.1 普通用户消息

```json
{
  "role": "user",
  "content": "请解释这个文件"
}
```

或内容块形式：

```json
{
  "role": "user",
  "content": [
    {
      "type": "text",
      "text": "请解释这个文件"
    }
  ]
}
```

### 4.2 助手文本消息

```json
{
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "这是一个 TypeScript CLI 项目。"
    }
  ]
}
```

### 4.3 工具调用消息

模型调用工具时，assistant 消息包含 `tool_use` 块：

```json
{
  "role": "assistant",
  "content": [
    {
      "type": "tool_use",
      "id": "toolu_01abc",
      "name": "Bash",
      "input": {
        "command": "rg -n \"queryModel\" .",
        "description": "Search request entry points"
      }
    }
  ]
}
```

工具执行结果作为下一条 `user` 消息发回：

```json
{
  "role": "user",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "toolu_01abc",
      "content": "services/api/claude.ts:1017:async function* queryModel(...)"
    }
  ]
}
```

工具失败时：

```json
{
  "role": "user",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "toolu_01abc",
      "is_error": true,
      "content": "Command failed: ..."
    }
  ]
}
```

### 4.4 多媒体与文档块

项目支持 image/document/search_result/tool_reference 等内容块，但复刻最小功能时可先只实现：

```json
{ "type": "text", "text": "..." }
```

以及工具相关：

```json
{ "type": "tool_use", "id": "...", "name": "...", "input": {} }
{ "type": "tool_result", "tool_use_id": "...", "content": "..." }
```

## 5. 消息归一化规则

源码不会直接把 UI 会话数组发给 API，而是执行 `normalizeMessagesForAPI()`。复刻时应至少实现以下规则：

1. 过滤不应进入模型的消息：进度消息、普通 system UI 消息、虚拟消息、合成 API 错误消息。
2. 本地命令输出类 system message 会转成 `user` 消息，使模型可引用历史命令输出。
3. 连续 `user` 消息会合并，因为部分 Provider 不支持连续 user turn。
4. assistant 消息中的 `tool_use.input` 会按工具 schema 做 API 兼容清理。
5. 如果未启用 tool search，需要移除 tool-search 专属字段，如 `caller`、`tool_reference`。
6. 修复工具配对关系：每个 `tool_use.id` 后续必须有匹配的 `tool_result.tool_use_id`。
7. 超过媒体数量限制时，丢弃较旧媒体块，避免 API 400。

最小复刻版可实现：

```ts
function normalizeMessagesForAPI(messages) {
  const result = []
  for (const msg of messages) {
    if (msg.type !== 'user' && msg.type !== 'assistant') continue
    if (msg.isVirtual) continue

    if (msg.type === 'user') {
      const last = result[result.length - 1]
      if (last?.role === 'user') {
        last.content = mergeUserContent(last.content, msg.content)
      } else {
        result.push({ role: 'user', content: msg.content })
      }
    }

    if (msg.type === 'assistant') {
      result.push({ role: 'assistant', content: normalizeAssistantContent(msg.content) })
    }
  }
  return ensureToolResultPairing(result)
}
```

## 6. System Prompt 格式

`system` 是 text block 数组，而不是单个字符串：

```json
[
  {
    "type": "text",
    "text": "x-anthropic-billing-header: cc_version=...; cc_entrypoint=cli;"
  },
  {
    "type": "text",
    "text": "You are Claude Code, Anthropic's official CLI for Claude.",
    "cache_control": {
      "type": "ephemeral"
    }
  },
  {
    "type": "text",
    "text": "项目上下文、用户上下文、工具约束等",
    "cache_control": {
      "type": "ephemeral"
    }
  }
]
```

系统提示构造顺序：

1. attribution header：`x-anthropic-billing-header: ...`
2. CLI 固定前缀：`You are Claude Code, Anthropic's official CLI for Claude.`
3. 运行时 system prompt。
4. advisor / Chrome / MCP 等条件性提示。
5. 按 prompt cache 策略拆块并添加 `cache_control`。

最小复刻可只发送：

```json
[
  {
    "type": "text",
    "text": "You are Claude Code, Anthropic's official CLI for Claude."
  }
]
```

## 7. Tools 格式

每个工具被转换成 Anthropic tool schema：

```json
{
  "name": "Bash",
  "description": "Executes a shell command...",
  "input_schema": {
    "type": "object",
    "properties": {
      "command": {
        "type": "string"
      },
      "description": {
        "type": "string"
      },
      "timeout": {
        "type": "number"
      }
    },
    "required": ["command"]
  }
}
```

可能出现的扩展字段：

```json
{
  "strict": true,
  "defer_loading": true,
  "cache_control": {
    "type": "ephemeral",
    "scope": "global",
    "ttl": "1h"
  },
  "eager_input_streaming": true
}
```

复刻时建议先实现标准字段：

```ts
{
  name: tool.name,
  description: tool.description,
  input_schema: zodToJsonSchema(tool.inputSchema)
}
```

内置工具池包括但不限于：

- `Bash`
- `Read`
- `Edit`
- `Write`
- `Glob`
- `Grep`
- `WebFetch`
- `WebSearch`
- `TodoWrite`
- `Agent`
- `TaskOutput`
- `TaskStop`
- MCP 资源与 MCP 工具

## 8. Prompt Cache 格式

启用 prompt caching 时，项目会在 system blocks、tools 或最后的消息内容块上添加：

```json
{
  "cache_control": {
    "type": "ephemeral"
  }
}
```

某些场景会带：

```json
{
  "cache_control": {
    "type": "ephemeral",
    "ttl": "1h",
    "scope": "global"
  }
}
```

消息级 cache marker 通常加在最后一个可缓存消息块上：

```json
{
  "role": "user",
  "content": [
    {
      "type": "text",
      "text": "继续",
      "cache_control": {
        "type": "ephemeral"
      }
    }
  ]
}
```

最小复刻可以不实现 prompt caching；要复刻 Claude Code 行为，则需要实现：

1. system prompt 分块 cache。
2. 工具 schema cache。
3. 最后一条消息 cache breakpoint。
4. cached microcompact 时的 `cache_edits` 和 `cache_reference`。

## 9. Thinking 字段

当模型支持 thinking 且未禁用时，项目发送：

```json
{
  "thinking": {
    "type": "adaptive"
  }
}
```

如果模型不支持 adaptive thinking，则发送预算式 thinking：

```json
{
  "thinking": {
    "type": "enabled",
    "budget_tokens": 4096
  }
}
```

禁用 thinking 时不发送，或内部配置为：

```json
{
  "thinking": {
    "type": "disabled"
  }
}
```

注意：thinking 启用时，源码通常不显式发送 `temperature`，因为 API 对 thinking 模式有约束。

## 10. Output Config

`output_config` 用于扩展输出控制：

```json
{
  "output_config": {
    "effort": "medium",
    "format": {
      "type": "json_schema",
      "schema": {}
    },
    "task_budget": {
      "type": "tokens",
      "total": 200000,
      "remaining": 150000
    }
  }
}
```

实际字段按模型能力和 beta header 控制。

## 11. Metadata

项目发送的 metadata 形态：

```json
{
  "metadata": {
    "user_id": "{\"device_id\":\"...\",\"account_uuid\":\"...\",\"session_id\":\"...\"}"
  }
}
```

也可通过 `CLAUDE_CODE_EXTRA_METADATA` 追加额外字段到该 JSON 字符串中。

## 12. Headers 与鉴权

客户端默认 headers：

```json
{
  "x-app": "cli",
  "User-Agent": "...",
  "X-Claude-Code-Session-Id": "..."
}
```

first-party Anthropic API 请求还可能带：

```json
{
  "x-client-request-id": "uuid"
}
```

鉴权来源：

1. Claude.ai subscriber OAuth：使用 `authToken`。
2. Anthropic API key：使用 `apiKey`。
3. `ANTHROPIC_AUTH_TOKEN` 或 API key helper：写入 `Authorization: Bearer ...`。
4. Bedrock：AWS 凭据或 `AWS_BEARER_TOKEN_BEDROCK`。
5. Vertex：Google Auth。
6. Foundry：Azure API key 或 Azure AD token。

支持自定义 headers：

```bash
ANTHROPIC_CUSTOM_HEADERS="Header-Name: value"
```

## 13. Provider 差异

运行时 Provider 由环境变量决定：

| Provider | 触发环境变量 |
| --- | --- |
| Anthropic first-party | 默认 |
| Bedrock | `CLAUDE_CODE_USE_BEDROCK=true` |
| Vertex | `CLAUDE_CODE_USE_VERTEX=true` |
| Foundry | `CLAUDE_CODE_USE_FOUNDRY=true` |

差异点：

- Bedrock 的部分 beta header 会放入 extra body 的 `anthropic_beta`，而不是顶层 `betas`。
- Vertex/Bedrock/Foundry 可能不接受 first-party 专属扩展字段。
- `defer_loading`、`eager_input_streaming`、部分 cache scope/TTL 需要按 Provider 和环境变量门控。

## 14. 最小可运行复刻请求

如果目标是先复刻最小 Agent 对话，可发送：

```json
{
  "model": "claude-sonnet-4-5",
  "max_tokens": 8192,
  "system": [
    {
      "type": "text",
      "text": "You are Claude Code, Anthropic's official CLI for Claude."
    }
  ],
  "messages": [
    {
      "role": "user",
      "content": "调查当前项目结构。"
    }
  ],
  "tools": [
    {
      "name": "Bash",
      "description": "Run a shell command and return stdout/stderr.",
      "input_schema": {
        "type": "object",
        "properties": {
          "command": {
            "type": "string"
          },
          "description": {
            "type": "string"
          }
        },
        "required": ["command"]
      }
    }
  ],
  "tool_choice": {
    "type": "auto"
  },
  "metadata": {
    "user_id": "{\"device_id\":\"local-dev\",\"account_uuid\":\"\",\"session_id\":\"session-local\"}"
  },
  "temperature": 1,
  "stream": true
}
```

模型返回 `tool_use` 后，执行本地命令，再继续发送：

```json
{
  "model": "claude-sonnet-4-5",
  "max_tokens": 8192,
  "system": [
    {
      "type": "text",
      "text": "You are Claude Code, Anthropic's official CLI for Claude."
    }
  ],
  "messages": [
    {
      "role": "user",
      "content": "调查当前项目结构。"
    },
    {
      "role": "assistant",
      "content": [
        {
          "type": "tool_use",
          "id": "toolu_01abc",
          "name": "Bash",
          "input": {
            "command": "ls",
            "description": "List project files"
          }
        }
      ]
    },
    {
      "role": "user",
      "content": [
        {
          "type": "tool_result",
          "tool_use_id": "toolu_01abc",
          "content": "src\nvendor\n"
        }
      ]
    }
  ],
  "tools": [
    {
      "name": "Bash",
      "description": "Run a shell command and return stdout/stderr.",
      "input_schema": {
        "type": "object",
        "properties": {
          "command": {
            "type": "string"
          },
          "description": {
            "type": "string"
          }
        },
        "required": ["command"]
      }
    }
  ],
  "tool_choice": {
    "type": "auto"
  },
  "metadata": {
    "user_id": "{\"device_id\":\"local-dev\",\"account_uuid\":\"\",\"session_id\":\"session-local\"}"
  },
  "temperature": 1,
  "stream": true
}
```

## 15. 推荐复刻顺序

1. 先实现 Anthropic Messages streaming 请求。
2. 实现 `user` / `assistant` / `tool_use` / `tool_result` 四种核心消息块。
3. 实现 1-3 个本地工具，例如 `Bash`、`Read`、`Edit`。
4. 实现工具执行循环：模型 `tool_use` -> 本地执行 -> 追加 `tool_result` -> 再请求模型。
5. 加入消息归一化：连续 user 合并、工具配对校验、无效消息过滤。
6. 加入 system prompt 分块。
7. 再加入 prompt caching、thinking、betas、MCP、tool search、子 Agent 等增强能力。

## 16. 源码索引

| 功能 | 文件 |
| --- | --- |
| 主模型请求组装 | `services/api/claude.ts` |
| streaming / non-streaming 调用 | `services/api/claude.ts` |
| Anthropic client / headers / provider | `services/api/client.ts` |
| 内部消息转 API 消息 | `utils/messages.ts` |
| system prompt 分块与 cache | `utils/api.ts`, `services/api/claude.ts` |
| 工具 schema 转换 | `utils/api.ts` |
| 工具注册表 | `tools.ts` |
| query 主循环 | `query.ts`, `QueryEngine.ts` |
| 模型/provider 解析 | `utils/model/*` |

