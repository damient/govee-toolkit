---
title: Configure
slug: configure
order: 3
description: The .env file the toolkit reads, the Wi-Fi credentials a device needs to join your network, and the API key cloud mode needs.
---

# Configure

The toolkit reads two files. `config.yaml` says which mode each device uses,
and [the modes page]({{base}}docs/modes/) covers that one. `.env` carries the
credentials, and this page covers it.

`lan` and `ble` need no credential to send a command. Read this page when you
put a device on your Wi-Fi network, or when you enable `cloud`.

## The `.env` file

A variable you export lasts as long as the shell. A `.env` file survives a new
one. The toolkit reads the first file it finds, in this order:

1. The file `--env-file` names, or the file `GOVEE_ENV_FILE` names. A file that
   is absent here is an error, because somebody asked for that file.
2. `.env` in the directory you work in, then in each directory above it. The
   search stops after the directory that holds `.git`, after your home
   directory, or at the root of the file system.
3. `~/.config/govee-toolkit/.env`.

Only `GOVEE_*` names are read, so a `.env` written for another project carries
nothing into the toolkit. A variable already in your environment wins over the
file, and `govee --no-env` reads no file at all.

`govee doctor` says which file the variables came from:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee doctor
variables from `/home/you/.config/govee-toolkit/.env`
no problem found</code></pre>
<button class="copy" type="button" data-copy="govee doctor">Copy</button>
</div>

## Wi-Fi

{{badge_lan}} — a device answers over `lan` once it is on your own Wi-Fi network. The Govee
Home app puts it there, and `govee provision` does the same over `ble`. Two
variables supply the network name and the password, so the password stays out
of your shell history:

```bash
GOVEE_WIFI_SSID=my-network
GOVEE_WIFI_PASSWORD=the password
```

<div class="terminal">
<pre><code><span class="prompt">$</span> govee provision AA:BB:CC:DD:EE:FF</code></pre>
<button class="copy" type="button" data-copy="govee provision AA:BB:CC:DD:EE:FF">Copy</button>
</div>

`--ssid` and `--password` take the same values on the command line, and
`--open` joins a network that has no password. The command needs a build that
carries Bluetooth: `cargo install govee-toolkit-cli --features ble`.

Four things to know before you run it:

- Use a 2.4 GHz network. No Govee device joins a 5 GHz one.
- The password travels in plaintext, with no key exchange. Anything in
  Bluetooth range while the command runs reads it.
- The device acknowledges the transfer, and the report says whether it
  accepted the credentials. It does not say the device joined the network:
  check the network afterwards.
- Close the vendor app first. It holds the one connection the radio accepts.

## The API key

{{badge_cloud}} — this mode goes through Govee's own API and needs a key.
[Start here]({{base}}docs/start/) says how to ask for one in the app. Give it
to the toolkit as `GOVEE_API_KEY`:

```bash
GOVEE_API_KEY=7c41b9d2-5a6e-4f08-9b31-2d84ec55af10
```

Never put the key in `config.yaml`. That file ends up in bug reports. The
toolkit never logs the key and never writes it to the device cache.
