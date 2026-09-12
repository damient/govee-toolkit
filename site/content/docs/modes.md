---
title: Modes
slug: modes
order: 4
description: Wi-Fi, Bluetooth or the cloud — what each path carries, and why the toolkit never changes path on its own.
---

# Modes

A command travels to your light over one of three paths. You choose which paths
to allow, for each device.

| Mode | Speed | It reaches the device from | What it carries |
| ---- | ----- | -------------------------- | --------------- |
| `lan` | fastest | the same Wi-Fi | everything, segments included |
| `ble` | fast | Bluetooth range, no Wi-Fi needed | it depends on the model |
| `cloud` | slowest | anywhere with internet | on/off, brightness, color, segments. Throttled, and no animation |

## One enabled mode means one mode

Allow several modes and the toolkit switches between them. Allow one and it
stays on that one: if the device is out of reach, the command fails and says
so.

This matters more than it sounds. A toolkit that fell back to the cloud after a
Wi-Fi timeout would turn a 5 millisecond command into a 2 second one, and you
would never know why. A toolkit that answered a segment animation with a plain
color change would report success for something your light never showed.
Neither happens here.

## What each mode needs

**`lan`** — turn on "LAN Control" for the device in the Govee Home app, and put
the device and your computer on the same network. Guest Wi-Fi and some mesh
setups separate them.

**`ble`** — a Bluetooth adapter on your computer, and the device within range.
One connection at a time: a connected device stops advertising, so a scan run
while another app holds the link finds nothing.

**`cloud`** — a Govee API key, and the device registered to that account.

## Health

Each device carries a health state per mode: `OK`, `DEGRADED` or `DOWN`. The
toolkit decides from the state it already holds, so a command does not wait for
a fresh timeout to learn that a mode is down.

## Where the truth lives

What a mode reaches on a given model is declared in that model's file in
`devices/`. [The devices page]({{base}}devices/) is the readable view of those
files.
