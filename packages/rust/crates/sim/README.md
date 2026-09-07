# govee-toolkit-sim

A fake Govee device, so the transports can be tested without hardware. There are
two, one per wire, and they are separate devices.

`Simulator` is the `lan` device on UDP. It answers discovery and status requests
the way real firmware does, including the parts that make the protocol awkward:
replies carry no request id, and nothing is acknowledged.

`ble::BleDevice` is the GATT device, and `ble::BleAdapter` is the radio that
finds it. It takes one connection, checks the frame length and the BCC, and
answers on the notify characteristic. Beside the faults the `lan` device has, it
can refuse the connection and it can stall under a burst, which is what a real
firmware does when it is written to too fast.

Both play the wire, not the firmware: neither interprets a payload, and what a
read answers is set by the test.

Unofficial and not affiliated with Govee.

[govee-toolkit]: https://github.com/damient/govee-toolkit
