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
2. Fill in `sku`, `family`, `name`, then `capabilities`. One entry per
   capability the hardware has; a capability it does not have is left out, not
   set to `false`.
3. Under `modes`, set the support level per mode (`full` | `partial` | `none` |
   `unknown`), list the capabilities reachable in that mode, and put the rest
   under `unreachable` with a reason (`transport` | `unimplemented` |
   `unprobed`) — [`../docs/compatibility.md`](../docs/compatibility.md) says
   what each one claims. A mode you did not probe stays `unknown`: `none` says
   the hardware cannot do it, which is a claim, and a failed probe looks exactly
   like an unimplemented feature.
4. Fill in the `commands` table. Set `documented: false` for any command found
   through reverse engineering, and document it in
   [`../docs/protocol/lan.md`](../docs/protocol/lan.md) as well. Mark the entry
   that reports the device's state `role: status` — that is how an SDK finds it,
   since no command name lives in SDK code. A file that marks none simply has no
   status command: verification is skipped and `status()` fails, rather than an
   SDK guessing an entry name. An entry whose arguments an SDK fills on its own
   marks those too, with an argument `role:`; no argument name lives in SDK code
   either. [`schema.yaml`](schema.yaml) lists both sets of roles.
5. Add a real capture under `../tests/fixtures/lan-captures/<SKU>/` and point
   `capture:` at it. **Redact it first** — a capture carries your MAC, your
   IP, your SSID and possibly an account token, and git keeps them after the
   fix.
   The checklist and the placeholders to use are in
   [`../tests/fixtures/README.md`](../tests/fixtures/README.md);
   `../tools/check-captures.sh` re-checks what it can.
6. Add a conformance vector for every command, under
   `../tests/fixtures/golden/<mode>/<SKU>.json`. `cargo test` fails on a command
   that has none, and its `source` has to say whether the bytes came from the
   capture or were worked out from the documented layout.
7. Fill in `verified` (who, firmware, date). Leave what you did not check as
   `?` or `TODO` — the compatibility table reads `verified.date`, and an empty
   one renders `?` rather than a tick.
8. Regenerate the tables in
   [`../docs/compatibility.md`](../docs/compatibility.md):

   ```bash
   cd ../packages/rust && cargo run -p xtask -- compat
   ```

   They are generated from these files and CI fails when they drift.

## SKU families

When several SKUs share the same protocol behavior in every mode, keep one
file and list the others under `aliases`. Split into separate files as soon as
any command differs.

## Validation

Device files are checked in CI by `cargo test` in
[`../packages/rust`](../packages/rust). The checks are structural — they say
whether a file is well-formed, never whether a device really behaves that way:

- every `frame:` parses, and only refers to arguments the command declares;
- every `${placeholder}` in a `payload:` has an argument behind it;
- a command declaring a `frame:` carries it through `${frame}`;
- a command with `documented: false` has a `notes:` line pointing at
  `../docs/protocol/`;
- at most one command per mode claims a given `role:`, at most one argument per
  command claims a given argument `role:`, and a command with a `role:` declares
  the arguments that role has an SDK fill;
- a probed mode either reaches every capability the hardware has or explains
  each one it does not, and names none the file did not declare;
- `aliases` resolve on lookup and `candidate_aliases` deliberately do not.

Add a conformance vector alongside a new command —
[`../tests/fixtures/README.md`](../tests/fixtures/README.md). One vector per
command is enough to stop every SDK from drifting on it.
