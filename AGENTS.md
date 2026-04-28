# Repository Guidelines

## Project Structure & Module Organization

This is currently a documentation-first Tauri project for Ran's SmartTavern (RST). Keep source-of-truth guidance in `README.md` and focused design documents under `docs/`.

- `README.md`: project map and document ownership.
- `docs/implementation_plan.md`: roadmap, milestones, and key decisions.
- `docs/01_architecture.md`: architecture, invariants, and LLM/program boundaries.
- `docs/02_st_mode.md`: SillyTavern-compatible character card, worldbook, and injection behavior.
- `docs/10_agent_data_and_simulation.md`: Agent data model, derived state, outcome planning, skill contracts, and SQLite.
- `docs/11_agent_runtime.md`: runtime loop, cognitive pass, active set, dirty flags, and validation rules.
- `docs/20_backend_contracts.md`: backend AI provider contracts.
- `docs/90_pitfalls_and_tests.md`: risk register and verification plans.
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

Treat `docs/90_pitfalls_and_tests.md` as the quality gate. When implementing features, add tests for every listed invariant that becomes executable. Prefer behavior names such as `agent_runtime_rejects_invalid_active_set` or `worldbook_respects_injection_order`.

## Commit & Pull Request Guidelines

Recent commits are short Chinese summaries, for example `继续完整文档` and `调整文档结构: ...`. Follow that style: imperative, focused, and scoped to one logical change. Pull requests should include a summary, affected docs or modules, linked issues if any, and screenshots only for UI changes.

## Agent-Specific Instructions

Do not overwrite unrelated local edits. Start concept changes in `docs/01_architecture.md`, then propagate details to mode, data, runtime, backend, and testing docs. Keep LLM responsibilities and deterministic program logic separate.
