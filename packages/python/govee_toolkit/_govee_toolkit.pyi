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

__version__: str
CORE_VERSION: str
MODES: tuple[str, ...]

class GoveeError(Exception):
    """Anything that went wrong between a call and the bytes on the wire."""

    code: str
    """The stable identifier the core gives the failure."""

class CodecError(GoveeError):
    """An unknown SKU, an unknown command, or an argument out of range. Nothing was
    sent.
    """

class TransportError(GoveeError):
    """A mode failed to carry the command, or nothing answered in time."""

class ConfigError(GoveeError):
    """The configuration could not be read, or it enables something that cannot work."""

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
        """The name the configuration gives it, if any."""
    @property
    def groups(self) -> list[str]:
        """The groups the configuration puts it in."""
    @property
    def modes(self) -> list[str]:
        """The enabled modes, in preference order."""
    @property
    def health(self) -> dict[str, Health]:
        """Its health per enabled mode. A mode is absent when no transport has heard
        from it.
        """

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
        """The level it reports. A percentage on every unit seen so far, and not
        normalized here.
        """
    @property
    def color(self) -> Color | None:
        """The color, as three channels. Reset to `(0, 0, 0)` in white mode."""
    @property
    def color_temp_kelvin(self) -> int | None:
        """The white temperature. `0` means the device is in color mode."""
    @property
    def raw(self) -> dict[str, Any]:
        """The whole reply, with every field the SDK does not model."""
    @property
    def is_white(self) -> bool:
        """Whether the device is in white mode. Mutually exclusive with color."""

@final
class Reply:
    """What one command's `reply:` layouts captured."""

    @property
    def id(self) -> str:
        """Which device answered."""
    @property
    def fields(self) -> dict[str, Any]:
        """Every field the exchanges captured, by the name the device file gives it."""

@final
class Config:
    """The configuration in force. Every field is read-only: the SDK reads the
    configuration once, at startup.
    """

    def __init__(self) -> None:
        """The default configuration: `lan` alone, and no device entry."""
    @staticmethod
    def load() -> Config:
        """Read `$XDG_CONFIG_HOME/govee-toolkit/config.yaml`.

        A missing file is the default configuration, not an error. A file that does not
        parse raises `ConfigError`.
        """
    @staticmethod
    def load_from(path: str | os.PathLike[str]) -> Config:
        """Read the configuration from one path."""

    @property
    def default_modes(self) -> list[str]:
        """The modes enabled for a device with no entry of its own."""
    @property
    def devices(self) -> list[str]:
        """The identity of every device the file names."""
    @property
    def stream_fallback_hz(self) -> float:
        """The rate a stream sends at when the device file measured none."""
    def to_dict(self) -> dict[str, Any]:
        """The whole configuration, as the core serializes it. It carries no credential:
        a key comes from the environment, never from the file.
        """

@final
class Catalog:
    """Every device the build knows. Reads no hardware."""

    @staticmethod
    def embedded() -> Catalog:
        """The catalog compiled into this build."""
    def skus(self) -> list[str]:
        """Every SKU that resolves, verified aliases included."""
    def has(self, sku: str) -> bool:
        """Whether a SKU resolves."""
    def device(self, sku: str) -> dict[str, Any]:
        """One device file, with every `include:` and every override applied.

        Raises `CodecError` with the code `unknown_sku` when nothing declares it.
        """
    def __len__(self) -> int:
        """How many device files the catalog holds."""

@final
class SegmentStream:
    """An open segment channel. The writers never block: each one replaces what the next
    frame carries.

    Close it, or leave it with `async with`. A stream that is dropped disarms the
    channel as well, and reports no failure.
    """

    @property
    def zones(self) -> int:
        """How many zones the frames carry. The firmware reads the count off the frame,
        so it never changes while the stream is open.
        """
    @property
    def rate_hz(self) -> float:
        """How fast frames go out, in hertz."""
    @property
    def frames_sent(self) -> int:
        """How many frames reached the wire."""
    @property
    def frames_superseded(self) -> int:
        """How many frames a later write replaced before they left."""
    @property
    def error(self) -> str | None:
        """What the emitting task failed with, if it failed. The stream stops sending,
        and the writers keep answering.
        """
    def set_all(self, colors: Sequence[Color]) -> None:
        """State every zone. The count must be the stream's own."""
    def set_zone(self, index: int, color: Color) -> None:
        """State one zone, by its zero-based index."""
    def fill(self, color: Color) -> None:
        """Put one color in every zone."""
    def clear(self) -> None:
        """Put black in every zone. The channel stays armed."""
    def buffer(self) -> list[Color]:
        """What the next frame carries."""
    async def close(self) -> None:
        """Disarm the channel and wait for the last frame to leave."""
    async def __aenter__(self) -> Self: ...
    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        tb: TracebackType | None,
    ) -> None: ...

@final
class EventStream:
    """The events of one SDK. Iterate it with `async for`.

    Every event is a dict, and `event` says which one it is. The records are the core's
    own, so `govee watch --json` prints the same ones. A subscription that falls behind
    reports `{"event": "lagged", "missed": n}` rather than hide the gap.
    """

    def __aiter__(self) -> EventStream: ...
    async def __anext__(self) -> dict[str, Any]: ...

@final
class StatusStream:
    """One device's status, as answers arrive. Iterate it with `async for`.

    It requests nothing: it reports the answers a status request or a verification
    already brought back.
    """

    def __aiter__(self) -> StatusStream: ...
    async def __anext__(self) -> DeviceStatus: ...

@final
class DeviceHandle:
    """A handle on one identity. It holds no state of its own: every answer comes from
    the SDK it was made by.
    """

    @property
    def id(self) -> str:
        """The MAC the device reports, uppercased."""
    @property
    def modes(self) -> list[str]:
        """The modes enabled for it, in preference order."""
    def health(self, mode: str) -> Health | None:
        """Its health in one mode. `None` when the transport that serves that mode has
        never heard from it.
        """

    def serving_mode(self) -> str:
        """The mode a command sent now would go over. Read from recorded state, so the
        answer can change before the next call.
        """
    def watch_status(self) -> StatusStream | None:
        """Watch its status as answers arrive, over the mode that would serve a command
        now. `None` when no enabled mode can, or when that transport has heard nothing.
        """

    def spec(self) -> dict[str, Any]:
        """What `devices/<SKU>.yaml` declares for it. Reads no hardware."""
    def describe(self) -> dict[str, Any]:
        """What `devices/<SKU>.yaml` declares, as the record `govee describe` prints:
        the modes in one place, the commands under the mode that carries them, and each
        argument's type, role and bound.
        """
    async def ensure_known(self) -> str:
        """Scan for the device if no mode knows it yet, then answer the mode a command
        would go over.

        The scan covers every enabled mode, whatever this handle is pinned to.
        """

    async def send(self, command: str, **args: Arg) -> Served:
        """Send a command, named as the device file names it.

        The arguments are the ones the entry declares, and each value is read under the
        type the entry declares for it. A value outside the declared range raises
        `CodecError`, and nothing is sent.
        """

    async def read(self, command: str, **args: Arg) -> Reply:
        """Run a command's exchanges and return what its `reply:` layouts captured."""

    async def status(self) -> DeviceStatus:
        """Ask the device for its state and wait for the answer."""
    def last_status(self) -> DeviceStatus | None:
        """The last status heard, without asking for a new one."""
    async def power(self, on: bool) -> Served:
        """Turn the device on or off."""
    async def brightness(self, level: int) -> Served:
        """Set the level, in the unit the device file declares. A level outside that
        range is an error, never a clamp.
        """

    async def color(self, rgb: Color) -> Served:
        """Set one color, as three channels."""
    async def identify(
        self, color: Color | None = None, full_brightness: bool | None = None
    ) -> None:
        """Power the device on and paint one color, so a person sees which fixture this
        identity drives.

        One pass: the device stays on and lit. `Govee.identify()` runs the whole walk,
        and powers the devices off at the end. The look the device held is lost.

        `None` takes the core's defaults: green, and the top of the brightness range the
        device file declares.
        """

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

        The identifiers are the mode's own: one the entry accepts is not one the device
        renders. `color` imposes a color, and `None` leaves the colors to the firmware.

        `None` takes the core's default for `sensitivity` and for `soft`.
        """

    async def segment(
        self,
        colors: Sequence[Color],
        zones: Sequence[int] | None = None,
        resolution: Resolution | None = None,
        gradient: bool = False,
    ) -> Served:
        """Paint the segments once.

        One color fills every zone, and a list states them all. A zone list takes one
        color. `resolution` takes `"app"` when it is `None`.
        """

    async def gradient(self, on: bool) -> Served:
        """Ask the firmware to interpolate between zones, and to wrap from the last zone
        back to the first.
        """

    async def provision_wifi(
        self,
        network: str,
        password: str,
        utc_offset_hours: int = 0,
        utc_offset_minutes: int = 0,
    ) -> str:
        """Put the device on a Wi-Fi network over `ble`.

        The device must be in Bluetooth range and closed in the phone controller. The
        password travels in plaintext: anything in Bluetooth range during the transfer
        reads it. The network must be 2.4 GHz.

        Answers `"accepted"` where the device acknowledged the transfer, and `"sent"`
        where its device file declares no acknowledgement.
        """

    async def open_stream(
        self,
        resolution: Resolution | None = None,
        rate: Rate | None = None,
        gradient: bool = False,
    ) -> SegmentStream:
        """Open the raw segment channel and paint it frame by frame.

        Power the device on first: arming a dark strip paints nothing. The channel holds
        the colors only while it is armed, and the device goes back to the color it
        showed before once the stream closes.

        `resolution` takes `"app"` when it is `None`, and `rate` takes `"measured"`.
        """

@final
class Outcome:
    """What one member answered."""

    @property
    def id(self) -> str:
        """The member."""
    @property
    def ok(self) -> bool:
        """Whether the call on this member succeeded."""
    @property
    def mode(self) -> str | None:
        """The mode that served the call. `None` where it failed."""
    @property
    def served(self) -> Served | None:
        """The command served. `None` on a failure and for `ensure_known()`."""
    @property
    def error(self) -> GoveeError | None:
        """What the call on one device raises. `None` where it succeeded."""

@final
class WalkReport:
    """What one identify walk covered, and what it failed at."""

    @property
    def lit(self) -> list[str]:
        """The devices the walk covered, in the order it lit them. Empty where the
        targets named none.
        """
    @property
    def failed(self) -> list[str]:
        """The devices that refused the opening blackout or the pass."""
    @property
    def stayed(self) -> list[str]:
        """The devices that refused the closing blackout, and hold the color."""
    @property
    def ok(self) -> bool:
        """Whether every device took every step."""

@final
class GroupHandle:
    """A handle on the members of a group. It holds no state of its own."""

    @property
    def members(self) -> list[str]:
        """The identities of the members, in the order of every outcome list."""
    async def ensure_known(self) -> list[Outcome]:
        """Scan for every member that no mode knows. Each outcome carries its mode."""
    async def power(self, on: bool) -> list[Outcome]:
        """Turn every member on or off."""
    async def brightness(self, level: int) -> list[Outcome]:
        """Set the level on every member, against its own range."""
    async def color(self, rgb: Color) -> list[Outcome]:
        """Set one color on every member."""
    async def color_temp(self, kelvin: int) -> list[Outcome]:
        """Set the white temperature on every member, in kelvin."""
    async def music(
        self,
        effect: int,
        sensitivity: int | None = None,
        soft: bool | None = None,
        color: Color | None = None,
    ) -> list[Outcome]:
        """`DeviceHandle.music()` on every member."""
    async def segment(
        self,
        colors: Sequence[Color],
        zones: Sequence[int] | None = None,
        resolution: Resolution | None = None,
        gradient: bool = False,
    ) -> list[Outcome]:
        """`DeviceHandle.segment()` on every member, against its own zones."""
    async def gradient(self, on: bool) -> list[Outcome]:
        """Set the interpolation between zones on every member."""

@final
class Govee:
    """The SDK. Start one and keep it: it holds the catalog, the configuration and one
    transport per mode.
    """

    @staticmethod
    async def start(
        config: Config | None = None, catalog: Catalog | None = None
    ) -> Govee:
        """Start the SDK. Without a configuration, it reads the file. Without a catalog,
        it reads the one the wheel carries.
        """

    async def scan(self) -> list[Device]:
        """Run a discovery scan on every mode and return what answered.

        The scans run at the same time, so the call takes the longest window and not
        their sum. Nothing on the send path calls this.
        """
    async def scan_on(self, modes: Sequence[str]) -> list[Device]:
        """Run a discovery scan on the modes named.

        A mode this build carries no transport for contributes nothing and is not an
        error.
        """

    def devices(self) -> list[Device]:
        """Every device known, across every mode. One reachable over two modes appears
        once.
        """
    def select(self, targets: Sequence[str], mode: str | None = None) -> list[str]:
        """The devices the targets name, in the order they were written.

        A target is an identity (`1C:8B:…`), a SKU (`H6159`), a name the configuration
        gives a device (`name:kitchen`), or a group it gives (`group:ambient`). `id:`,
        `sku:`, `name:` and `group:` state the kind where the target alone does not. A
        SKU, a name and a group select among the devices the SDK knows, so scan first.

        `mode` is the one mode the caller will drive. A SKU, a name and a group then
        match among the devices that enable it. An identity selects itself either way.
        """
    def modes(self) -> list[str]:
        """The modes this build carries a transport for. Not a preference order: that is
        each device's own configuration.
        """
    def problems(self) -> list[str]:
        """Everything wrong with the configuration, as one sentence each."""
    @property
    def config(self) -> Config:
        """The configuration in force."""
    @property
    def catalog(self) -> Catalog:
        """The device catalog in force."""
    def device(self, target: str) -> DeviceHandle:
        """A handle for one device, by its identity or by the name the configuration
        gives it. It reads the configuration and no scan.

        A bare target is a name where the configuration gives one, and an identity where
        it reads as one. `id:` and `name:` state the kind.
        """
    def device_on(self, target: str, mode: str) -> DeviceHandle:
        """A handle that drives the device over one mode alone.

        Every call on it goes over `mode` or raises. Use it where the caller serves one
        mode by design, such as a bridge that reaches a device over `lan`: a handle from
        `device()` would move to the next enabled mode when that one stops answering.

        `target` reads as it does for `device()`.
        """
    def targets(self, target: str) -> list[str]:
        """The identities that one target names, from the configuration and with no
        scan: one device, or every member of a group in identity order.
        """
    def group(self, target: str, mode: str | None = None) -> GroupHandle:
        """A handle for the devices `targets()` reads. Every verb on it answers one
        `Outcome` per member and raises for nothing a member does. `mode` pins every
        member, as `device_on()` does.
        """
    async def identify(
        self,
        targets: str | Sequence[str] | None = None,
        *,
        color: Color | None = None,
        wait: float | None = None,
        hold: float | None = None,
        keep: bool | None = None,
        mode: str | None = None,
    ) -> WalkReport:
        """Take the devices off, light each in turn, then take them off again: the walk
        `govee identify` runs.

        `targets` is one target or several, read as `select()` reads them: an identity,
        a SKU, a name or a group. `None` walks every device a scan over the mode finds,
        and an empty list walks none. A SKU, a name and a group scan first.

        Every keyword is optional. `color` is green, `wait` is the wait between two
        steps (1 s), and `hold` is how long the last device holds the color (5 s), both
        in seconds. `keep` leaves every device on and lit at the end. `mode` is the one
        mode the walk drives, `"lan"` by default.

        Raises `ConfigError` with `mode_not_enabled` before it sends anything where a
        device does not enable the mode. A device that fails during the walk is in the
        report instead.
        """
    def events(self) -> EventStream:
        """Subscribe to what the SDK reports. Iterate it with `async for`."""
    async def close(self) -> None:
        """Release what every transport holds. Call it before the program ends, or `ble`
        loses the last frame it wrote.
        """
