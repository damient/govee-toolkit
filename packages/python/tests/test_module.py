"""What the module exports, and what it says about itself."""

import pytest

import govee_toolkit
from helpers import VERSION, manifest_version, repository_root

EXPORTED = (
    "Govee",
    "Config",
    "Catalog",
    "Device",
    "DeviceHandle",
    "DeviceStatus",
    "Health",
    "Reply",
    "Served",
    "SegmentStream",
    "EventStream",
    "GoveeError",
    "CodecError",
    "TransportError",
    "ConfigError",
)


@pytest.mark.parametrize("name", EXPORTED)
def test_the_module_exports_the_public_api(name):
    assert hasattr(govee_toolkit, name)
    assert name in govee_toolkit.__all__


def test_the_version_is_a_release_number():
    assert VERSION.match(govee_toolkit.__version__)


def test_the_version_is_the_one_the_manifest_declares():
    root = repository_root()
    if root is None:
        pytest.skip("no checkout: the manifest is not here to compare against")
    declared = manifest_version(root / "packages" / "python" / "pyproject.toml")
    assert declared is not None
    assert govee_toolkit.__version__ == declared


def test_the_core_version_is_the_crate_the_binding_was_built_from():
    assert VERSION.match(govee_toolkit.CORE_VERSION)
    root = repository_root()
    if root is None:
        pytest.skip("no checkout: the manifest is not here to compare against")
    declared = manifest_version(root / "packages" / "rust" / "Cargo.toml")
    assert declared is not None
    assert govee_toolkit.CORE_VERSION == declared


def test_the_modes_are_the_three_the_sdk_knows():
    assert govee_toolkit.MODES == ("lan", "ble", "cloud")
