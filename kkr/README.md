# KKR - Multi-Backend Agent Framework

A high-performance, multi-backend framework for building AI agents with **Rust** for performance-critical operations and **Python** for flexibility and ML integrations.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         KKR API                              │
├─────────────────────────────────────────────────────────────┤
│                    Backend Bridge                            │
│              (Intelligent Routing Layer)                     │
├──────────────────────┬──────────────────────────────────────┤
│    Rust Backend      │         Python Backend               │
│  ┌────────────────┐  │  ┌────────────────────────────────┐  │
│  │ • Compression  │  │  │ • ML Inference                 │  │
│  │ • Vector Ops   │  │  │ • Embeddings                   │  │
│  │ • Batch Proc   │  │  │ • Complex Logic                │  │
│  │ • Binary Ops   │  │  │ • API Integrations             │  │
│  └────────────────┘  │  └────────────────────────────────┘  │
└──────────────────────┴──────────────────────────────────────┘
```

## Features

- **Dual Backend**: Rust for speed, Python for flexibility
- **Intelligent Routing**: Automatic backend selection based on operation type
- **Unified API**: Same interface regardless of backend
- **Async-First**: Built for high concurrency
- **Type-Safe**: Full type hints and protocols

## Installation

```bash
# Python package
pip install kkr

# With Rust backend (requires Rust toolchain)
pip install kkr[rust]
cd kkr/backends/rust && maturin develop

# Full installation
pip install kkr[all]
```

## Quick Start

```python
import asyncio
from kkr import create_backend, BackendType

async def main():
    # Create hybrid backend (uses both Rust and Python)
    backend = await create_backend(BackendType.HYBRID)

    # Check health
    health = await backend.health_check()
    print(f"Backend healthy: {health['bridge_initialized']}")

    # Compress data (routed to Rust)
    data = b"Hello, World!" * 1000
    compressed = await backend.compress(data)
    print(f"Compressed: {len(data)} -> {len(compressed)} bytes")

    # Generate embeddings (routed to Python)
    embeddings = await backend.embed(["Hello", "World"])
    print(f"Embeddings shape: {len(embeddings)}x{len(embeddings[0])}")

    # Find similar vectors (routed to Rust for speed)
    query = embeddings[0]
    results = await backend.find_similar(query, embeddings, top_k=1)
    print(f"Most similar: index {results[0][0]}, score {results[0][1]:.4f}")

asyncio.run(main())
```

## Backend Selection

### Rust Backend (Performance)
Best for:
- Compression/decompression (LZ4, Zstandard)
- Vector operations (similarity, normalization)
- Batch processing
- Binary data operations

### Python Backend (Flexibility)
Best for:
- ML model inference
- Embedding generation
- Complex business logic
- External API integrations

### Hybrid Mode (Recommended)
Automatically routes operations to the optimal backend:
```python
backend = await create_backend(BackendType.HYBRID)
```

## Configuration

Create `kkr.yaml` in your project root:

```yaml
rust:
  enabled: true
  max_workers: 4
  default_compression: lz4

python:
  enabled: true
  default_model: gpt-4
  embedding_model: text-embedding-ada-002

hybrid:
  enabled: true
  prefer_rust_for:
    - compression
    - vector_ops
  prefer_python_for:
    - inference
    - embeddings
```

Or use environment variables:
```bash
export KKR_RUST_ENABLED=true
export KKR_PYTHON_DEFAULT_MODEL=gpt-4
```

## Project Structure

```
kkr/
├── core/                 # Protocols and types
│   ├── protocol.py       # Abstract interfaces
│   └── types.py          # Shared type definitions
├── backends/
│   ├── rust/             # Rust backend (PyO3)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs    # Main entry point
│   │       ├── compute.rs
│   │       ├── storage.rs
│   │       └── vector.rs
│   └── python/           # Python backend
│       └── src/
│           ├── backend.py
│           ├── inference.py
│           └── storage.py
├── bindings/             # Backend bridge
│   ├── bridge.py         # Unified interface
│   └── router.py         # Intelligent routing
├── config/               # Configuration
│   └── settings.py
└── sdk/                  # Client SDKs
    ├── python/
    ├── js/
    └── rust/
```

## Building Rust Backend

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install maturin
pip install maturin

# Build and install
cd kkr/backends/rust
maturin develop --release
```

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Format code
ruff format .

# Type check
mypy kkr/
```

## License

MIT
