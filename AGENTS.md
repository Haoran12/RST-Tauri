# Repository Guidelines

## Project Structure & Module Organization

This is currently a documentation-first Tauri project for Ran's SmartTavern (RST). Keep source-of-truth guidance in `README.md` and focused design documents under `docs/`.

- `README.md`: project map and document ownership.
- `docs/implementation_plan.md`: roadmap, milestones, and key decisions.
- `docs/01_architecture.md`: architecture, invariants, and LLM/program boundaries.
- `docs/02_app_data_and_modules.md`: application data directories, configuration snapshots, frontend/backend module layout, and module ownership.
- `docs/70_st_mode.md`: SillyTavern mode overview and compatibility principles.
- `docs/71_st_character_cards.md`: SillyTavern-compatible TavernCard V3 behavior.
- `docs/72_st_worldbook_model.md`: SillyTavern-compatible worldbook and CharacterBook data model.
- `docs/73_st_worldbook_injection.md`: SillyTavern-compatible worldbook injection workflow.
- `docs/74_st_presets.md`: SillyTavern-compatible presets with RST provider decoupling.
- `docs/75_st_runtime_assembly.md`: ST runtime state, request assembly, and provider mapping.
- `docs/76_st_regex.md`: SillyTavern-compatible Regex extension data model, scope rules, and runtime hooks.
- `docs/10_agent_data_model.md`: Agent data model overview and three-layer semantics.
- `docs/11_agent_runtime.md`: runtime loop, cognitive pass, active set, dirty flags, and validation rules.
- `docs/12_agent_simulation.md`: deterministic derived state, environment tiers, and attribute tiers.
- `docs/13_agent_llm_io.md`: PromptBuilder, CognitivePass I/O, LLM node index, and dirty flags.
- `docs/14_agent_persistence.md`: Agent SQLite tables, indexes, and persistence boundaries.
- `docs/15_agent_location_system.md`: Agent location hierarchy, natural regions, inherited region facts, route graph, and travel estimates.
- `docs/16_agent_timeline_and_canon.md`: Agent time anchors, sessions, mainline cursor, and canon eligibility.
- `docs/17_agent_knowledge_model.md`: KnowledgeEntry, access policy, content schemas, TruthGuidance, and reveal events.
- `docs/18_agent_character_model.md`: CharacterRecord, base attributes, body baseline, temporary state, and mana expression.
- `docs/19_agent_combat_and_skills.md`: combat resolution, mana combat math, invariants, and skill contracts.
- `docs/20_backend_contracts.md`: backend AI provider contracts.
- `docs/21_agent_scene_llm_io.md`: SceneInitializer and SceneStateExtractor structured I/O.
- `docs/22_agent_outcome_narration_io.md`: OutcomePlanner, ReactionWindow, StyleConstraints, and SurfaceRealizer I/O.
- `docs/30_logging_and_observability.md`: Agent Trace, runtime logs, retention, and observability rules.
- `docs/90_pitfalls_and_tests.md`: risk register and testing quality gate.
- `docs/91_test_matrix.md`: staged test cases and verification plans.
- `docs/reference/`: external or historical reference notes; do not treat these as primary specs.

When app code is added, prefer conventional Tauri layout: frontend in `src/`, Rust backend in `src-tauri/`, static assets in `public/` or `assets/`, and tests near verified modules.

## Build, Test, and Development Commands

No package manifest or Tauri workspace is present yet, so there are no runnable build or test commands. Until implementation files are added, validate docs with:

- `git diff -- README.md docs AGENTS.md`: review documentation-only changes.
- `git status --short`: confirm changed files before committing.

Once app scaffolding lands, document exact commands here, such as `npm run dev`, `npm test`, `cargo test`, and `npm run tauri build`.

## Coding Style & Naming Conventions

Use Markdown headings with clear ownership boundaries. Keep docs concise, update the latest version directly, and avoid historical "before/after" notes. Existing documents use Chinese prose with English technical identifiers; preserve that style unless a file is already English-only. Use numbered prefixes for major docs (`01_`, `10_`, `20_`, `90_`).

## Testing Guidelines

Treat `docs/90_pitfalls_and_tests.md` as the risk gate and `docs/91_test_matrix.md` as the executable verification plan. When implementing features, add tests for every listed invariant that becomes executable. Prefer behavior names such as `agent_runtime_rejects_invalid_active_set` or `worldbook_respects_injection_order`.

## Commit & Pull Request Guidelines

Recent commits are short Chinese summaries, for example `继续完整文档` and `调整文档结构: ...`. Follow that style: imperative, focused, and scoped to one logical change. Pull requests should include a summary, affected docs or modules, linked issues if any, and screenshots only for UI changes.

## Agent-Specific Instructions

Do not overwrite unrelated local edits. Start concept changes in `docs/01_architecture.md`, then propagate details to mode, data, runtime, backend, and testing docs. Keep LLM responsibilities and deterministic program logic separate.
