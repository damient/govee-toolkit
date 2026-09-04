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
| `docs/` | Mode model (`modes.md`) and per-protocol documentation |
| `packages/` | Published SDKs (python, node, php) plus the Art-Net bridge |
| `apps/` | Web playground and Electron app (not published) |
| `integrations/` | Matter bridge, Home Assistant, Homebridge |
| `tools/` | Device simulator for tests |
| `tests/fixtures/` | Real UDP / BLE captures per SKU |

SDKs **read** `devices/*.yaml` — they do not duplicate protocol logic. An SDK
only implements the transports and generic parsing.

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

<!-- TODO: complete once the packages are implemented -->

- Python: `pytest`
- Node: the project's standard runner
- PHP: PHPUnit

The `tools/device-simulator/` lets you test without hardware.

## Releases

Each package is versioned and released independently through tags:
`python-vX.Y.Z`, `node-vX.Y.Z`, `php-vX.Y.Z`.

GitHub Actions is **disabled on the repository** for now, and the workflows are
set to `workflow_dispatch` only: there is nothing to build yet. To bring CI back
along with the first real test step, re-enable Actions in Settings > Actions and
restore the triggers commented at the top of each workflow file.

## License

All contributions are accepted under the [MIT](LICENSE) license.
