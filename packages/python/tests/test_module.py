"""What the module exports, and what it says about itself."""

import subprocess
import sys

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


def test_an_unknown_name_is_an_attribute_error():
    """The module reads `__version__` lazily, and answers nothing else."""
    unknown = "no_such_name"
    with pytest.raises(AttributeError):
        getattr(govee_toolkit, unknown)


def test_the_version_is_not_read_on_the_import():
    """The metadata read opens a file, so it waits for the first read."""
    source = "import govee_toolkit as g; print('__version__' in vars(g))"
    done = subprocess.run(
        [sys.executable, "-c", source], capture_output=True, text=True, check=True
    )
    assert done.stdout.strip() == "False"


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
    """The core reports its own version, so nothing here can drift from it."""
    assert VERSION.match(govee_toolkit.CORE_VERSION)


def test_the_modes_are_the_names_the_core_knows():
    assert isinstance(govee_toolkit.MODES, tuple)
    assert "lan" in govee_toolkit.MODES
    assert len(set(govee_toolkit.MODES)) == len(govee_toolkit.MODES)
    assert all(name == name.lower() for name in govee_toolkit.MODES)
