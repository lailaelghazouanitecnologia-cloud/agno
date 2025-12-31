"""
Backend Bridge
==============
Unified interface for multiple backends with automatic routing.
"""

import asyncio
from typing import Any, AsyncIterator, Dict, List, Optional, Type
from dataclasses import dataclass, field
from enum import Enum

from kkr.core import (
    BackendProtocol,
    BackendType,
    BackendConfig,
    Request,
    Response,
    Status,
    ComputeRequest,
    InferenceRequest,
    VectorSearchRequest,
)


class OperationType(Enum):
    """Types of operations for routing decisions."""
    COMPUTE = "compute"      # CPU-intensive → Rust
    INFERENCE = "inference"  # ML operations → Python
    STORAGE = "storage"      # Data persistence → Either
    VECTOR = "vector"        # Vector ops → Rust (perf) or Python (flexibility)
    GENERAL = "general"      # General operations → Python


@dataclass
class BackendBridge:
    """
    Bridge that manages multiple backends and routes requests appropriately.

    The bridge automatically selects the best backend based on:
    - Operation type (compute → Rust, inference → Python)
    - Request characteristics
    - Backend availability
    - Performance requirements
    """

    rust_backend: Optional[BackendProtocol] = None
    python_backend: Optional[BackendProtocol] = None
    default_backend: BackendType = BackendType.PYTHON
    _initialized: bool = False

    async def initialize(
        self,
        rust_config: Optional[BackendConfig] = None,
        python_config: Optional[BackendConfig] = None,
    ) -> None:
        """Initialize available backends."""

        # Try to initialize Rust backend
        if rust_config:
            try:
                from kkr_rust_backend import RustBackend, BackendConfig as RustConfig
                rust_cfg = RustConfig(
                    max_workers=rust_config.max_workers,
                    timeout_ms=rust_config.timeout_ms,
                    enable_caching=rust_config.enable_caching,
                    enable_tracing=rust_config.enable_tracing,
                )
                self.rust_backend = RustBackend(rust_cfg)
            except ImportError:
                print("Warning: Rust backend not available, using Python only")

        # Initialize Python backend
        if python_config:
            from kkr.backends.python.src import PythonBackend
            self.python_backend = PythonBackend()
            await self.python_backend.initialize(python_config)

        self._initialized = True

    async def shutdown(self) -> None:
        """Shutdown all backends."""
        if self.python_backend:
            await self.python_backend.shutdown()
        self._initialized = False

    def _determine_operation_type(self, request: Request) -> OperationType:
        """Determine the type of operation from the request."""
        if isinstance(request, ComputeRequest):
            return OperationType.COMPUTE
        elif isinstance(request, InferenceRequest):
            return OperationType.INFERENCE
        elif isinstance(request, VectorSearchRequest):
            return OperationType.VECTOR
        else:
            return OperationType.GENERAL

    def _select_backend(self, op_type: OperationType) -> BackendProtocol:
        """Select the best backend for an operation type."""

        # Routing logic
        routing = {
            OperationType.COMPUTE: self.rust_backend or self.python_backend,
            OperationType.INFERENCE: self.python_backend or self.rust_backend,
            OperationType.VECTOR: self.rust_backend or self.python_backend,
            OperationType.STORAGE: self.python_backend or self.rust_backend,
            OperationType.GENERAL: self.python_backend or self.rust_backend,
        }

        backend = routing.get(op_type)

        if not backend:
            raise RuntimeError("No backend available for operation")

        return backend

    async def execute(self, request: Request) -> Response:
        """Execute a request on the appropriate backend."""
        if not self._initialized:
            raise RuntimeError("Bridge not initialized")

        op_type = self._determine_operation_type(request)
        backend = self._select_backend(op_type)

        return await backend.execute(request)

    async def execute_stream(self, request: Request) -> AsyncIterator[Response]:
        """Execute a streaming request."""
        if not self._initialized:
            raise RuntimeError("Bridge not initialized")

        op_type = self._determine_operation_type(request)
        backend = self._select_backend(op_type)

        async for response in backend.execute_stream(request):
            yield response

    async def health_check(self) -> Dict[str, Any]:
        """Get health status of all backends."""
        status = {
            "bridge_initialized": self._initialized,
            "backends": {},
        }

        if self.rust_backend:
            status["backends"]["rust"] = await self.rust_backend.health_check()

        if self.python_backend:
            status["backends"]["python"] = await self.python_backend.health_check()

        return status


@dataclass
class HybridBackend(BackendProtocol[Request, Response]):
    """
    Hybrid backend that combines Rust and Python for optimal performance.

    - Rust: Compression, vector operations, batch processing
    - Python: ML inference, complex logic, integrations
    """

    bridge: BackendBridge = field(default_factory=BackendBridge)

    @property
    def backend_type(self) -> BackendType:
        return BackendType.HYBRID

    @property
    def is_ready(self) -> bool:
        return self.bridge._initialized

    async def initialize(self, config: BackendConfig) -> None:
        """Initialize hybrid backend."""
        await self.bridge.initialize(
            rust_config=config,
            python_config=config,
        )

    async def shutdown(self) -> None:
        """Shutdown hybrid backend."""
        await self.bridge.shutdown()

    async def execute(self, request: Request) -> Response:
        """Execute on appropriate backend."""
        return await self.bridge.execute(request)

    async def execute_stream(self, request: Request) -> AsyncIterator[Response]:
        """Stream from appropriate backend."""
        async for response in self.bridge.execute_stream(request):
            yield response

    async def health_check(self) -> Dict[str, Any]:
        """Get combined health status."""
        return await self.bridge.health_check()

    # Convenience methods for common operations

    async def compress(self, data: bytes, algorithm: str = "lz4") -> bytes:
        """Compress data using Rust backend."""
        if self.bridge.rust_backend:
            if algorithm == "lz4":
                return self.bridge.rust_backend.compress_lz4(data)
            else:
                return self.bridge.rust_backend.compress_zstd(data)
        else:
            # Fallback to Python
            import lz4.frame
            return lz4.frame.compress(data)

    async def decompress(self, data: bytes, algorithm: str = "lz4") -> bytes:
        """Decompress data using Rust backend."""
        if self.bridge.rust_backend:
            if algorithm == "lz4":
                return self.bridge.rust_backend.decompress_lz4(data)
            else:
                return self.bridge.rust_backend.decompress_zstd(data)
        else:
            import lz4.frame
            return lz4.frame.decompress(data)

    async def embed(self, texts: List[str]) -> List[List[float]]:
        """Generate embeddings using Python backend."""
        if self.bridge.python_backend:
            return await self.bridge.python_backend._inference.embed(texts)
        raise RuntimeError("Python backend required for embeddings")

    async def find_similar(
        self,
        query: List[float],
        candidates: List[List[float]],
        top_k: int = 10,
    ) -> List[tuple]:
        """Find similar vectors using Rust backend."""
        if self.bridge.rust_backend:
            return self.bridge.rust_backend.find_similar(query, candidates, top_k)
        else:
            # Fallback to numpy
            import numpy as np
            query_np = np.array(query)
            candidates_np = np.array(candidates)

            # Cosine similarity
            norms = np.linalg.norm(candidates_np, axis=1) * np.linalg.norm(query_np)
            similarities = np.dot(candidates_np, query_np) / norms

            indices = np.argsort(similarities)[::-1][:top_k]
            return [(int(i), float(similarities[i])) for i in indices]
