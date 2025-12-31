"""
Python Backend Implementation
=============================
Full-featured Python backend for flexibility and ML integrations.
"""

import asyncio
import time
from typing import Any, AsyncIterator, Dict, List, Optional
from dataclasses import dataclass, field

from kkr.core import (
    BackendProtocol,
    BackendType,
    BackendConfig,
    Request,
    Response,
    Status,
    HealthStatus,
)


@dataclass
class PythonBackend(BackendProtocol[Request, Response]):
    """
    Python backend implementation.

    Best suited for:
    - ML/AI operations (model inference, embeddings)
    - Rapid prototyping
    - Complex business logic
    - Integration with Python ecosystem (numpy, pandas, etc.)
    """

    config: Optional[BackendConfig] = None
    _start_time: float = field(default_factory=time.time)
    _is_initialized: bool = False

    @property
    def backend_type(self) -> BackendType:
        return BackendType.PYTHON

    @property
    def is_ready(self) -> bool:
        return self._is_initialized

    async def initialize(self, config: BackendConfig) -> None:
        """Initialize the Python backend."""
        self.config = config
        self._start_time = time.time()
        self._is_initialized = True

        # Initialize sub-engines
        from .inference import InferenceEngine
        from .storage import StorageEngine

        self._inference = InferenceEngine(config)
        self._storage = StorageEngine(config)

    async def shutdown(self) -> None:
        """Shutdown the backend gracefully."""
        self._is_initialized = False
        # Cleanup resources

    async def execute(self, request: Request) -> Response:
        """Execute a request."""
        start = time.time()

        try:
            # Process based on request type
            result = await self._process_request(request)

            return Response(
                request_id=request.id,
                status=Status.COMPLETED,
                data=result,
                duration_ms=(time.time() - start) * 1000,
            )
        except Exception as e:
            return Response(
                request_id=request.id,
                status=Status.FAILED,
                error=str(e),
                duration_ms=(time.time() - start) * 1000,
            )

    async def execute_stream(self, request: Request) -> AsyncIterator[Response]:
        """Execute a request with streaming response."""
        start = time.time()

        try:
            async for chunk in self._process_stream(request):
                yield Response(
                    request_id=request.id,
                    status=Status.RUNNING,
                    data=chunk,
                    duration_ms=(time.time() - start) * 1000,
                )

            yield Response(
                request_id=request.id,
                status=Status.COMPLETED,
                duration_ms=(time.time() - start) * 1000,
            )
        except Exception as e:
            yield Response(
                request_id=request.id,
                status=Status.FAILED,
                error=str(e),
                duration_ms=(time.time() - start) * 1000,
            )

    async def health_check(self) -> Dict[str, Any]:
        """Return health status."""
        import psutil
        import os

        process = psutil.Process(os.getpid())
        memory_mb = process.memory_info().rss / (1024 * 1024)

        return HealthStatus(
            healthy=self._is_initialized,
            backend_type="python",
            version="0.1.0",
            uptime_seconds=time.time() - self._start_time,
            memory_usage_mb=memory_mb,
            active_connections=0,
            details={
                "python_version": __import__("sys").version,
                "asyncio_tasks": len(asyncio.all_tasks()),
            },
        ).__dict__

    async def _process_request(self, request: Request) -> Any:
        """Process a generic request."""
        # Route to appropriate handler based on request type
        return {"processed": True, "request_id": str(request.id)}

    async def _process_stream(self, request: Request) -> AsyncIterator[Any]:
        """Process a streaming request."""
        for i in range(10):
            await asyncio.sleep(0.1)
            yield {"chunk": i, "request_id": str(request.id)}


# Convenience function for quick backend creation
def create_python_backend(config: Optional[Dict[str, Any]] = None) -> PythonBackend:
    """Create a configured Python backend."""
    backend = PythonBackend()

    if config:
        backend_config = BackendConfig(
            backend_type=BackendType.PYTHON,
            name="python-backend",
            version="0.1.0",
            settings=config,
            max_workers=config.get("max_workers", 4),
            timeout_ms=config.get("timeout_ms", 30000),
        )
    else:
        backend_config = BackendConfig(
            backend_type=BackendType.PYTHON,
            name="python-backend",
            version="0.1.0",
            settings={},
        )

    # Note: initialize() is async, caller should await it
    return backend
