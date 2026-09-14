"""Control Govee devices from your own network.

Every behavior is in the extension module `govee_toolkit._govee_toolkit`,
which binds the `govee-toolkit` core. This module only re-exports it.

The API is asyncio only. Modes are explicit: a command goes over an enabled
mode, or it fails and says so.
"""

from importlib import metadata as _metadata

from . import _govee_toolkit as _core
from ._govee_toolkit import (
    CORE_VERSION,
    MODES,
    Catalog,
    CodecError,
    Config,
    ConfigError,
    Device,
    DeviceHandle,
    DeviceStatus,
    EventStream,
    Govee,
    GoveeError,
    Health,
    Reply,
    SegmentStream,
    Served,
    StatusStream,
    TransportError,
)

try:
    __version__ = _metadata.version("govee-toolkit")
except _metadata.PackageNotFoundError:
    __version__ = _core.__version__

__all__ = [
    "CORE_VERSION",
    "MODES",
    "Catalog",
    "CodecError",
    "Config",
    "ConfigError",
    "Device",
    "DeviceHandle",
    "DeviceStatus",
    "EventStream",
    "Govee",
    "GoveeError",
    "Health",
    "Reply",
    "SegmentStream",
    "Served",
    "StatusStream",
    "TransportError",
    "__version__",
]
