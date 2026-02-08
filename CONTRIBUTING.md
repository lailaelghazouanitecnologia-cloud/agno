# Contributing to Agno

Agno is an open-source Rust project and we welcome contributions.

## How to contribute

Please follow the [fork and pull request](https://docs.github.com/en/get-started/quickstart/contributing-to-projects) workflow:

- Fork the repository.
- Create a new branch for your feature.
  - Add your feature or improvement.
  - **Ensure your Pull Request follows our guidelines (see below).**
  - Send a pull request.

## Pull Request Guidelines

1.  **Title Format:** Your PR title must start with a type tag enclosed in square brackets, followed by a space and a concise subject.
    - Example: `[feat] Add user authentication`
    - Valid types: `[feat]`, `[fix]`, `[docs]`, `[test]`, `[refactor]`, `[build]`, `[ci]`, `[chore]`, `[perf]`, `[style]`, `[revert]`.
2.  **Link to Issue:** The PR description should ideally reference the issue it addresses using keywords like `fixes #<issue_number>`, `closes #<issue_number>`, or `resolves #<issue_number>`.

## Prerequisites

- **Rust 1.93.0+** (`rustup update stable`)
- **Node 22+** and **Bun 1.3+** (for test projects)
- **Python 3.x** (for test projects)

## Project structure

```
agno/
  common/     — Shared error types, config, traits (5 crates)
  kkr/        — Agent framework core: providers, tools, inner monologue (35+ crates)
  entity/     — Advanced agent framework: orchestration, safety, embeddings (28 crates)
  roska/      — AST-based code analysis at configurable depths (4 crates)
  knowledge/  — Knowledge graph and persistence (5 crates)
  mos/        — Autonomous coding agent CLI (main binary)
  projects/   — Test projects (c-compiler-js, lang-repl, etc.)
```

## Development setup

1. Clone the repository:
   ```bash
   git clone https://github.com/agno-agi/agno.git
   cd agno
   ```

2. Build the main binary (MOS):
   ```bash
   cd mos && cargo build
   ```

3. Run the test suite:
   ```bash
   cd mos && cargo test
   ```

4. Build all workspaces:
   ```bash
   cargo build --workspace
   ```

## Building individual workspaces

Each workspace can be built independently:

```bash
cd common && cargo build
cd kkr && cargo build
cd roska && cargo build
cd knowledge && cargo build
cd entity && cargo build
cd mos && cargo build
```

## Running tests

```bash
# Run MOS tests (main binary)
cd mos && cargo test

# Run tests for a specific workspace
cd kkr && cargo test
cd roska && cargo test
cd knowledge && cargo test

# Run a specific test
cargo test -p mos -- graph_ops::tests::test_gap_analysis
```

## Code quality

- Run `cargo clippy` to check for lint issues.
- Run `cargo fmt --check` to verify formatting.
- Ensure `cargo build` produces no errors (warnings are tracked and being reduced).
- All existing tests must pass before submitting a PR.

## Adding a new LLM Provider

1. Create a new crate under `kkr/crates/providers/` (e.g., `kkr-provider-mymodel`).
2. Implement the `Provider` trait from `kkr-core`.
3. Add the crate to `kkr/Cargo.toml` workspace members.
4. Add tests in the crate's `src/` or `tests/` directory.

## Adding a new Tool

1. Create a new crate under `kkr/crates/tools/` (e.g., `kkr-tool-mytool`).
2. Implement the `Tool` trait from `kkr-core`.
3. Add the crate to `kkr/Cargo.toml` workspace members.
4. Add usage examples and tests.

## Architecture notes

- **MOS** is the main binary. It orchestrates builds using a graph-driven supervisor.
- **KKR** provides the Agent/Tool/Capsule abstractions and LLM provider integrations.
- **Roska** analyzes code at 4 depth levels (Overview, Structure, Detail, Body) to control context cost.
- **Knowledge** maintains a persistent graph of features, modules, and decisions.
- The **build supervisor** (`mos/src/build.rs`) uses `TaskQueue`, `TierBudget`, `RuleSet`, and `PlanGraph` to orchestrate multi-phase builds.

## License

This project is licensed under the terms of the [Apache-2.0 license](/LICENSE).
