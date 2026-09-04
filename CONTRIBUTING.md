# Contributing to govee-toolkit

A community project, not affiliated with Govee.

## Guiding principle

**LAN path latency and reliability come first.** A contribution that adds
another mode or a third-party integration must not slow down or complicate the
`lan` fast path.

## Monorepo layout

| Directory | Contents |
| --------- | -------- |
| `devices/` | Source of truth: per-SKU YAML definitions |
| `docs/` | Mode model (`modes.md`), architecture and per-protocol documentation |
| `packages/rust/` | The protocol core, and the reference implementation |
| `packages/` | Published SDKs (python, node, php) plus the Art-Net bridge |
| `apps/` | Web playground and Electron app (not published) |
| `integrations/` | Matter bridge, Home Assistant, Homebridge |
| `tools/` | Device simulator for tests |
| `tests/fixtures/` | Real UDP / BLE captures per SKU, and the conformance vectors |

SDKs **read** `devices/*.yaml` — they do not duplicate protocol logic. An SDK
only implements the transports and generic parsing.

The protocol is implemented once, in `packages/rust`. Node and Python bind to
that core; PHP is the one hand-written port, and the conformance vectors are
what keep it honest. See [`docs/architecture.md`](docs/architecture.md).

`lan`, `ble` and `cloud` are **modes**, not a fallback chain: the user enables
one or several per device. A contribution must not make a mode implicit, and
must not silently substitute one mode for another — see
[`docs/modes.md`](docs/modes.md).

## Adding a SKU

1. Copy an existing file from `devices/` (or start from `devices/schema.yaml`).
2. Fill in capabilities, per-mode support level and the command tables.
3. Add a real capture under `tests/fixtures/lan-captures/<sku>/`.
4. Update the tables in [`docs/compatibility.md`](docs/compatibility.md).

Details: [`devices/README.md`](devices/README.md).

## Documenting an undocumented command

1. Try the payload through the **raw payload field** in the playground
   (`apps/playground/`).
2. Add the observed frame to `tests/fixtures/lan-captures/`.
3. Document it in `docs/protocol/lan.md`: payload structure, compatible SKUs,
   sample frame, observed behavior.
4. Formalize the command in the relevant `devices/*.yaml` files.

## Tests

From `packages/rust`:

```bash
cargo test                                    # unit, conformance and doc tests
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
cargo deny check                              # licenses and advisories
```

`cargo test` also validates every `devices/*.yaml` and replays the conformance
vectors in `tests/fixtures/golden/` — add one alongside any new command.

<!-- TODO: complete once the bindings and the PHP port exist -->

- Python: `pytest`
- Node: the project's standard runner
- PHP: PHPUnit, running the same conformance vectors

The `tools/device-simulator/` lets you test without hardware.

## Releases

Each package is versioned and released independently through tags:
`rust-vX.Y.Z`, `python-vX.Y.Z`, `node-vX.Y.Z`, `php-vX.Y.Z`.

`ci.yml` runs on push and pull request. The release workflows are still
`workflow_dispatch` only — nothing is publishable yet; restore the tag trigger
at the top of one with its first real release.

## License

All contributions are accepted under the [MIT](LICENSE) license.
