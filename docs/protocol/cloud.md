# Govee Cloud API (`cloud` mode)

`cloud` is one of the three modes a user can enable per device — see
[`../modes.md`](../modes.md). It is opt-in and never enabled implicitly.

It reaches a device from anywhere, at the cost of an internet round-trip: high
latency, rate limits and a reduced capability set. Use it for a device that is
not on the local network, not for anything latency-sensitive.

The three endpoints below, and the five entries a light carries, were probed
against a live account. The rate limits were not: they are Govee's own figures,
and this page says so where it states one.

## 1. Authentication

- **API key:** requested from the Govee Home app (profile → settings → "Apply
  for API key").
- **Header:** `Govee-API-Key: <key>`
- **Base URL:** `https://openapi.api.govee.com`

The key is user-supplied configuration. It is never required to use `lan` or
`ble` mode, and the SDK must start fine without one.

**Where it is stored:** the `GOVEE_API_KEY` environment variable, a `.env`
file, or a separate file whose path the configuration names. Never
`~/.config/govee-toolkit/config.yaml` — that file ends up in bug reports. The
key is never logged and never written to the device cache. See
[`../security.md`](../security.md).

In this repository, development keys live in a gitignored `.env` at the root,
which the SDK finds on its own. See
[`../../CONTRIBUTING.md`](../../CONTRIBUTING.md).

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

A state answer also carries capabilities that this SDK models nothing of, each
with an empty value. A capability that the state endpoint names is **not**
evidence that the control endpoint writes it. Send the instance to the control
endpoint to establish that, and **watch the device**: this API answers
`success` for a capability it accepts, and acceptance is not application.

What a device keeps between two commands is in [`state.md`](state.md), and
applies to this mode as it does to the other two.

## 4. Rate limits

Govee documents two limits. Neither was confirmed against a live account here:

- **Per account:** 10 000 requests a day.
- **Per device:** 10 requests a minute.
- **Quota headers returned: none.** A device list answer and a state answer
  both carried `date` and `content-type`, and no quota header of any kind. A
  client cannot read how much quota it has left; it must count its own
  requests.

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

## 5. The segment channel over this mode

`cloud` mode carries power, brightness, color and color temperature, plus the
state read. It also reaches the segment channel on a device whose account list
declares the segment instances. The control endpoint takes them, and the
device applies them:

- **Per-segment color.** One capability that carries an array of zone indices
  and one packed color.
- **Per-segment brightness.** The same capability, under a second instance.
  `ble` reaches this too. `lan` does not: that channel carries color only.

Three limits hold here:

- **The zone range is the API's own.** Read the accepted array length, and the
  range of a zone index, off the account list. That range can differ from the
  zones the Govee app exposes and from the addressable LEDs.
- **Which end of the strip holds zone 0 is a property of the unit.** Paint one
  zone at each end of the range to establish it, and record what you saw in the
  device file. Do not carry the answer from one unit to another.
- **There is no segment stream.** This mode takes one request every few
  seconds, so it cannot carry a moving pattern. Use `lan` or `ble` for that.

A device file that declares no command for an instance marks the capability
`unreachable: unimplemented` under `cloud`. When several modes are enabled and
the SDK moves to `cloud`, a command outside the carried set **fails
explicitly**; it is never approximated.

## 6. Music over this mode

This mode reaches music on a device whose account list declares the music
capability. The device listens on its own microphone: this mode sends no audio
and carries no stream.

The capability takes a structured value, and the account list declares its
fields per device:

- **The effect**, as an enum. The list gives each value a name. Those
  identifiers belong to this API. They are not the sub-mode codes the `ble`
  music frame takes, and nothing maps one set onto the other.
- **The sensitivity**, as a percentage.
- **Two optional fields**, an automatic-color flag and one packed color.

Three limits hold here:

- **A declared field is not an applied field.** Send each optional field and
  watch the device (§3). A device file declares the fields that one unit
  applied, and its `verified:` block says what the others did.
- **This mode reads back no music state.** The state endpoint answers an empty
  value for the music instance while the device plays music.
- **A name is not a rendering.** The account list names each effect, and that
  name is the API's own label. Match a name to what the device renders before
  you repeat it.

<!-- TODO: detailed per-capability table, mode by mode -->
