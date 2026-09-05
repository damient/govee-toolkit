---
description: Run every check CI runs, report a pass/fail list, then fix what failed
allowed-tools: Bash, Read, Edit, Write, Glob, Grep
---

Run the same checks `.github/workflows/ci.yml` runs, locally, then fix every
failure — including ones this session did not cause.

## Run

`./tools/qa.sh` — it runs each check with output captured and ends with a
summary. Exit code: 0 all passed, 1 something failed, 2 all passed but a check
was skipped for a missing tool. Pass a substring as `$1` to run one check
(`./tools/qa.sh clippy`).

Arguments: $ARGUMENTS — if a check name is given, run only that one.

## Report

Report the summary as a table, one row per check, in the order the script ran
them: check, result (pass / fail / skipped), and for a failure the one-line
cause. Do not paste the raw log; quote only the lines that name the problem.
Name the missing tool and its install command for anything skipped — a skip is
a check CI will still run.

## Fix

Then fix every failure, and re-run the affected check to confirm.

- `rust fmt` — run `cargo +nightly fmt --all`, never hand-format.
- `rust clippy` — fix the cause. Reach for `#[allow]` only when the lint is
  wrong here, and say why in a comment.
- `rust test` — a failing conformance vector in `tests/fixtures/golden/` means
  the core and the fixture disagree. Decide which one is right from the
  capture the fixture cites; never edit a fixture to match the code when its
  bytes came from a capture.
- `spelling` — a real typo gets fixed. A SKU or a protocol term that `typos`
  splits into a word goes in `_typos.toml`, as narrow an entry as will do.
- `file length` — split the file along its responsibilities. Do not trim
  comments to get under the limit.
- `licenses and advisories` — a rejected license is a dependency to drop, not
  an entry to add to `deny.toml`; the repository is MIT with no copyleft
  dependencies.
- `rust msrv` — a feature that needs a newer compiler raises `rust-version` in
  `packages/rust/Cargo.toml` in the same commit. Do not raise it silently to
  make the check pass.

If a failure is not yours and the fix is larger than the check itself, say so
and ask before rewriting it.

Do not commit or push. End with what is left failing, or state plainly that
everything passes.
