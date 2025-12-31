"""
KKR Core Protocols
==================
Abstract interfaces that all backends must implement.
This provides a unified API regardless of the underlying backend (Rust, Python, etc.)
"""

from abc import ABC, abstractmethod
from typing import Any, AsyncIterator, Dict, Generic, List, Optional, TypeVar, Union
from dataclasses import dataclass
from enum import Enum


class BackendType(Enum):
    """Supported backend types."""
    RUST = "rust"
    PYTHON = "python"
    HYBRID = "hybrid"  # Rust for performance-critical, Python for flexibility


@dataclass
class BackendConfig:
    """Configuration for a backend."""
    backend_type: BackendType
    name: str
    version: str
    settings: Dict[str, Any]

    # Performance settings
    max_workers: int = 4
    timeout_ms: int = 30000
    use_async: bool = True

    # Feature flags
    enable_caching: bool = True
    enable_tracing: bool = False
    enable_metrics: bool = True


T = TypeVar('T')
R = TypeVar('R')


class BackendProtocol(ABC, Generic[T, R]):
    """
    Base protocol that all backends must implement.

    This abstraction allows seamless switching between:
    - Rust backend (high performance, systems-level operations)
    - Python backend (flexibility, rapid prototyping, ML integrations)
    - Hybrid mode (best of both worlds)
    """

    @property
    @abstractmethod
    def backend_type(self) -> BackendType:
        """Return the type of this backend."""
        ...

    @property
    @abstractmethod
    def is_ready(self) -> bool:
        """Check if the backend is initialized and ready."""
        ...

    @abstractmethod
    async def initialize(self, config: BackendConfig) -> None:
        """Initialize the backend with configuration."""
        ...

    @abstractmethod
    async def shutdown(self) -> None:
        """Gracefully shutdown the backend."""
        ...

    @abstractmethod
    async def execute(self, request: T) -> R:
        """Execute a request and return the result."""
        ...

    @abstractmethod
    async def execute_stream(self, request: T) -> AsyncIterator[R]:
        """Execute a request and stream results."""
        ...

    @abstractmethod
    async def health_check(self) -> Dict[str, Any]:
        """Return health status of the backend."""
        ...


class StorageProtocol(ABC):
    """Protocol for storage backends (databases, caches, etc.)."""

    @abstractmethod
    async def connect(self) -> None:
        """Establish connection to storage."""
        ...

    @abstractmethod
    async def disconnect(self) -> None:
        """Close connection to storage."""
        ...

    @abstractmethod
    async def get(self, key: str) -> Optional[Any]:
        """Get a value by key."""
        ...

    @abstractmethod
    async def set(self, key: str, value: Any, ttl: Optional[int] = None) -> None:
        """Set a value with optional TTL in seconds."""
        ...

    @abstractmethod
    async def delete(self, key: str) -> bool:
        """Delete a key, return True if deleted."""
        ...

    @abstractmethod
    async def exists(self, key: str) -> bool:
        """Check if key exists."""
        ...


class ComputeProtocol(ABC):
    """Protocol for compute-intensive operations (typically Rust)."""

    @abstractmethod
    async def process_batch(self, items: List[Any]) -> List[Any]:
        """Process a batch of items efficiently."""
        ...

    @abstractmethod
    async def transform(self, data: bytes) -> bytes:
        """Transform binary data."""
        ...

    @abstractmethod
    async def compress(self, data: bytes) -> bytes:
        """Compress data."""
        ...

    @abstractmethod
    async def decompress(self, data: bytes) -> bytes:
        """Decompress data."""
        ...


class InferenceProtocol(ABC):
    """Protocol for ML/AI inference (typically Python with Rust acceleration)."""

    @abstractmethod
    async def embed(self, texts: List[str]) -> List[List[float]]:
        """Generate embeddings for texts."""
        ...

    @abstractmethod
    async def generate(
        self,
        prompt: str,
        max_tokens: int = 1024,
        temperature: float = 0.7,
        **kwargs
    ) -> str:
        """Generate text from a prompt."""
        ...

    @abstractmethod
    async def generate_stream(
        self,
        prompt: str,
        max_tokens: int = 1024,
        temperature: float = 0.7,
        **kwargs
    ) -> AsyncIterator[str]:
        """Stream generated text."""
        ...


class VectorProtocol(ABC):
    """Protocol for vector operations (search, similarity, etc.)."""

    @abstractmethod
    async def upsert(
        self,
        collection: str,
        vectors: List[List[float]],
        ids: List[str],
        metadata: Optional[List[Dict[str, Any]]] = None
    ) -> None:
        """Insert or update vectors."""
        ...

    @abstractmethod
    async def search(
        self,
        collection: str,
        query_vector: List[float],
        top_k: int = 10,
        filter: Optional[Dict[str, Any]] = None
    ) -> List[Dict[str, Any]]:
        """Search for similar vectors."""
        ...

    @abstractmethod
    async def delete(self, collection: str, ids: List[str]) -> None:
        """Delete vectors by IDs."""
        ...
