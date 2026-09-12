# Compatibility

Which Govee devices work, in which [modes](modes.md), and how far.

> 🚧 The table below lists every verified device, and how far each mode goes on
> it. `lan` mode is fully exercised, including the undocumented segment channel,
> and so is `ble` mode, Wi-Fi provisioning included. A hidden network is the one
> case nobody provisioned. The other models on Govee's list are untested rather
> than unsupported — they need someone with the hardware to confirm them.

The authoritative data lives in [`../devices/`](../devices/), one YAML file per
SKU or SKU family. This page is the human-readable view of it — the YAML wins on
any disagreement.

The two tables below are generated from the device files by
`cargo run -p xtask -- compat`, and CI fails when they drift. Everything else on
this page is written by hand.

## Support levels

| Level | Meaning |
| ----- | ------- |
| **full** | Every capability the hardware has is reachable in this mode |
| **capped** | Every capability the transport carries is reachable, and the transport carries less than the hardware has. A boundary of the transport |
| **partial** | A capability is out of reach because this device file does not reach it yet. Work left |
| **none** | Not reachable in this mode |
| **?** | Not tested yet. The device file spells this `unknown` |

`capped` and `partial` are not the same answer. `capped` says the mode goes as
far as the transport goes: nobody can do better over it, on any device. Every
RGBIC device is `capped` over `lan`, because `lan` carries no music command and
no per-segment brightness. `partial` says this device file falls short of what
the transport carries, so somebody can close the gap. Both list each capability
out of reach, with the reason, under `modes.<mode>.unreachable`.

The level follows from those reasons, and `cargo test` checks it: a mode whose
every unreachable capability is `transport` is `capped`, and one that names an
`unimplemented` or `unprobed` capability is `partial`.

`none` and `?` are not the same answer. `none` says somebody established the
hardware cannot do it; `?` says nobody looked. A failed probe and an
unimplemented feature look identical from outside, so `?` stays until someone
probes it, and enabling an unprobed mode is allowed — that is how it gets
probed.

A mode marked `full`, `capped` or `partial` says what the hardware supports, not
what is enabled: the user chooses which modes to turn on, per device. See
[`modes.md`](modes.md).

## Why a capability is out of reach

`capped` and `partial` both say a mode falls short of the hardware; the device
file says of what, and why. Each capability a mode does not reach is listed
under `modes.<mode>.unreachable` with one of three reasons:

| Reason | Meaning |
| ------ | ------- |
| `transport` | Established that this transport does not carry it. Per-segment brightness over `lan` is one: the channel carries color only, and no command will change that |
| `unimplemented` | The transport carries it, but this device file declares no command for it yet. Work left to do, not a boundary |
| `unprobed` | Nobody checked whether this mode reaches it. The default |

The three are not interchangeable. `transport` is a claim about the protocol and
needs the evidence any other claim needs — the section of
[`protocol/`](protocol/) that establishes it. `unprobed` is what an unanswered
question looks like, and it stays until someone answers it.

On a mode that is `full`, `capped` or `partial`, every capability the hardware
has is either reached or listed here with a reason; `cargo test` fails on one that is
neither. A mode left `unknown` owes no answer, since nobody probed it.

## Support by SKU

A SKU that looks like another is not the same device: lengths differ, and with
them segment counts and native resolution. Candidate aliases stay declared as
such in the device file until someone verifies them.

<!-- generated: support-by-sku -->
| SKU | Family | Name | `lan` | `ble` | `cloud` | Verified |
| --- | ------ | ---- | ----- | ----- | ------- | -------- |
| [H6008](../devices/H6008.yaml) | rgbww-bulb | Smart LED Bulb RGBWW | full | capped | full | ✅ 2026-09-12 |
| [H6114](../devices/H6114.yaml) | rgb-car-strip | RGB Car LED Strip Lights | none | full | none | ✅ 2026-09-07 |
| [H61A0](../devices/H61A0.yaml) | rgbic-neon-rope | 3m RGBIC LED Neon Rope Lights | capped | full | full | ✅ 2026-09-10 |
<!-- /generated -->

## Capabilities by SKU

<!-- generated: capabilities-by-sku -->
| SKU | brightness | color | colortemp | music | power | segment_brightness | segments |
| --- | ---------- | ----- | --------- | ----- | ----- | ------------------ | -------- |
| H6008 | ✅ | ✅ | ✅ | — | ✅ | — | — |
| H6114 | ✅ | ✅ | — | ✅ | ✅ | — | — |
| H61A0 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
<!-- /generated -->

Capabilities are hardware facts; what is reachable depends on the active mode.
The columns are the capabilities the device files declare, so one no file
declares has no column.
Where the undocumented `razer` channel is implemented, `lan` reaches per-segment
color beyond the 10 zones the Govee app exposes, but not per-segment brightness.
Where it is implemented, `ble` reaches per-segment brightness and a narrower
per-segment color — fifteen zones by mask, fewer than the LEDs the unit
addresses individually. The device file carries the counts.

## Prerequisites per mode

- **`lan`** — "LAN Control" must be enabled for the device in the Govee Home
  app, and the device must be on the same network as the host. Not every SKU
  exposes the switch: Govee's own list of models that do is mirrored in
  [`lan-supported-devices.md`](lan-supported-devices.md).
- **`ble`** — a Bluetooth adapter on the host, and the device within range. One
  connection at a time: a connected device stops advertising, so a scan run
  while another app holds the link finds nothing.
- **`cloud`** — a Govee API key, and the device registered to that account.

## Known limitations

Verified on the devices in the table above — confirm before generalizing to
another:

- **Per-segment brightness does not travel over `lan`.** Brightness is global
  there. `ble` carries it per zone and by mask, and `cloud` per zone.
- **Nothing is ever rejected.** Out-of-range values are clamped in silence and
  unknown commands are ignored with no error — a failed probe looks exactly like
  an unsupported feature.
- **`brightness: 0` does not turn a device off**, it is clamped to 1.
- **Segment count and native resolution depend on the length of the unit**, not
  only on the SKU. A measured value for a 5 m strip says nothing about a 3 m one.
- **Firmware updates change behavior without notice.** Re-verify after one; a
  device file records the firmware its numbers were taken on.

## My device is not listed

First check [`lan-supported-devices.md`](lan-supported-devices.md): if Govee
lists it, `lan` mode should work and only needs someone to verify and declare
it.

Adding a device is mostly filling a YAML file and attaching a capture — see
[`../devices/README.md`](../devices/README.md), and
[`../CONTRIBUTING.md`](../CONTRIBUTING.md) for the workflow.
