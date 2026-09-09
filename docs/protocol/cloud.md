# Govee Cloud API (`cloud` mode)

`cloud` is one of the three modes a user can enable per device — see
[`../modes.md`](../modes.md). It is opt-in and never enabled implicitly.

It reaches a device from anywhere, at the cost of an internet round-trip: high
latency, rate limits and a reduced capability set. Use it for a device that is
not on the local network, not for anything latency-sensitive.

Nothing on this page was probed against a live account in this repository. It
describes the API Govee documents, and it says where a number is theirs rather
than a measurement.

## 1. Authentication

- **API key:** requested from the Govee Home app (profile → settings → "Apply
  for API key").
- **Header:** `Govee-API-Key: <key>`
- **Base URL:** `https://openapi.api.govee.com`

The key is user-supplied configuration. It is never required to use `lan` or
`ble` mode, and the SDK must start fine without one.

**Where it is stored:** the `GOVEE_API_KEY` environment variable, or a separate
file whose path the configuration names. Never
`~/.config/govee-toolkit/config.yaml` — that file ends up in bug reports. The
key is never logged and never written to the device cache. See
[`../security.md`](../security.md).

## 2. Endpoints

| Role | Method | Path |
| ---- | ------ | ---- |
| Device list | GET | `/router/api/v1/user/devices` |
| Control | POST | `/router/api/v1/device/control` |
| State | POST | `/router/api/v1/device/state` |

The device list answers with every device the account owns, wherever it is.
That list is the whole of discovery in this mode: there is no scan, and a
device the account does not own is unreachable.

## 3. The capability model

This API does not carry a command name. It names a **capability**, an
**instance** of it, and a **value**:

```json
{
  "requestId": "1",
  "payload": {
    "sku": "<model>",
    "device": "<MAC>",
    "capability": { "type": "<capability>", "instance": "<instance>", "value": 1 }
  }
}
```

The device files hold those names, under the `cloud:` command table — see
[`../../devices/schema.yaml`](../../devices/schema.yaml). The SDK builds the
capability object from the file and wraps it in the request; no capability name
is written in code.

Three properties of this model differ from the `lan` envelope:

- **A color travels as one integer**, `0xRRGGBB`, and not as three fields.
- **A state answer is a list.** Each entry carries an instance and a
  `state.value`. The device file says which instance answers into which
  argument, and the argument's role says which field of the reported status it
  fills.
- **`requestId` is echoed back.** Nothing correlates on it here: one request
  gets one answer on the same connection.

## 4. Rate limits

Govee documents two limits. Neither was confirmed against a live account here:

- **Per account:** 10 000 requests a day.
- **Per device:** 10 requests a minute.
- **Quota headers returned:** _TODO_ — confirm which headers the answers carry.

A refusal answers `429`, and its `Retry-After` header, when there is one, says
how long to wait.

Implementation consequence: `cloud` mode must **throttle** and coalesce
commands (do not relay a brightness slider on every tick). This is a property
of the mode, and applications must expect it. The transport keeps one request
every `min_interval` per device, and a command that must wait longer than
`max_wait` fails rather than sit in a queue nobody can see.

**The answer is the verification.** The API reports whether it accepted the
command, so this mode sends no status request after a write. A second request
would spend the quota to learn what the first one already said.

## 5. What only the cloud can do

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

A device file marks such a command `channel: iot`. The transport refuses one
rather than approximate it over HTTPS: this build carries the HTTPS channel
alone.

## 6. What the cloud cannot do

The cloud API does not expose the per-segment color channel that `lan` reaches,
nor its frame rate. A device reached in `cloud` mode is limited to power /
brightness / color.

When several modes are enabled and the SDK moves to `cloud`, a command outside
that set **fails explicitly**; it is never approximated.

<!-- TODO: detailed per-capability table, mode by mode -->
