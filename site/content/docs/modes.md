---
title: Modes
slug: modes
order: 5
description: Wi-Fi, Bluetooth or the cloud — where each path reaches your light from, and what each one needs.
---

# Modes

A command travels to your light over one of three paths. You choose which paths
to allow, for each device.

| Mode | Speed | It reaches the device from |
| ---- | ----- | -------------------------- |
| {{badge_lan}} | fastest | the same Wi-Fi |
| {{badge_ble}} | fast | Bluetooth range, no Wi-Fi needed |
| {{badge_cloud}} | slowest | anywhere with internet |

## What each mode needs

{{badge_lan}} — turn on "LAN Control" for the device in the Govee Home app, and put
the device and your computer on the same network. Guest Wi-Fi and some mesh
setups separate them.

{{badge_ble}} — a Bluetooth adapter on your computer, and the device within range.
One connection at a time: a connected device stops advertising, so a scan run
while another app holds the link finds nothing.

{{badge_cloud}} — a Govee API key, and the device registered to that account.
The API is throttled, and it carries no animation.

## Where the truth lives

What a mode reaches depends on the model. [The devices page]({{base}}devices/)
says what each one answers.
