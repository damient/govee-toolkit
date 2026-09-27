"""The SDK at startup, and what it knows before any scan."""

import pytest

from govee_toolkit import MODES, Config, ConfigError, Govee
from helpers import call

pytestmark = pytest.mark.asyncio


async def test_start_and_close():
    sdk = await Govee.start(Config())
    await sdk.close()


async def test_an_identity_selects_itself_and_an_unknown_model_selects_nothing(govee):
    devices = await govee.devices(["aa:bb:cc:dd:ee:ff:00:11"])
    assert [handle.id for handle in devices] == ["AA:BB:CC:DD:EE:FF:00:11"]
    assert len(devices) == 1
    assert devices.list() == []
    with pytest.raises(ConfigError) as refused:
        await govee.devices(["H6008"])
    assert refused.value.code == "no_such_target"


async def test_an_empty_target_list_selects_no_device(govee):
    devices = await govee.devices([])
    assert len(devices) == 0
    assert devices.members == []


async def test_the_modes_are_the_transports_this_build_carries(govee):
    modes = govee.modes()
    assert set(modes) <= set(MODES)


async def test_the_default_configuration_raises_no_problem(govee):
    assert govee.problems() == []


async def test_the_configuration_in_force_is_the_one_it_started_with(govee):
    assert govee.config.default_modes == ["lan"]


async def test_the_catalog_is_reachable_from_the_facade(govee):
    assert govee.catalog.skus()


NAMED = """\
devices:
  "AA:BB:CC:DD:EE:FF":
    name: kitchen
  "11:22:33:44:55:66":
    name: twin
  "22:33:44:55:66:77":
    name: twin
"""


async def test_a_handle_takes_a_name_the_configuration_gives(tmp_path):
    path = tmp_path / "config.yaml"
    path.write_text(NAMED, encoding="utf-8")
    sdk = await Govee.start(Config.load_from(path))
    try:
        assert sdk.device("kitchen").id == "AA:BB:CC:DD:EE:FF"
        assert sdk.device("name:KITCHEN", mode="lan").id == "AA:BB:CC:DD:EE:FF"
        assert sdk.device("kitchen", mode="lan").pinned == "lan"
        assert sdk.device("kitchen").pinned is None
        [member] = (await sdk.devices(["kitchen"], mode="lan")).members
        assert member.pinned == "lan"
        assert sdk.device("id:AA:BB:CC:DD:EE:FF").id == "AA:BB:CC:DD:EE:FF"
        for target, code in [
            ("name:attic", "no_such_target"),
            ("twin", "ambiguous_target"),
            ("sku:H6008", "target_not_understood"),
        ]:
            with pytest.raises(ConfigError) as refused:
                sdk.device(target)
            assert refused.value.code == code, target
    finally:
        await sdk.close()


GROUPED = """\
devices:
  "BB:00:00:00:00:02":
    name: hall
    groups: [ambient]
  "AA:00:00:00:00:01":
    groups: [Ambient]
    modes: [ble]
"""


async def test_a_group_names_its_members_and_each_answers_alone(tmp_path):
    path = tmp_path / "config.yaml"
    path.write_text(GROUPED, encoding="utf-8")
    sdk = await Govee.start(Config.load_from(path))
    try:
        group = await sdk.devices(["group:ambient", "hall"], mode="lan")
        ids = [handle.id for handle in group.members]
        assert ids == ["AA:00:00:00:00:01", "BB:00:00:00:00:02"]
        assert [handle.id for handle in await sdk.devices(["hall"])] == ids[1:]

        outcomes = await group.power(True)
        assert [o.id for o in outcomes] == ids
        assert not any(o.ok for o in outcomes)
        assert outcomes[0].error.code == "mode_not_enabled"
        assert outcomes[1].error.code == "unknown_device"
        assert outcomes[1].served is None

        with pytest.raises(ConfigError) as refused:
            sdk.device("ambient")
        assert refused.value.code == "target_not_understood"
    finally:
        await sdk.close()


async def test_enables_keeps_the_members_that_enable_the_mode(tmp_path):
    path = tmp_path / "config.yaml"
    path.write_text(GROUPED, encoding="utf-8")
    sdk = await Govee.start(Config.load_from(path))
    try:
        kept = await sdk.devices(["group:ambient"], enables="ble")
        assert [handle.id for handle in kept] == ["AA:00:00:00:00:01"]
        with pytest.raises(ConfigError) as refused:
            await sdk.devices(["hall"], enables="ble")
        assert refused.value.code == "no_such_target"
        with pytest.raises(ValueError):
            await sdk.devices(["hall"], enables="radio")
    finally:
        await sdk.close()


async def test_apply_sends_no_step_to_a_group_whose_members_the_scan_misses(tmp_path):
    path = tmp_path / "config.yaml"
    path.write_text(GROUPED, encoding="utf-8")
    sdk = await Govee.start(Config.load_from(path))
    try:
        group = await sdk.devices(["group:ambient"], mode="lan")
        applied = await group.apply(power=True, brightness=50)
        assert not applied.ok
        assert applied.steps == []
        assert [o.ok for o in applied.reached] == [False, False]
        with pytest.raises(ValueError):
            await group.apply(segment={"colours": [[255, 0, 0]]})
    finally:
        await sdk.close()


async def test_an_identify_walk_over_an_empty_list_walks_no_device(govee):
    report = await govee.identify([])
    assert report.lit == []
    assert report.failed == []
    assert report.stayed == []
    assert report.ok


@pytest.mark.parametrize(
    "options",
    [
        {"hold": -1},
        {"wait": float("nan")},
        {"color": (256, 0, 0)},
        {"mode": "radio"},
    ],
)
async def test_an_identify_option_out_of_range_is_refused(govee, options):
    with pytest.raises(ValueError):
        await call(govee.identify, [], **options)


async def test_an_identify_keyword_the_list_does_not_name_is_refused(govee):
    with pytest.raises(TypeError):
        await call(govee.identify, [], hold_ms=0)


async def test_an_identify_walk_over_a_mode_the_device_does_not_enable_is_refused(
    tmp_path,
):
    path = tmp_path / "config.yaml"
    path.write_text(GROUPED, encoding="utf-8")
    sdk = await Govee.start(Config.load_from(path))
    try:
        with pytest.raises(ConfigError) as refused:
            await sdk.identify("AA:00:00:00:00:01", wait=0, hold=0)
        assert refused.value.code == "mode_not_enabled"
    finally:
        await sdk.close()
