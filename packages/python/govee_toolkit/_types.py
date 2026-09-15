"""The type aliases the stubs annotate the API with.

The aliases are a real module and not stub-only names: the extension module
builds no object for a name that exists in a `.pyi` alone, and `mypy.stubtest`
reports every one of them.
"""

from collections.abc import Sequence
from typing import TypeAlias

Color: TypeAlias = tuple[int, int, int]
"""One RGB triple, each channel 0-255."""

Arg: TypeAlias = bool | int | str | bytes | Sequence[int] | Sequence[Color]
"""A value a device file's command takes as an argument."""

Resolution: TypeAlias = str | int
"""A segment count, or the name of one the device file declares."""

Rate: TypeAlias = str | float
"""Frames per second, or the name of a rate the device file declares."""
