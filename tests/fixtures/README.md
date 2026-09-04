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
