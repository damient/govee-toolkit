"""A handle for an identity no scan found.

Nothing reaches the wire here. The send path refuses to scan, so a device no
transport knows fails before a command is encoded, with `unknown_device`.
"""

import pytest

from govee_toolkit import TransportError
from helpers import call

pytestmark = pytest.mark.asyncio


async def test_a_handle_carries_the_identity_and_its_modes(handle, unknown_id):
    assert handle.id == unknown_id
    assert handle.modes == ["lan"]


async def test_an_unknown_device_has_no_health(handle):
    assert handle.health("lan") is None


async def test_send_refuses_an_unknown_device(handle):
    with pytest.raises(TransportError) as raised:
        await call(handle.send, "power")
    assert raised.value.code == "unknown_device"


async def test_a_verb_refuses_an_unknown_device(handle):
    with pytest.raises(TransportError) as raised:
        await call(handle.power, True)
    assert raised.value.code == "unknown_device"


async def test_read_refuses_an_unknown_device(handle):
    with pytest.raises(TransportError) as raised:
        await call(handle.read, "status")
    assert raised.value.code == "unknown_device"


async def test_status_refuses_an_unknown_device(handle):
    with pytest.raises(TransportError) as raised:
        await call(handle.status)
    assert raised.value.code == "unknown_device"


async def test_the_specification_needs_a_known_device(handle):
    with pytest.raises(TransportError) as raised:
        await call(handle.spec)
    assert raised.value.code == "unknown_device"


async def test_the_serving_mode_needs_a_known_device(handle):
    with pytest.raises(TransportError) as raised:
        await call(handle.serving_mode)
    assert raised.value.code == "unknown_device"


async def test_no_mode_watches_a_device_no_mode_knows(govee, unknown_id):
    """A watch reads from the mode that would serve a command, and there is
    none."""
    assert govee.device(unknown_id).watch_status() is None
