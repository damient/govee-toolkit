# Govee Cloud API (`cloud` mode)

`cloud` is one of the three modes a user can enable per device — see
[`../modes.md`](../modes.md). It is opt-in and never enabled implicitly.

It reaches a device from anywhere, at the cost of an internet round-trip: high
latency, rate limits and a reduced capability set. Use it for a device that is
not on the local network, not for anything latency-sensitive.

## Authentication

- **API key:** requested from the Govee Home app (profile → settings → "Apply
  for API key").
- **Header:** `Govee-API-Key: <key>`
- **Base URL:** _TODO_ (confirm the current API version)

The key is user-supplied configuration. It is never required to use `lan` or
`ble` mode, and the SDK must start fine without one.

**Where it is stored:** the `GOVEE_API_KEY` environment variable, or a separate
file whose path the configuration names. Never
`~/.config/govee-toolkit/config.yaml` — that file ends up in bug reports. The
key is never logged and never written to the device cache. See
[`../security.md`](../security.md).

## Endpoints

| Role | Method | Path |
| ---- | ------ | ---- |
| Device list | GET | _TODO_ |
| Control | POST | _TODO_ |
| State | GET | _TODO_ |

## Rate limits

- **Global quota:** _TODO_ (documented by Govee, to confirm)
- **Per device:** _TODO_
- **Quota headers returned:** _TODO_

Implementation consequence: `cloud` mode must **throttle** and coalesce commands
(do not relay a brightness slider on every tick). This is a property of the
mode, and applications should expect it.

## What only the cloud can do

Two features are **cloud-only** and cannot be reached over LAN, wherever this
has been checked:

- **Internal scenes and DIY scenes** — published over MQTT to AWS IoT, with an
  account topic and a transaction id. The `pt`, `ptReal`, `ptIotOp` and `bulb`
  commands belong to that channel; probing them over UDP stays silent, because
  the command exists but not on that transport.
- **Per-segment brightness.** Over LAN, brightness is global — the segment
  channel carries color only.

The manufacturer's scene library therefore needs `cloud` mode enabled, account
and internet included. See [`lan.md`](lan.md) § 2.5.

## What the cloud cannot do

The cloud API does not expose the per-segment color channel that `lan` reaches,
nor its frame rate. A device reached in `cloud` mode is limited to power /
brightness / color.

When several modes are enabled and the SDK moves to `cloud`, a command outside
that set **fails explicitly**; it is never approximated.

<!-- TODO: detailed per-capability table, mode by mode -->
