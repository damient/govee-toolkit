#!/usr/bin/env python3
"""Write the stub docstrings from the doc comments of the PyO3 binding.

Usage:
  tools/sync-stubs.py            rewrite the stubs
  tools/sync-stubs.py --check    exit 1 on drift, and write nothing

The binding owns the prose. `packages/python/govee_toolkit/_govee_toolkit.pyi`
owns the types, and `mypy.stubtest` checks those against the built module.
Nothing checked the prose, so the two wordings drifted apart.

The tool rewrites every docstring that a `///` or a `create_exception!` string
covers. It leaves the rest of the stub alone, byte for byte.
"""

import re
import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BINDING = ROOT / "packages/python/src"
STUB = ROOT / "packages/python/govee_toolkit/_govee_toolkit.pyi"
WIDTH = 88

# `Config::new` carries `#[new]`, which PyO3 exposes under this name.
RENAMED = {("Config", "new"): "__init__"}


def doc_comments() -> dict[tuple[str, str], str]:
    """Every `///` block of the binding, by the class and member it documents.

    A class is keyed by the `name = "..."` of its `#[pyclass]`, so the key is
    the name Python sees and not the Rust identifier.
    """
    found: dict[tuple[str, str], str] = {}
    for path in sorted(BINDING.glob("*.rs")):
        lines = path.read_text().splitlines()
        current: str | None = None
        block: list[str] = []
        for i, line in enumerate(lines):
            text = line.strip()
            if text.startswith("///"):
                block.append(text[3:].lstrip())
                continue
            window = "\n".join(lines[i : i + 8])
            if text.startswith("#[") and "pyclass" in window:
                named = re.search(r'name = "(\w+)"', window)
                if named:
                    current = named.group(1)
                    if block:
                        found[current, "__doc__"] = "\n".join(block).strip()
                block = []
                continue
            member = re.match(r"(?:pub(?:\(crate\))? )?fn (\w+)", text)
            if member and block:
                if current:
                    name = RENAMED.get((current, member.group(1)), member.group(1))
                    found[current, name] = "\n".join(block).strip()
                block = []
                continue
            if text and not text.startswith("#"):
                block = []
    for name, message in exception_docs().items():
        found[name, "__doc__"] = message
    return found


def exception_docs() -> dict[str, str]:
    """The message of every `create_exception!`, which is that class's docstring."""
    source = (BINDING / "errors.rs").read_text()
    pattern = r'create_exception!\(\s*_govee_toolkit,\s*(\w+),\s*\w+,\s*"([^"]*)"'
    return dict(re.findall(pattern, source))


def paragraphs(doc: str) -> list[str]:
    out: list[str] = []
    current: list[str] = []
    for line in doc.split("\n"):
        if line.strip():
            current.append(line.strip())
        elif current:
            out.append(" ".join(current))
            current = []
    if current:
        out.append(" ".join(current))
    return out


def render(doc: str, indent: str) -> list[str]:
    """One docstring, wrapped to the width ruff checks.

    The opening `\"\"\"` shares the first line, so that line wraps three columns
    short. A docstring of one short paragraph stays on one line.
    """
    paras = paragraphs(doc)
    width = WIDTH - len(indent)
    if len(paras) == 1 and len(indent) + 3 + len(paras[0]) + 3 <= WIDTH:
        return [f'{indent}"""{paras[0]}"""']
    blocks: list[list[str]] = []
    for position, para in enumerate(paras):
        if position == 0:
            filled = textwrap.fill(para, width=width, initial_indent="@@@")
            filled = filled.replace("@@@", "", 1)
        else:
            filled = textwrap.fill(para, width=width)
        blocks.append([indent + line for line in filled.split("\n")])
    out: list[str] = []
    for position, block in enumerate(blocks):
        if position:
            out.append("")
        out.extend(block)
    out[0] = indent + '"""' + out[0][len(indent) :]
    out.append(indent + '"""')
    return out


def rewrite(docs: dict[tuple[str, str], str]) -> tuple[str, list[str]]:
    """The stub, with every docstring the binding covers replaced."""
    lines = STUB.read_text().splitlines()
    out: list[str] = []
    current: str | None = None
    member: str | None = None
    unmatched: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        for pattern, is_class in (
            (r"^class (\w+)", True),
            (r"^    (?:async )?def (\w+)", False),
            (r"^    (\w+): ", False),
        ):
            found = re.match(pattern, line)
            if found:
                if is_class:
                    current, member = found.group(1), "__doc__"
                else:
                    member = found.group(1)
                break
        opening = re.match(r'^(\s*)"""', line)
        if opening and current and member:
            indent = opening.group(1)
            stripped = line.strip()
            if stripped.endswith('"""') and len(stripped) > 5:
                end = i
            else:
                end = i + 1
                while end < len(lines) and '"""' not in lines[end]:
                    end += 1
            doc = docs.get((current, member))
            if doc:
                out.extend(render(doc, indent))
            else:
                unmatched.append(f"{current}.{member}")
                out.extend(lines[i : end + 1])
            i = end + 1
            member = None
            continue
        out.append(line)
        i += 1
    return "\n".join(out) + "\n", unmatched


def main() -> int:
    check = "--check" in sys.argv[1:]
    docs = doc_comments()
    if not docs:
        print("sync-stubs: no doc comment in packages/python/src", file=sys.stderr)
        return 1
    wanted, unmatched = rewrite(docs)
    if wanted == STUB.read_text():
        print(f"sync-stubs: the stubs match the binding ({len(docs)} doc comments)")
        return 0
    if check:
        print(
            "sync-stubs: the stubs and the binding disagree. "
            "Run tools/sync-stubs.py and commit the result.",
            file=sys.stderr,
        )
        return 1
    STUB.write_text(wanted)
    print(f"sync-stubs: wrote {STUB.relative_to(ROOT)} from {len(docs)} doc comments")
    if unmatched:
        print("  no doc comment covers: " + ", ".join(unmatched))
    return 0


if __name__ == "__main__":
    sys.exit(main())
