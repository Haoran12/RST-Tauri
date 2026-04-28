# 12 Agent 程序化派生与解算

本文档承载 Agent 模式中由程序确定的派生、档位翻译和硬规则解算：

- 环境档位翻译：物理环境如何影响感知、行动与长期压力
- 灵力档位翻译：原始数值如何转为 LLM 可读的档位与压力
- Mana Combat Resolution：灵力对抗的可验证物理后果
- Skill Model：技能契约与 LLM 描述文本的边界

基础数据模型见 [10_agent_data_model.md](10_agent_data_model.md)。LLM 节点 I/O 契约见 [13_agent_llm_io.md](13_agent_llm_io.md)。运行时调用顺序见 [11_agent_runtime.md](11_agent_runtime.md)。

---

## 1. 程序化派生：环境档位翻译

LLM 不擅长把 raw 数值（`50.0 m/s`、`-30.0 ℃`、`视距 8 m`）翻译成行为后果。这一步在程序里做：`EmbodimentResolver` 与 `SceneFilter` 协同把 Layer 1 `physical_conditions` 的原始量映射为**档位 + 具体后果**，分别写入 `EmbodimentState.body_constraints.environmental_strain`（影响该角色行动）和 `FilteredSceneView.weather_perception`（角色对天气的主观感受）。

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
    // 写入 EmbodimentState.body_constraints；驱动 action_feasibility 与跨回合身体状态
    pub wind_tier: WindImpactTier,
    pub temperature_tier: TemperatureFeelTier,
    pub surface_tier: SurfaceImpactTier,
    pub respiration_tier: RespirationImpactTier,
    pub movement_penalty: f64,           // 0.0-1.0
    pub balance_penalty: f64,            // 0.0-1.0
    pub cold_strain: f64,                // 累积冷损耗（按时间累加，到阈值由 OutcomePlanner 候选 + EffectValidator 生成冻伤事件）
    pub heat_strain: f64,
    pub respiration_strain: f64,         // 累积呼吸损耗（沙暴/浓烟久留触发咳嗽/缺氧伤害）
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
4. `cold_strain` / `heat_strain` 跨回合累积；到阈值由 OutcomePlanner 候选 + EffectValidator 生成具体伤势事件（冻伤/中暑），写回 Layer 1。
5. `disrupted_actions` 是 LLM 选择行动时的硬约束（在 IntentPlan 验证阶段比对），不是建议。
6. SurfaceRealizer 如需在叙事中提到风速/温度的具体数字，应通过 `SurfaceRealizerInput` 单独传入 raw 值（叙事用），不经 `FilteredSceneView`。
7. **L1 字段须保持自洽**：`physical_conditions` 各子字段间存在因果（暴雨 → wetness↑ → slipperiness↑；沙暴 → dust_density↑ → visibility↓ + respiration 受影响）。`SceneStateExtractor` 在产出 L1 时由 prompt 模板要求一并填齐；档位翻译层只负责把 L1 翻译成档位，不补全 L1 缺失。
8. 翻译公式集中在 `EmbodimentResolver::translate_environment(...)` 与 `SceneFilter::derive_weather_perception(...)`，两者共享同一份阈值表（避免两侧档位不一致）。

---

## 2. 程序化派生：灵力档位翻译

灵力的"档位"用于身份识别（"是凡人/修士/超凡/传说"），灵力的"数值差"用于实力对比（感知层是体感强弱，对抗解算层是实际胜负）。两者都不让 LLM 自己估算 raw 数值。

档位边界数值参考 `rp_cards\` 锚点（凡人 100 / 入门 500–800 / 瓶颈 1300–1450 / 大成 2400 / 仙灵修行瓶颈 5000 / 神祇 苍角 8800 / 高阶仙灵 NaN），可在 `world_base.yaml` 中按世界重写。

```rust
pub enum ManaPotencyTier {
    // 单个角色 / 法器 / 法术 / 灵脉的灵力强度档位（默认边界，可由世界配置覆盖）
    Mundane,       // [0, 200)         凡人 / 无修行（锚: 人类无修行 100）
    Awakened,      // [200, 1000)      入门（锚: 妖精入门 500, 人类入门 700, 仙灵诞生 800）
    Adept,         // [1000, 1800)     成熟/精英（锚: 妖精瓶颈 1400, 人类瓶颈 1300, 齐松 1450）
    Master,        // [1800, 2600)     大成（锚: 仙灵不修行成型 1800, 人妖大成 2400）
    Ascendant,     // [2600, 5600)     高阶（锚: 仙灵修行瓶颈 5000）
    Transcendent,  // [5600, +∞)       超越/超凡（锚: 苍角 7200, 高阶仙灵 NaN）
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

pub enum ManaPerceptionDelta {
    // Δ = target.displayed_mana_power - observer.effective_mana_power
    // 用于"感觉差距多大"，与档位识别正交（同档可有显著差，跨档也可被技巧/状态拉平）
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

pub struct PerceivedManaProfile {
    pub source_id: String,                            // 被感知者 / 来源
    pub tier_assessment: Option<ManaPotencyTier>,     // 对方档位识别（被压制时为压制后的档）
    pub delta: ManaPerceptionDelta,                   // 感知差距档位
    pub attribute_assessment: Option<ManaAttribute>,  // 仅 |Δ| < 1000 且未被严重干扰时较准
    pub confidence: f64,                              // 0.0-1.0
    pub concealment_suspected: bool,                  // 感觉对方在压制气息
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
    pub interferences: Vec<String>,                   // "屏蔽阵法残留", "灵雾阻隔感知"
    pub overload_risk: bool,                          // 灵觉过载风险（高敏锐度撞 Saturated 环境）
    pub descriptors: Vec<String>,                     // ["灵气浓郁如蜜, 呼吸间满是清甜"]
}
```

**感知规则（认知层）**——由 `SceneFilter::derive_mana_perception(...)` 程序化实施：

1. **观察者灵力** = `observer.effective_mana_power`（已含 L1 伤势 / 疲惫 / 突破修正）。
2. **目标显示灵力** `target.displayed_mana_power`：
   - 默认 = `target.effective_mana_power`。
   - 若目标具备压制能力且本回合启用：`displayed = effective - suppression_amount`（压制量来自 L1 状态，不让 LLM 自己定）。
3. **Δ = target.displayed_mana_power − observer.effective_mana_power**，按上述 9 档桶映射到 `ManaPerceptionDelta`。
4. **档位识别**：
   - `|Δ| < 1000`：可识别 `tier_assessment = ManaPotencyTier::from_power(displayed)` 与 `attribute_assessment`，`confidence ≥ 0.7`。
   - `|Δ| ∈ [1000, 2000)`：可识别 tier，但 attribute 不稳；descriptors 偏向"远胜 / 远不及"。
   - `|Δ| ≥ 2000`：`tier_assessment = None`，descriptors 偏向"无法测度 / 如同蝼蚁"。
5. **Mundane (Tier0) 观察者**：仅能将 `effective_mana_power ≥ 1000` 的存在感知为"超出常理"，无具体档位；环境灵气仅给"格外厚重 / 压抑"等体感。
6. **零灵觉**（`SensoryCapabilities.mana.acuity == 0`）：`mana_signals = []`，`mana_environment.density_tier` 由间接体感（呼吸/温度异常）回填，`dominant_attribute = None`。
7. **隐匿 / 压制**：
   - 压制后档位 `displayed_tier = ManaPotencyTier::from_power(displayed)` 直接落在 tier_assessment 上。
   - **破绽判定**（`concealment_suspected`）：当 `observer.effective_mana_power ≥ target.effective_mana_power − 200` 时（即观察者实力已能"接近"压制前的目标），置 true（"似有若无的违和感"）。否则压制看起来天衣无缝，false。
   - 灵觉敏锐度可作为额外破绽来源：`acuity ≥ 0.85` 且 `target.suppression_amount ≥ 1000` 时也强制 `concealment_suspected = true`（高灵觉天然能闻到压制痕迹）。
8. **环境干扰**：`ManaField.interferences` 中的 jam/scramble 按强度降低 `confidence`；`mana_haze` 让该回合所有 mana_signals 的 |Δ| 视为额外 +500（拉远感知，便于隐匿者进出）。
9. **属性相生相克**：观察者擅长属性与目标属性相同 → confidence +；相克 → 易识别（descriptors 含"违逆 / 刺骨"），同时影响 `attribute_assessment` 准确度与 descriptors 色彩。

**关键不变量**：

1. CognitivePass 永远不读 raw `mana_power`，只读 tier / delta / descriptors。`FilteredSceneView` 中不暴露 raw 数值。
2. 档位边界、Δ 桶边界、压制破绽阈值都是**世界配置项**（默认值同上，对 rp_cards 锚点校准），改边界需同时更新角色卡解析与单元测试。
3. 感知层只写**事实级感受**（"远胜 / 难测 / 似有压制"），**不写信念**（"他一定是神祇 / 他在装弱 / 他没安好心"）。这些信念由 CognitivePass 的 LLM 基于感受 + `prior_subjective_state` 自行生成。
4. ManaPotencyTier 同时为 `KnowledgeEntry { facet: CultivationRealm }` 的内部表征：`access_policy` 决定"谁能读取这一档", 跨档感知精度决定"感知到的是真档还是被压制的档"。
5. SurfaceRealizer 如需在叙事中提到"修为相差一筹/远胜/碾压"等具体差距文字，从 `ManaSignal.perceived.delta` 与 `tier_assessment` 取，不回查 raw mana_power。

---

## 3. Mana Combat Resolution（程序化灵力对抗解算）

对抗解算层与感知层用的是**不同**输入：

- 感知层：`displayed_mana_power`（含压制）→ 角色"觉得"对方多强。
- 对抗解算层：`effective_mana_power`（不含压制；压制只是没主动用全力）→ 实际对抗按真实底力 + 技能 + 身体状态计算。

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
    // 由 |combat_delta| 桶映射；与感知层 ManaPerceptionDelta 共享 150/300/1000 三个阈值
    // 对抗解算层不再细分 1000 以上：到了"无力应对"就够用了
    Indistinguishable,       // |Δ| < 150       势均力敌, 胜负看临场发挥/技巧
    SlightEdge,              // Δ ∈ [150, 300)  攻方略占上风
    MarkedEdge,              // Δ ∈ [300, 1000) 攻方明显优势
    Crushing,                // Δ ≥ 1000        守方基本无力应对, 仅能逃避或求饶
    // 负向（攻方反吃亏）对称展开
}
```

### 3.1 对抗解算公式（程序化）

```
combat_power = effective_mana_power × max(0.1, 1 + Σ_modifiers) × soul_factor
```

仅有**两个独立乘区**：加算修正区（多数因子在此叠加），与灵魂状态乘区（单独成区）。其余因子全部以**加和**方式落到 `Σ_modifiers` 内，不互乘。

1. **基础有效灵力** `effective_mana_power = base_mana_power + L1 状态修正`（突破/中毒/压制解除等，皆为 L1 真相，不含伤势疲惫——后者落入加算修正区）。
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

5. **outcome_tier** 按 `combat_delta = actor_combat_power − target_combat_power` 落桶（150 / 300 / 1000，1000 以上即 Crushing）；细化由 `disrupting_factors` 列出（程序生成的具体说明，例 ["攻方显著疲惫 -0.20", "守方身体重伤 -0.40 + 恐惧 -0.10 + 灵魂破损 ×0.5"]）。

6. 程序化对抗解算只决定**可验证物理后果**（伤势 / 法力消耗 / 位置变化）是否可写回 L1；公开退让、站队、敌对升级等外显社会事件可由 OutcomePlanner 候选输出，但内心恐惧 / 屈服 / 记仇仍由下游角色 CognitivePass 解读。

### 3.2 关键不变量

1. 对抗解算公式只读 L1 的 `effective_mana_power`、L1 的身体状态、L1 的技能/属性数据；**不读 displayed_mana_power**（压制是认知层的事，不影响真实对抗）。
2. `combat_delta` 与 `ManaPerceptionDelta` 共享 150/300/1000 三个阈值，保证"我感觉略胜"与"实际略胜"在同一刻度上。对抗解算层在 1000 以上不再细分（结果都是 Crushing）；感知层仍区分 `FarAbove(1000-2000)` 与 `Overwhelming(≥2000)`，但两者**对应的对抗结论一致**（皆为"基本无力应对"），区别只在体感（"远胜，难敌" vs "无法测度，压顶之势"）与是否可识别 tier。
3. 当 `disrupting_factors` 与 `outcome_tier` 出现"违和"（例如攻方 base_mana_power 高但身体状态极差导致 combat_delta 反而为负），SurfaceRealizer 必须在叙事中体现这种反差，而不是按"谁灵力高谁赢"硬写。
4. **以弱胜强**在该框架下要求**多个加算修正叠加 + 可能的灵魂状态打击**：守方若同时陷入"显著疲惫 (-0.20) + 身体重伤 (-0.40) + 恐惧 (-0.10) = Σ = -0.70"，加算系数 = max(0.1, 0.30) = 0.30；再叠加灵魂破损 soul_factor = 0.5，总系数 0.15，足以让基础灵力差 1500 的弱者翻盘。"算计 / 偷袭 / 中毒 / 惊扰魂魄"必须落到具体的 L1 状态字段上，由公式自然得出，不允许 LLM 在对抗解算口径上手抹平差距。

---

## 4. Skill Model（契约 + LLM）

```rust
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

技能的"该角色掌握哪些技能"以 `KnowledgeEntry { kind: CharacterFacet, facet: KnownAbility | HiddenAbility }` 表达，统一受 `access_policy` 约束。OutcomePlanner 可以读取 `notes` 理解复杂效果，但硬状态变化只能落在 `effect_contract` 允许的范围内；超出范围的候选效果由 EffectValidator 转入 `blocked_effects` 或 `soft_effects`。

---

