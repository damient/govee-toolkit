"""Helpers the tests share."""

import inspect
import re
from pathlib import Path

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
