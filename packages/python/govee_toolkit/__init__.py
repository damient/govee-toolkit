"""Control Govee devices from your own network.

Every behavior is in the extension module `govee_toolkit._govee_toolkit`,
which binds the `govee-toolkit` core. This module only re-exports it.

The API is asyncio only. Modes are explicit: a command goes over an enabled
mode, or it fails and says so.
"""

from typing import TYPE_CHECKING

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
    GroupHandle,
    Health,
    Outcome,
    Reply,
    SegmentStream,
    Served,
    StatusStream,
    TransportError,
    WalkReport,
)

__version__: str
"""The version of the installed distribution."""

# The module reads `__version__` off the distribution metadata on the first
# read, and not on the import: the read opens a file on the disk.
#
# A type checker must not see the `__getattr__`, or it answers `str` for every
# name the module does not have. `__version__` above is what it reads instead.
if not TYPE_CHECKING:

    def __getattr__(name):
        if name != "__version__":
            raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
        from importlib import metadata

        try:
            version = metadata.version("govee-toolkit")
        except metadata.PackageNotFoundError:
            from . import _govee_toolkit

            version = _govee_toolkit.__version__
        globals()["__version__"] = version
        return version


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
    "GroupHandle",
    "Health",
    "Outcome",
    "Reply",
    "SegmentStream",
    "Served",
    "StatusStream",
    "TransportError",
    "WalkReport",
    "__version__",
]
