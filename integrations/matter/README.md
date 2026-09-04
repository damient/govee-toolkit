# Matter bridge

Exposes Govee devices as Matter endpoints, so they are reachable from any
Matter controller — Home Assistant, Apple Home, Google Home, Alexa, SmartThings
— through a single integration.

First among the integrations: one bridge covers every ecosystem, where the
other plugins cover one platform each. Starts once the core SDK
(`packages/python` and `packages/node`) is stable.

## Scope

- Matter endpoints for the capabilities every controller understands: power,
  brightness, color, color temperature.
- The undocumented LAN scenes and segments are **not** expressible in the
  Matter data model today. They stay reachable through the SDKs, the playground
  and the first-party integrations; the bridge exposes what Matter can carry.
- Devices are driven in the modes the user enabled for them — the bridge does
  not enable a mode on its own. See [`../../docs/modes.md`](../../docs/modes.md).

## Open questions

- Bridge topology: one bridge node exposing many Govee devices, vs one Matter
  node per device? _TODO_
- Which SDK does the bridge consume — `packages/python` or `packages/node`?
  _TODO_
- Commissioning flow and credential storage. _TODO_
- Which Matter SDK / stack to build on. _TODO_
- Can segments be approximated with several endpoints per physical device?
  _TODO_
