"""How a Python value crosses into a command argument.

The binding converts before it sends. A value it accepts reaches the send
path and fails there, on the unknown device these tests use; a value it
refuses raises `ValueError` and nothing is sent either way.
"""

import pytest

from govee_toolkit import TransportError
from helpers import call

pytestmark = pytest.mark.asyncio


ACCEPTED = [
    pytest.param(True, id="bool"),
    pytest.param(7, id="int"),
    pytest.param("text", id="str"),
    pytest.param(b"\x01\x02", id="bytes"),
    pytest.param([(255, 0, 0), (0, 255, 0)], id="colors"),
    pytest.param([0, 1, 2], id="zones"),
]

REFUSED = [
    pytest.param(object(), id="object"),
    pytest.param({"a": 1}, id="dict"),
    pytest.param(None, id="none"),
]


@pytest.mark.parametrize("value", ACCEPTED)
async def test_an_accepted_value_reaches_the_send_path(handle, value):
    with pytest.raises(TransportError) as raised:
        await call(handle.send, "power", value=value)
    assert raised.value.code == "unknown_device"


@pytest.mark.parametrize("value", REFUSED)
async def test_a_value_of_no_argument_type_is_refused(handle, value):
    with pytest.raises(ValueError):
        await call(handle.send, "power", value=value)


async def test_a_color_takes_three_channels(handle):
    with pytest.raises(ValueError):
        await call(handle.color, (255, 0))


async def test_a_channel_outside_0_to_255_is_refused(handle):
    with pytest.raises(ValueError):
        await call(handle.color, (256, 0, 0))


async def test_a_color_reaches_the_send_path(handle):
    with pytest.raises(TransportError) as raised:
        await call(handle.color, (255, 0, 0))
    assert raised.value.code == "unknown_device"


@pytest.mark.parametrize("resolution", ["app", "native", 12])
async def test_a_resolution_is_a_name_or_a_zone_count(handle, resolution):
    with pytest.raises(TransportError) as raised:
        await call(handle.open_stream, resolution=resolution)
    assert raised.value.code == "unknown_device"


async def test_an_unnamed_resolution_is_refused(handle):
    with pytest.raises(ValueError):
        await call(handle.open_stream, resolution="every")


async def test_an_unnamed_rate_is_refused(handle):
    with pytest.raises(ValueError):
        await call(handle.open_stream, rate="fast")


async def test_an_unnamed_mode_is_refused(govee):
    with pytest.raises(ValueError):
        await call(govee.scan_on, ["radio"])
