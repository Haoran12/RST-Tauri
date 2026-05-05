PromptBuilder control plane module for Agent LLM nodes.

This file is intentionally short. The source of truth remains:
- `docs/13_agent_llm_io.md`
- `docs/21_agent_scene_llm_io.md`
- `docs/22_agent_outcome_narration_io.md`

The Rust implementation here should remain a thin, deterministic adapter for:
- static node contracts
- structured input packaging
- budget estimation and deterministic pruning
- provider-facing message layout
