# Govee BLE protocol (`ble` mode)

`ble` is one of the three modes a user can enable per device — see
[`../modes.md`](../modes.md). It is opt-in and never enabled implicitly.

It works without the device being on the network, within Bluetooth range, and
requires a BLE adapter on the host. Capability coverage depends on the SKU
family and is generally narrower than `lan`.

## Status

Nothing probed yet — the LAN work covered what was needed, and no device has
been sniffed over BLE. Everything below is a template waiting for someone with
hardware and a BLE sniffer.

One data point carried over from the LAN work: the raw LAN channel uses a
variable-length dialect prefixed `0xBB`, **distinct** from the 20-byte `0x33`
frames used over BLE. Frames do not port between the two.

## General

- **GATT service:** _TODO_
- **Write characteristic:** _TODO_
- **Notify characteristic:** _TODO_
- **Frame size:** _TODO_ (typically 20 bytes)
- **Checksum:** _TODO_ (XOR of the preceding bytes?)

## Generic frame format

```
| header | opcode | sub-cmd | payload ... | checksum |
```

_TODO: confirm per SKU family._

## Notes per SKU family

### Family _TODO_

| Function | Opcode | Payload | Verified on |
| -------- | ------ | ------- | ----------- |
| Power | _TODO_ | _TODO_ | _TODO_ |
| Brightness | _TODO_ | _TODO_ | _TODO_ |
| Color | _TODO_ | _TODO_ | _TODO_ |

<!-- TODO: duplicate this section per family -->

## Captures

Real frames live under `tests/fixtures/ble-captures/<sku>/`.
