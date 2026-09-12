---
title: When it does not work
slug: troubleshooting
order: 6
faq: true
description: Nothing answers the scan, a command does nothing, a value is refused — what each one means.
---

# When it does not work

## Nothing answers `govee scan`

Check these, in order:

1. **LAN Control is on.** Govee Home app, your device, settings, **LAN
   Control**. Every command over Wi-Fi needs that switch.
2. **One network.** Your computer and the light must be on the same network.
   Guest Wi-Fi and some mesh setups keep them apart, even in one house.
3. **The model carries the switch.** Not every model does. Check Govee's list.
4. **Your firewall.** Discovery uses multicast on UDP. A firewall that drops it
   makes every device invisible.

Run `govee doctor`. It reports what it can see of your configuration.

## A command reports success and nothing happens

The firmware ignores a command it does not know, and it answers nothing either
way. A failed probe and an unimplemented feature look the same from outside.

If the command comes from `send`, the device file may declare something this
model does not carry. If it comes from a verb, report it — that is a real
finding.

## A value is refused

The toolkit refuses a value outside the range the device file declares. It does
not adjust it. The firmware would clamp the value in silence and report
success, and you would believe a setting was applied when it was not.

`govee describe <device>` prints the range for each command.

## Brightness 0 does not turn the light off

On a verified model, `brightness: 0` is clamped to 1 by the firmware. Use
`govee off` to turn the device off.

## Per-segment brightness does not work over Wi-Fi

The segment channel on `lan` carries color only. Brightness is global there.
`ble` carries brightness per zone, and `cloud` carries it per zone. This is a
property of the transport, not a gap in the code.

## The Bluetooth scan finds nothing

A Govee device stops advertising once something connects to it. If the Govee
app, or another program, holds the connection, close it and scan again.

## After a firmware update, behavior changed

Firmware updates change behavior with no notice. A device file records the
firmware its numbers were measured on. Re-verify after an update, and report
what changed.

## Report a problem

Open an issue on [GitHub]({{repo}}/issues) with your model number, your
operating system, and the output of `govee doctor`. Redact your network name
and your API key.
