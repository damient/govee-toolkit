# Playground

Interactive test tool. A dev tool, not a published library, hence `apps/` and
not `packages/`.

- **Backend** (`server/`): Node server consuming `packages/node` directly.
  Exposes a local API plus a WebSocket for real-time state push.
- **Frontend** (`web/`): plain HTML/JS, no framework.

## Planned UI

- Detected device list with a LAN_OK / LAN_DEGRADED / LAN_DOWN state badge
- Per device: power toggle, brightness slider, color picker, effects/scenes
  dropdown (including undocumented commands)
- Bottom log: every command sent, with timestamp and measured latency
- **Raw payload field**: send a custom JSON command straight to a device, to
  try a discovery before formalizing it in `devices/*.yaml`

The web playground must stay usable standalone, without Electron.

<!-- TODO: run instructions -->
