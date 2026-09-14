"""What every test in this directory shares.

No test here reaches hardware and no test here reaches the network. The
catalog is compiled into the extension module, and a device that no scan
found fails before any byte leaves the process.
"""

import os

import pytest
import pytest_asyncio

from govee_toolkit import Config, Govee


@pytest.fixture(scope="session", autouse=True)
def isolated_state(tmp_path_factory):
    """Point every path the core reads at an empty temporary directory.

    The core reads the configuration file, the device cache and a `.env` from
    the machine. A cache written by a real run puts devices in
    `Govee.devices()` that no test discovered, and the checkout's own `.env`
    names a configuration file through `GOVEE_CONFIG`. `GOVEE_ENV_FILE`
    replaces the `.env` search, and the process environment wins over a file,
    so both are set here.
    """
    root = tmp_path_factory.mktemp("state")
    empty_env = root / "env"
    empty_env.write_text("", encoding="utf-8")
    wanted = {
        "XDG_CONFIG_HOME": str(root / "config"),
        "XDG_CACHE_HOME": str(root / "cache"),
        "GOVEE_ENV_FILE": str(empty_env),
        # A file that is not there is the default configuration, not an error.
        "GOVEE_CONFIG": str(root / "config.yaml"),
    }
    previous = {name: os.environ.get(name) for name in wanted}
    os.environ.update(wanted)
    yield root
    for name, value in previous.items():
        if value is None:
            os.environ.pop(name, None)
        else:
            os.environ[name] = value


@pytest_asyncio.fixture
async def govee():
    """A started SDK on the default configuration, closed after the test."""
    sdk = await Govee.start(Config())
    try:
        yield sdk
    finally:
        await sdk.close()


@pytest.fixture
def unknown_id():
    """An identity no scan found. Every command for it fails, and says so."""
    return "AA:BB:CC:11:22:33"


@pytest.fixture
def handle(govee, unknown_id):
    """A handle on that identity, over `lan`."""
    if "lan" not in govee.modes():
        pytest.skip("this build carries no `lan` transport")
    return govee.device(unknown_id)
