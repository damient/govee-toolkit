# Modes

A **mode** is a way of talking to a device: `lan`, `ble` or `cloud`. None of
them is a fallback for another — they are three transports with different
trade-offs, and **the user chooses which ones to enable, per device**.

| Mode | Latency | Range | Capabilities | Requires |
| ---- | ------- | ----- | ------------ | -------- |
| `lan` | lowest | same network | full, including undocumented scenes/segments | LAN Control enabled in the Govee Home app |
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

```yaml
# TODO: config file location and exact format to be settled with the SDKs
defaults:
  modes: [lan]              # applies to any device without an explicit entry

devices:
  "AA:BB:CC:DD:EE:FF":
    modes: [lan]            # single mode — strict, never switches
  "11:22:33:44:55:66":
    modes: [lan, ble]       # preferred lan, may switch to ble
  "99:88:77:66:55:44":
    modes: [cloud]          # remote device, cloud only
```

### Single mode

The device is only ever reached over that mode. If it becomes unreachable, the
command **fails and is reported as failed**; nothing switches implicitly. Use
it when predictability matters more than availability (a show, a DMX rig, a
latency-sensitive setup).

### Several modes

The SDK uses the first available mode in the list and may switch to the next
one, driven by the per-device circuit breaker:

- States: `OK` | `DEGRADED` | `DOWN`, tracked **per device and per mode**.
- 2–3 consecutive timeouts on a mode → that mode goes `DEGRADED`, the SDK moves
  to the next enabled mode for a cooldown (e.g. 30 s), then retries the
  preferred one.
- The mode is chosen from the breaker state already known, never from a fresh
  timeout on each call: a fresh timeout would cost the fast path a round-trip.

Switching is always observable: the SDK reports which mode served each command,
and every mode transition is an event the application can subscribe to.

## Capability differences between modes

Modes are not interchangeable: `cloud` does not expose the undocumented scenes
and segments reachable over `lan`. When several modes are enabled and the SDK
switches, a command unsupported by the active mode **fails explicitly** rather
than being silently approximated.

`devices/<SKU>.yaml` declares capabilities per mode, so an application can know
in advance what it loses on a switch.

## Defaults

`lan` alone is the default: it is the only mode that never leaves the local
network. Everything else is opt-in.
