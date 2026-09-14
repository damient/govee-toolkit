"""A handle for an identity no scan found.

Nothing reaches the wire here. The send path refuses to scan, so a device no
transport knows fails before a command is encoded, with `unknown_device`.
"""

import pytest

from helpers import refuses_unknown

pytestmark = pytest.mark.asyncio


async def test_a_handle_carries_the_identity_and_its_modes(handle, unknown_id):
    assert handle.id == unknown_id
    assert handle.modes == ["lan"]


async def test_an_unknown_device_has_no_health(handle):
    assert handle.health("lan") is None


async def test_send_refuses_an_unknown_device(handle):
    await refuses_unknown(handle.send, "power")


async def test_a_verb_refuses_an_unknown_device(handle):
    await refuses_unknown(handle.power, True)


async def test_read_refuses_an_unknown_device(handle):
    await refuses_unknown(handle.read, "status")


async def test_status_refuses_an_unknown_device(handle):
    await refuses_unknown(handle.status)


async def test_the_specification_needs_a_known_device(handle):
    await refuses_unknown(handle.spec)


async def test_the_serving_mode_needs_a_known_device(handle):
    await refuses_unknown(handle.serving_mode)


async def test_no_mode_watches_a_device_no_mode_knows(govee, unknown_id):
    """A watch reads from the mode that would serve a command, and there is
    none."""
    assert govee.device(unknown_id).watch_status() is None
