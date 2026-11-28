"""How a Python value crosses into a command argument.

The binding reads the shape of a value and the device file states its type:
the send path reads a list of whole numbers as zone indices, byte values or
one color, under what the entry declares. A shape the binding takes reaches
that path and fails there, on the unknown device these tests use; a shape it
takes for no argument raises `ValueError` and nothing is sent either way.
"""

import pytest

from govee_toolkit import CodecError, ConfigError, load_scenes
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
        await call(govee.scan, ["radio"])


async def test_a_drawing_reaches_the_send_path(handle):
    await refuses_unknown(
        handle.draw, [(255, 0, 0)], cells=[0, 1], args={"background": (0, 0, 0)}
    )


@pytest.mark.parametrize(
    "keywords",
    [
        pytest.param({"cells": ["first"]}, id="cell name"),
        pytest.param({"cells": [-1]}, id="negative cell"),
        pytest.param({"resolution": "every"}, id="resolution"),
        pytest.param({"args": {"speed": object()}}, id="argument"),
    ],
)
async def test_a_drawing_refuses_a_bad_keyword(handle, keywords):
    with pytest.raises(ValueError):
        await call(handle.draw, [(255, 0, 0)], **keywords)


async def test_a_drawing_refuses_a_bad_color(handle):
    with pytest.raises(ValueError):
        await call(handle.draw, [(256, 0, 0)])


@pytest.mark.parametrize("scene", ["Aurora", 2867])
async def test_a_scene_name_or_code_reaches_the_send_path(handle, scene):
    await refuses_unknown(handle.scene, scene)


@pytest.mark.parametrize("scene", [True, 1.5, None, ["Aurora"]])
async def test_a_scene_is_a_name_or_a_code(handle, scene):
    with pytest.raises(ValueError):
        await call(handle.scene, scene)


async def test_a_scene_on_a_set_is_a_name_or_a_code(govee):
    devices = await govee.devices([])
    with pytest.raises(ValueError):
        await call(devices.scene, True)
    assert await devices.scene("Aurora") == []


async def test_apply_takes_a_scene_name_or_code(govee):
    devices = await govee.devices([])
    assert (await devices.apply(scene="Aurora")).ok
    assert (await devices.apply(scene=2867)).ok
    with pytest.raises(ValueError):
        await call(devices.apply, scene=True)


def test_a_library_that_no_fetch_stored_is_missing():
    with pytest.raises(ConfigError) as raised:
        load_scenes("H6022")
    assert raised.value.code == "scene_list_missing"


def test_a_model_name_that_names_no_file_is_refused():
    with pytest.raises(CodecError) as raised:
        load_scenes("../H6022")
    assert raised.value.code == "bad_sku"
