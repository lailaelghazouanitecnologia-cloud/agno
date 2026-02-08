# Project Review — Agno / MOS

**Date**: 2026-02-08
**Scope**: Full codebase review across all 6 workspaces (common, kkr, entity, roska, knowledge, mos)
**Focus**: Architecture, code quality, integration, dead code, test coverage, consistency

---

## Executive Summary

Agno is an ambitious Rust-based multi-agent framework (~87 crates, 6 workspaces) centered around **MOS**, an autonomous coding agent with graph-driven supervision. The architecture is well-designed with clear separation of concerns: KKR provides the agent core, Roska handles code analysis, Knowledge manages persistent graphs, and MOS orchestrates everything.

**Key findings**:
- 68 tests pass, 0 fail — solid unit test coverage for active code
- 74 compiler warnings — primarily from unused subsystems
- Several well-designed modules (DecisionEngine, ContextAssembler, PlanGraph, TaskQueue) are implemented and tested but **not integrated** into the main execution path
- CONTRIBUTING.md is for a Python project, not this Rust codebase
- License inconsistency between workspaces (Apache-2.0 vs MIT)
- Duplicate utility functions across modules

---

## 1. Architecture Assessment

### What works well

The graph-driven supervisor is the standout design:

```
Plan Graph (features + specs)   ←→   Project Graph (modules + files)
                    ↓ gap analysis ↓
             Prioritized Actions
                    ↓
         Agent Dispatch (model-routed)
                    ↓
         Graph Update + Token Tracking
```

**Strengths**:
- **Dynamic action derivation** — no hardcoded phase enum; actions come from gap analysis
- **Feature state machine** — NotStarted → Implemented → Tested → Verified
- **Depth-driven context** — Roska's 4-level depth system (10-300 tokens/file) controls cost
- **Model routing** — cheap models for leaf work, expensive for architecture
- **Loop detection + escalation** — auto-upgrades model on consecutive errors
- **Plan parsing resilience** — handles varied JSON schemas, disk-written plans, name-based dep resolution
- **Token tracking per action** — prompt/completion separation, bottleneck detection

### Architecture gap: two parallel systems

The codebase contains **two parallel supervisor architectures**:

| Component | Status | Location |
|-----------|--------|----------|
| Inline supervisor loop | **Active** (used in `cmd_build`) | `main.rs:374-818` |
| `DecisionEngine` | Implemented, tested, **unused** | `decision.rs` |
| `TaskQueue` | Implemented, tested, **unused** | `queue.rs` |
| `TierBudget` | Implemented, tested, **unused** | `tiers.rs` |
| `ContextAssembler` | Implemented, tested, **unused** | `context.rs` |
| `PlanGraph` / `Plan` | Implemented, tested, **unused** | `plans.rs` |

The active `cmd_build` in `main.rs` implements the supervisor loop inline with ~450 lines of procedural code. The designed-but-unused modules (`DecisionEngine`, `TaskQueue`, `ContextAssembler`, `PlanGraph`) represent a more modular architecture that would replace this inline code.

**Recommendation**: Either integrate the designed modules or remove them. Having 5 unused but fully-tested subsystems adds maintenance burden and confusion about which architecture is canonical.

---

## 2. Code Quality

### Build warnings (74 total)

Broken down by category:

| Category | Count | Examples |
|----------|-------|---------|
| Never constructed structs | ~15 | `DecisionInput`, `ContextAssembler`, `PlanGraph`, `Plan` |
| Never used functions | ~12 | `derive_actions_heuristic`, `update_project_graph`, `estimate_tokens` |
| Never used methods | ~20 | `DecisionEngine::*`, `ContextAssembler::*`, `Plan::*` |
| Unused imports | ~5 | `EvalContext`, `CompletedAction` |
| Unused variables | ~3 | `task`, `name` |
| Never read fields | ~2 | `agent_name`, `max_context_nodes` |
| Dead variants | ~1 | `SimulationHalt` |

All warnings originate from the unused subsystems described above.

### Duplicate code

| Function | Locations | Notes |
|----------|-----------|-------|
| `extract_json` | `util.rs:30`, `decision.rs:391` | Identical implementations |
| `sanitize_id` | `util.rs:48`, `scanner.rs:510`, `inner_loop.rs:295` | Identical implementations |
| `estimate_tokens` | `util.rs:61`, `context.rs:318` | Identical implementations |
| `derive_actions_heuristic` | `decision.rs:307` | Near-duplicate of `graph_ops::derive_actions` |

**Recommendation**: Consolidate duplicates into `util.rs` and import from there.

### Code structure

The `main.rs` file is 819 lines with two large async functions (`cmd_run` at ~190 lines, `cmd_build` at ~445 lines). The build loop in particular handles:
- Gap analysis
- Action selection
- Context injection
- Model selection
- Prompt composition
- Agent dispatch
- Result processing
- Graph updates
- Plan parsing fallbacks
- Feature matching
- Token tracking
- Request persistence
- Performance summary

This is the core of the system and would benefit from extraction into a dedicated supervisor module using the existing (but unused) `TaskQueue` + `DecisionEngine` subsystems.

---

## 3. Test Coverage

### Current state: 68 tests, all passing

| Module | Tests | Coverage |
|--------|-------|----------|
| `supervisor` (via graph_ops) | 14 | Gap analysis, action derivation, plan parsing, feature lifecycle |
| `tiers` | 8 | Tier ordering, budget, escalation, display |
| `queue` | 9 | Push/pop, budget, loop detection, consecutive failures |
| `decision` | 7 | Input building, prompt generation, response parsing, heuristic fallback |
| `rules` | 8 | Conditions (always, action, file, task, error, composite), rule rendering |
| `context` | 4 | Context rendering, plan generation, token estimation |
| `plans` | 4 | Graph operations, status lifecycle, tree rendering |
| `routing` | 5 | Model selection, loop detection, escalation, de-escalation |
| `inner_loop` | 2 | Phase initialization, empty project handling |
| `scanner` | 2 | ID sanitization, nonexistent path handling |
| `util` | 4 | JSON extraction, camel-to-hyphen, token estimation, ID sanitization |
| `cli` | 1 | Version string |

**Notable**: The unused modules (`decision`, `queue`, `tiers`, `plans`, `context`) have thorough unit tests despite not being integrated. This suggests they were designed for integration but the refactor wasn't completed.

### Missing test coverage

- No integration tests for the full `cmd_build` loop
- No tests for `tools.rs` (provider construction, event handling, graph persistence)
- No tests for `config.rs` (TOML loading, override application, API key resolution)
- No tests for `persist.rs` (request serialization/deserialization)
- No tests for `prompt.rs::render_performance_summary` edge cases
- Scanner's `scan_feature_files` and `render_generic_file` lack tests for non-Rust languages

---

## 4. Consistency Issues

### License mismatch

- `/LICENSE` → Apache-2.0
- `/CONTRIBUTING.md` → References Apache-2.0
- `/kkr/Cargo.toml` → `license = "MIT"`
- Other workspace Cargo.toml files → No license field

**Recommendation**: Align all Cargo.toml files to Apache-2.0 to match the repository license.

### CONTRIBUTING.md is for a Python project

The CONTRIBUTING.md references:
- `uv`, `pip install`, Python virtual environments
- `ruff` (Python formatter), `mypy` (Python type checker)
- `pytest` for testing
- `libs/agno/agno/` Python package paths
- Adding VectorDB, Model Provider, Tool instructions for a Python API

This doesn't match the Rust codebase at all. It appears to be from the original Python-based agno project.

**Recommendation**: Rewrite CONTRIBUTING.md for the Rust codebase, covering `cargo build`, `cargo test`, workspace structure, and the actual development workflow.

---

## 5. Entity Workspace

The `entity/` workspace contains 28 crates implementing an alternative agent framework with:
- Stateful agents (descriptor, planner, coder)
- Orchestration (runtime, context, session, team, workflow)
- Safety (guardrails, safety policies)
- Data (memory, embeddings, vector DB, knowledge)
- Extensions (skills, reasoning)

This workspace is **not used by MOS**. The relationship between entity and kkr/mos is unclear. MOS depends on KKR for its agent core, not entity.

**Recommendation**: Clarify the relationship in README.md. If entity is a separate project or planned replacement for KKR, document that. If it's deprecated, consider removing it.

---

## 6. Performance Observations

From test project results (documented in memory):

| Project | Language | Tests Passing | Tokens Used |
|---------|----------|---------------|-------------|
| c-compiler-js | JavaScript | 78/88 (89%) | ~950K |
| lang-repl | Python | 7/7 (100%) | minimal |
| ts-transpiler | TypeScript | 0/20 (0%) | ~950K |
| c-compiler-bun | TypeScript/Bun | 14/76 (18%) | ~2.28M |

Key insight: **95% of tokens are input** (context re-reading). The `ContextAssembler` and targeted roska scanning were designed to address this but aren't integrated into the main build loop. Integrating them could significantly reduce token costs.

---

## 7. Recommendations Summary

### High priority

1. **Integrate or remove unused subsystems** — `DecisionEngine`, `TaskQueue`, `TierBudget`, `ContextAssembler`, `PlanGraph` are implemented and tested but create 74 warnings and architectural confusion
2. **Rewrite CONTRIBUTING.md** — Current version is for a Python project
3. **Fix license inconsistency** — KKR says MIT, repository says Apache-2.0
4. **Consolidate duplicate utilities** — `extract_json`, `sanitize_id`, `estimate_tokens`

### Medium priority

5. **Extract `cmd_build` into a proper supervisor module** — 445-line function with 8+ responsibilities
6. **Add integration tests** — The supervisor loop is untested end-to-end
7. **Clarify entity workspace relationship** — 28 crates with no documented connection to MOS
8. **Integrate `ContextAssembler`** — Would address the 95% input token problem

### Low priority

9. **Add config loading tests** — TOML parsing edge cases
10. **Test non-Rust language scanning** — `render_generic_file` for TS/JS/Python
11. **Document the inner loop phases** — Perception, Deliberation, Simulation, Execution, Reflection are well-coded but only partially wired up in `cmd_run`

---

## 8. Crate Count Summary

| Workspace | Crates | Role |
|-----------|--------|------|
| common | 5 | Shared error types, config, serialization, traits, tools |
| kkr | 35+ | Agent core, 6 LLM providers, 17 tools, inner monologue, AST, graph |
| entity | 28 | Alternative agent framework (not used by MOS) |
| roska | 4 | Code analysis: descriptor, generator, processor, differ |
| knowledge | 5 | Knowledge graph: core, graph, memory, project, persist |
| mos | 1 | Autonomous coding agent (19 source modules, 68 tests) |
| **Total** | **~79** | |
