# Compatibility

Which Govee devices work, in which [modes](modes.md), and how far.

> 🚧 One device verified so far, the **H61A0**: `lan` mode fully exercised,
> including the undocumented segment channel. The other 270 models on Govee's
> list are untested rather than unsupported — most should work with someone to
> confirm them. Contributions welcome.

The authoritative data lives in [`../devices/`](../devices/), one YAML file per
SKU or SKU family. This page is the human-readable view of it — the YAML wins on
any disagreement.

<!-- TODO: generate the tables below from devices/*.yaml and check them in CI,
     so this page cannot drift from the data. -->

## Support levels

| Level | Meaning |
| ----- | ------- |
| **full** | Every capability the hardware has is reachable in this mode |
| **partial** | Only some capabilities are reachable — the device file lists which |
| **none** | Not reachable in this mode |
| **?** | Not tested yet |

A mode marked `full` or `partial` says what the hardware supports, not what is
enabled: the user chooses which modes to turn on, per device. See
[`modes.md`](modes.md).

## Support by SKU

A SKU that looks like another is not the same device: lengths differ, and with
them segment counts and native resolution. Candidate aliases stay declared as
such in the device file until someone verifies them.

| SKU | Family | Name | `lan` | `ble` | `cloud` | Verified |
| --- | ------ | ---- | ----- | ----- | ------- | -------- |
| [H61A0](../devices/H61A0.yaml) | rgbic-neon-rope | RGBIC LED Neon Rope Lights | full | ? | partial | ✅ `lan`, incl. segment channel |

## Capabilities by SKU

| SKU | power | brightness | color | colortemp | scenes | segments | sensors |
| --- | ----- | ---------- | ----- | --------- | ------ | -------- | ------- |
| H61A0 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — |

Capabilities are hardware facts. What is reachable depends on the active mode.
Where the undocumented `razer` channel is implemented, `lan` reaches per-segment
color at a resolution well beyond the 10 zones the Govee app exposes — but
**not** internal scenes or per-segment brightness, which are cloud-only whatever
the device.

## Prerequisites per mode

- **`lan`** — "LAN Control" must be enabled for the device in the Govee Home
  app, and the device must be on the same network as the host. Not every SKU
  exposes the switch: Govee's own list of models that do is mirrored in
  [`lan-supported-devices.md`](lan-supported-devices.md).
- **`ble`** — a Bluetooth adapter on the host, and the device within range.
- **`cloud`** — a Govee API key, and the device registered to that account.

## Known limitations

Verified on H61A0, likely to hold more widely — confirm before generalizing:

- **Internal scenes, DIY scenes and per-segment brightness are cloud-only.** They
  travel over MQTT to AWS IoT, not over UDP. Over `lan`, brightness is global.
  See [`protocol/lan.md`](protocol/lan.md) § 2.4.
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


That means nobody has tested it yet — the odds are good it works. Adding one is
mostly filling a YAML file and attaching a capture — see
[`../devices/README.md`](../devices/README.md), and
[`../CONTRIBUTING.md`](../CONTRIBUTING.md) for the workflow.
