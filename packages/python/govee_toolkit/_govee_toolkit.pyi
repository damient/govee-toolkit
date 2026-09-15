"""Type stubs for the extension module."""

import os
from collections.abc import Sequence
from types import TracebackType
from typing import Any, Self, final

from ._types import Arg, Color, Rate, Resolution

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

__version__: str
CORE_VERSION: str
MODES: tuple[str, ...]

class GoveeError(Exception):
    """Anything that went wrong between a call and the bytes on the wire."""

    code: str
    """The stable identifier the core gives the failure."""

class CodecError(GoveeError):
    """An unknown SKU, an unknown command, or an argument out of range."""

class TransportError(GoveeError):
    """A mode failed to carry the command, or nothing answered in time."""

class ConfigError(GoveeError):
    """The configuration could not be read, or it enables what cannot work."""

@final
class Health:
    """A device's health in one mode."""

    @property
    def state(self) -> str:
        """`"ok"`, `"degraded"` or `"down"`."""
    @property
    def failures(self) -> int:
        """Consecutive unanswered verifications."""
    @property
    def available(self) -> bool:
        """Whether a command would be sent right now."""

@final
class Device:
    """A device the SDK knows about."""

    @property
    def id(self) -> str:
        """The MAC it reports, uppercased."""
    @property
    def sku(self) -> str:
        """The SKU it is encoded under."""
    @property
    def name(self) -> str | None:
        """The name the configuration gives it."""
    @property
    def modes(self) -> list[str]:
        """The enabled modes, in preference order."""
    @property
    def health(self) -> dict[str, Health]:
        """The health per enabled mode."""

@final
class Served:
    """A command that was served."""

    @property
    def id(self) -> str:
        """The device it went to."""
    @property
    def mode(self) -> str:
        """The mode that served it."""
    @property
    def command(self) -> str:
        """The device file entry that was sent."""
    @property
    def cmd(self) -> str:
        """The name the wire carries, where it carries one."""

@final
class DeviceStatus:
    """What a device reported about itself. No firmware fills every field."""

    @property
    def id(self) -> str:
        """Which device answered."""
    @property
    def on(self) -> bool | None:
        """Whether it is on."""
    @property
    def brightness(self) -> int | None:
        """The level it reports, unmodified."""
    @property
    def color(self) -> Color | None:
        """The color, as three channels."""
    @property
    def color_temp_kelvin(self) -> int | None:
        """`0` means color mode."""
    @property
    def raw(self) -> dict[str, Any]:
        """The whole reply, every field included."""
    @property
    def is_white(self) -> bool:
        """Whether the device is in white mode."""

@final
class Reply:
    """What one command's reply layouts captured."""

    @property
    def id(self) -> str:
        """Which device answered."""
    @property
    def fields(self) -> dict[str, Any]:
        """Every field, by its device file name."""

@final
class Config:
    """The configuration in force. Every field is read-only."""

    def __init__(self) -> None:
        """Build the default configuration: `lan` alone."""
    @staticmethod
    def load() -> Config:
        """Read the file. A missing file is the default."""
    @staticmethod
    def load_from(path: str | os.PathLike[str]) -> Config:
        """Read the configuration from one path."""

    @property
    def default_modes(self) -> list[str]:
        """The modes a device without an entry gets."""
    @property
    def devices(self) -> list[str]:
        """The identity of every device the file names."""
    @property
    def stream_fallback_hz(self) -> float:
        """The rate a stream sends at when the device file measured none."""
    def to_dict(self) -> dict[str, Any]:
        """The whole configuration. It carries no credential."""

@final
class Catalog:
    """The device files the core carries."""

    @staticmethod
    def embedded() -> Catalog:
        """The catalog built into this build."""
    def skus(self) -> list[str]:
        """Every SKU that resolves, verified aliases included."""
    def has(self, sku: str) -> bool:
        """Whether the catalog holds this SKU."""
    def device(self, sku: str) -> dict[str, Any]:
        """The device file, resolved."""
    def __len__(self) -> int:
        """How many device files the catalog holds."""

@final
class SegmentStream:
    """An open segment stream. Close it, or use it as a context manager."""

    @property
    def zones(self) -> int:
        """How many zones the stream writes."""
    @property
    def rate_hz(self) -> float:
        """The rate the stream sends at."""
    @property
    def frames_sent(self) -> int:
        """How many frames reached the wire."""
    @property
    def frames_superseded(self) -> int:
        """How many frames a later frame replaced."""
    @property
    def error(self) -> str | None:
        """What stopped the stream, if anything did."""
    def set_all(self, colors: Sequence[Color]) -> None:
        """Write one color per zone."""
    def set_zone(self, index: int, color: Color) -> None:
        """Write one zone."""
    def fill(self, color: Color) -> None:
        """Write one color to every zone."""
    def clear(self) -> None:
        """Write black to every zone."""
    def buffer(self) -> list[Color]:
        """The colors the next frame carries."""
    async def close(self) -> None:
        """Stop the stream and release the transport."""
    async def __aenter__(self) -> Self: ...
    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        tb: TracebackType | None,
    ) -> None: ...

@final
class EventStream:
    """An async iterator over the events the SDK reports.

    Every event is a dict, and `event` names which one it is. The records are
    the core's own, so `govee watch --json` prints the same ones.
    """

    def __aiter__(self) -> EventStream: ...
    async def __anext__(self) -> dict[str, Any]: ...

@final
class StatusStream:
    """An async iterator over one device's status, as answers arrive."""

    def __aiter__(self) -> StatusStream: ...
    async def __anext__(self) -> DeviceStatus: ...

@final
class DeviceHandle:
    """One device, and the commands it serves."""

    @property
    def id(self) -> str:
        """The identity the handle names."""
    @property
    def modes(self) -> list[str]:
        """The enabled modes, in preference order."""
    def health(self, mode: str) -> Health | None:
        """Its health in one mode. `None` when that mode never heard from it."""

    def serving_mode(self) -> str:
        """The mode a command would go over."""
    def watch_status(self) -> StatusStream | None:
        """Watch its status as answers arrive, over the mode that serves it.

        `None` when no enabled mode can serve it, or when that transport has
        heard nothing. It requests nothing of its own.
        """

    def spec(self) -> dict[str, Any]:
        """What the device file declares."""
    def describe(self) -> dict[str, Any]:
        """The same, as the record `govee describe` prints."""
    async def ensure_known(self) -> str:
        """Scan if no mode knows the device, then name the mode it answers on."""

    async def send(self, command: str, **args: Arg) -> Served:
        """Send one device file command.

        Every value is read under the type the entry declares for that
        argument, so a list of whole numbers is zone indices, byte values or
        one color as the file says.
        """

    async def read(self, command: str, **args: Arg) -> Reply:
        """Send one device file command and read what it answers."""

    async def status(self) -> DeviceStatus:
        """Ask the device for its state."""
    def last_status(self) -> DeviceStatus | None:
        """The last status read."""
    async def power(self, on: bool) -> Served:
        """Turn the device on or off."""
    async def brightness(self, level: int) -> Served:
        """Set the brightness. Out of range is an error, never a clamp."""

    async def color(self, rgb: Color) -> Served:
        """Set one color on the device."""
    async def color_temp(self, kelvin: int) -> Served:
        """Set the white temperature, in kelvin. It ends color mode."""

    async def music(
        self,
        effect: int,
        sensitivity: int | None = None,
        soft: bool | None = None,
        color: Color | None = None,
    ) -> Served:
        """Play an effect the device renders from its own microphone.

        The effect identifiers are the mode's own. `color` imposes a color,
        and `None` leaves the colors to the firmware. `None` takes the core's
        default for the other two, which every surface takes.
        """

    async def segment(
        self,
        colors: Sequence[Color],
        zones: Sequence[int] | None = None,
        resolution: Resolution | None = None,
        gradient: bool = False,
    ) -> Served:
        """Paint the segments once.

        One color fills every zone, and a list states them all. A zone list
        takes one color. `resolution` is `"app"` when it is `None`.
        """

    async def gradient(self, on: bool) -> Served:
        """Turn the gradient between zones on or off."""

    async def provision_wifi(
        self,
        network: str,
        password: str,
        utc_offset_hours: int = 0,
        utc_offset_minutes: int = 0,
    ) -> str:
        """Put the device on a Wi-Fi network over `ble`.

        The network must be 2.4 GHz, and the password travels in plaintext.
        Answers `"accepted"` where the device acknowledged the transfer, and
        `"sent"` where its device file declares no acknowledgement.
        """

    async def open_stream(
        self,
        resolution: Resolution | None = None,
        rate: Rate | None = None,
        gradient: bool = False,
    ) -> SegmentStream:
        """Open the raw segment channel and paint it frame by frame.

        Power the device on first: arming a dark strip paints nothing. The
        device goes back to the color it showed before once the stream closes.

        `resolution` is `"app"` when it is `None`, and `rate` is `"measured"`.
        """

@final
class Govee:
    """The SDK. Start one and keep it."""

    @staticmethod
    async def start(
        config: Config | None = None, catalog: Catalog | None = None
    ) -> Govee:
        """Start the SDK.

        Without a configuration, it reads the file. Without a catalog, it
        reads the device files the wheel carries.
        """

    async def scan(self) -> list[Device]:
        """Scan every mode for devices."""
    async def scan_on(self, modes: Sequence[str]) -> list[Device]:
        """Run a discovery scan on the modes named."""

    def devices(self) -> list[Device]:
        """Every device known, across every mode."""
    def modes(self) -> list[str]:
        """The modes this build carries a transport for."""
    def problems(self) -> list[str]:
        """What is wrong with the configuration."""
    @property
    def config(self) -> Config:
        """The configuration in force."""
    @property
    def catalog(self) -> Catalog:
        """The device files this build carries."""
    def device(self, id: str) -> DeviceHandle:
        """A handle on one device."""
    def events(self) -> EventStream:
        """An async iterator over the SDK's events."""
    async def close(self) -> None:
        """Release what every transport holds. Call it before the program ends."""
