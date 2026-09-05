# Modes

A **mode** is a way of talking to a device: `lan`, `ble` or `cloud`. None of
them is a fallback for another — they are three transports with different
trade-offs, and **the user chooses which ones to enable, per device**.

| Mode | Latency | Range | Capabilities | Requires |
| ---- | ------- | ----- | ------------ | -------- |
| `lan` | lowest | same network | full, including the undocumented segment channel; not internal scenes | LAN Control enabled in the Govee Home app |
| `ble` | low | Bluetooth range | partial, depends on SKU family | a BLE adapter on the host |
| `cloud` | highest (internet round-trip) | anywhere | reduced: power / brightness / color | a Govee API key, subject to rate limits |

Details per mode: [`protocol/lan.md`](protocol/lan.md),
[`protocol/ble.md`](protocol/ble.md), [`protocol/cloud.md`](protocol/cloud.md).

## Two levels

1. **What the device supports** — declared in `devices/<SKU>.yaml` under
   `modes:`. This is hardware fact, not a preference; it is the same for
   everyone and is not user-editable.
2. **What the user enables** — declared in the runtime configuration. Only a
   mode supported by the device can be enabled; enabling an unsupported one is a
   configuration error, reported at startup.

## Selecting modes per device

The user picks **one or several** modes per device, as an ordered list. Order is
preference order: the first entry is the preferred mode.

YAML, at `~/.config/govee-toolkit/config.yaml` — `$XDG_CONFIG_HOME` and
`GOVEE_CONFIG` both override it. Devices are keyed by the MAC they report in a
discovery reply, not by address: a DHCP lease renews and the device is at a
different one, still the same device.

```yaml
defaults:
  modes: [lan]              # applies to any device without an explicit entry

devices:
  "AA:BB:CC:DD:EE:FF":
    modes: [lan]            # single mode — strict, never switches
  "11:22:33:44:55:66":
    modes: [lan, ble]       # preferred lan, may switch to ble
  "99:88:77:66:55:44":
    modes: [cloud]          # remote device, cloud only
    name: "hallway"         # for logs and interfaces; never read as identity
```

A key the file does not define is refused rather than ignored: a misspelled
option that was silently dropped would read as a setting that did not work.

Two other sections are optional. `lan:` tunes the transport — scan window,
refresh interval, cache location, breaker thresholds. `catalog:` decides whether
`~/.config/govee-toolkit/devices/*.yaml` may replace the device files the build
shipped:

```yaml
catalog:
  local_devices: false      # opt-in, and off by default
```

It is off because a device file is a claim about a model, not about one unit.
Turning it on is the right move while probing a SKU that has not shipped yet;
every file that replaces one is logged, every run.

### The cloud API key does not live here

`cloud` mode needs a Govee API key. It is read from the `GOVEE_API_KEY`
environment variable, or from a separate file the configuration points at — one
the operator can lock down on its own.

It is **never** stored in `config.yaml`. That file gets pasted into bug reports.
The key is also never logged and never written to the device cache. See
[`security.md`](security.md).

### Single mode

The device is only ever reached over that mode. If it becomes unreachable, the
command **fails and is reported as failed**; nothing switches implicitly. Use
it when predictability matters more than availability (a show, a DMX rig, a
latency-sensitive setup).

### Several modes

The SDK uses the first available mode in the list and may switch to the next
one, driven by the per-device circuit breaker:

- States: `OK` | `DEGRADED` | `DOWN`, tracked **per device and per mode**.
- Three consecutive failures take a mode to `DEGRADED`: the SDK moves to the
  next enabled mode, and after a 30 s cooldown lets one command through to probe
  the preferred one. Two consecutive answers bring it back to `OK`.
- Six take it to `DOWN`, which is the same shape with a five-minute cooldown —
  a mode silent for minutes is not worth probing every 30 seconds.
- The thresholds and both cooldowns are the defaults; `lan:` in the
  configuration tunes them.
- The mode is chosen from the breaker state already known, never from a fresh
  timeout on each call: a fresh timeout would cost the fast path a round-trip.

Switching is always observable: the SDK reports which mode served each command,
and every mode transition is an event the application can subscribe to.

## Capability differences between modes

Modes are not interchangeable, and neither is a superset of the other: `cloud`
does not expose the undocumented segment channel `lan` reaches, and `lan` does
not reach the internal scenes or the per-segment brightness `cloud` carries
([`protocol/cloud.md`](protocol/cloud.md)). When several modes are enabled and
the SDK switches, a command unsupported by the active mode **fails explicitly**
rather than being silently approximated.

`devices/<SKU>.yaml` declares capabilities per mode, so an application can know
in advance what it loses on a switch — and, for each capability a mode does not
reach, whether that is a boundary of the transport or a question nobody has
answered yet. See [`compatibility.md`](compatibility.md).

## Defaults

`lan` alone is the default: it is the only mode that never leaves the local
network. Everything else is opt-in.
