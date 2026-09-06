# Compatibility

Which Govee devices work, in which [modes](modes.md), and how far.

> 🚧 One device verified so far, the **H61A0**: `lan` mode fully exercised,
> including the undocumented segment channel, and `ble` mode everywhere except
> Wi-Fi provisioning, which has never been sent to a device. The other 270
> models on Govee's list are untested rather than unsupported — they need
> someone with the hardware to confirm them.

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
| **partial** | Only some capabilities are reachable — the device file lists which |
| **none** | Not reachable in this mode |
| **?** | Not tested yet. The device file spells this `unknown` |

`none` and `?` are not the same answer. `none` says somebody established the
hardware cannot do it; `?` says nobody looked. A failed probe and an
unimplemented feature look identical from outside, so `?` stays until someone
probes it, and enabling an unprobed mode is allowed — that is how it gets
probed.

A mode marked `full` or `partial` says what the hardware supports, not what is
enabled: the user chooses which modes to turn on, per device. See
[`modes.md`](modes.md).

## Why a capability is out of reach

`partial` says a mode falls short of the hardware; the device file says of what,
and why. Each capability a mode does not reach is listed under
`modes.<mode>.unreachable` with one of three reasons:

| Reason | Meaning |
| ------ | ------- |
| `transport` | Established that this transport does not carry it. Per-segment brightness over `lan` is one: the channel carries color only, and no command will change that |
| `unimplemented` | The transport carries it, but this device file declares no command for it yet. Work left to do, not a boundary |
| `unprobed` | Nobody checked whether this mode reaches it. The default |

The three are not interchangeable. `transport` is a claim about the protocol and
needs the evidence any other claim needs — the section of
[`protocol/`](protocol/) that establishes it. `unprobed` is what an unanswered
question looks like, and it stays until someone answers it.

On a mode that is `full` or `partial`, every capability the hardware has is
either reached or listed here with a reason; `cargo test` fails on one that is
neither. A mode left `unknown` owes no answer, since nobody probed it.

## Support by SKU

A SKU that looks like another is not the same device: lengths differ, and with
them segment counts and native resolution. Candidate aliases stay declared as
such in the device file until someone verifies them.

<!-- generated: support-by-sku -->
| SKU | Family | Name | `lan` | `ble` | `cloud` | Verified |
| --- | ------ | ---- | ----- | ----- | ------- | -------- |
| [H6114](../devices/H6114.yaml) | rgb-car-strip | RGB Car LED Strip Lights | none | full | none | ✅ 2026-09-06 |
| [H61A0](../devices/H61A0.yaml) | rgbic-neon-rope | 3m RGBIC LED Neon Rope Lights | partial | partial | partial | ✅ 2026-09-06 |
<!-- /generated -->

## Capabilities by SKU

<!-- generated: capabilities-by-sku -->
| SKU | brightness | color | colortemp | music | power | scenes | segment_brightness | segments |
| --- | ---------- | ----- | --------- | ----- | ----- | ------ | ------------------ | -------- |
| H6114 | ✅ | ✅ | — | ✅ | ✅ | ✅ | — | — |
| H61A0 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
<!-- /generated -->

Capabilities are hardware facts; what is reachable depends on the active mode.
The columns are the capabilities the device files declare, so one no file
declares has no column.
Where the undocumented `razer` channel is implemented, `lan` reaches per-segment
color beyond the 10 zones the Govee app exposes, but neither internal scenes nor
per-segment brightness. On the H61A0 `ble` reaches per-segment brightness and a
narrower per-segment color — fifteen zones by mask, against the unit's 42
individually addressable LEDs.

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

Verified on H61A0 only — confirm before generalizing:

- **Internal scenes and DIY scenes are cloud-only.** They travel over MQTT to
  AWS IoT, not over UDP. See [`protocol/lan.md`](protocol/lan.md) § 2.5.
- **Per-segment brightness does not travel over `lan`.** Brightness is global
  there. `ble` carries it, per zone and by mask, and so does the cloud.
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
