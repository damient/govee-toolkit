"""The SDK at startup, and what it knows before any scan."""

import pytest

from govee_toolkit import MODES, Config, ConfigError, Govee

pytestmark = pytest.mark.asyncio


async def test_start_and_close():
    sdk = await Govee.start(Config())
    await sdk.close()


async def test_no_device_is_known_before_a_scan(govee):
    assert govee.devices() == []


async def test_an_identity_selects_itself_and_an_unknown_model_selects_nothing(govee):
    assert govee.select(["aa:bb:cc:dd:ee:ff:00:11"]) == ["AA:BB:CC:DD:EE:FF:00:11"]
    with pytest.raises(ConfigError) as refused:
        govee.select(["H6008"])
    assert refused.value.code == "no_such_target"


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
        assert sdk.device_on("name:KITCHEN", "lan").id == "AA:BB:CC:DD:EE:FF"
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
