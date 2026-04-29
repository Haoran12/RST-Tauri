# 15 Agent Location System

本文定义 Agent 模式的结构化地点系统：地点层级、地区事实继承、相邻 / 路线图、路程估算与 SceneInitializer 的地点上下文输入。

Location System 属于 Layer 1 Truth Store。它提供可审计的结构化事实与程序派生结果，不由受限 LLM 自行猜测、检索或写入。

---

## 1. 目标与边界

Location System v1 支持：

- 解析地点名称与别名，得到稳定 `location_id`。
- 通过 `parent_id` 判断地点归属，例如 `c县 -> b州 -> a国`。
- 表达自然地理地带，例如山脉、平原、丘陵、高原、荒漠、流域、海域。
- 从父级地区继承公开事实，例如 `b州普遍尚武` 默认适用于 `c县`。
- 通过空间覆盖关系判断自然地理带对地点的影响，例如 `青岚山脉` 穿过 `临水县`。
- 记录地点之间的相邻、道路、河道、山口、传送阵等带权关系。
- 基于路线图估算多跳路程、耗时、风险和置信度。
- 为 `SceneInitializerInput.location_context` 提供公开、可访问、可追溯的地点上下文。

第一版不做：

- GIS 坐标与真实地图投影。
- 大规模自动从长篇设定抽取完整地图。
- 政治边界历史版本、复杂税制、经济物流模拟。
- 把低置信度推断自动固化为客观真相。

---

## 2. 核心原则

1. `parent_id` 是层级归属的权威事实，不靠地点名称或行政级别猜测。
2. `LocationLevel` 是程序推断用的抽象层级，`type_label` 是世界观 / UI 显示名称。
3. `WorldRoot` 是技术根节点；其下最高叙事地理域是 `Realm`。
4. 行政 / 组织 / 场所归属继承只沿 `parent_id` 链生效，子地点显式事实可以覆盖父级事实。
5. 自然地理影响通过 `LocationSpatialRelation` 生效，不改变 `parent_id`，不自动成为行政继承链。
6. 路程估算只使用 `LocationEdge` 路线图；层级相同或同父级只能产生低置信度邻近提示。
7. 程序可输出带置信度的估算；估算不得自动写回 `location_edges`、`location_spatial_relations` 或 `KnowledgeEntry.content`。
8. LLM 只能读取程序整理后的 `LocationContext`，不得伪造、改写或猜测 `location_id`。

---

## 3. 数据模型

### 3.1 LocationNode

```rust
pub struct LocationNode {
    pub location_id: String,
    pub name: String,
    pub aliases: Vec<LocationAlias>,
    pub polity_id: Option<String>,
    pub parent_id: Option<String>,
    pub canonical_level: LocationLevel,
    pub type_label: String,
    pub tags: Vec<String>,
    pub status: LocationStatus,
    pub metadata: serde_json::Value,
    pub schema_version: String,
}

pub enum LocationLevel {
    WorldRoot,        // 技术根节点：一个 Agent World 的地点图谱根
    Realm,            // 界域 / 大陆 / 位面 / 星球 / 大世界 / 梦境层
    Continent,        // 大陆级；可选，若 Realm 已是大陆可跳过
    NaturalRegion,    // 山地 / 平原 / 丘陵 / 高原 / 荒漠 / 流域 / 海域 / 湿地
    Polity,           // 国家 / 帝国 / 王国 / 城邦联盟
    MajorRegion,      // 州 / 省 / 大区 / 行省 / 道
    LocalRegion,      // 县 / 郡 / 领
    Settlement,       // 聚居地(乡镇 / 城区 / 营地)
    DistrictOrSite,   // 建筑 / 宗门山门 / 港口 / 遗迹 / 山谷
    RoomOrSubsite,    // 房间 / 院落 / 洞府 / 密室
}
```

`NaturalRegion` 指不属于明确行政区划的自然地理地带，例如山脉、平原、丘陵、高原、荒漠、森林带、流域、湖区、海域。它可以被 `Realm` / `Continent` / `MajorRegion` 包含，也可以覆盖、穿过或邻接多个 `MajorRegion` / `LocalRegion`；这种跨域关系由 `LocationSpatialRelation` 表达，不靠多重 `parent_id`。

`Settlement` 指有稳定居民、社会功能或常驻组织的聚居地点，例如城市、镇、乡、村、寨、集市镇、边堡、驿站聚落、宗门外围坊市或长期营地。它不是行政区域；县、郡、领地属于 `LocalRegion`，街区、港口、学院、宗门山门、遗迹等通常属于 `DistrictOrSite`，单个房间、院落、洞府、密室属于 `RoomOrSubsite`。

示例：

```text
world_root(WorldRoot)
└── 青岚界(Realm)
    └── 东洲(Continent)
        ├── 青岚山脉(NaturalRegion)
        └── 大梁国(Polity)
            └── 云北州(MajorRegion)
                └── 临水县(LocalRegion)
                    ├── 临水城(Settlement)
                    │   ├── 西市(DistrictOrSite)
                    │   └── 县衙(DistrictOrSite)
                    │       └── 后堂(RoomOrSubsite)
                    └── 白石村(Settlement)
```

### 3.2 国家 / 政体层级模板

不同国家可以配置显示名和允许父子关系，但程序核心推断仍使用 `parent_id` 与 `canonical_level`。

```json
{
  "polity_id": "da_liang",
  "level_labels": {
    "NaturalRegion": "自然地带",
    "Polity": "国",
    "MajorRegion": "州",
    "LocalRegion": "县",
    "Settlement": "城镇"
  },
  "allowed_parent_child": [
    ["Polity", "MajorRegion"],
    ["MajorRegion", "LocalRegion"],
    ["LocalRegion", "Settlement"],
    ["Settlement", "DistrictOrSite"],
    ["DistrictOrSite", "RoomOrSubsite"]
  ]
}
```

该模板用于编辑器校验、UI 显示和导入辅助，不是层级归属的第二套真相。

### 3.3 LocationSpatialRelation

`LocationSpatialRelation` 表达非树状空间关系，尤其是自然地理地带与行政区、聚居地、场所之间的覆盖、穿过、包含部分、邻接等关系。它不用于路线耗时计算；路线耗时仍由 `LocationEdge` 负责。

```rust
pub struct LocationSpatialRelation {
    pub relation_id: String,
    pub source_location_id: String,
    pub target_location_id: String,
    pub relation: LocationSpatialRelationKind,
    pub coverage: Option<CoverageEstimate>,
    pub confidence: FactConfidence,
    pub source: FactSource,
    pub schema_version: String,
}

pub enum LocationSpatialRelationKind {
    Overlaps,          // 两个区域有重叠
    Crosses,           // source 穿过 target
    SourceContainsPartOfTarget, // source 包含 target 的一部分
    SourcePartlyWithinTarget,   // source 的一部分位于 target 内
    AdjacentTo,        // 空间邻接但不表示可通行路线
    WithinNaturalBand, // source 位于 target 自然地理带影响范围内
}
```

示例：

```text
青岚山脉 overlaps 云北州
青岚山脉 crosses 临水县
临水县 source_contains_part_of_target 青岚山脉
白石村 within_natural_band 青岚山脉南麓
```

自然地理影响与行政继承分开：

- `云北州` 的行政 / 风俗事实沿 `parent_id` 可以被 `临水县` 继承。
- `青岚山脉` 的地形 / 气候 / 阻隔事实通过 `LocationSpatialRelation` 影响 `临水县` 或其中场景。
- 若两者冲突，程序必须把来源分别写入 `LocationContext`，由规则或用户设定决定优先级；不得把自然地理事实复制成行政父级事实。

### 3.4 LocationEdge

```rust
pub struct LocationEdge {
    pub edge_id: String,
    pub from_location_id: String,
    pub to_location_id: String,
    pub relation: LocationEdgeRelation,
    pub bidirectional: bool,
    pub distance_km: Option<DistanceEstimate>,
    pub travel_time: Option<TravelTimeEstimate>,
    pub terrain_cost: f32,
    pub safety_cost: f32,
    pub seasonal_modifiers: Vec<SeasonalRouteModifier>,
    pub allowed_modes: Vec<TravelMode>,
    pub confidence: FactConfidence,
    pub source: FactSource,
    pub schema_version: String,
}

pub enum LocationEdgeRelation {
    Adjacent,
    Road,
    RiverRoute,
    SeaRoute,
    MountainPass,
    ForestTrail,
    BorderCrossing,
    TeleportGate,
    ContainsShortcut,
}
```

`LocationEdge` 表达可通行关系或相邻关系。`parent_id` 只表达包含 / 归属，`LocationSpatialRelation` 只表达空间覆盖 / 穿过 / 重叠；两者都不自动生成路线边。若需要从县城到州府的路程，必须存在路线边或显式估算策略。

---

## 4. 地点解析

地点解析由 `LocationResolver` 程序执行：

1. 使用 `location_aliases` 查找候选 `location_id`。
2. 若唯一命中，返回该节点、父级链和公开可访问事实。
3. 若多重命中，返回 `LocationAmbiguity`，不得让 LLM 猜硬 ID。
4. 若未命中，LLM 可在作者编辑流程中提出候选节点；候选节点必须待用户确认或由导入规则确认后才写入 Layer 1。

同名地点必须靠 `polity_id`、父级链、上下文锚点、别名 locale 或用户确认消歧。

---

## 5. 地区事实继承与自然地理影响

`KnowledgeEntry { kind: RegionFact }` 的 `subject_id` 指向 `LocationNode.location_id`。这里的 `Region` 是知识分类名，覆盖国家、地区、聚居地、场所等可作为地理主体的 LocationNode。

行政 / 组织 / 场所归属事实的继承字段保存在 `content` 的结构化字段中：

```json
{
  "fact_type": "customs",
  "summary_text": "云北州民风尚武",
  "applies_to_location_id": "yunbei_state",
  "inheritance": {
    "inheritable": true,
    "applies_to_descendants": true,
    "max_depth": null,
    "blocked_location_ids": [],
    "override_policy": "child_overrides_parent"
  },
  "confidence": "asserted",
  "extensions": {}
}
```

查询 `临水县` 的地点上下文时，程序沿父级链合并行政 / 归属事实：

```text
world_root -> 青岚界 -> 东洲 -> 大梁国 -> 云北州 -> 临水县
```

父级继承合并规则：

- 父级 `inheritable = true` 且 `applies_to_descendants = true` 的事实默认进入候选。
- `blocked_location_ids` 命中当前地点或其父链时不继承。
- 子地点同 `fact_type` 显式事实按 `child_overrides_parent` 覆盖父级事实。
- Knowledge 访问仍由 `KnowledgeAccessResolver` 最终判定；继承只决定候选范围，不提升访问权限。

自然地理影响的合并规则：

- `LocationFactResolver` 先读取当前地点及父链，再读取与这些地点有关的 `LocationSpatialRelation`。
- 与当前地点有 `Overlaps` / `Crosses` / `SourceContainsPartOfTarget` / `SourcePartlyWithinTarget` / `WithinNaturalBand` 关系的 `NaturalRegion`，可贡献地形、气候、阻隔、资源、通行风险等事实。
- 自然地理事实进入 `LocationContext.natural_region_facts`，不进入 `inherited_public_facts`。
- 自然地理事实默认不覆盖行政事实；若存在冲突，必须保留两个来源并交给规则层或用户设定处理。

---

## 6. 路线与路程估算

`RoutePlanner` 使用 `LocationEdge` 构建带权图。不同交通方式使用不同 cost profile，例如 `walking`、`horse`、`carriage`、`boat`、`flying`、`teleport`。

查询流程：

1. 解析起点和终点 `location_id`。
2. 按 `allowed_modes`、季节、封锁、风险过滤不可用边。
3. 使用 Dijkstra / A* 计算最小 cost 路线。
4. 聚合距离、时间、风险、缺失字段和置信度。
5. 若无连通路径，返回 `unreachable_or_unknown`，不得硬算精确路程。

同父级、同层级或名称邻近只产生 `ProximityHint`：

```text
临水县 与 白河县 同属 云北州：
- proximity_hint = same_parent_region
- confidence = low
- 不确认直接相邻、具体距离或一日可达
```

有路线边时才输出可用路程：

```text
临水县 -> 白水渡 -> 白河县
- distance_km = 72..90
- carriage = 2..3 days
- horse = 1..2 days
- risk = rain_season_ferry_delay
- confidence = medium
```

---

## 7. 运行时接入

`SceneInitializer` 不直接读取全量地点图谱。运行时由程序把当前锚点解析成 `LocationContext`：

- `LocationResolver`：名称 / ID 解析、父级链、歧义。
- `LocationFactResolver`：可继承地区事实 + 本地事实 + 自然地理影响 + Knowledge 访问裁剪。
- `RoutePlanner`：相邻地点、出口、路线提示、路程估算。

`LocationContext` 是 LLM 输入，不是存储真相。LLM 可以基于它补齐光照、气味、地表、背景实体等允许域，但不能新增命名地点、路线边或地区事实。

---

## 8. 一致性要求

- 每个 Agent World 必须有且只有一个 `WorldRoot` 节点。
- 除 `WorldRoot` 外，持久地点必须有 `parent_id`，除非处于待确认导入状态。`NaturalRegion.parent_id` 表示其主挂载地理域，不表示它只能影响该父级下的地点。
- `parent_id` 不得形成环。
- `LocationNode.canonical_level` 必须与父级模板兼容；不兼容时需要显式 override reason。
- `NaturalRegion` 跨越多个行政节点时，必须使用 `LocationSpatialRelation`，不得通过多父级或复制节点表达。
- `location_aliases` 允许一对多；解析时必须保留歧义。
- 双向边可存一条 `bidirectional = true`，查询时展开为两个方向；单向边不得反向通行。
- `distance_km`、`travel_time`、`confidence` 的估算来源必须可追溯。
- 回滚 Agent 回合时，地点节点、路线边、空间关系、别名与地区事实变更必须随 `state_commit_records.rollback_patch` 回滚。
