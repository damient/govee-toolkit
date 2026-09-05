# govee-lan

The `lan` transport for [govee-toolkit]: UDP discovery, a device cache, one
shared socket and a per-device circuit breaker.

It carries the bytes [`govee-core`](https://crates.io/crates/govee-core)
produces, for one mode. It never falls back to another one: an unreachable
device is an error, and what to do about it is the caller's decision.

Unofficial and not affiliated with Govee.

[govee-toolkit]: https://github.com/damient/govee-toolkit
