# 19 Agent 对抗解算与技能契约

本文档承载 Mana Combat Resolution、程序化对抗公式、关键不变量与 Skill Model 契约。

环境和基础属性档位派生见 [12_agent_simulation.md](12_agent_simulation.md)。角色属性模型见 [18_agent_character_model.md](18_agent_character_model.md)。运行时反应窗口见 [11_agent_runtime.md](11_agent_runtime.md)。

---

## 1. Mana Combat Resolution（程序化灵力对抗解算）

对抗解算层与感知层用的是**不同**输入：

- 感知层：`displayed_mana_power`（含持久显露倾向、运行时显露状态、压制、伪装/放大）→ 角色"觉得"对方多强。
- 对抗解算层：`effective_mana_power`（不含持久显露倾向、运行时显露状态与压制；压制只是没主动用全力）→ 实际对抗按真实底力 + 技能 + 身体状态计算。

```rust
pub struct ManaCombatResolution {
    // 对抗解算层使用，不进入 CognitivePass
    pub actor_id: String,
    pub target_id: String,
    pub actor_combat_power: f64,         // = effective_mana_power × max(0.1, 1 + Σ_modifiers) × soul_factor
    pub target_combat_power: f64,
    pub combat_delta: f64,               // actor_combat_power − target_combat_power
    pub outcome_tier: CombatOutcomeTier,
    pub disrupting_factors: Vec<String>, // 程序生成: ["攻方处于深度疲惫, 输出折半", "守方擅长水属性, 克制对手火属性"]
}

pub enum CombatOutcomeTier {
    // 由 |combat_delta| 桶映射；与感知层 AttributeDelta 共享 150/300/1000 三个阈值
    // 对抗解算层不再细分 1000 以上：到了"无力应对"就够用了
    Indistinguishable,       // |Δ| < 150       势均力敌, 胜负看临场发挥/技巧
    SlightEdge,              // Δ ∈ [150, 300)  攻方略占上风
    MarkedEdge,              // Δ ∈ [300, 1000) 攻方明显优势
    Crushing,                // Δ ≥ 1000        守方无力应对, 仅能逃避或求饶
    // 负向（攻方反吃亏）对称展开
}
```

### 1.1 对抗解算公式（程序化）

```
combat_power = effective_mana_power × max(0.1, 1 + Σ_modifiers) × soul_factor
```

仅有**两个独立乘区**：加算修正区（多数因子在此叠加），与灵魂状态乘区（单独成区）。其余因子全部以**加和**方式落到 `Σ_modifiers` 内，不互乘。

1. **基础有效灵力** `effective_mana_power = base_attributes.mana_power + L1 状态修正`（突破/中毒/压制解除等，皆为 L1 真相，不含伤势疲惫——后者落入加算修正区）。
2. **加算修正区** `Σ_modifiers`（同区内所有修正以加和方式叠加）：

   **技能**：
   - 本命法术：**+0.10 ~ +0.15**
   - 克制属性：+0.10 ~ +0.20
   - 受克制：-0.10 ~ -0.20
   - mastery_rank：novice -0.15 ~ master +0.15

   **身体**：
   - 轻伤：-0.05 ~ -0.15
   - **严重疲惫：-0.25**
   - **身体重伤 / 灵力枯竭：-0.20 ~ -0.50**（按伤势严重度落区间）
   - `EnvironmentalStrain.disrupted_actions` 按 disrupted 程度：-0.10 ~ -0.40

   **心境**（来自 Layer 3 EmotionState 与 L1 突发情绪事件，按已有情绪标签程序化映射，不让 LLM 在对抗解算时即兴选择）：
   - **自信 / 愤怒：+0.05 ~ +0.10**
   - 恐惧 / 迟疑：-0.05 ~ -0.15
   - 崩溃：-0.20 ~ -0.40

   **环境**：
   - 本属性 `Rich/Dense`：**通常 +0.1 ~ +0.15**
   - 本属性 `Saturated`：至 +0.20
   - `mana_haze`：-0.10
   - **明确设定的例外**（特定阵法 / 上古遗迹 / 神祇坐镇地脉等）：由 L1 `KnowledgeEntry { kind: RegionFact / FactionFact }` 的 `content.combat_modifiers` 字段显式给出非常规修正值，直接加入 `Σ_modifiers`，可超出上述区间。

3. **灵魂状态乘区** `soul_factor`（独立乘区，是除加算区外唯一的乘子）：
   - 灵魂完整：1.0
   - **灵魂破损 / 抽离：0.2 ~ 0.7**（按程度落区间，下限对应"魂飞魄散"级）

4. **下限保护**：加算系数以 `max(0.1, 1 + Σ_modifiers)` 截下限，避免修正过深导致 combat_power 趋零或为负而引发除零 / 碾压判定异常。

5. **outcome_tier** 按 `combat_delta = actor_combat_power − target_combat_power` 落桶（默认 150 / 300 / 1000，1000 以上即 Crushing）；桶边界来自 `WorldRulesSnapshot.combat_delta_thresholds`，细化由 `disrupting_factors` 列出（程序生成的具体说明，例 ["攻方显著疲惫 -0.20", "守方身体重伤 -0.40 + 恐惧 -0.10 + 灵魂破损 ×0.5"]）。

6. 程序化对抗解算只决定**可验证物理后果**（伤势 / 法力消耗 / 位置变化）是否可写回 L1；公开退让、站队、敌对升级等外显社会事件可由 OutcomePlanner 候选输出，但内心恐惧 / 屈服 / 记仇仍由下游角色 CognitivePass 解读。

### 1.2 关键不变量

1. 对抗解算公式只读 L1 的 `effective_mana_power`、L1 的身体状态、L1 的技能/属性数据；**不读 displayed_mana_power**（持久显露倾向与当前封息/抑制/外放/威压都是感知与环境压力层的事，不影响真实对抗）。
2. `combat_delta` 与 `AttributeDelta` 共享同一份 `WorldRulesSnapshot` 中的 150/300/1000 默认阈值，保证"我感觉略胜"与"实际略胜"在同一刻度上。对抗解算层在 Crushing 阈值以上不再细分；感知层仍可额外配置 `far` / `overwhelming` 边界（默认 2000），但两者**对应的对抗结论一致**（皆为"基本无力应对"），区别只在体感（"远胜，难敌" vs "无法测度，压顶之势"）与是否可识别 tier。
3. 当 `disrupting_factors` 与 `outcome_tier` 出现"违和"（例如攻方 `base_attributes.mana_power` 高但身体状态极差导致 combat_delta 反而为负），SurfaceRealizer 必须在叙事中体现这种反差，而不是按"谁灵力高谁赢"硬写。
4. **以弱胜强**在该框架下要求**多个加算修正叠加 + 可能的灵魂状态打击**：守方若同时陷入"显著疲惫 (-0.20) + 身体重伤 (-0.40) + 恐惧 (-0.10) = Σ = -0.70"，加算系数 = max(0.1, 0.30) = 0.30；再叠加灵魂破损 soul_factor = 0.5，总系数 0.15，足以让基础灵力差 1500 的弱者翻盘。"算计 / 偷袭 / 中毒 / 惊扰魂魄"必须落到具体的 L1 状态字段上，由公式自然得出，不允许 LLM 在对抗解算口径上手抹平差距。

---

## 2. Skill Model（契约 + LLM）

```rust
pub enum TriggerMode {
    Active,
    Reaction,
    Passive,
    Channeled,
}

pub enum DeliveryChannel {
    Gaze,
    Voice,
    Touch,
    Projectile,
    Scent,
    SpiritualLink,
    Ritual,
    Field,
}

pub enum ImpactScope {
    Body,
    Perception,
    Mind,
    Soul,
    Scene,
}

pub enum TargetKind {
    SelfTarget,
    Character,
    Location,
    Area,
    Object,
    Knowledge,
}

pub enum EffectIntensityTier {
    Minor,
    Moderate,
    Major,
    Severe,
    Overwhelming,
}

pub enum DurationPolicy {
    Instant,
    Turns { max_turns: u32 },
    Scene,
    UntilDispelled,
    PermanentUntilStateChange,
}

pub struct CostProfile {
    pub mana_reserve_delta: Option<f64>,  // 消耗为负，恢复为正；从 TemporaryCharacterState.mana_reserve_current 校验
    pub fatigue_delta: Option<f64>,       // 通常为正，写入 TemporaryCharacterState.fatigue
    pub cooldown_turns: Option<u32>,      // 写入 cooldowns
    pub material_refs: Vec<String>,       // item_id / knowledge_id / inventory ref；第一版只做存在性与权限校验
    pub required_conditions: Vec<String>, // semantic: posture / ritual_state / focus / soul_anchor 等
}

pub struct Skill {
    pub skill_id: String,
    pub name: String,
    pub trigger_mode: TriggerMode,         // active / reaction / passive / channeled
    pub delivery_channel: DeliveryChannel, // gaze / voice / touch / projectile / scent / spiritual_link / ritual / field
    pub impact_scope: ImpactScope,         // body / perception / mind / soul / scene
    pub effect_contract: SkillEffectContract,
    pub notes: String,                     // llm_readable: 技能意象、限制、常见表现；不直接参与程序判断
}

pub struct SkillEffectContract {
    pub allowed_target_kinds: Vec<TargetKind>,
    pub allowed_state_domains: Vec<StateDomain>,      // body / resource / position / perception / mind / soul / scene / knowledge_reveal
    pub cost_profile: CostProfile,                    // semantic: 法力/体力/冷却/材料等成本
    pub max_intensity_tier: EffectIntensityTier,      // 程序校验硬效果强度上限
    pub allows_injury: bool,
    pub allows_position_change: bool,
    pub allows_knowledge_reveal: bool,
    pub requires_line_of_effect: bool,
    pub duration_policy: DurationPolicy,
    pub opens_reaction_window: bool,
    pub allows_interrupt: bool,
    pub max_reaction_depth_override: Option<u8>,  // 默认 None；若 Some(2) 必须由 EffectValidator 校验
}

pub struct CharacterSkillUseProfile {
    pub character_id: String,
    pub skill_id: String,
    pub mastery_rank: u8,  // 1-5: novice / trained / skilled / expert / master
    pub notes: String,
}
```

`StateDomain` 在数据模型中定义，用于连接技能效果、`ConditionState` 和状态更新计划。第一版 `CostProfile` 只覆盖 `mana_reserve`、`fatigue`、`cooldown` 与材料引用；魂伤、恐惧、感知干扰等通过带 `StateDomain` 的 `ConditionState` 表达，不引入通用资源池。

技能的"该角色掌握哪些技能"以 `KnowledgeEntry { kind: CharacterFacet, facet: KnownAbility | HiddenAbility }` 表达，统一受 `access_policy` 约束。OutcomePlanner 可以读取 `notes` 理解复杂效果，但硬状态变化只能落在 `effect_contract` 允许的范围内；超出范围的候选效果由 EffectValidator 转入 `blocked_effects` 或 `soft_effects`。

---
