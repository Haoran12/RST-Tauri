# 12 Agent 程序化派生与解算

本文档承载 Agent 模式中由程序确定的环境与基础属性派生、档位翻译：

- 环境档位翻译：物理环境如何影响感知、行动与长期压力
- 基础属性档位翻译：基础属性与 mana_power 如何转为 LLM 可读的档位、差距与压力

Mana Combat Resolution 与 Skill Model 已拆分到 [19_agent_combat_and_skills.md](19_agent_combat_and_skills.md)。基础数据模型见 [10_agent_data_model.md](10_agent_data_model.md)。LLM 节点 I/O 契约入口见 [13_agent_llm_io.md](13_agent_llm_io.md)。运行时调用顺序见 [11_agent_runtime.md](11_agent_runtime.md)。

---

## 1. 程序化派生：环境档位翻译

LLM 不擅长把 raw 数值（`50.0 m/s`、`-30.0 ℃`、`视距 8 m`）翻译成行为后果。这一步在程序里做：`EmbodimentResolver` 与 `SceneFilter` 协同把 Layer 1 `physical_conditions` 的原始量映射为**档位 + 具体后果**，分别写入 `EmbodimentState.body_constraints.environmental_strain`（影响该角色本回合行动）和 `FilteredSceneView.weather_perception`（角色对天气的主观感受）。跨回合冷 / 热 / 呼吸累计不保存在 Layer 2，而由 StateCommitter 写回 Layer 1 `TemporaryCharacterState.environmental_exposure`。

```rust
pub enum WindImpactTier {
    Calm,         // < 0.5 m/s
    Breeze,       // 0.5-5 m/s
    Moderate,     // 5-10 m/s
    Strong,       // 10-17 m/s    远程命中失准, 头发衣物明显被吹动
    Gale,         // 17-25 m/s    行动困难, 小型投射物偏移严重
    Storm,        // 25-32 m/s    站立困难, 小物件被吹飞, 树枝折断
    Hurricane,    // > 32 m/s     无法稳定站立, 大物件被卷起, 强行移动会被推走
}

pub enum TemperatureFeelTier {
    // 档位是相对该角色 BaselineBodyProfile.comfort_temperature_range 的偏离量映射
    // 同样 -30℃: 对人类是 SevereCold, 对厚毛皮的狐狸精可能只是 Cold
    Sweltering,   // 极易中暑
    Hot,
    Warm,
    Comfortable,
    Cool,
    Cold,         // 需保暖措施, 不耐久暴露
    SevereCold,   // 长时间暴露失温, 暴露皮肤受冻伤
    Lethal,       // 短时间致命
}

pub enum SurfaceImpactTier {
    Stable,
    Slippery,     // 跑动失败概率显著, 急停难
    Treacherous,  // 几乎无法稳定行动
}

pub enum VisibilityTier {
    Clear,        // > 100 m
    Hazy,         // 20-100 m
    Limited,      // 5-20 m       仅近距离辨识
    Blind,        // < 5 m        几乎瞎走
}

pub enum PrecipitationIntensityTier {
    None,         // 无降水
    Light,        // 细雨/小雪/零星冰雹
    Moderate,     // 中雨/中雪 行动与能见度略受影响
    Heavy,        // 大雨/大雪/冰雹 行动与能见度明显受影响，持续暴露有伤害风险
    Torrential,   // 暴雨/暴雪/沙暴/ 对于Transcendent以下的人物来说行动能力与视野能见度几乎归零，持续暴露有伤害风险
}

pub enum RespirationImpactTier {
    // 由 airborne (烟/尘/雾) + precipitation (沙暴) + mana_haze 综合给出
    Free,         // 呼吸顺畅
    Irritating,   // 刺激, 偶尔咳嗽, 长时间暴露不适
    Choking,      // 持续咳嗽, 呼吸吃力, 持续动作受影响
    Suffocating,  // 短时间致命, 必须捂口鼻或脱离
}

pub enum SurfaceVisualState {
    // 给 LLM 的"地面长什么样"; 可叠加（既积雪又结冰）
    Dry,
    Damp,
    Wet,          // 湿润但无积水
    Puddled,      // 积水
    Snowy,        // 积雪
    Icy,          // 结冰
    Bloody,
    Cluttered,    // 碎屑/法器残骸/瓦砾
}

pub struct EnvironmentalStrain {
    // 写入 EmbodimentState.body_constraints；驱动本回合 action_feasibility。
    // exposure_*_delta 由 StateCommitter 累加到 L1 TemporaryCharacterState.environmental_exposure。
    pub wind_tier: WindImpactTier,
    pub temperature_tier: TemperatureFeelTier,
    pub surface_tier: SurfaceImpactTier,
    pub respiration_tier: RespirationImpactTier,
    pub movement_penalty: f64,           // 0.0-1.0
    pub balance_penalty: f64,            // 0.0-1.0
    pub exposure_cold_delta: f64,        // 本回合冷暴露增量；不在 L2 持久化
    pub exposure_heat_delta: f64,
    pub exposure_respiration_delta: f64, // 本回合呼吸暴露增量；不在 L2 持久化
    pub disrupted_actions: Vec<String>,  // 具体限制说明，例 "无法施展持续吟唱的法术"、"远程瞄准命中-40%"
}

pub struct WeatherPerception {
    // 写入 FilteredSceneView；这是 LLM 在 CognitivePass 中读取的版本
    pub wind_tier: WindImpactTier,
    pub temperature_tier: TemperatureFeelTier,
    pub visibility_tier: VisibilityTier,
    pub respiration_tier: RespirationImpactTier,
    pub surface_visual: Vec<SurfaceVisualState>,    // 同时多种状态: 例 [Snowy, Icy]
    pub surface_tier: SurfaceImpactTier,            // 实际打滑程度（与 EnvironmentalStrain 同源）
    pub precipitation: Option<PrecipitationDescriptor>,
    pub effect_hints: Vec<String>,                  // 程序生成的具体后果描述: ["呼气结成白霜", "细小石子被风卷起拍在脸上", "脚下青苔湿滑"]
}

pub struct PrecipitationDescriptor {
    pub kind: PrecipitationKind,                    // 雨/雪/冰雹/沙暴/灵雨
    pub intensity_tier: PrecipitationIntensityTier,
    pub mana_attribute: Option<ManaAttribute>,      // 仅 SpiritRain 有
}

pub enum PrecipitationKind {
    Rain, Snow, Hail, Sandstorm, SpiritRain,
}
```

**关键不变量**：

1. CognitivePass 的 LLM **只读 tier + effect_hints**，不应从 raw 数值推断后果。`FilteredSceneView` 中不放 raw 数值。
2. 物种差异在档位翻译时已校准（用 `BaselineBodyProfile.comfort_temperature_range`），下游不用再判断"对该角色冷不冷"。
3. 灵力升温/冰寒（`TemperatureModifier.kind = 灵力*`）已在 `Temperature.felt_celsius` 中合并；档位只看最终 felt 值。
4. `exposure_cold_delta` / `exposure_heat_delta` / `exposure_respiration_delta` 是本回合增量；跨回合累计值保存在 Layer 1 `TemporaryCharacterState.environmental_exposure`。累计到阈值后，由 OutcomePlanner 候选 + EffectValidator 生成具体伤势 / 状态事件（冻伤 / 中暑 / 缺氧），写回 Layer 1。
5. `disrupted_actions` 是 LLM 选择行动时的硬约束（在 IntentPlan 验证阶段比对），不是建议。
6. SurfaceRealizer 如需在叙事中提到风速/温度的具体数字，应通过 `SurfaceRealizerInput` 单独传入 raw 值（叙事用），不经 `FilteredSceneView`。
7. **L1 字段须保持自洽**：`physical_conditions` 各子字段间存在因果（暴雨 → wetness↑ → slipperiness↑；沙暴 → dust_density↑ → visibility↓ + respiration 受影响）。`SceneStateExtractor` 在产出 L1 时由 prompt 模板要求一并填齐；档位翻译层只负责把 L1 翻译成档位，不补全 L1 缺失。
8. 翻译公式集中在 `EmbodimentResolver::translate_environment(...)` 与 `SceneFilter::derive_weather_perception(...)`，两者共享同一份阈值表（避免两侧档位不一致）。

---

## 2. 程序化派生：基础属性档位翻译

六项基础属性（`physical` / `agility` / `endurance` / `insight` / `mana_power` / `soul_strength`）使用同一数值标尺和同一默认档位边界。属性 raw 值和运行时计算值使用 `f64` 存储与计算，以便比例修正、状态叠加和调参；普通 UI 默认四舍五入显示为整数。UI 展示值只服务阅读，不参与档位、差距或仲裁判断。

属性"档位"用于把 raw 能力底盘翻译为 LLM 可读层级；属性"差距"用于可观察对抗中的相对判断。两者都不让 LLM 自己估算 raw 数值。档位边界默认沿用原 `mana_power` 锚点（凡人 100 / 入门 500–800 / 瓶颈 1300–1450 / 大成 2400 / 仙灵修行瓶颈 5000 / 神祇 苍角 8800 / 高阶仙灵 NaN），World 可在 `./data/worlds/<world_id>/world_base.yaml` 中重写；YAML 中阈值可写整数或小数，编译后统一为 `f64`。运行时由 `ConfigCompiler` 编译成 `WorldRulesSnapshot`，Resolver / Filter 只读快照，不在感知派生过程中读取配置文件。

```rust
pub enum AttributeKind {
    Physical,
    Agility,
    Endurance,
    Insight,
    ManaPower,
    SoulStrength,
}

pub enum AttributeTier {
    // 单项基础属性的能力层级（默认边界，可由世界配置覆盖）
    Mundane,       // [0, 200)
    Awakened,      // [200, 1000)
    Adept,         // [1000, 1800)
    Master,        // [1800, 2600)
    Ascendant,     // [2600, 5600)
    Transcendent,  // [5600, +∞)
}

pub enum AmbientManaDensityTier {
    // 环境灵气浓度档位（ManaField.ambient_density 的翻译）
    Barren,         // 几近无灵气，普通修士难以汲取
    Sparse,         // 寻常人间街市
    Normal,         // 山林荒野默认水平
    Rich,           // 灵山福地，修行加成
    Dense,          // 灵脉所在 / 仙府 / 阵法核心，凡人会有压迫感
    Saturated,      // 神祇驻地 / 上古遗迹，弱者会过载乃至昏厥
}

pub enum AttributeDelta {
    // Δ = target_effective_or_displayed - observer_effective
    // 用于"感觉/实际差距多大"，与档位识别正交（同档可有显著差，跨档也可被技巧/状态拉平）
    Indistinguishable,       // |Δ| < 150          相若, 难分高下
    SlightlyBelow,           // Δ ∈ [-300, -150)   略弱
    NotablyBelow,            // Δ ∈ [-1000, -300)  显著弱
    FarBelow,                // Δ ∈ [-2000, -1000) 远不及, 基本无力应对（对抗解算=Crushing）
    Crushed,                 // Δ < -2000          蝼蚁差距, 无法测度（对抗解算=Crushing）
    SlightlyAbove,           // Δ ∈ [150, 300)     略胜
    NotablyAbove,            // Δ ∈ [300, 1000)    显著强
    FarAbove,                // Δ ∈ [1000, 2000)   远胜, 守方基本无力应对（对抗解算=Crushing）
    Overwhelming,            // Δ ≥ 2000           压顶, 无法测度（对抗解算=Crushing）
}

pub enum ManaExpressionMode {
    // 运行时灵力显露状态：只改变感知与环境压力，不改变 effective_mana_power
    Sealed,        // 封息：几乎无外泄
    Suppressed,    // 抑制：主动/被迫压低气息
    Natural,       // 自然：不刻意收放；由长期倾向校准默认外显
    Released,      // 外放：灵压影响场景与低阶稳定
    Dominating,    // 威压：以气势压迫感知、情绪与认知清晰度
}

pub enum ManaExpressionTendency {
    // 持久层默认倾向：来自体质、性格、修行体系或长期训练
    Inward,
    Neutral,
    Expressive,
}

pub enum ManaExpressionIntentionality {
    Intentional,
    Unintentional,
    Forced,
}

pub enum ManaPresenceRadiusTier {
    SelfOnly,
    Touch,
    Close,
    Room,
    Area,
    Scene,
}

pub struct ManaExpressionProfile {
    // 运行时内部派生；ratio 原值不进入 CognitivePass
    pub character_id: String,
    pub baseline_tendency: ManaExpressionTendency,
    pub mode: ManaExpressionMode,
    pub intentionality: ManaExpressionIntentionality,
    pub tendency_factor: f64,
    pub mode_factor: f64,
    pub display_ratio: f64,
    pub pressure_ratio: f64,
    pub radius_tier: ManaPresenceRadiusTier,
    pub overstated_signal: bool,        // display 高于可持续真实外放时置 true，用于破绽/疲劳/识破
}

pub struct EffectiveAttributeProfile {
    // 运行时内部派生；不进入 CognitivePass raw 输入
    pub character_id: String,
    pub values: HashMap<AttributeKind, f64>,
    pub tiers: HashMap<AttributeKind, AttributeTier>,
    pub descriptors: HashMap<AttributeKind, Vec<String>>,
}

pub struct PerceivedAttributeProfile {
    pub source_id: String,
    pub attribute_kind: AttributeKind,
    pub tier_assessment: Option<AttributeTier>,
    pub delta: Option<AttributeDelta>,
    pub confidence: f64,                          // 0.0-1.0
    pub evidence: Vec<AttributeEvidenceKind>,      // observation / combat_exchange / mana_signal / soul_pressure ...
    pub descriptors: Vec<String>,                 // 程序生成: ["步伐极稳", "反应稍慢", "气息浩瀚如海"]
}

pub enum AttributeEvidenceKind {
    Appearance,
    Movement,
    SustainedAction,
    InjuryResponse,
    CombatExchange,
    TacticalRead,
    ManaSignal,
    SoulPressure,
    SkillEffect,
}

pub struct PerceivedManaProfile {
    pub source_id: String,                            // 被感知者 / 来源
    pub tier_assessment: Option<AttributeTier>,       // 对方 mana_power 档位识别（内敛/压制/伪装后为显示档）
    pub delta: AttributeDelta,                        // 感知差距档位
    pub expression_assessment: Option<ManaExpressionMode>, // 对方当前气息状态的粗判：封息/抑制/自然/外放/威压等
    pub attribute_assessment: Option<ManaAttribute>,  // 仅 |Δ| < 1000 且未被严重干扰时较准
    pub confidence: f64,                              // 0.0-1.0
    pub concealment_suspected: bool,                  // 感觉对方在收敛/压制/伪装气息
    pub pressure_response: Option<AttributeDelta>,     // 自身受到的灵压体感差距；不等同于对抗结论
    pub descriptors: Vec<String>,                     // 程序生成: ["气息浩瀚如海", "似有若无, 形迹诡异"]
}

pub struct ManaSignal {
    // FilteredSceneView.mana_signals 中的单个气息：源于具体实体 / 法术 / 灵脉
    pub source_kind: ManaSourceKind,                  // Character / Artifact / SpellResidue / Formation / SpiritVein
    pub direction_hint: Option<String>,               // 方位与距离的粗化描述（不给精确坐标）
    pub perceived: PerceivedManaProfile,
}

pub struct ManaEnvironmentSense {
    // 整体环境灵气感知（区别于针对单一来源的 ManaSignal）
    pub density_tier: AmbientManaDensityTier,
    pub dominant_attribute: Option<ManaAttribute>,
    pub character_presences: Vec<ManaPresenceSense>,   // 同场人物当前显露状态造成的局部灵压体感
    pub interferences: Vec<String>,                   // "屏蔽阵法残留", "灵雾阻隔感知"
    pub overload_risk: bool,                          // 灵觉过载风险（高敏锐度撞 Saturated 环境）
    pub descriptors: Vec<String>,                     // ["灵气浓郁如蜜, 呼吸间满是清甜"]
}

pub struct ManaPresenceSense {
    pub source_id: String,
    pub expression_assessment: Option<ManaExpressionMode>,
    pub radius_tier: ManaPresenceRadiusTier,
    pub pressure_delta: AttributeDelta,
    pub cognitive_effect_hints: Vec<String>,           // llm_readable: "注意力被牵引", "呼吸发紧", "判断变保守"
}
```

**属性派生规则**——由 `AttributeResolver::derive_effective_attributes(...)` 程序化实施：

1. **基础值**来自 Layer 1 `CharacterRecord.base_attributes`，均为 `f64`。
2. **有效值** = `max(0.0, (base_value + flat_delta) * max(0.1, 1.0 + ratio_modifier_sum))`。伤势、疲惫、疼痛、状态效果、技能、环境等只影响本回合 effective，不改写 base。
3. **档位判断**直接使用 effective `f64` 与阈值比较，区间为 `[lower, upper)`；例如实际值 `999.6` 仍低于 `1000.0`，不会因为 UI 显示为 `1000` 而进入下一档。
4. `physical` 主要影响肉身爆发、承载、擒拿、推挤、冲撞；`agility` 主要影响移动、平衡、闪避、抢位、精细动作；`endurance` 主要影响持续损耗、抗痛、伤后维持；`insight` 主要影响威胁识别、破绽解释和战术阅读；`mana_power` 主要影响灵力/法力强度、对冲、压制和施法；`soul_strength` 主要影响精神/灵魂/压制类底盘。
5. `EffectiveAttributeProfile` 属于程序内部派生和 trace 对象；CognitivePass 不读取 raw `values`，只读取由 EmbodimentResolver / SceneFilter 派生出的 tier、delta、descriptors、constraints。

**灵力显露倾向与运行时状态**——由 `AttributeResolver::derive_mana_expression(...)` 与 `SceneFilter::derive_mana_presence(...)` 程序化实施：

`base_attributes.mana_power` 是长期底盘；`effective_mana_power` 是叠加伤势、状态、突破等 L1 修正后的真实可用底力；`displayed_mana_power` 是他人灵觉读到的外显强度；`mana_presence_pressure` 是这股气息对环境和旁人认知造成的压力。四者必须分开：持久的内敛/外放倾向通过 `tendency_factor` 校准默认外显；运行时的封息/抑制/自然/外放/威压通过 `mode_factor` 表示当前场景的实际显露状态。高 base / high effective 的角色可以长期倾向外放但此刻封息，也可以长期内敛但此刻主动威压；两者都不直接改变对抗解算使用的 `effective_mana_power`。

持久层 `CharacterRecord.mana_expression_tendency` 只保存三档默认倾向；`tendency_factor` 可被特定人物覆盖，未覆盖时使用世界默认值：

| 倾向 | 默认 `tendency_factor` | 默认作用 |
|---|---:|---|
| `Inward` 内敛倾向 | -0.5 | display 偏低，气息描述更收束；转入 `Sealed` / `Suppressed` 成本较低 |
| `Neutral` 一般倾向 | -0.2 | display 略低于真实 effective；姿态切换无明显偏置 |
| `Expressive` 外放倾向 | 0.1 | display 偏高，情绪或突破时更容易无意识泄露；维持 `Sealed` / `Suppressed` 成本较高 |

运行时 `TemporaryCharacterState.mana_expression.mode` 表达当前场景状态：

| 姿态 | 默认 `mode_factor` | 典型影响 |
|---|---:|---|
| `Sealed` 封息 | -0.7 | 近乎无气息；行动/施法前可能需要解除 |
| `Suppressed` 抑制 | -0.3 | 有意或被迫压低气息；远距离难察，近距离或高灵觉可能出现违和感 |
| `Natural` 自然 | 0.0 | 不刻意收放；具体落点由持久倾向校准 |
| `Released` 外放 | 0.2 | 灵压影响房间/区域体感；弱者可能动作迟滞、灵觉过载 |
| `Dominating` 威压 | 0.4 | 主动压迫感知与认知清晰度；可触发恐惧、退缩、反应窗口或状态候选 |

派生规则：

1. `CharacterRecord.mana_expression_tendency` 提供长期默认倾向；`ManaExpressionState.mode` 提供当前场景状态。二者共同决定目标 `display_ratio` / `pressure_ratio`，但只有 runtime mode 表示本回合真实姿态。
2. `ManaExpressionState.intentionality` 标记姿态来源：主动控制（Intentional）、情绪/伤势/突破等无意识泄露或收缩（Unintentional）、禁制/法器/他人压制等外部强制（Forced）。Validator 必须能追溯 `source_id` 或状态来源。
3. `tendency_factor = character.mana_expression_tendency_factor_override.unwrap_or(world_rules.mana_rules.tendency_factors[tendency])`。人物级覆盖用于特殊体质、性格或修行法门；覆盖值必须通过配置范围校验。
4. `mode_factor = world_rules.mana_rules.mode_factors[mode]`，默认值为 `Sealed=-0.7`、`Suppressed=-0.3`、`Natural=0.0`、`Released=0.2`、`Dominating=0.4`。
5. `display_ratio = min(2.0, max(0.0, 1.0 + tendency_factor + mode_factor))`。`display_ratio` 是倍率，不含 `effective_mana_power`；它可写入 `ManaExpressionProfile` 和 Trace，但不得进入 CognitivePass。
6. `displayed_mana_power = max(0.0, effective_mana_power * display_ratio + display_flat_delta + illusion_bonus - concealment_penalty)`。若没有额外 L1 技能/法器/阵法/干扰来源，则简化为 `effective_mana_power * display_ratio`。`illusion_bonus`、`display_flat_delta` 和 `concealment_penalty` 只来自 L1 来源；不能由 LLM 临场编造。
7. `pressure_ratio` 第一版默认复用 `display_ratio`，再按模式的 `pressure_multiplier` 或技能/场景规则修正；`mana_presence_pressure = effective_mana_power * pressure_ratio`，再按距离、遮蔽、阵法、属性相克和环境干扰衰减，映射为观察者视角的 `pressure_delta`。
8. `display_ratio > 1.0` 代表更强的外显、显摆、威吓或伪装信号，不代表真实对抗变强；若缺少资源/技能支撑且长期维持，`overstated_signal = true`，用于疲劳、破绽和识破判定。
9. `Released` / `Dominating` 会向 `ManaField.character_presences` 写入派生源，进而影响同场角色的 `ManaEnvironmentSense.character_presences`、`SalienceModifiers.attention_biases`、`ReasoningModifiers.threat_bias` / `overload_bias`。
10. `Sealed` / `Suppressed` 不删除真实 `effective_mana_power`；对抗、施法上限和压制破绽仍按 effective 计算。它们只降低 displayed 和 presence，并可能增加行动前解除姿态的成本或延迟。

**感知规则（认知层）**——由 `SceneFilter::derive_attribute_perception(...)` 和 `SceneFilter::derive_mana_perception(...)` 程序化实施：

1. 对他人 `physical`、`agility`、`endurance`、`insight`、`soul_strength` 的 `PerceivedAttributeProfile` 必须有观察依据；静止外观只能给低置信度，不能凭空生成确定档位。
2. `physical` 通过体型、负重、冲撞、擒拿、碰撞结果感知；`agility` 通过移动、闪避、变向、平衡、出手速度感知；`endurance` 通过长时间行动、受伤后维持、疲惫恢复、痛苦反应感知；`insight` 通过识破虚招、反应选择、战术阅读、话术破绽判断感知；`soul_strength` 通过神魂压力、精神冲击抗性、恐惧/魅惑抵抗、灵魂类技能交互感知。
3. `insight` 只提升线索解释质量与置信度，不自动读取隐藏事实、GodOnly Knowledge 或不可见实体。
4. **观察者灵力** = `observer.effective_mana_power`（已含 L1 伤势 / 疲惫 / 突破修正）。
5. **目标显示灵力** `target.displayed_mana_power`：
   - 默认来自 `ManaExpressionState::Natural`，具体强弱由 `mana_expression_tendency` 的 `tendency_factor` 校准：默认内敛倾向 `0.5x`、一般倾向 `0.8x`、外放倾向 `1.1x`。
   - 若目标处于 `Sealed` / `Suppressed`，displayed 按姿态比例下降；若处于 `Released` / `Dominating`，displayed 可上升，但只代表外显信号。
   - 额外隐匿、压制、伪装或放大来自 L1 状态 / 技能 / 阵法修正；不让 LLM 自己定。
6. **Δ = target.displayed_mana_power − observer.effective_mana_power**，按上述 9 档桶映射到 `AttributeDelta`。
7. **mana_power 档位识别**：
   - `|Δ| < 1000`：可识别 `tier_assessment = AttributeTier::from_value(AttributeKind::ManaPower, displayed)` 与 `attribute_assessment`，`confidence ≥ 0.7`。
   - `|Δ| ∈ [1000, 2000)`：可识别 tier，但 attribute 不稳；descriptors 偏向"远胜 / 远不及"。
   - `|Δ| ≥ 2000`：`tier_assessment = None`，descriptors 偏向"无法测度 / 如同蝼蚁"。
8. **Mundane (Tier0) 观察者**：仅能将 `effective_mana_power ≥ 1000` 的存在感知为"超出常理"，无具体档位；环境灵气仅给"格外厚重 / 压抑"等体感。
9. **零灵觉**（`SensoryCapabilities.mana.acuity == 0`）：`mana_signals = []`，`mana_environment.density_tier` 由间接体感（呼吸/温度异常）回填，`dominant_attribute = None`。
10. **隐匿 / 压制**：
   - 封息/抑制后档位 `displayed_tier = AttributeTier::from_value(AttributeKind::ManaPower, displayed)` 直接落在 tier_assessment 上。
   - **破绽判定**（`concealment_suspected`）：当 `observer.effective_mana_power ≥ target.effective_mana_power − 200` 时（即观察者实力已能"接近"压制前的目标），置 true（"似有若无的违和感"）。否则压制看起来天衣无缝，false。
   - 灵觉敏锐度可作为额外破绽来源：`acuity ≥ 0.85` 且 `target.suppression_amount ≥ 1000` 时也强制 `concealment_suspected = true`（高灵觉天然能闻到压制痕迹）。
11. **外放 / 威压**：`Released` / `Dominating` 的 `pressure_delta` 可写入观察者的 `ReasoningModifiers`，形成"注意力被牵引 / 判断更保守 / 灵觉过载"等输入；是否真正恐惧、屈服或误判仍由 CognitivePass 基于 L2 + prior L3 生成。
12. **环境干扰**：`ManaField.interferences` 中的 jam/scramble 按强度降低 `confidence`；`mana_haze` 让该回合所有 mana_signals 的 |Δ| 视为额外 +500（拉远感知，便于隐匿者进出）。
13. **属性相生相克**：观察者擅长属性与目标属性相同 → confidence +；相克 → 易识别（descriptors 含"违逆 / 刺骨"），同时影响 `attribute_assessment` 准确度与 descriptors 色彩。

**关键不变量**：

1. CognitivePass 永远不读 raw 基础属性或 raw `mana_power`，只读 tier / delta / expression_assessment / pressure_hints / descriptors / constraints。`FilteredSceneView` 中不暴露 raw 数值。
2. 档位边界、Δ 桶边界、压制破绽阈值都是**世界配置项**（默认值同上，对 rp_cards 锚点校准），由 `WorldRulesSnapshot` 同时供角色卡解析、`AttributeResolver`、`SceneFilter::derive_attribute_perception(...)`、`SceneFilter::derive_mana_perception(...)`、`CombatMathResolver` 使用；改边界需同时更新配置 schema、迁移规则与单元测试。
3. 感知层只写**事实级感受**（"远胜 / 难测 / 似有压制"），**不写信念**（"他一定是神祇 / 他在装弱 / 他没安好心"）。这些信念由 CognitivePass 的 LLM 基于感受 + `prior_subjective_state` 自行生成。
4. `AttributeTier` 对 `AttributeKind::ManaPower` 的应用同时为 `KnowledgeEntry { facet: CultivationRealm }` 的内部表征：`access_policy` 决定"谁能读取这一档", 跨档感知精度决定"感知到的是真档还是被压制的档"。
5. SurfaceRealizer 如需在叙事中提到"修为相差一筹/远胜/碾压"等具体差距文字，从 `ManaSignal.perceived.delta` 与 `tier_assessment` 取，不回查 raw mana_power。
6. UI 展示的整数属性值不参与档位判断；Trace 可记录 full precision，普通 UI 默认隐藏小数。
7. `ManaExpressionTendency` 是持久倾向，`ManaExpressionMode` 是运行时离散状态。LLM 可请求"封息/抑制/自然/外放/威压"这类动作意图，但不能输出 `tendency_factor`、`mode_factor`、`display_ratio`、`pressure_ratio` 或 raw displayed 值；具体倍率由程序按配置、人物覆盖、倾向和技能契约派生。

---

## 3. 对抗解算与 Skill Model

Mana Combat Resolution、对抗公式、关键不变量和 Skill Model 契约已拆分到 [19_agent_combat_and_skills.md](19_agent_combat_and_skills.md)。

本文件只保留环境档位与基础属性档位的程序化派生规则。
