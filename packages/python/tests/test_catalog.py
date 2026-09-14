"""The embedded catalog. It reads no hardware and no file."""

import pytest

from govee_toolkit import Catalog, CodecError, GoveeError
from helpers import repository_root


@pytest.fixture(scope="module")
def catalog():
    return Catalog.embedded()


def test_the_catalog_carries_devices(catalog):
    assert catalog.skus()
    assert len(catalog) > 0


def test_every_listed_sku_resolves(catalog):
    for sku in catalog.skus():
        assert catalog.has(sku)


def test_a_device_file_comes_back_whole(catalog):
    sku = catalog.skus()[0]
    device = catalog.device(sku)
    assert isinstance(device, dict)
    assert device["sku"]
    assert "commands" in device
    assert "modes" in device


def test_every_device_file_of_the_checkout_is_embedded(catalog):
    root = repository_root()
    if root is None:
        pytest.skip("no checkout: the device files are not here to compare against")
    on_disk = {
        path.stem for path in (root / "devices").glob("*.yaml") if path.stem != "schema"
    }
    assert on_disk
    assert on_disk <= set(catalog.skus())


def test_an_unknown_sku_is_refused(catalog):
    unknown = "H0000"
    assert not catalog.has(unknown)
    with pytest.raises(CodecError) as raised:
        catalog.device(unknown)
    assert raised.value.code == "unknown_sku"


def test_a_codec_failure_is_a_govee_failure(catalog):
    with pytest.raises(GoveeError):
        catalog.device("H0000")
