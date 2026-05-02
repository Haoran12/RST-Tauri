# 18 Agent 角色模型

本文档承载 CharacterRecord、BaseAttributes、BaselineBodyProfile、TemporaryCharacterState、灵力显露长期倾向与运行时显露状态。

Agent 三层数据语义见 [10_agent_data_model.md](10_agent_data_model.md)。属性档位与程序化派生见 [12_agent_simulation.md](12_agent_simulation.md)。对抗解算与技能契约见 [19_agent_combat_and_skills.md](19_agent_combat_and_skills.md)。

---

## 1. 角色静态档案（CharacterRecord）

角色不再有大而全的"static_profile" blob。所有"客观属于该角色的事实"都拆为多条 `KnowledgeEntry { kind: CharacterFacet, subject: Character{id, facet} }`，按需查询。

以下五项作为非 Knowledge 的角色基本数据保留在 Layer 1：

```rust
pub struct CharacterRecord {
    pub character_id: String,
    pub base_attributes: BaseAttributes,               // 六项基础属性；f64 存储和计算，UI 默认按整数展示
    pub baseline_body_profile: BaselineBodyProfile,    // 物种/感官基线/灵觉基线（用于 EmbodimentResolver 与 SceneFilter）
    pub mana_expression_tendency: ManaExpressionTendency, // 长期灵力显露倾向：内敛/一般/外放；不等于当前场景姿态
    pub mana_expression_tendency_factor_override: Option<f64>, // 可选人物级 tendency_factor 覆盖；无则用世界默认值
    pub mind_model_card_knowledge_id: String,          // 指向 KnowledgeEntry 中的 MindModelCard，避免双写漂移
    pub temporary_state: TemporaryCharacterState,       // 当前客观身体/资源/跨域临时状态；每回合由机械演化与 StateCommitter 更新
    pub schema_version: String,
}

pub struct BaseAttributes {
    pub physical: f64,       // 肉身基础质量、爆发、承载、擒拿/推挤/冲撞底盘；不等于血量
    pub agility: f64,        // 动作协调、变向、身法、精细动作；影响闪避/抢位/拦截/脱困
    pub endurance: f64,      // 持续作战、抗疲劳、抗痛、伤后维持；缓冲持续损耗状态
    pub insight: f64,        // 洞察、识招、战术阅读、看破；不承担纯精神/灵魂抗性底盘
    pub mana_power: f64,     // 灵力/法力强度与运用底盘；取代独立 mana_potency 口径
    pub soul_strength: f64,  // 神魂强度、意志根基、内在稳定；精神/灵魂/压制类底盘
}

pub struct BaselineBodyProfile {
    pub species: String,                           // "人类" / "妖精-狐" / "仙灵-龙" / ...
    pub comfort_temperature_range: (f64, f64),     // 物种舒适带（℃），用于 TemperatureFeelTier 校准
    pub mana_sense_baseline: ManaSenseBaseline,    // 灵觉基线（acuity / overload_threshold / 属性偏向）
    pub mana_attribute_affinity: Vec<ManaAttribute>,  // 擅长属性（影响感知 confidence 与施法效率）
    pub size_class: String,                        // "humanoid" / "small_beast" / "kaiju" 等（影响平衡/移动公式）
}

pub struct ManaSenseBaseline {
    pub acuity: f64,                               // 0.0-1.0；凡人 0.0；普通修士 0.4-0.6；高阶仙灵 ~1.0
    pub overload_threshold: f64,                   // 触发感知过载的环境密度阈值（与档位相关）
    pub attribute_bias: Option<ManaAttribute>,     // 天生敏感的属性
}

pub struct TemporaryCharacterState {
    pub injuries: Vec<InjuryState>,
    pub fatigue: f64,                              // 0.0-1.0
    pub pain_load: f64,                            // 0.0-1.0
    pub mana_reserve_current: Option<f64>,
    pub mana_expression: ManaExpressionState,       // 当前场景灵力显露状态：封息/抑制/自然/外放/威压
    pub mana_suppression: Vec<ManaSuppressionState>,
    pub environmental_exposure: EnvironmentalExposureState, // 跨回合环境暴露累计；由 EmbodimentResolver 增量计算，StateCommitter 写入
    pub active_conditions: Vec<ConditionState>,    // poison / stun / restraint / bleeding / fear / soul_damage ...
    pub cooldowns: Vec<CooldownState>,
    pub transient_signals: Vec<String>,            // llm_readable: 手抖/脸红/气息紊乱等外显短态
    pub schema_version: String,
}

pub struct InjuryState {
    pub injury_id: String,
    pub body_region: String,
    pub severity: String,                          // bruise / light / moderate / severe / critical
    pub effect_tags: Vec<String>,                  // mobility_penalty / bleeding / pain / mana_flow_blocked ...
    pub source_event_id: Option<String>,
}

pub struct ManaSuppressionState {
    pub source_id: String,
    pub multiplier: f64,
    pub expires_at_turn: Option<String>,
}

pub struct EnvironmentalExposureState {
    pub cold_strain: f64,                         // 0.0+；持续寒冷累计，到阈值后转为冻伤 / 行动惩罚候选
    pub heat_strain: f64,                         // 0.0+；持续高温累计，到阈值后转为中暑 / 脱水候选
    pub respiration_strain: f64,                  // 0.0+；烟尘 / 沙暴 / 缺氧累计，到阈值后转为咳嗽 / 缺氧 / 窒息候选
    pub last_updated_turn: Option<String>,
}

pub enum ManaExpressionTendency {
    Inward,        // 长期偏内敛：体质、性格或修行法门让气息默认收束
    Neutral,       // 长期一般：自然状态接近真实有效灵力
    Expressive,    // 长期偏外放：气息更容易被察觉或自然形成存在感
}

pub enum ManaExpressionMode {
    Sealed,        // 封息：几乎不外泄，通常需要技能/法器/禁制维持
    Suppressed,    // 抑制：有意或被迫压低气息，但仍可能被近距/高灵觉察觉
    Natural,       // 自然：不刻意收放；具体外显强弱由长期倾向校准
    Released,      // 外放：灵压影响场景体感和低阶行动稳定
    Dominating,    // 威压：以气势压迫感知、情绪与认知清晰度
}

pub enum ManaExpressionIntentionality {
    Intentional,   // 主动控制
    Unintentional, // 情绪、伤势、突破、恐惧等导致的无意识泄露或收缩
    Forced,        // 禁制、法器、他人压制或场景规则造成
}

pub enum ManaPresenceRadiusTier {
    SelfOnly,      // 只影响自身与贴身接触
    Touch,
    Close,         // 近身几步
    Room,
    Area,
    Scene,
}

pub struct ManaExpressionState {
    pub mode: ManaExpressionMode,
    pub display_ratio: f64,                         // = clamp(1 + tendency_factor + mode_factor, 0, 2)
    pub pressure_ratio: f64,                        // 环境灵压强度倍率；用于派生 CharacterManaPresence
    pub radius_tier: ManaPresenceRadiusTier,
    pub intentionality: ManaExpressionIntentionality,
    pub source_id: Option<String>,                  // skill_id / condition_id / item_id / actor intent
    pub expires_at_turn: Option<String>,
}

pub struct ConditionState {
    pub condition_id: String,
    pub domain: StateDomain,
    pub condition_kind: String,
    pub intensity: f64,
    pub source_id: Option<String>,
}

pub enum StateDomain {
    Body,
    Resource,
    Position,
    Perception,
    Mind,
    Soul,
    Scene,
    KnowledgeReveal,
}

pub struct CooldownState {
    pub ability_id: String,
    pub remaining_turns: u32,
}
```

注意：

- `MindModelCard` 只以 `KnowledgeEntry` 形式保存（subject 自我认知层）；`CharacterRecord` 仅保存 `mind_model_card_knowledge_id` 指针，避免同一事实在角色表和知识表双写漂移。
- `base_attributes` 是角色长期能力底盘，属于 Layer 1 raw 数值；运行时由 `AttributeResolver` 叠加伤势、疲惫、状态、技能、环境等修正，派生 `effective_attributes` 后供 EmbodimentResolver / SceneFilter / CombatMathResolver 使用。
- `mana_expression_tendency` 是角色长期默认显露倾向，来源于体质、性格、修行体系或长期训练。它通过 `tendency_factor` 参与所有运行时状态下的 `display_ratio` 计算，不表示当前回合正在内敛或外放。`mana_expression_tendency_factor_override` 可为特定人物覆盖默认系数；为空时使用 `WorldRulesSnapshot.mana_rules.tendency_factors`。
- 基础属性和 `mana_power` 存储与内部计算均使用 `f64`，以便比例修正和调参；普通 UI 默认四舍五入显示为整数，普通编辑也按整数步进。UI 展示取整不得写回存储，除非用户显式编辑保存。
- `temporary_state` 主要归入 Layer 1：伤势、疲惫、痛感、灵力消耗、冷却、毒素、恐惧压制、魂伤、短暂身体反应等都属于当前客观/半客观运行态。CognitivePass 只能通过 Layer 2 的 `EmbodimentState` 看到其派生结果，不直接读取原始状态。
- `base_attributes.mana_power` 是 raw 灵力/法力数值；当前**有效灵力**还需叠加 L1 中的伤势、状态、突破等修正后再喂给 `AttributeTier::from_value(AttributeKind::ManaPower, ...)`。raw 永远不进入 CognitivePass。
- `mana_expression` 表达角色本回合/本场景如何显露灵力（封息、抑制、自然、外放、威压），属于 Layer 1 当前状态，可由主动意图、情绪/伤势等无意识状态或外部强制来源产生。它只影响 `displayed_mana_power`、`CharacterManaPresence`、旁人的 salience / reasoning modifiers，不直接提高真实对抗底力。
- `effective_mana_power` 用于实际施法、压制破绽和对抗解算；`displayed_mana_power` 是感知层显示值，基础公式为 `effective_mana_power * display_ratio`，并可叠加合法 L1 隐匿/伪装修正。两者都是运行时派生值，不单独作为角色基础字段持久化。
- `comfort_temperature_range` 与基础属性默认值在角色卡解析时从对应种族卡（如 `humanbeing.yaml` / `yaoguai.yaml`）读取并可被角色级覆盖。

---
