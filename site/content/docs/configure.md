---
title: Configure
slug: configure
order: 3
description: The config.yaml that names your devices, the .env file the toolkit reads, the Wi-Fi credentials a device needs to join your network, and the API key cloud mode needs.
---

# Configure

The toolkit reads two files. `config.yaml` names your devices and says which
mode each one uses. `.env` carries the credentials.

`lan` and `ble` need no credential to send a command. Read this page when you
put a device on your Wi-Fi network, or when you enable `cloud`.

## The `config.yaml` file

The toolkit does not create this file. Without it, every device uses `lan`.
Write it yourself at `~/.config/govee-toolkit/config.yaml`, or point
`GOVEE_CONFIG` or `--config` at another path.

Each device is keyed by the MAC it reports. `govee scan` lists them:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee scan
AA:BB:CC:DD:EE:FF  H6159  -  [lan]
11:22:33:44:55:66  H619A  -  [lan]</code></pre>
<button class="copy" type="button" data-copy="govee scan">Copy</button>
</div>

Give a device a name, and the modes it may use:

```yaml
defaults:
  modes: [lan]

devices:
  "AA:BB:CC:DD:EE:FF":
    name: "kitchen"
  "11:22:33:44:55:66":
    name: "desk"
    modes: [lan, ble]
```

Every command takes the name in place of the MAC. Write `name:kitchen` where a
name reads as a MAC or a SKU.
Two devices can share a name: a command that drives one device then refuses
it, and `govee devices` and `govee identify` take both.

<div class="terminal">
<pre><code><span class="prompt">$</span> govee brightness kitchen 40</code></pre>
<button class="copy" type="button" data-copy="govee brightness kitchen 40">Copy</button>
</div>

The toolkit refuses a key it does not know, so a misspelling fails at start.
`govee doctor` reports what is wrong with the file. The
[modes page]({{base}}docs/modes/) describes `modes:` and the other sections.

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
