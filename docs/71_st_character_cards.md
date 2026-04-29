# 71 ST 角色卡

本文定义 ST 模式的角色卡兼容边界。世界书字段的完整模型见 [72_st_worldbook_model.md](72_st_worldbook_model.md)，世界书运行时注入见 [73_st_worldbook_injection.md](73_st_worldbook_injection.md)。

## 1. 兼容目标

RST 支持 TavernCard V3，并以 SillyTavern 当前 `spec-v2.d.ts` 和角色卡解析逻辑为准。角色卡导入 / 导出必须保留：

- 标准 `spec` / `spec_version` / `data` 字段。
- `data.extensions` 中 RST 不理解的扩展字段。
- `data.character_book` 中的内嵌 CharacterBook。
- `data.extensions.world` 中的外部世界书绑定信息。

## 2. TavernCard V3

```typescript
interface TavernCardV3 {
  spec: 'chara_card_v3';
  spec_version: string;  // >= 3.0 and < 4.0
  data: {
    name: string;
    description: string;
    personality: string;
    scenario: string;
    first_mes: string;
    mes_example: string;
    creator_notes: string;
    system_prompt: string;
    post_history_instructions: string;
    alternate_greetings: string[];
    tags: string[];
    creator: string;
    character_version: string;
    extensions: Record<string, any>;
    character_book?: CharacterBook;
  };
}
```

`CharacterBook` 的结构在 [72_st_worldbook_model.md](72_st_worldbook_model.md) 中定义。角色卡文档只声明角色卡拥有内嵌书，不重复定义世界书字段。

## 3. 角色卡与世界书

ST 角色卡可能通过两种方式携带 lore：

| 来源 | 字段 | 运行时处理 |
|---|---|---|
| 内嵌 CharacterBook | `data.character_book` | 转换为 ST 外部世界书 entry 形态后参与扫描 |
| 绑定外部世界书 | `data.extensions.world` | 作为 Character lore 来源参与世界书合并 |

若角色卡同时存在内嵌 `character_book` 和 `extensions.world` 绑定，RST 按 ST 兼容目标优先使用绑定的外部世界书；内嵌书仍保留在导入数据中，避免导出时丢失。

角色卡世界书绑定与 API 配置无关。`data.extensions.world`、内嵌 `data.character_book` 和角色 scoped Regex 授权不得以 Provider、model、endpoint 或 `active_api_config_id` 分组；切换 API 配置不能改写角色卡、切换角色绑定世界书或重新请求 scoped Regex 授权。

## 4. 导入导出规则

- 导入 PNG / JSON 角色卡时，解析出完整 TavernCard V3 JSON，并把原始未知字段保存在对应 `extensions` 或原始对象中。
- 导出角色卡时，不能只导出 RST 内部归一化字段；必须重建 TavernCard V3 结构。
- 编辑基础角色字段时，只更新 `data` 下对应字段，不重写 `extensions`。
- 编辑内嵌 CharacterBook 时，使用 [72_st_worldbook_model.md](72_st_worldbook_model.md) 的 CharacterBook 反向转换规则。

## 5. 会话关系

ST 会话只保存角色引用、聊天记录和 ST 兼容 `chat_metadata`，不保存 API 配置或预设引用。运行时会话结构见 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)。
