"""
KKR - Multi-Backend Agent Framework
====================================

A high-performance, multi-backend framework for building AI agents.

Backends:
- Rust: High-performance compute, compression, vector operations
- Python: ML inference, complex logic, integrations
- Hybrid: Best of both worlds with intelligent routing

Usage:
------
    from kkr import create_backend, BackendType

    # Create a hybrid backend
    backend = await create_backend(BackendType.HYBRID)

    # Or use specific backends
    rust_backend = await create_backend(BackendType.RUST)
    python_backend = await create_backend(BackendType.PYTHON)
"""

__version__ = "0.1.0"
__author__ = "KKR Team"

from kkr.core import (
    # Protocols
    BackendType,
    BackendConfig,
    BackendProtocol,
    StorageProtocol,
    ComputeProtocol,
    InferenceProtocol,
    VectorProtocol,
    # Types
    Status,
    Priority,
    Request,
    Response,
    ComputeRequest,
    InferenceRequest,
    VectorSearchRequest,
    HealthStatus,
)

from kkr.bindings import (
    BackendBridge,
    HybridBackend,
    BackendRouter,
)

from kkr.config import Settings, load_settings


async def create_backend(
    backend_type: BackendType = BackendType.HYBRID,
    config: dict = None,
) -> BackendProtocol:
    """
    Create and initialize a backend.

    Args:
        backend_type: Type of backend to create (RUST, PYTHON, or HYBRID)
        config: Optional configuration dictionary

    Returns:
        Initialized backend instance

    Example:
        backend = await create_backend(BackendType.HYBRID)
        response = await backend.execute(request)
    """
    settings = load_settings()

    backend_config = BackendConfig(
        backend_type=backend_type,
        name=f"{backend_type.value}-backend",
        version=__version__,
        settings=config or {},
        max_workers=config.get("max_workers", 4) if config else 4,
        timeout_ms=config.get("timeout_ms", 30000) if config else 30000,
    )

    if backend_type == BackendType.RUST:
        try:
            from kkr_rust_backend import RustBackend
            return RustBackend()
        except ImportError:
            raise ImportError(
                "Rust backend not available. Build with: cd kkr/backends/rust && maturin develop"
            )

    elif backend_type == BackendType.PYTHON:
        from kkr.backends.python.src import PythonBackend
        backend = PythonBackend()
        await backend.initialize(backend_config)
        return backend

    else:  # HYBRID
        backend = HybridBackend()
        await backend.initialize(backend_config)
        return backend


__all__ = [
    # Version
    "__version__",
    # Factory
    "create_backend",
    # Protocols
    "BackendType",
    "BackendConfig",
    "BackendProtocol",
    "StorageProtocol",
    "ComputeProtocol",
    "InferenceProtocol",
    "VectorProtocol",
    # Types
    "Status",
    "Priority",
    "Request",
    "Response",
    "ComputeRequest",
    "InferenceRequest",
    "VectorSearchRequest",
    "HealthStatus",
    # Bindings
    "BackendBridge",
    "HybridBackend",
    "BackendRouter",
    # Config
    "Settings",
    "load_settings",
]
