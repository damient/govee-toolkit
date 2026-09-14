"""The exceptions, and the code every one carries."""

import pytest

from govee_toolkit import (
    Catalog,
    CodecError,
    ConfigError,
    GoveeError,
    TransportError,
)


@pytest.mark.parametrize("subclass", [CodecError, TransportError, ConfigError])
def test_every_failure_is_a_govee_failure(subclass):
    assert issubclass(subclass, GoveeError)


def test_a_govee_failure_is_an_exception():
    assert issubclass(GoveeError, Exception)


def test_a_failure_carries_the_code_the_core_gives_it():
    with pytest.raises(GoveeError) as raised:
        Catalog.embedded().device("H0000")
    assert raised.value.code == "unknown_sku"
    assert isinstance(raised.value.code, str)


def test_the_message_says_what_was_asked_for():
    with pytest.raises(CodecError) as raised:
        Catalog.embedded().device("H0000")
    assert "H0000" in str(raised.value)
