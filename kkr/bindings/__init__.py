"""
KKR Bindings
============
Bridge layer for seamless backend integration.
"""

from .bridge import BackendBridge, HybridBackend
from .router import BackendRouter

__all__ = ["BackendBridge", "HybridBackend", "BackendRouter"]
