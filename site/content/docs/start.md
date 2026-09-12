---
title: Start here
slug: start
order: 1
description: What you need before a Govee light answers your own computer, and the one switch that makes it reachable.
---

# Start here

This page covers what a Govee light needs before your own computer can reach
it. You do not have to write code to follow it.

## What you need

1. A Govee light.
2. A computer on the same Wi-Fi network as the light.
3. The Govee Home app on your phone, once, to turn on one switch.

## Turn on LAN Control

Open the Govee Home app, select your device, open its settings, then turn on
**LAN Control**. This switch lets your own network reach the device. Without
it, nothing on this site works over Wi-Fi.

Not every model has the switch. Govee publishes the list of models that do, and
[the devices page]({{base}}devices/) tells you what is verified here.

## API for cloud (optional)

`cloud` mode goes through Govee's own API, so it needs an API key and the
device registered to that account. Ask for the key in the Govee Home app:
profile, then settings, then **Apply for API key**.

Give the key to the toolkit through the `GOVEE_API_KEY` environment variable:

```bash
export GOVEE_API_KEY="the key from the app"
```

Never put the key in `config.yaml`. That file ends up in bug reports. The
toolkit never logs the key and never writes it to the device cache.

## Where to go next

- [Install]({{base}}docs/install/) — the command line, the Rust crate, and
  where Python and Node.js stand.
- [Reference]({{base}}reference/) — every command and every method, with an
  example in each language.
- [Modes]({{base}}docs/modes/) — Wi-Fi, Bluetooth or the cloud, and how you
  choose.
- [When it does not work]({{base}}docs/troubleshooting/) — nothing answers the
  scan, a command does nothing, a value is refused.
