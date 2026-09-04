# Fixtures

Real frames captured from hardware, used by the SDK tests and replayed by
`tools/device-simulator/`.

```
lan-captures/<SKU>/<command>.json     # UDP payloads observed on 4001/4002/4003
ble-captures/<SKU>/<command>.txt      # BLE frames, hex
```

Every entry should be referenced from the matching `devices/<SKU>.yaml`
(`capture:` field) and, for an undocumented command, from
`docs/protocol/lan.md`.

Do not commit anything containing an API key, an account token or a MAC you
would rather not publish — anonymize before committing.

## Conformance vectors

```
golden/<mode>/<SKU>.json
```

Golden vectors: arguments in, exact bytes out. They are the contract between
implementations — the Rust core, its bindings and any hand-written port must
all produce the same envelope and the same frame for the same call, and a port
that drifts fails here before it reaches a device.

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
