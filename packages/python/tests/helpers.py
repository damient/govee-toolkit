"""Helpers the tests share."""

import inspect
import re
from pathlib import Path

import pytest

from govee_toolkit import TransportError

VERSION = re.compile(r"^\d+\.\d+\.\d+")


async def call(method, *args, **kwargs):
    """Call one API method, and await the answer if it is awaitable.

    The binding converts the arguments before it builds the future, so a
    refused argument raises from the call and not from the await. This helper
    catches the failure at either point.
    """
    answer = method(*args, **kwargs)
    if inspect.isawaitable(answer):
        return await answer
    return answer


async def refuses_unknown(method, *args, **kwargs):
    """Assert the call fails with `unknown_device`, and nothing was sent.

    The send path refuses to scan, so every call on an identity no transport
    knows ends the same way. A call that reaches it is a call the binding
    accepted the arguments of.
    """
    with pytest.raises(TransportError) as raised:
        await call(method, *args, **kwargs)
    assert raised.value.code == "unknown_device"


def repository_root():
    """The checkout this package sits in, or `None` outside one.

    The tests run against an installed wheel too, and a wheel carries no
    repository. A test that needs a file from the checkout skips instead.
    """
    for parent in Path(__file__).resolve().parents:
        if (parent / "devices").is_dir() and (parent / "packages").is_dir():
            return parent
    return None


def manifest_version(path):
    """The first `version = "…"` a TOML manifest declares."""
    for line in path.read_text(encoding="utf-8").splitlines():
        found = re.match(r'^version\s*=\s*"([^"]+)"', line.strip())
        if found:
            return found.group(1)
    return None
