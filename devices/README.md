# Device files

One YAML file per device model. A device file says how that model speaks, and
every SDK reads it. An SDK carries the transports (UDP, Bluetooth, HTTP) and
generic parsing, never the protocol of one model.

A **mode** is one way to reach a device: `lan` over your network, `ble` over
Bluetooth, `cloud` over Govee's API. A device file says which modes the
hardware supports, not which ones are on. The user turns modes on per device,
in the runtime configuration — see [`../docs/modes.md`](../docs/modes.md).

- [`schema.yaml`](schema.yaml) — every field, with its rules
- [`families/`](families/) — command tables several models share, pulled in by
  `include:`
- any `<SKU>.yaml` — a worked example. Start from the model closest to yours.
- [`../docs/compatibility.md`](../docs/compatibility.md) — which devices work,
  generated from these files

## Add a model

1. Copy `schema.yaml` to `<SKU>.yaml`, in capitals: `H6159.yaml`.
2. Fill in `sku`, `family`, `name` and `capabilities`. Leave out a capability
   the hardware lacks, rather than setting it to `false`.
3. Under `modes`, set a support level per mode and list what that mode reaches.
   Put the rest under `unreachable`, with a reason. A mode you did not test
   stays `unknown`: `none` claims the hardware cannot do it, and a failed probe
   looks exactly like a missing feature.
4. Fill in the `commands` table, and describe every undocumented command in
   [`../docs/protocol/lan.md`](../docs/protocol/lan.md) as well. Mark commands
   and arguments with the `role:` values [`schema.yaml`](schema.yaml) lists:
   an SDK finds them by role, because no command name lives in SDK code.
5. Attach a real capture under `../tests/fixtures/lan-captures/<SKU>/`.
   **Redact it first** — git keeps a leaked capture after the fix. The
   checklist is in
   [`../tests/fixtures/README.md`](../tests/fixtures/README.md).
6. Add a conformance vector per command, under
   `../tests/fixtures/golden/<mode>/<SKU>.json`. Its `source` says where the
   bytes come from: a capture, or the documented layout. Only a capture is
   evidence.
7. Fill in `verified`: who tested, the firmware version, the date. Write `?` or
   `TODO` for what you did not test. That is a good answer.
8. Run `cargo run -p xtask -- compat` from `../packages/rust` to regenerate the
   compatibility tables. CI fails when they drift.

`cargo test` in [`../packages/rust`](../packages/rust) then checks the file. It
checks the shape only, never whether the device behaves that way.

## One command in several modes

A command's name and its argument names are the contract; the layout is not.
Two modes carry one command in two formats — a `frame:` here, a `payload:`
there — and one call reaches both. So a mode that gains a command another mode
already carries takes the same names. `music` is the example to follow: the six
arguments it declares over `ble` are the six a `lan` entry would declare.

Two rules keep this honest:

- name an argument for what the wire carries, not for what an SDK would like to
  offer. `color_mode` is `0` or `1` because the byte is;
- a mode that cannot carry a field does not get the command with that field
  missing. It declares no command, and the SDK fails and says so.

## Several models in one file

Where several models behave the same in every mode, keep one file and list the
others under `aliases`. Split them as soon as one command differs.

Where several models share part of a dialect to the byte, put that part in
[`families/<name>.yaml`](families/) and name it in each file's `include:`. A
family carries the layout and nothing else: what one unit answered stays in
that model's own file, under `verified:`. Name a family after the wire it
describes, not after a product — a model includes the dialects it speaks.

`cargo run -p xtask -- dupes` reports a command layout two device files declare
and no family carries. CI runs it, so the next model cannot copy a table
instead of including one.

Two mechanisms keep a family usable where one model differs:

- `range: capability` on an integer argument takes the bounds from that
  model's `capabilities:`, through the argument's `role:`. The number then
  lives in one place, and a family declares a layout several models bound
  differently.
- `overrides:` patches one field of a command a family brought in — a bound, a
  `notes:`, or `drop:` to remove the command. It reaches no local command: a
  file changes its own command where it writes it. Both are documented in
  [`schema.yaml`](schema.yaml).
