"""How a Python value crosses into a command argument.

The binding reads the shape of a value and the device file states its type:
the send path reads a list of whole numbers as zone indices, byte values or
one color, under what the entry declares. A shape the binding takes reaches
that path and fails there, on the unknown device these tests use; a shape it
takes for no argument raises `ValueError` and nothing is sent either way.
"""

import pytest

from helpers import call, refuses_unknown

pytestmark = pytest.mark.asyncio


ACCEPTED = [
    pytest.param(True, id="bool"),
    pytest.param(7, id="int"),
    pytest.param("text", id="str"),
    pytest.param(b"\x01\x02", id="bytes"),
    pytest.param([(255, 0, 0), (0, 255, 0)], id="colors"),
    pytest.param([0, 1, 2], id="whole numbers"),
]

REFUSED = [
    pytest.param(object(), id="object"),
    pytest.param({"a": 1}, id="dict"),
    pytest.param(None, id="none"),
]


@pytest.mark.parametrize("value", ACCEPTED)
async def test_an_accepted_value_reaches_the_send_path(handle, value):
    await refuses_unknown(handle.send, "power", value=value)


@pytest.mark.parametrize("value", REFUSED)
async def test_a_shape_no_argument_takes_is_refused(handle, value):
    with pytest.raises(ValueError):
        await call(handle.send, "power", value=value)


async def test_a_color_takes_three_channels(handle):
    with pytest.raises(ValueError):
        await call(handle.color, (255, 0))


async def test_a_channel_outside_0_to_255_is_refused(handle):
    with pytest.raises(ValueError):
        await call(handle.color, (256, 0, 0))


async def test_a_color_reaches_the_send_path(handle):
    await refuses_unknown(handle.color, (255, 0, 0))


@pytest.mark.parametrize("resolution", ["app", "native", "groups", 12])
async def test_a_resolution_is_a_name_or_a_zone_count(handle, resolution):
    await refuses_unknown(handle.open_stream, resolution=resolution)


async def test_an_unnamed_resolution_is_refused(handle):
    with pytest.raises(ValueError):
        await call(handle.open_stream, resolution="every")


async def test_an_unnamed_rate_is_refused(handle):
    with pytest.raises(ValueError):
        await call(handle.open_stream, rate="fast")


async def test_an_unnamed_mode_is_refused(govee):
    with pytest.raises(ValueError):
        await call(govee.scan_on, ["radio"])
