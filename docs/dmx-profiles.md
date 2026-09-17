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
| SKU | Name | `basic` | `full` | `pixel` | `pixel-native` |
| --- | ---- | ------- | ------ | ------- | -------------- |
| [H6008](../devices/H6008.yaml) | Smart LED Bulb RGBWW | 4 | 6 | — | — |
| [H6022](../devices/H6022.yaml) | Table Lamp 2 | 4 | 6 | 397 | 397 |
| [H6114](../devices/H6114.yaml) | RGB Car LED Strip Lights | — | — | — | — |
| [H61A0](../devices/H61A0.yaml) | 3m RGBIC LED Neon Rope Lights | 4 | 6 | 31 | 127 |
<!-- /generated -->

The first channel of every personality is the dimmer. Slot 0 powers the device
off, and every other slot powers it on and sets the brightness.

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

Red, green and blue go out as they are, and the control channel is not scaled.
