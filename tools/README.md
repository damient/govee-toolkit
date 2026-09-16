# Tools

| Tool | Where |
| ---- | ----- |
| Device simulator | [`packages/rust/crates/sim`](../packages/rust/crates/sim) |
| Local CI mirror | [`qa.sh`](qa.sh), or `/qa` in Claude Code |
| Local CI mirror, site only | [`qa-site.sh`](qa-site.sh) |
| Local CI mirror, Python only | [`qa-python.sh`](qa-python.sh) |
| Local CI mirror, Node only | [`qa-node.sh`](qa-node.sh) |
| Pass/fail reporter the four share | [`lib/qa.sh`](lib/qa.sh) |
| Build artifact sweep | [`clean-target.sh`](clean-target.sh) |
| Redaction check | [`check-captures.sh`](check-captures.sh) |
| File length, per language | [`check-file-length.sh`](check-file-length.sh) |
| Codec layering | [`check-no-io.sh`](check-no-io.sh) |
| Release notes from the changelog | [`release-notes.sh`](release-notes.sh) |
| Stub prose from the binding | [`sync-stubs.py`](sync-stubs.py) |
| Generated catalog and tables | [`packages/rust/crates/xtask`](../packages/rust/crates/xtask) |

`sync-stubs.py` writes the docstrings of
`packages/python/govee_toolkit/_govee_toolkit.pyi` from the `///` of
`packages/python/src`. The stub owns the types, which `mypy.stubtest` checks
against the built module; nothing checked the prose, and the two wordings
drifted apart. Run it after editing a doc comment of the binding:

```bash
tools/sync-stubs.py           # rewrite the stubs
tools/sync-stubs.py --check   # what CI runs
```

The simulator is a Rust crate rather than a tool of its own: the transport tests
drive it in-process on ephemeral loopback ports, and a second implementation of
the same wire protocol would be one more place for it to be wrong.

`qa.sh` runs the checks of `.github/workflows/ci.yml` in the same order and
prints a pass/fail summary. It reports a check whose tool is missing as skipped
rather than passed, and names the install command. The three it leaves out —
sign-off, commit convention and changelog entry — walk a pull request's commit
range, which does not exist locally. The workflow stays the authority; this is
a mirror of it kept in step by hand.

`qa-site.sh` does the same for the site: it mirrors
`.github/workflows/pages.yml` — the device catalog, the build, the three
linters and the file length. `qa.sh` runs it as one check, so a full run covers
the site too, and it runs on its own for the site alone:

```bash
tools/qa-site.sh            # every site check
tools/qa-site.sh lint       # the checks whose name holds "lint"
cd site && npm run qa       # the same script
```

It needs `site/node_modules`, which `cd site && npm install` writes, and it
regenerates `dist/catalog.json` when cargo is there. Both exit codes are the
ones `qa.sh` uses: 1 for a failed check, 2 for a skipped one.

`qa-python.sh` does the same for the Python binding, which is a cargo
workspace of its own. It mirrors the `python` job of `ci.yml`: ruff for the
format and the lint, mypy for the types, `mypy.stubtest` for the stubs,
`cargo fmt` and clippy for the binding, then the wheel and pytest. `qa.sh` runs
it as one check, and it runs on its own for the package alone:

```bash
tools/qa-python.sh          # every Python check
tools/qa-python.sh lint     # the checks whose name holds "lint"
```

It needs `ruff`, `mypy` and `maturin` on the `PATH`, and reports a missing one
as skipped. The tests and `stubtest` run on an interpreter that imports
`pytest`, `pytest_asyncio` and `mypy.stubtest`, which the `python3` on the
`PATH` rarely does. Make that interpreter once:

```bash
python3 -m venv packages/python/target/qa-tools-venv
packages/python/target/qa-tools-venv/bin/pip install pytest pytest-asyncio mypy
```

The script finds it there. `GOVEE_QA_PYTHON` names another one, and the
directory is under `target/`, which git ignores.

The three checks that read the stubs do different work, and a green run needs
all three. ruff checks their style, mypy checks that they are internally
consistent, and only `stubtest` imports the built module and compares it against
them. A signature that drifts from the Rust reaches a user's editor unless
`stubtest` runs. It runs with no allowlist: a name that the stubs carry and the
built module does not is a failure, so a type alias lives in the real module
`govee_toolkit/_types.py`.

`qa-node.sh` mirrors the `node` job of `ci.yml`: `cargo fmt` and clippy for
the binding, then the addon, the generated loader and type definition, and
`node --test`. `qa.sh` runs it as one check, and it runs on its own for the
package alone:

```bash
tools/qa-node.sh            # every Node check
tools/qa-node.sh clippy     # the checks whose name holds "clippy"
```

It needs Node.js 20 or newer and `packages/node/node_modules`, which
`cd packages/node && npm ci` writes, and reports a missing one as skipped.

`lib/qa.sh` holds what the four scripts share: the check runner, the skip rule
and the summary. Each script sources it and declares its own checks, so the
report reads the same either way. It carries no shebang, so it names its shell
with a `# shellcheck shell=bash` directive.

`clean-target.sh` removes the build artifacts that no later build reads. cargo
keeps the artifacts of every earlier build and collects none of them, so
`packages/rust/target` grows without a bound: one week of probe runs left
388054 files and 50 GiB. `cargo-sweep` reads the access time of each artifact,
so it removes the stale ones and keeps what the last build touched. It keeps
every artifact of an installed toolchain, so the script removes the older
copies of one artifact itself. Install cargo-sweep with `cargo install
cargo-sweep`; `clean-target.sh --help` lists the flags.

`qa.sh` stamps before its first check and sweeps after the last one, so a
passing run keeps its own artifacts and drops everything older. The Python
binding is a cargo workspace of its own, so it carries a target directory of
its own: `qa.sh` stamps and sweeps it with a second run of the script, which
`--root` points at that workspace. A run for one check (`qa.sh clippy`) sweeps
nothing: it builds a fraction of the artifacts.

`check-captures.sh` scans every tracked file under `tests/fixtures/` and
`devices/` for what a packet capture carries out of a home network: a MAC that
is not one of the documented placeholders, an IPv4 address outside the RFC 5737
documentation ranges, a credential, an `ssid` or `bssid` with a value. It prints
the file and line, and is quiet when there is nothing to report. The
placeholders it accepts, and what to substitute for what, are in
[`../tests/fixtures/README.md`](../tests/fixtures/README.md).

Its patterns are narrow on purpose — a false positive that blocks a legitimate
capture costs more than a miss — so it is a backstop for the checklist, not a
replacement for reading the capture.

The scripts here are linted with `shellcheck -x` and formatted with
`shfmt -i 2`, which `qa.sh` runs as two checks of its own and the `lint` job of
`ci.yml` repeats. `-x` follows the `# shellcheck source=SCRIPTDIR/lib/qa.sh`
directive, so the reporter is read rather than guessed at. No `-ci`: a case
branch is not indented in this repository.

`check-no-io.sh` fails when anything under `packages/rust/src/codec/` imports
`std::net`, `std::fs`, `std::thread`, `tokio` or `socket2`, or writes an
`async fn` or an `.await`. The codec
does no I/O, and with the Rust side a single crate this script is what
guarantees it. `cargo check --no-default-features` is the other half of it.

`release-notes.sh` takes a package and a release tag, and prints the changelog
section for that version:

```bash
tools/release-notes.sh rust rust-v0.3.0
```

It fails when the tag, the version in the package manifest and the changelog
heading are not the same number, or when the section exists with no entries
under it. The release workflows run it as their first step, so a tag pushed
past a manifest nobody bumped stops there instead of publishing.

`xtask` generates what is derived from `devices/*.yaml`:

```bash
cd packages/rust
cargo run -p xtask                    # dist/catalog.json, the release artifact
cargo run -p xtask -- compat          # the tables in docs/compatibility.md
cargo run -p xtask -- compat --check  # fails when they have drifted
cargo run -p xtask -- dupes           # fails on a layout two device files
                                      # declare and no family carries
```

No script supplies the local credentials: the SDK reads the repository's
gitignored `.env` on its own, so `cargo`, `govee` and the examples all find it.
The convention, and what must never be committed, are in
[`../CONTRIBUTING.md`](../CONTRIBUTING.md), "Local credentials".
