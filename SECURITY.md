# Security policy

This project speaks an unauthenticated, unencrypted UDP protocol to consumer
hardware, and parses packets that arrive from anywhere on the local network. The
threat model is stated here rather than implied, because most of what looks like
a vulnerability in `lan` mode is the protocol working as the firmware
implements it.

## Reporting a vulnerability

Use GitHub private security advisories, and nothing else:

<https://github.com/damient/govee-toolkit/security/advisories/new>

Do not open a public issue, a pull request or a discussion for a security
report. There is no security email address; the advisory is the channel.

The project has a single maintainer and nothing has shipped yet, so these
windows are best-effort rather than a commitment:

| Step | Target |
| ---- | ------ |
| Acknowledgement of the report | 7 days |
| First assessment (in scope, severity, plan) | 30 days |
| Fix on `main`, advisory published | 90 days from the report |

Coordinated disclosure is preferred. If a report goes unanswered past the
acknowledgement window, publishing is a reasonable next step.

## Supported versions

Nothing is released. `main` is the only supported ref: a fix lands there and
there is no earlier version to backport it to.

A supported-versions table appears here with the first release.

## In scope

- **Memory-unsafety or a panic reachable from a malformed network packet.** A
  panic in library code is a bug in this project, not an error path — the
  workspace lints ban `unwrap`, `expect` and `panic`, and `unsafe_code` is
  forbidden. A discovery reply, a status reply or a raw frame that crashes a
  parser is a valid report.
- **A secret in a log, an error message or a committed capture.** Cloud API
  keys, tokens, and anything in `tests/fixtures/lan-captures/` that identifies
  an account rather than a device.
- **Path traversal or an arbitrary write through the device catalog or the
  config loader** — a device file, an overlay directory or a config path that
  reads or writes outside where it should.
- **A dependency advisory `cargo deny check` should have caught**, including a
  gap in `deny.toml` that let one through.

## Out of scope

### The LAN protocol itself

The protocol has no authentication and no encryption. Anyone on the same network
can control the devices, can read every command sent to them, and can answer a
discovery request pretending to be a device. This SDK reproduces the protocol
the firmware speaks; it cannot add authentication to it, and a report that the
protocol lacks it will be closed as working as intended. See
[`docs/protocol/lan.md`](docs/protocol/lan.md).

Treat `lan` mode as trusting the local network, and segment the network if that
trust is not warranted.

### Govee firmware and the Govee cloud

Vulnerabilities in the devices themselves or in Govee's cloud services are for
Govee to receive, not this project. Report them to Govee. Findings that only
work by exploiting a firmware defect will not be published here, and no
firmware exploit will be added to this repository — the reverse engineering
here exists for interoperability.

## Credential handling

The Govee cloud API key is the only secret this project handles: it never lives
in the configuration file, never appears in a log or an error, and `lan` and
`ble` need no credential at all. The rules and the reasons are in
[`docs/security.md`](docs/security.md#credentials), and the cloud side of them
in [`docs/protocol/cloud.md`](docs/protocol/cloud.md).
