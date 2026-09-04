# Device simulator

Fake Govee device (UDP + BLE), so the SDKs and the playground can be tested
without hardware and CI can run transport tests.

## Planned scope

- Answers multicast discovery on `239.255.255.250:4001`, replies on `4002`
- Accepts commands on `4003` and keeps in-memory state
- Replays the frames in `tests/fixtures/` for a given SKU
- Fault injection: latency, packet loss, full silence — to exercise the
  per-mode circuit breaker (`OK` → `DEGRADED` → `DOWN`), single-mode failure
  reporting, and switching between several enabled modes

<!-- TODO: run instructions -->
