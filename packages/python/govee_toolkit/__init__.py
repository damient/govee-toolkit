"""govee-toolkit — Python SDK.

TODO: public API. Planned modules:
  discovery.py     — multicast scan + persistent cache
  modes/lan.py     — reused UDP socket, fire-and-verify
  modes/ble.py     — BLE mode (opt-in)
  modes/cloud.py   — Cloud mode (opt-in, throttled)
  selector.py      — per-device enabled modes + preference order
  breaker.py       — per-device, per-mode circuit breaker (OK / DEGRADED / DOWN)
  registry.py      — loads devices/*.yaml
"""
