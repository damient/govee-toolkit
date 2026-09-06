# Govee BLE protocol (`ble` mode)

`ble` is one of the three modes a user can enable per device — see
[`../modes.md`](../modes.md). It is opt-in and never enabled implicitly.

It works without the device being on the network, within Bluetooth range, and
requires a BLE adapter on the host. Capability coverage depends on the SKU
family and is generally narrower than `lan`.

## Status

Nothing probed yet: no device has been sniffed over BLE. Everything below is a
template.

`packages/rust/src/ble/` carries the transport, behind the cargo feature of the
same name. It is written to the shape below and to a 20-byte frame, and nothing
in it is confirmed: the GATT UUIDs, the frame length, the advertised-name
prefixes and the way a reply is matched to its request each carry that note in
the code, and each is a TODO until a capture backs it. No device file declares a
`ble` command yet.

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
