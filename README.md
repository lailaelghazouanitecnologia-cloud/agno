# Agno — Rust Multi-Agent Framework

Autonomous coding agent framework built in Rust. ~87 crates across 6 workspaces.

## Architecture

```
agno/
├── mos/          MOS — autonomous coding agent (supervisor + agent loop)
├── kkr/          KKR — agent core, providers, tools, inner monologue
├── roska/        Roska — code analysis at configurable depth levels
├── knowledge/    Knowledge Graph — persistent project understanding
├── common/       Shared error types and utilities
├── entity/       Entity system
└── projects/     Test projects built by MOS
```

## MOS — The Autonomous Coding Agent

MOS is a graph-driven autonomous coding agent that builds software from natural language tasks.

### How it works

```
Task → Plan Graph (Features + Specs)
         ↕ gap analysis ↕
Roska → Project Graph (Modules + Files)
         ↓
Actions (prioritized, context-aware)
         ↓
Prompt Composition (from graph context + roska depth)
         ↓
Agent Dispatch → Code
```

1. **Plan Graph**: Decomposes the task into Feature nodes with Specs (what we WANT)
2. **Project Graph**: Roska scans existing code at configurable depth (what we HAVE)
3. **Gap Analysis**: Compares plan vs project to derive prioritized actions
4. **Dynamic Prompts**: Composes prompts from graph context, not static templates
5. **Agent Loop**: Dispatches actions (Plan, Scaffold, Implement, Test, Fix, Verify)
6. **Performance Tracking**: Per-action token breakdown (in/out), timing, bottleneck detection

### Usage

```bash
# Initialize a project
cd my-project && mos init

# Single agent run
mos "Build a REST API in TypeScript"

# Supervised multi-phase build (graph-driven)
mos build "Build a C compiler with micro-VMs in TypeScript"

# Inspect knowledge graph
mos graph

# Scan project with roska
mos scan

# Show model profiles
mos models
```

### CLI Options

```
--model <name>       Override default model
--base-url <url>     API base URL (OpenAI-compatible)
--workspace <path>   Working directory (default: .)
--max-iter <n>       Max supervisor iterations (default: 20)
--autonomous         Skip approval prompts
```

### Configuration

MOS is configured via `.agent/agent.toml`:

```toml
[agent]
name = "mos"
approval = "autonomous"

[models.architect]
model = "gpt-4"
max_tokens = 16384

[models.coder]
model = "gpt-3.5-turbo"
max_tokens = 16384

[routing]
depth_0 = "architect"   # workspace-level decisions
depth_1 = "coder"       # module-level implementation
depth_2 = "coder"       # file-level details
depth_3 = "micro"       # function-level edits
```

## Workspaces

### KKR — Agent Core

Agent loop, OpenAI-compatible provider, native coding tools (fs, shell, git, search), inner monologue engine, and plan execution.

### Roska — Code Analysis

Parses code into AST descriptors at 4 depth levels:
- **Depth 0 (Overview)**: ~10 tokens/file — crate names, file counts
- **Depth 1 (Structure)**: ~60 tokens — module signatures, public API
- **Depth 2 (Detail)**: ~150 tokens — function bodies, types
- **Depth 3 (Body)**: ~300 tokens — full source code

### Knowledge Graph

Persistent graph with typed nodes (Feature, Module, File, Function, Concept) and edges (Parent, DependsOn, Implements, Tests). Supports specs, decisions, conversations. Serialized to disk for cross-session memory.

## Supervisor Features

- **Graph-driven gap analysis**: Plan vs project state comparison
- **Adaptive iterations**: Plan=5, Scaffold=10, Test=15, Fix=20, Implement=25
- **Token tracking**: Input/output token separation per action
- **Timing instrumentation**: Wall-clock duration per action
- **Context tagging**: Records what graph context was injected per agent call
- **Performance summary**: Identifies bottlenecks (slow actions, high token usage)
- **Dependency-aware priority**: Features with unmet deps get lower priority
- **Action rotation**: Prevents getting stuck on failing features (max 2 retries)
- **Fallback plan creation**: If Plan action writes files instead of JSON, creates features from workspace

## Test Projects

Projects in `projects/` are built autonomously by MOS for testing:

| Project | Language | Description | Status |
|---------|----------|-------------|--------|
| c-compiler-js | JavaScript | C subset compiler | 78/88 tests (89%) |
| lang-repl | Python | Language REPL | 7/7 examples pass |
| ts-transpiler | TypeScript | TS→JS transpiler | Parser bugs |
| micro-compiler | TypeScript | Modular compiler with micro-VMs | In progress |
| c-compiler-bun | TypeScript/Bun | C compiler with micro-VMs | New |

## Build

```bash
# Build MOS
cd mos && cargo build

# Run tests (25 tests)
cargo test

# Requires: Rust 1.93+, Node 22+
```

## Environment

MOS works with any OpenAI-compatible API. Set your API key:

```bash
export OPENAI_API_KEY="your-key"
# or
export MOS_API_KEY="your-key"
```
