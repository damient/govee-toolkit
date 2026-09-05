# Governance

One maintainer, [@damient](https://github.com/damient). There is no committee,
no steering group and no vote. This page says how decisions actually get made,
so a contributor knows what to expect before spending time on a change.

## How decisions are made

- **Open an issue or a discussion before a large change.** A design agreed in
  an issue is a pull request that gets merged; a large pull request that arrives
  unannounced may be refused on direction rather than on code.
- Small fixes, device files and documentation go straight to a pull request.
- The maintainer reviews and merges. Disagreement is settled in the thread; if
  it stays unsettled, the maintainer decides and says why.
- Every contribution is certified with `Signed-off-by` — Developer Certificate
  of Origin 1.1, `git commit -s`. There is no CLA. See
  [`CONTRIBUTING.md`](CONTRIBUTING.md).
- Code is accepted under the [MIT](LICENSE) license.

## What gets a device file merged

A `devices/<SKU>.yaml` pull request is merged when:

1. The file validates — `cargo test` checks the schema, and an undocumented
   command carries `documented: false`, a `notes:` line and a pointer to the
   matching section of [`docs/protocol/lan.md`](docs/protocol/lan.md).
2. A real capture is attached under `tests/fixtures/lan-captures/<SKU>/` and
   referenced from `capture:`.
3. A conformance vector exists under `tests/fixtures/golden/` for every new
   command, and its `source` says whether the bytes come from that capture or
   were worked out from the documented layout.
4. Nothing is claimed that was not observed. `verified` fields, capabilities,
   measurements and compatibility rows come from a device you ran the command
   on. An entry marked `?` or `TODO` is merged; an entry filled in from
   inference is not.

Details: [`devices/README.md`](devices/README.md).

## Non-negotiables

These four are settled. A pull request that contradicts one is refused
regardless of how good the code is.

1. **The `lan` fast path wins.** Latency and reliability on `lan` come before
   any other mode, any integration and any convenience. A change that adds work
   to the send path needs a reason.
2. **Modes are explicit.** One enabled mode means one mode: an unreachable
   device makes the command fail and say so. No mode is ever substituted
   silently, and a command a mode cannot serve fails rather than being
   approximated. See [`docs/modes.md`](docs/modes.md).
3. **`devices/*.yaml` is the single source of truth.** SDKs implement transports
   and generic parsing. No SKU name and no command name belongs in Rust code —
   if you are about to write one, the device file is missing something instead.
4. **Never invent verification.** Unverified is `?` or a `TODO`, and that is a
   good answer.

## Becoming a maintainer

There is no application. A contributor who has landed a sustained run of
reviewed work — device files with real captures, protocol findings, or a
package they carry — and who reviews other people's pull requests, gets offered
commit access by the maintainer. Areas are handed over whole where that makes
sense: a language binding, an integration, the device catalogue.

The expectation that comes with it is the four points above, and the writing
conventions in [`CLAUDE.md`](CLAUDE.md).

## If the project is abandoned

MIT, so a fork is always available and needs nobody's permission.

If the maintainer stops responding for six months, the project is unmaintained
in practice. Anyone may fork it and say so; a fork that picks up the work will
be linked from the README and the repository archived, if the maintainer comes
back and there is a fork worth pointing at. Published package names are not
transferred automatically — ask, and expect no answer to mean no.
