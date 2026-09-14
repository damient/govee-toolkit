"""The configuration. Nothing here starts a transport."""

from govee_toolkit import Config


def test_the_default_configuration_enables_lan_alone():
    config = Config()
    assert config.default_modes == ["lan"]
    assert config.devices == []


def test_the_default_configuration_carries_a_stream_rate():
    assert Config().stream_fallback_hz > 0


def test_a_missing_file_is_the_default_configuration(tmp_path):
    loaded = Config.load_from(tmp_path / "missing.yaml")
    assert loaded.default_modes == Config().default_modes
    assert loaded.devices == []


def test_load_without_a_file_is_the_default_configuration():
    loaded = Config.load()
    assert loaded.default_modes == ["lan"]
