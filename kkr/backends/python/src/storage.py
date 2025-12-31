"""
Storage Engine
==============
Storage operations for the Python backend.
"""

import asyncio
import time
from typing import Any, Dict, Optional
from dataclasses import dataclass, field

from kkr.core import BackendConfig, StorageProtocol


@dataclass
class StorageEngine(StorageProtocol):
    """
    In-memory storage engine with TTL support.

    For production, extend with:
    - Redis backend
    - PostgreSQL backend
    - S3/GCS backend
    """

    config: BackendConfig
    _cache: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    _connected: bool = False

    async def connect(self) -> None:
        """Establish connection to storage."""
        self._connected = True

    async def disconnect(self) -> None:
        """Close connection to storage."""
        self._connected = False
        self._cache.clear()

    async def get(self, key: str) -> Optional[Any]:
        """Get a value by key."""
        if key not in self._cache:
            return None

        entry = self._cache[key]

        # Check TTL
        if entry.get("expires_at") and time.time() > entry["expires_at"]:
            del self._cache[key]
            return None

        return entry["value"]

    async def set(self, key: str, value: Any, ttl: Optional[int] = None) -> None:
        """Set a value with optional TTL in seconds."""
        entry = {
            "value": value,
            "created_at": time.time(),
            "expires_at": time.time() + ttl if ttl else None,
        }
        self._cache[key] = entry

    async def delete(self, key: str) -> bool:
        """Delete a key, return True if deleted."""
        if key in self._cache:
            del self._cache[key]
            return True
        return False

    async def exists(self, key: str) -> bool:
        """Check if key exists (and not expired)."""
        value = await self.get(key)
        return value is not None

    async def clear(self) -> None:
        """Clear all cached data."""
        self._cache.clear()

    async def keys(self, pattern: str = "*") -> list:
        """Get all keys matching pattern."""
        import fnmatch
        return [k for k in self._cache.keys() if fnmatch.fnmatch(k, pattern)]

    async def size(self) -> int:
        """Get number of items in cache."""
        return len(self._cache)

    async def cleanup_expired(self) -> int:
        """Remove expired entries, return count removed."""
        now = time.time()
        expired = [
            k for k, v in self._cache.items()
            if v.get("expires_at") and now > v["expires_at"]
        ]
        for key in expired:
            del self._cache[key]
        return len(expired)
