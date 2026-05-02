# 71 ST 角色卡

本文定义 ST 模式的角色卡兼容边界。实现时以 `E:\AIPlay\ST_latest` 中的真实代码为准，重点参考：

- `src/character-card-parser.js`
- `src/endpoints/characters.js`
- `src/validator/TavernCardValidator.js`
- `src/types/spec-v2.d.ts`
- `public/scripts/world-info.js`

世界书字段的完整模型见 [72_st_worldbook_model.md](72_st_worldbook_model.md)，世界书运行时注入见 [73_st_worldbook_injection.md](73_st_worldbook_injection.md)。

## 1. 兼容目标

ST 模式目标不是只理解 TavernCard JSON，而是复刻 SillyTavern 的角色卡体验：

- 角色列表、头像显示、导入、导出以 ST 的 PNG 角色卡为主要兼容对象。
- PNG 内的角色数据是 `tEXt` metadata，而不是旁挂 JSON。
- JSON 导入 / 导出必须保留完整卡对象和未知字段。
- `data.extensions.world` 按 ST 当前实现是外部世界书名称字符串。
- `data.character_book` 是内嵌 CharacterBook，默认不会直接参与运行时扫描；需要按 ST 流程导入为外部世界书并绑定。
- API 配置与角色卡、世界书、预设、聊天 metadata 解耦；这是 RST 的有意偏离，但导入 / 导出字段仍保持 ST 兼容。

## 2. TavernCard V3

SillyTavern 当前类型文件仍以 `spec-v2.d.ts` 描述基础数据形态，V3 validator 的实际检查更宽松：`spec === 'chara_card_v3'`，`Number(spec_version) >= 3.0 && < 4.0`，且 `data` 是对象。V3 validator 不检查 `data.character_book` 的内部结构；RST 可以在导入时做额外诊断，但不能因为 ST 未强校验的扩展字段缺失而丢弃整张卡。

RST 的 V3 数据面应按下列形态保存和导出：

```typescript
interface TavernCardV3 {
  spec: 'chara_card_v3';
  spec_version: string;  // Number(value) >= 3.0 && < 4.0
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

    // V3 / ST 扩展字段必须保留，例如 group_only_greetings、depth_prompt 等。
    [key: string]: any;
  };

  // 顶层未知字段也必须保留。
  [key: string]: any;
}
```

`CharacterBook` 的结构在 [72_st_worldbook_model.md](72_st_worldbook_model.md) 中定义。

## 3. PNG 角色卡

ST 的 PNG 读写规则如下：

1. 读取 PNG 时只读取 `tEXt` chunks。
2. 若存在 keyword 为 `ccv3` 的 `tEXt` chunk，优先读取它。
3. 若不存在 `ccv3`，再读取 keyword 为 `chara` 的 `tEXt` chunk。
4. `ccv3` 和 `chara` 的 text 都是 base64 编码的 UTF-8 JSON 字符串。
5. 写入 PNG 时，ST 会删除已有 keyword 为 `chara` 或 `ccv3` 的 `tEXt` chunk。
6. ST 总是写入一个 `chara` chunk；同时尝试把同一 JSON 改成 `spec = 'chara_card_v3'`、`spec_version = '3.0'` 后写入 `ccv3` chunk。

RST 导入 PNG 时必须遵守同样优先级：`ccv3` 优先，`chara` 兜底。导出 ST 兼容 PNG 时必须至少写回 `chara`，并应写回 `ccv3`，避免新版 ST 和旧版工具读取结果不一致。

## 4. 头像与文件身份

SillyTavern 的角色文件同时承担三件事：

- 角色卡容器：`data/<user>/characters/<avatar>.png`。
- 头像图片：角色列表和聊天头像直接使用该 PNG。
- 角色标识：很多运行时引用使用 `avatar` 文件名或去扩展名后的 character key。

RST 内部可以使用稳定资源 ID，但 ST 兼容导入 / 导出必须保留这个事实：导出的角色卡 PNG 本身就是头像容器。单独的头像 sidecar 文件只能作为 RST 内部缓存或编辑中间产物，不能替代 ST 兼容 PNG。

## 5. 角色卡与世界书

ST 角色卡与世界书有两类关系：

| 来源 | 字段 / 设置 | ST 实际行为 |
|---|---|---|
| 外部世界书绑定 | `data.extensions.world: string` | 字符串为 `data/<user>/worlds/<name>.json` 的 `<name>`，作为 Character lore 参与 `getSortedEntries()` |
| 角色额外世界书 | `world_info.charLore[].extraBooks` | 按角色文件名匹配，和 `extensions.world` 一起作为 Character lore |
| 内嵌 CharacterBook | `data.character_book` | 不直接扫描；用户执行 Import Card Lore 后转换为外部世界书并绑定到角色 |

因此 RST 不应把 `data.character_book` 当成运行时直接激活的 Character lore。正确流程是：

1. 导入角色卡时完整保留 `data.character_book`。
2. UI 提示该角色有内嵌世界书。
3. 用户选择导入时，按 [72_st_worldbook_model.md](72_st_worldbook_model.md) 的转换规则写出外部世界书。
4. 写入后把角色 `data.extensions.world` 设为该世界书名称。
5. 后续运行时只从外部世界书来源读取 Character lore。

若角色已经有 `data.extensions.world` 且该外部世界书存在，ST 不会自动把内嵌书混入运行时。内嵌书仍应保留在导入数据中，直到用户明确执行导入、替换或移除。

## 6. 导入导出规则

- JSON 导入：解析完整对象，保留顶层未知字段、`data` 未知字段和 `data.extensions` 未知字段。
- PNG 导入：按 `ccv3` → `chara` 顺序读取 base64 JSON，再走同一 JSON 导入路径。
- PNG 导出：以角色头像 PNG 为容器，写回 ST 兼容 `tEXt` metadata。
- 基础字段编辑：只更新对应字段，不重建整个对象，不清空未知扩展。
- 世界书绑定编辑：写入 `data.extensions.world` 的 ST 世界书名称字符串；RST 内部 `lore_id` 到 ST 名称的映射只能存在于 RST 自己的索引或导入记录中。
- 内嵌 CharacterBook 编辑：只有在编辑卡内嵌书时才改 `data.character_book`；导入为外部世界书时应保留 `originalData` 以便未来反向导出。

### 6.1 导入流程

RST 必须把角色卡导入拆成“容器解析”和“卡数据归一”两步：

1. 用户上传 PNG / JSON。
2. PNG：读取图片二进制，提取 `tEXt` metadata，按 `ccv3` → `chara` 优先级取得 base64 JSON；失败时报告“不是有效 ST 角色卡”，不能静默创建空角色。
3. JSON：直接解析为 TavernCard 对象；若是旧版卡，可转换为 V3 数据面，但必须保留原始未知字段。
4. 运行 TavernCard 基础校验；V3 只要求 `spec`、`spec_version` 和 `data` 满足 ST validator 兼容边界。
5. 建立 RST 内部角色资源 ID，同时保存 ST 兼容 `avatar` 文件名 / stem，供角色头像、chat metadata、Regex allow list、`world_info.charLore` 和导出映射使用。
6. 若 `data.character_book` 存在，只显示可导入提示；不得在导入角色卡时自动把内嵌书混入运行时世界书来源。

### 6.2 导出流程

RST 至少支持两种导出：

- ST PNG 导出：选择当前头像 PNG 作为容器，删除旧 `chara` / `ccv3` metadata，写入当前 TavernCard JSON；导出文件本身就是头像和角色卡。
- ST JSON 导出：导出完整 TavernCard JSON，不包含 RST 内部索引字段、API 配置 ID 或 provider 连接信息。

导出时必须从保存的原始对象出发做增量更新：只覆盖用户实际编辑过的标准字段、世界书绑定和内嵌 CharacterBook；未知顶层字段、`data` 未知字段、`data.extensions` 未知字段和扩展脚本字段都要原样保留。

## 7. 头像上传、存储与显示

ST 的头像不是独立于角色卡的资源，而是角色 PNG 文件本身。RST 可以有内部头像缓存，但 ST 兼容层必须遵守以下规则：

- 导入 PNG 角色卡时，上传的 PNG 同时作为头像图像和角色卡容器保存；文件 stem 形成 ST 兼容 `avatar` / character key。
- 导入 JSON 角色卡时，必须要求用户选择头像或使用默认头像生成 PNG 容器；保存后仍以 PNG 角色卡作为 ST 兼容导出基础。
- 更换头像时，只替换 PNG 图像像素与尺寸处理结果，必须把当前 TavernCard JSON metadata 重新写回新 PNG；不能因为换头像丢失角色字段、世界书绑定、Regex 脚本或未知扩展。
- 角色列表、聊天头像和角色选择器显示同一个角色 PNG 或其缩略图缓存；缩略图只是派生缓存，不是角色卡源数据。
- 删除或重命名角色时，需要同步更新 RST 内部角色索引和 ST 兼容 `avatar` 文件名映射；不得改写世界书、预设或 API 配置。

## 8. 会话关系

ST 会话只保存角色引用、聊天记录和 ST 兼容 `chat_metadata`。会话不保存 API 配置引用；切换 API 配置不能改写角色卡、角色世界书绑定或 scoped Regex 授权。运行时会话结构见 [75_st_runtime_assembly.md](75_st_runtime_assembly.md)。
