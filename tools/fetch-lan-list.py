#!/usr/bin/env python3
"""Refresh `docs/lan-supported-devices.json` from the list of the WLAN guide.

Usage:
  tools/fetch-lan-list.py            rewrite the JSON file and the table of the page
  tools/fetch-lan-list.py --check    exit 1 when the list changed, write nothing
  tools/fetch-lan-list.py --market GB

The guide page reads its list from the endpoint below. The script prints the
models that the list adds, removes or moves to another category.

A model that the file already holds keeps its name and its category: those are
cleaned by hand. A new model gets its name with the spaces collapsed and the
full-width brackets replaced. A new category takes the spelling of an existing
one that differs only in case.
"""

import argparse
import datetime
import json
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIST = ROOT / "docs/lan-supported-devices.json"
ENDPOINT = "https://app2.govee.com/bff-app/v1/user/guide/device/info?market={market}"


def fetch(market: str) -> dict[str, dict[str, str]]:
    """The models of the list, by SKU, with their raw name and category."""
    with urllib.request.urlopen(ENDPOINT.format(market=market), timeout=30) as reply:
        body = json.load(reply)
    if body.get("status") != 200:
        sys.exit(f"the endpoint answered {body.get('status')}: {body.get('message')}")
    models: dict[str, dict[str, str]] = {}
    for group in body["data"]:
        for entry in group["categoryList"]:
            models[entry["nodeSku"]] = {
                "name": entry["nodeName"],
                "category": group["name"],
            }
    if not models:
        sys.exit("the endpoint answered an empty list")
    return models


def clean(text: str) -> str:
    """Collapses the spaces, and replaces the full-width brackets."""
    text = text.replace("\uff08", " (").replace("\uff09", ") ")
    text = re.sub(r"\s*\(\s*", " (", text)
    text = re.sub(r"\s*\)", ")", text)
    return " ".join(text.split())


def merge(
    old: list[dict[str, str]], fresh: dict[str, dict[str, str]]
) -> list[dict[str, str]]:
    known = {m["sku"]: m for m in old}
    spelling = {m["category"].lower(): m["category"] for m in old}
    models = []
    for sku in sorted(fresh):
        if (
            sku in known
            and clean(known[sku]["category"]).lower()
            == clean(fresh[sku]["category"]).lower()
        ):
            models.append(known[sku])
            continue
        category = clean(fresh[sku]["category"])
        category = spelling.get(category.lower(), category)
        name = known[sku]["name"] if sku in known else clean(fresh[sku]["name"])
        models.append({"sku": sku, "name": name, "category": category})
    return models


def report(old: list[dict[str, str]], new: list[dict[str, str]]) -> bool:
    before = {m["sku"]: m for m in old}
    after = {m["sku"]: m for m in new}
    lines = [
        f"+ {s}  {after[s]['name']}  ({after[s]['category']})"
        for s in sorted(after.keys() - before.keys())
    ]
    lines += [
        f"- {s}  {before[s]['name']}" for s in sorted(before.keys() - after.keys())
    ]
    lines += [
        f"~ {s}  {before[s]['category']} -> {after[s]['category']}"
        for s in sorted(before.keys() & after.keys())
        if before[s]["category"] != after[s]["category"]
    ]
    print("\n".join(lines) if lines else "no change", flush=True)
    return bool(lines)


def write(document: dict) -> None:
    text = json.dumps(document, indent=2, ensure_ascii=False)
    # One model per line, so that a diff names the models that changed.
    text = re.sub(
        r"\{\n\s+(\"sku\": [^\n]+)\n\s+(\"name\": [^\n]+)\n"
        r"\s+(\"category\": [^\n]+)\n\s+\}",
        r"{ \1 \2 \3 }",
        text,
    )
    LIST.write_text(text + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check", action="store_true", help="exit 1 when the list changed"
    )
    parser.add_argument(
        "--market", default="US", help="the market the endpoint answers for"
    )
    args = parser.parse_args()

    document = json.loads(LIST.read_text(encoding="utf-8"))
    models = merge(document["models"], fetch(args.market))
    changed = report(document["models"], models)
    if args.check:
        sys.exit(1 if changed else 0)

    # The date moves on a run without a change too: it says when the list was
    # last read.
    document["models"] = models
    document["retrieved"] = datetime.datetime.now(datetime.UTC).date().isoformat()
    write(document)
    subprocess.run(
        ["cargo", "run", "-q", "-p", "xtask", "--", "lan"],
        cwd=ROOT / "packages/rust",
        check=True,
    )


if __name__ == "__main__":
    main()
