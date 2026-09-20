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
