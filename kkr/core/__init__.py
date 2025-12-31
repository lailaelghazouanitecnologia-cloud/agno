"""
KKR Core
========
Core abstractions and protocols for multi-backend architecture.
"""

from .protocol import (
    BackendType,
    BackendConfig,
    BackendProtocol,
    StorageProtocol,
    ComputeProtocol,
    InferenceProtocol,
    VectorProtocol,
)

from .types import (
    Status,
    Priority,
    Request,
    Response,
    BatchRequest,
    StreamRequest,
    ComputeRequest,
    InferenceRequest,
    VectorSearchRequest,
    VectorUpsertRequest,
    SearchResult,
    VectorSearchResponse,
    InferenceResponse,
    HealthStatus,
)

__all__ = [
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
    "BatchRequest",
    "StreamRequest",
    "ComputeRequest",
    "InferenceRequest",
    "VectorSearchRequest",
    "VectorUpsertRequest",
    "SearchResult",
    "VectorSearchResponse",
    "InferenceResponse",
    "HealthStatus",
]
