# Security model

What this SDK protects, what it does not, and what that means for the network
you run it on. To report a vulnerability, see [`../SECURITY.md`](../SECURITY.md)
— the process is there and is not repeated here.

## The LAN protocol is unauthenticated and unencrypted

This is a property of the devices, not of the SDK, and nothing here can change
it:

- **No authentication.** A control frame on UDP port `4003` carries no key, no
  token and no signature. Anyone who can send a UDP packet to a device can
  control it.
- **No encryption.** Payloads are plaintext JSON, and the raw segment frames are
  plaintext bytes. Anyone who can see the traffic can read every command.
- **No integrity.** There is a checksum in the raw frame, but it detects
  corruption, not tampering.
- **Discovery replies are unauthenticated.** Discovery is a multicast request to
  `239.255.255.250:4001`; any host on the segment can answer on port `4002` and
  claim any MAC, IP and SKU. A device identity learned from discovery is a
  **claim, not proof**.

The practical consequence: **anyone on the same layer-2 network can control your
devices**, and can impersonate one. The trust boundary is the network segment,
not the SDK.

`ble` has the same shape — commands go to a device in radio range, without a
session the SDK controls. `cloud` is the exception: it is TLS to Govee's API,
authenticated with an API key, and the trust boundary is the account.

## What the SDK does

- **Validates every argument against the device file, and never clamps.** An
  out-of-range value is an error before a byte is sent. The firmware clamps in
  silence, so an SDK that clamped too would report success for a value the
  device never applied.
- **Refuses unknown configuration keys** rather than ignoring them: a misspelled
  option that was silently dropped reads as a setting that did not work.
- **Validates `schema_version` on every device file.** An unknown version is a
  typed error, not a file read as if it were the version the build understands.
- **Never panics in library code.** No `unsafe`, no `panic` / `unwrap` /
  `expect`; malformed input from the network is an error value, not a crash.
  A hostile discovery reply is parsed the same way a legitimate one is, and a
  reply that does not parse is dropped.
- **Treats the device cache as a hint.** A cached address that has stopped
  answering is the circuit breaker's problem; a corrupt or foreign-versioned
  cache is discarded rather than trusted.
- **Binds nothing.** The SDK opens sockets to talk to devices; it exposes no
  listening service, no HTTP endpoint and no remote control surface of its own.

## What the SDK does not defend against

- **A hostile host on the same network.** It can control the devices directly,
  without going through this SDK at all.
- **A spoofed discovery reply.** The SDK cannot tell a device from a host
  claiming to be one. It can only note that the identity it has is the one it
  was told.
- **Replay or interception of commands.** There is nothing to replay-protect
  with.
- **A device file you did not write.** `payload:` and `frame:` are executable:
  the core builds bytes straight from them. A device file decides what gets sent
  to your hardware.

## What an operator can do

- **Segment the network.** Put the devices on their own VLAN or IoT SSID, and
  allow only the host running the SDK to reach it. This is the one measure that
  actually addresses the threat, because the protocol offers no other boundary.
- **Do not expose ports 4001–4003 across networks.** Multicast discovery is
  link-local by design; forwarding it is exporting the trust boundary.
- **Treat the configuration file as untrusted-adjacent.** It is a plain YAML
  file in `$XDG_CONFIG_HOME/govee-toolkit/`; anyone who can write it decides
  which modes are enabled and, if `catalog.local_devices` is on, which device
  files are used. Keep it owned by the user running the SDK and not
  world-writable.
- **Leave `catalog.local_devices` off** unless you are probing a SKU. It is off
  by default; when on, every file that replaces a shipped one is logged, every
  run.

## Credentials

The only credential this project handles is the Govee cloud API key, used by
`cloud` mode ([`protocol/cloud.md`](protocol/cloud.md)). It is never needed for
`lan` or `ble`, and the SDK starts fine without one.

- **It comes from the `GOVEE_API_KEY` environment variable, or from a separate
  file whose path the configuration names** — a file the operator can give
  restrictive permissions of its own.
- **It never lives in `~/.config/govee-toolkit/config.yaml`.** People paste that
  file into bug reports and issues; a key in it leaks the day someone asks for
  help.
- **It is never logged.** Not at any level, not in an error, not in an event.
- **It is never written to the device cache**, or to any other file the SDK
  writes.

A key that has leaked is revoked and reissued in the Govee Home app; the SDK
holds nothing that can invalidate it.

## Reverse engineering

The undocumented commands here were found by decompiling the vendor's desktop
app and capturing its own traffic, for **interoperability**. Nothing in this
repository bypasses an authentication mechanism — there is none on the LAN
protocol to bypass. What may and may not be committed as a result is in
[`../CONTRIBUTING.md`](../CONTRIBUTING.md#legal-and-provenance).
