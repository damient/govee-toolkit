# DMX profiles

The DMX channel table of each device file, for an operator who patches a desk.
[`dmx.md`](dmx.md) states how the table is derived and what each slot does.

The two tables below are generated from [`../devices/`](../devices/) by
`cargo run -p xtask -- dmx`, and CI fails when they drift. Everything else on
this page is written by hand.

`govee-dmx profile <SKU>` prints the same table for one device, with every
channel and its offset. Use the command to patch one fixture, and this page to
compare devices.

## Personalities

A cell holds how many channels the personality takes. `—` says the device file
gives the bridge no way to drive one channel of it: a capability the hardware
does not have, a capability `lan` does not reach, or a measurement nobody made.
A personality wider than 512 channels is an error when the patch loads, and it
is never truncated to fit.

<!-- generated: dmx-personalities -->
| SKU | Name | `full` | `segment` | `pixel` |
| --- | ---- | ------ | --------- | ------- |
| [H6008](../devices/H6008.yaml) | Smart LED Bulb RGBWW | 6 | — | — |
| [H6022](../devices/H6022.yaml) | Table Lamp 2 | 6 | 48 | 399 |
| [H6114](../devices/H6114.yaml) | RGB Car LED Strip Lights | — | — | — |
| [H61A0](../devices/H61A0.yaml) | 3m RGBIC LED Neon Rope Lights | 6 | 48 | 129 |
<!-- /generated -->

Channel 1 and channel 2 are the same on every personality and every model.
Channel 1 is the dimmer: slot 0 powers the device off, and every other slot
powers it on and sets the brightness. Channel 2 is the mode channel.

A device whose every zone is one addressable LED serves `pixel` alone, because
`segment` would lay out the same table. Where the device file declares
`capabilities.segments.groups`, `segment` lays out that many zones instead, and
the bridge paints each one over its own run of LEDs.

## Scaled channels

A DMX slot holds 0 to 255. The dimmer and the white channel scale into the
range the device file declares. The step count says how many distinct values
the 255 carrying slots reach on that device.

A step count below 255 quantizes: the desk sends a smooth fade, and the device
applies steps. That is the resolution of the hardware, and the operator must
know it before the show.

<!-- generated: dmx-scaled-channels -->
| SKU | Dimmer | Dimmer steps | White temperature | White steps |
| --- | ------ | ------------ | ----------------- | ----------- |
| H6008 | 1 to 100 | 100 | 2700 to 6500 | 255 |
| H6022 | 0 to 100 | 101 | 2700 to 6500 | 255 |
| H6114 | — | — | — | — |
| H61A0 | 1 to 100 | 100 | 2000 to 9000 | 255 |
<!-- /generated -->

Red, green and blue scale into the pair the `lan` color command declares for
each component. Every device file declares the whole byte today, so each slot
goes out as it is. The mode channel is not scaled.
