# Fixtures

Real frames captured from hardware, used by the SDK tests.

```
lan-captures/<SKU>/<command>.json     # UDP payloads observed on 4001/4002/4003
ble-captures/<SKU>/<command>.txt      # BLE frames, hex
```

Every entry should be referenced from the matching `devices/<SKU>.yaml`
(`capture:` field) and, for an undocumented command, from
`docs/protocol/lan.md`.

## Redaction checklist

A capture is a recording of a home network. Go through this list before
committing one, every time. `tools/check-captures.sh` enforces most of it and
runs in CI, but it only catches the shapes it knows.

Replace, never delete: a capture that still parses and still reproduces the
behavior is the point. Substitute values of the same shape and the same
length — a MAC stays six bytes, an IPv4 address stays four — so offsets,
lengths and checksums still line up.

Use the same placeholders everyone else uses, so two captures of the same
command stay comparable:

| What | Where it appears | Replace with |
| ---- | ---------------- | ------------ |
| Device MAC / `device` | discovery reply, device file, config examples | `AA:BB:CC:DD:EE:FF` |
| A second and third device | multi-device captures | `11:22:33:44:55:66`, `99:88:77:66:55:44` |
| BLE address | `ble-captures/`, advertisement dumps | the same MAC placeholders |
| Device IP | discovery reply `ip`, packet headers | `192.0.2.10` |
| The capturing host's IP | packet headers, `bind` addresses | `192.0.2.2` |
| Router / gateway / DNS | pcap headers, network notes | `192.0.2.1` |
| The subnet itself | anywhere it is written out | `192.0.2.0/24` |
| Wi-Fi SSID | `ssid` keys, notes about the setup | `EXAMPLE-SSID` |
| BSSID | `bssid` keys | `AA:BB:CC:DD:EE:FF` |
| API key, account token, bearer token, `account_topic` | cloud captures, HTTP headers | `REDACTED` |
| Account e-mail or user id | cloud captures | `REDACTED` |
| Device nickname | `deviceName`, app screenshots, notes | `test-device` |

`192.0.2.0/24` is the RFC 5737 documentation range and is guaranteed never to be
routed. Use `198.51.100.0/24` or `203.0.113.0/24` when one capture needs a
second subnet.

Two values are protocol constants and stay as they are: the discovery multicast
group `239.255.255.250`, and the ports `4001` / `4002` / `4003`.

Things that are easy to miss:

- pcap files carry the frames you did not open — filter and re-export rather
  than trimming in a viewer;
- a device nickname often names the household ("Clara's desk"), and so does an
  SSID;
- the `sku` field stays, it is the model and not an identifier;
- screenshots attached to the pull request need the same treatment as the
  capture.

**A pull request carrying an unredacted capture is closed, not amended.** Git
keeps the blob after the fix, so the only remedy is a fresh branch with a clean
capture.

## Conformance vectors

```
golden/<mode>/<SKU>.json
```

Golden vectors: arguments in, exact bytes out. They are the contract between
implementations — the Rust core and its bindings must all produce the same
envelope and the same frame for the same call, and one that drifts fails here
before it reaches a device.

Each file holds:

- `vectors` — a `message` (the JSON envelope, compared structurally) and, for a
  raw-channel command, `frame_hex` (compared byte for byte);
- `errors` — calls that must fail, with the stable `code` they must fail with.
  Rejecting a clamped value is part of the contract, so it is tested like any
  other behavior.

Every vector carries a `source`. A frame taken from a real capture says so; one
worked out from the documented layout says that instead. The distinction
matters: only the first is evidence.

Run them with `cargo test` from [`../../packages/rust`](../../packages/rust).
