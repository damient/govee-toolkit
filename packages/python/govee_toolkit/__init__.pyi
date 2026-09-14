"""Type stubs for the package. The API lives in the extension module."""

from ._govee_toolkit import (
    CORE_VERSION as CORE_VERSION,
    MODES as MODES,
    Catalog as Catalog,
    CodecError as CodecError,
    Config as Config,
    ConfigError as ConfigError,
    Device as Device,
    DeviceHandle as DeviceHandle,
    DeviceStatus as DeviceStatus,
    EventStream as EventStream,
    Govee as Govee,
    GoveeError as GoveeError,
    Health as Health,
    Reply as Reply,
    SegmentStream as SegmentStream,
    Served as Served,
    StatusStream as StatusStream,
    TransportError as TransportError,
)

__version__: str

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
