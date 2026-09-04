# Art-Net / DMX bridge

Listens for Art-Net, maps DMX channels to a Govee device + segment, and pushes
over LAN via `packages/node`.

## Planned mapping

<!-- TODO: mapping config format (universe / start channel → device + segment) -->

| DMX channel | Meaning |
| ----------- | ------- |
| _TODO_ | |

## Latency

Art-Net can push up to 44 frames per second. The bridge must coalesce and
rate-limit before sending to a device — see `docs/protocol/lan.md`.

<!-- TODO -->
