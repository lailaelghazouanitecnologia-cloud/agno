"""
Backend Router
==============
Intelligent routing of operations to backends based on rules and metrics.
"""

import time
from typing import Any, Callable, Dict, List, Optional, Tuple
from dataclasses import dataclass, field
from enum import Enum

from kkr.core import BackendType, Request


class RoutingStrategy(Enum):
    """Strategy for routing requests."""
    ROUND_ROBIN = "round_robin"
    LEAST_LATENCY = "least_latency"
    CAPABILITY_BASED = "capability_based"
    LOAD_BALANCED = "load_balanced"


@dataclass
class RoutingRule:
    """A rule for routing requests to backends."""
    name: str
    condition: Callable[[Request], bool]
    target_backend: BackendType
    priority: int = 0


@dataclass
class BackendMetrics:
    """Metrics for a backend."""
    request_count: int = 0
    error_count: int = 0
    total_latency_ms: float = 0.0
    last_request_time: float = 0.0

    @property
    def avg_latency_ms(self) -> float:
        if self.request_count == 0:
            return 0.0
        return self.total_latency_ms / self.request_count

    @property
    def error_rate(self) -> float:
        if self.request_count == 0:
            return 0.0
        return self.error_count / self.request_count


@dataclass
class BackendRouter:
    """
    Routes requests to appropriate backends based on rules and metrics.
    """

    strategy: RoutingStrategy = RoutingStrategy.CAPABILITY_BASED
    rules: List[RoutingRule] = field(default_factory=list)
    metrics: Dict[BackendType, BackendMetrics] = field(default_factory=dict)

    # Backend capabilities
    _capabilities: Dict[BackendType, set] = field(default_factory=lambda: {
        BackendType.RUST: {
            "compression", "decompression", "vector_ops", "batch_processing",
            "parallel_compute", "binary_operations"
        },
        BackendType.PYTHON: {
            "inference", "embeddings", "ml_ops", "complex_logic",
            "api_integrations", "data_processing"
        },
    })

    def __post_init__(self):
        # Initialize metrics for all backend types
        for backend_type in BackendType:
            if backend_type not in self.metrics:
                self.metrics[backend_type] = BackendMetrics()

    def add_rule(
        self,
        name: str,
        condition: Callable[[Request], bool],
        target: BackendType,
        priority: int = 0,
    ) -> None:
        """Add a routing rule."""
        rule = RoutingRule(
            name=name,
            condition=condition,
            target_backend=target,
            priority=priority,
        )
        self.rules.append(rule)
        # Sort by priority (higher first)
        self.rules.sort(key=lambda r: r.priority, reverse=True)

    def remove_rule(self, name: str) -> bool:
        """Remove a routing rule by name."""
        for i, rule in enumerate(self.rules):
            if rule.name == name:
                del self.rules[i]
                return True
        return False

    def route(
        self,
        request: Request,
        available_backends: List[BackendType],
    ) -> BackendType:
        """Route a request to the best backend."""

        # First, check explicit rules
        for rule in self.rules:
            if rule.target_backend in available_backends and rule.condition(request):
                return rule.target_backend

        # Then, use strategy
        if self.strategy == RoutingStrategy.ROUND_ROBIN:
            return self._route_round_robin(available_backends)
        elif self.strategy == RoutingStrategy.LEAST_LATENCY:
            return self._route_least_latency(available_backends)
        elif self.strategy == RoutingStrategy.LOAD_BALANCED:
            return self._route_load_balanced(available_backends)
        else:  # CAPABILITY_BASED
            return self._route_capability_based(request, available_backends)

    def _route_round_robin(self, available: List[BackendType]) -> BackendType:
        """Simple round-robin routing."""
        # Find least recently used
        min_time = float('inf')
        selected = available[0]

        for backend in available:
            last_time = self.metrics[backend].last_request_time
            if last_time < min_time:
                min_time = last_time
                selected = backend

        return selected

    def _route_least_latency(self, available: List[BackendType]) -> BackendType:
        """Route to backend with lowest average latency."""
        min_latency = float('inf')
        selected = available[0]

        for backend in available:
            latency = self.metrics[backend].avg_latency_ms
            # Use a small default for backends with no data
            if latency == 0.0:
                latency = 1.0  # Assume fast
            if latency < min_latency:
                min_latency = latency
                selected = backend

        return selected

    def _route_load_balanced(self, available: List[BackendType]) -> BackendType:
        """Route based on current load (request count)."""
        min_count = float('inf')
        selected = available[0]

        for backend in available:
            count = self.metrics[backend].request_count
            if count < min_count:
                min_count = count
                selected = backend

        return selected

    def _route_capability_based(
        self,
        request: Request,
        available: List[BackendType],
    ) -> BackendType:
        """Route based on backend capabilities and request type."""
        request_type = type(request).__name__.lower()

        # Map request types to capabilities
        capability_map = {
            "computerequest": "batch_processing",
            "inferencerequest": "inference",
            "vectorsearchrequest": "vector_ops",
            "vectorupsertrequest": "vector_ops",
        }

        needed_capability = capability_map.get(request_type, "general")

        # Find backend with this capability
        for backend in available:
            if needed_capability in self._capabilities.get(backend, set()):
                return backend

        # Default to first available
        return available[0]

    def record_request(
        self,
        backend: BackendType,
        latency_ms: float,
        success: bool,
    ) -> None:
        """Record metrics for a completed request."""
        metrics = self.metrics[backend]
        metrics.request_count += 1
        metrics.total_latency_ms += latency_ms
        metrics.last_request_time = time.time()

        if not success:
            metrics.error_count += 1

    def get_stats(self) -> Dict[str, Any]:
        """Get routing statistics."""
        return {
            "strategy": self.strategy.value,
            "rules_count": len(self.rules),
            "backends": {
                bt.value: {
                    "request_count": m.request_count,
                    "error_rate": m.error_rate,
                    "avg_latency_ms": m.avg_latency_ms,
                }
                for bt, m in self.metrics.items()
            },
        }
