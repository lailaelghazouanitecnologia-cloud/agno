"""
KKR Core Types
==============
Shared type definitions used across all backends.
"""

from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Any, Dict, List, Optional, Union
from uuid import UUID, uuid4


class Status(Enum):
    """Operation status."""
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"


class Priority(Enum):
    """Task priority levels."""
    LOW = 0
    NORMAL = 1
    HIGH = 2
    CRITICAL = 3


@dataclass
class Request:
    """Base request type for all operations."""
    id: UUID = field(default_factory=uuid4)
    timestamp: datetime = field(default_factory=datetime.utcnow)
    priority: Priority = Priority.NORMAL
    metadata: Dict[str, Any] = field(default_factory=dict)
    timeout_ms: Optional[int] = None


@dataclass
class Response:
    """Base response type for all operations."""
    request_id: UUID
    status: Status
    timestamp: datetime = field(default_factory=datetime.utcnow)
    data: Optional[Any] = None
    error: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)
    duration_ms: Optional[float] = None


@dataclass
class BatchRequest(Request):
    """Request for batch operations."""
    items: List[Any] = field(default_factory=list)
    batch_size: int = 100
    parallel: bool = True


@dataclass
class StreamRequest(Request):
    """Request for streaming operations."""
    chunk_size: int = 1024
    buffer_size: int = 10


@dataclass
class ComputeRequest(Request):
    """Request for compute operations (typically Rust backend)."""
    operation: str
    input_data: bytes = b""
    params: Dict[str, Any] = field(default_factory=dict)


@dataclass
class InferenceRequest(Request):
    """Request for inference operations."""
    model_id: str
    prompt: str
    max_tokens: int = 1024
    temperature: float = 0.7
    top_p: float = 1.0
    stop_sequences: List[str] = field(default_factory=list)
    stream: bool = False


@dataclass
class VectorSearchRequest(Request):
    """Request for vector search operations."""
    collection: str
    query_vector: List[float]
    top_k: int = 10
    filter: Optional[Dict[str, Any]] = None
    include_metadata: bool = True
    include_vectors: bool = False


@dataclass
class VectorUpsertRequest(Request):
    """Request for vector upsert operations."""
    collection: str
    vectors: List[List[float]]
    ids: List[str]
    metadata: Optional[List[Dict[str, Any]]] = None


# Result types for specific operations

@dataclass
class SearchResult:
    """Single search result."""
    id: str
    score: float
    vector: Optional[List[float]] = None
    metadata: Optional[Dict[str, Any]] = None


@dataclass
class VectorSearchResponse(Response):
    """Response for vector search."""
    results: List[SearchResult] = field(default_factory=list)
    total_count: int = 0


@dataclass
class InferenceResponse(Response):
    """Response for inference."""
    text: str = ""
    tokens_used: int = 0
    model_id: Optional[str] = None
    finish_reason: Optional[str] = None


@dataclass
class HealthStatus:
    """Health check response."""
    healthy: bool
    backend_type: str
    version: str
    uptime_seconds: float
    memory_usage_mb: float
    active_connections: int
    details: Dict[str, Any] = field(default_factory=dict)
