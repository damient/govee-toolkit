# govee-toolkit-sim

A fake Govee device on UDP, so the transport can be tested without hardware.

It answers discovery and status requests the way real firmware does, including
the parts that make the protocol awkward: replies carry no request id, and
nothing is acknowledged.

Unofficial and not affiliated with Govee.

[govee-toolkit]: https://github.com/damient/govee-toolkit
