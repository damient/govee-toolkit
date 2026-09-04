# Device database

One YAML file per SKU (or SKU family). These files are the **single source of
truth** for protocol behavior: every SDK reads them instead of duplicating the
protocol, and implements only the transports (UDP socket, BLE, HTTP) and
generic parsing.

A device file declares which **modes** (`lan`, `ble`, `cloud`) the hardware
supports and what each one can reach. It does **not** declare which modes are
enabled — that is a user choice, made per device in the runtime configuration.
See [`../docs/modes.md`](../docs/modes.md).

- [`schema.yaml`](schema.yaml) — reference schema, field by field
- [`H61A0.yaml`](H61A0.yaml) — RGBIC LED Neon Rope Lights, verified over `lan`
  including the undocumented segment channel

For **which devices work**, rather than how to declare one, see
[`../docs/compatibility.md`](../docs/compatibility.md), the readable view of
these files.

## Adding a SKU

1. Copy `schema.yaml` to `<SKU>.yaml` (uppercase, e.g. `H6159.yaml`).
2. Fill in `sku`, `family`, `name`, then `capabilities`.
3. Under `modes`, set the support level per mode (`full` | `partial` | `none`)
   and list the capabilities reachable in that mode.
4. Fill in the `commands` table. Set `documented: false` for any command found
   through reverse engineering, and document it in
   [`../docs/protocol/lan.md`](../docs/protocol/lan.md) as well.
5. Add a real capture under `../tests/fixtures/lan-captures/<SKU>/` and point
   `capture:` at it.
6. Fill in `verified` (who, firmware, date).
7. Update the tables in [`../docs/compatibility.md`](../docs/compatibility.md).

## SKU families

When several SKUs share the same protocol behavior in every mode, keep one
file and list the others under `aliases`. Split into separate files as soon as
any command differs.

## Validation

<!-- TODO: add a schema validation script and wire it into CI -->
