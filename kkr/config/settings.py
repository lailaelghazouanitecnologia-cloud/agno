"""
Settings Management
===================
Centralized configuration for KKR backends.
"""

import os
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
from pathlib import Path

import yaml


@dataclass
class RustBackendSettings:
    """Settings for the Rust backend."""
    enabled: bool = True
    max_workers: int = 4
    timeout_ms: int = 30000
    enable_caching: bool = True
    enable_tracing: bool = False

    # Compression settings
    default_compression: str = "lz4"  # lz4 or zstd
    zstd_level: int = 3

    # Vector settings
    vector_normalize: bool = True


@dataclass
class PythonBackendSettings:
    """Settings for the Python backend."""
    enabled: bool = True
    max_workers: int = 4
    timeout_ms: int = 60000
    enable_caching: bool = True
    enable_tracing: bool = False

    # Inference settings
    default_model: str = "gpt-4"
    embedding_model: str = "text-embedding-ada-002"
    max_tokens: int = 4096
    temperature: float = 0.7

    # Storage settings
    cache_ttl: int = 3600


@dataclass
class HybridSettings:
    """Settings for hybrid mode."""
    enabled: bool = True

    # Routing preferences
    prefer_rust_for: List[str] = field(default_factory=lambda: [
        "compression", "vector_ops", "batch_processing"
    ])
    prefer_python_for: List[str] = field(default_factory=lambda: [
        "inference", "embeddings", "ml_ops"
    ])

    # Fallback behavior
    fallback_to_python: bool = True


@dataclass
class LoggingSettings:
    """Logging configuration."""
    level: str = "INFO"
    format: str = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
    file: Optional[str] = None
    enable_json: bool = False


@dataclass
class Settings:
    """Main settings container."""
    app_name: str = "kkr"
    version: str = "0.1.0"
    environment: str = "development"

    # Backend settings
    rust: RustBackendSettings = field(default_factory=RustBackendSettings)
    python: PythonBackendSettings = field(default_factory=PythonBackendSettings)
    hybrid: HybridSettings = field(default_factory=HybridSettings)

    # Logging
    logging: LoggingSettings = field(default_factory=LoggingSettings)

    # Feature flags
    features: Dict[str, bool] = field(default_factory=lambda: {
        "enable_metrics": True,
        "enable_tracing": False,
        "enable_profiling": False,
        "enable_caching": True,
    })

    def to_dict(self) -> Dict[str, Any]:
        """Convert settings to dictionary."""
        return {
            "app_name": self.app_name,
            "version": self.version,
            "environment": self.environment,
            "rust": {
                "enabled": self.rust.enabled,
                "max_workers": self.rust.max_workers,
                "timeout_ms": self.rust.timeout_ms,
                "enable_caching": self.rust.enable_caching,
                "enable_tracing": self.rust.enable_tracing,
                "default_compression": self.rust.default_compression,
                "zstd_level": self.rust.zstd_level,
            },
            "python": {
                "enabled": self.python.enabled,
                "max_workers": self.python.max_workers,
                "timeout_ms": self.python.timeout_ms,
                "enable_caching": self.python.enable_caching,
                "default_model": self.python.default_model,
                "embedding_model": self.python.embedding_model,
            },
            "hybrid": {
                "enabled": self.hybrid.enabled,
                "prefer_rust_for": self.hybrid.prefer_rust_for,
                "prefer_python_for": self.hybrid.prefer_python_for,
            },
            "logging": {
                "level": self.logging.level,
                "format": self.logging.format,
            },
            "features": self.features,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Settings":
        """Create settings from dictionary."""
        rust_data = data.get("rust", {})
        python_data = data.get("python", {})
        hybrid_data = data.get("hybrid", {})
        logging_data = data.get("logging", {})

        return cls(
            app_name=data.get("app_name", "kkr"),
            version=data.get("version", "0.1.0"),
            environment=data.get("environment", "development"),
            rust=RustBackendSettings(**rust_data) if rust_data else RustBackendSettings(),
            python=PythonBackendSettings(**python_data) if python_data else PythonBackendSettings(),
            hybrid=HybridSettings(**hybrid_data) if hybrid_data else HybridSettings(),
            logging=LoggingSettings(**logging_data) if logging_data else LoggingSettings(),
            features=data.get("features", {}),
        )


def load_settings(
    config_path: Optional[str] = None,
    env_prefix: str = "KKR_",
) -> Settings:
    """
    Load settings from file and environment variables.

    Priority (highest to lowest):
    1. Environment variables (KKR_*)
    2. Config file (YAML/JSON)
    3. Default values
    """
    settings_dict: Dict[str, Any] = {}

    # Load from config file
    if config_path:
        path = Path(config_path)
        if path.exists():
            with open(path) as f:
                if path.suffix in [".yaml", ".yml"]:
                    settings_dict = yaml.safe_load(f) or {}
                elif path.suffix == ".json":
                    import json
                    settings_dict = json.load(f)

    # Try default paths
    if not settings_dict:
        for default_path in ["kkr.yaml", "kkr.yml", "config/kkr.yaml"]:
            path = Path(default_path)
            if path.exists():
                with open(path) as f:
                    settings_dict = yaml.safe_load(f) or {}
                break

    # Override with environment variables
    env_overrides = _get_env_overrides(env_prefix)
    _deep_merge(settings_dict, env_overrides)

    return Settings.from_dict(settings_dict)


def _get_env_overrides(prefix: str) -> Dict[str, Any]:
    """Extract settings from environment variables."""
    overrides: Dict[str, Any] = {}

    env_mapping = {
        f"{prefix}RUST_ENABLED": ("rust", "enabled"),
        f"{prefix}RUST_MAX_WORKERS": ("rust", "max_workers"),
        f"{prefix}RUST_TIMEOUT_MS": ("rust", "timeout_ms"),
        f"{prefix}PYTHON_ENABLED": ("python", "enabled"),
        f"{prefix}PYTHON_MAX_WORKERS": ("python", "max_workers"),
        f"{prefix}PYTHON_TIMEOUT_MS": ("python", "timeout_ms"),
        f"{prefix}PYTHON_DEFAULT_MODEL": ("python", "default_model"),
        f"{prefix}HYBRID_ENABLED": ("hybrid", "enabled"),
        f"{prefix}LOG_LEVEL": ("logging", "level"),
        f"{prefix}ENVIRONMENT": ("environment",),
    }

    for env_var, path in env_mapping.items():
        value = os.environ.get(env_var)
        if value is not None:
            # Convert value types
            if value.lower() in ("true", "false"):
                value = value.lower() == "true"
            elif value.isdigit():
                value = int(value)

            # Set nested value
            current = overrides
            for key in path[:-1]:
                current = current.setdefault(key, {})
            current[path[-1]] = value

    return overrides


def _deep_merge(base: Dict, override: Dict) -> None:
    """Deep merge override into base."""
    for key, value in override.items():
        if key in base and isinstance(base[key], dict) and isinstance(value, dict):
            _deep_merge(base[key], value)
        else:
            base[key] = value
