# DMX — order of the work

A working plan for `packages/rust/crates/dmx`. The design it implements is
[`dmx.md`](dmx.md). **Delete this page when step 11 lands**: the milestone
belongs to [`roadmap.md`](roadmap.md), and a finished plan is history that the
commits already carry.

Each step is one pull request. The order puts every pure module before the
socket that feeds it, so each step is testable when it lands.

## 1. The crate

Add `packages/rust/crates/dmx` to the workspace members. The manifest carries
`publish = false` until step 11: a name on crates.io is hard to take back.

- `name = "govee-toolkit-dmx"`, `[[bin]] name = "govee-dmx"`.
- `default = ["artnet"]`, plus an empty `sacn` feature.
- `[lints] workspace = true`, and the version and edition from the workspace.
- `govee-toolkit` with `default-features = false, features = ["lan"]`.

**Done when** `cargo check -p govee-toolkit-dmx` passes and `govee-dmx
--version` answers.

## 2. `src/profile/` — the channel table

The largest step, and the one with no I/O. It reads a device from the catalog
and answers which personalities the device serves, and what each channel does.

- The four personalities of [`dmx.md`](dmx.md), derived from `capabilities:`,
  from `modes.lan.capabilities` and `unreachable`, and from the `role:` each
  `lan` command claims.
- The slot conversions, each one a function: the dimmer, an RGB component, the
  white temperature.
- The step count a scaled channel resolves to, which step 3 prints.
- A personality above 512 channels is an error, not a truncation.

**Tests** run over the whole catalog: every device whose `lan` mode carries
`segments` must answer a valid pixel personality, and no personality may
overlap its own channels or leave a hole.

**Done when** the module compiles with no `std::net`, no `std::fs` and no
`tokio`. Keep it that way: the module moves into `packages/rust` when the site
and the bindings want the same table.

## 3. `govee-dmx profile <SKU>`

Print the table for one SKU, so an operator can patch a desk. `--json` prints
the same thing for a machine.

**Done when** the output names every channel, its offset, and the step count of
each scaled channel.

## 4. `xtask dmx`

Write `docs/dmx-profiles.md` from the device files, between generated markers,
the way `xtask compat` writes `docs/compatibility.md`. Add
`cargo run -p xtask -- dmx --check` to `ci.yml` beside the `compat` check.

**Done when** CI fails on a device file that changed and a page that did not.

## 5. `src/patch/` — the patch file

Parse and validate. No socket yet.

- Both spellings of the address: a 15-bit `universe`, or `net` plus `subnet`
  plus `universe`.
- Refuse an overlap between two entries on one universe, an address past 512, a
  personality the device cannot serve, and an unknown key.
- An error names the entry and the channel range, because the operator reads it
  next to a desk.

**Done when** a fixture patch of several devices on one universe loads, and
each refusal above has a test.

## 6. `src/input/artnet` — the parser

Bytes to `UniverseFrame`, with no socket. This is where the captures pay.

- ArtDmx: the header, the protocol version, Net and SubUni, the length, the
  sequence and the physical port.
- Refuse an odd length, a length above 512 and a version below 14.
- Drop a packet older than the last accepted one, in a window of 256.

**Tests** read captures from `tests/fixtures/artnet/`. Record them from a desk
and from a media server, so two senders are covered.

**Done when** a captured packet produces the channel values the desk showed.

## 7. The socket and `--dry-run`

Bind UDP 6454, receive a broadcast frame and a unicast frame, and resolve each
frame against the patch. `--dry-run` prints what it received and what it
resolved, and writes to no device.

**Done when** a desk on the network drives the printed output, and the bridge
sends nothing.

This is the first step that an operator can use: it debugs a patch before any
device is at risk.

## 8. `src/apply/` — the send path

Join the resolved channels to the device.

- The dimmer at 0 powers the device off. Above 0 it powers it on and sets the
  brightness.
- A pixel personality opens a `SegmentStream` and writes zones. A `basic` or
  `full` personality sends the `color`, `brightness` and `color_temp` roles.
- Compare against the last values sent, and send nothing where nothing changed.
- Send the current values once after 10 seconds of silence, because nothing
  acknowledges a LAN frame.
- Report `frames_superseded` per device.

**Done when** a static look puts no traffic on the network, and a fade reaches
the device at the rate the device file measured.

## 9. ArtPoll and ArtPollReply

Answer a poll, one reply for each group of 4 universes in the patch. Carry the
node name from the patch.

**Done when** a desk lists the node with no manual entry of its address.

## 10. Signal loss, and a device that fails

- `on_signal_loss`: `hold`, `black` or `off`, after the timeout.
- A device that becomes unreachable logs the failure, and every other device
  keeps running. Retry the stream with a backoff.

**Done when** a test pulls the sender away and the configured result happens,
and a test kills one simulated device and the others still receive frames.

## 11. Release

- An end-to-end test against `crates/sim`, with no hardware.
- `publish = true`, `packages/rust/crates/dmx/CHANGELOG.md`,
  `.github/workflows/dmx-release.yml` on the `dmx-vX.Y.Z` tag, and the row in
  the root `CHANGELOG.md` table. Follow `cli-release.yml`, which releases the
  other binary.
- A `README.md` for the crate, and the package in the root `README.md`.
- `features.md` and `roadmap.md` go to ✅.
- Delete this page.

## 12. The site

`dist/catalog.json` carries the channel table, and the devices page shows it.
Run `tools/qa-site.sh` after the catalog changes.

## What is not in this plan

The three open questions of [`dmx.md`](dmx.md): a device wider than 512
channels, the gradient channel, and the HTP merge. Each one needs a real rig
before it is designed. sACN is step 6 again with another parser, and nothing
else changes.
