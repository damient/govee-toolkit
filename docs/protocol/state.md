# Device state between commands

What a device keeps, and what one command changes in another command's
settings. These behaviors belong to the firmware, so they apply over `lan`,
`ble` and `cloud` alike. A mode changes how a command travels, not what the
device stores.

Each fact here was measured on a physical unit. The measurement, the SKU and
the firmware version stay in that unit's device file, under `verified:`.

## 1. A power off keeps the color setting

A device stores its color setting through a power off. The `power` command with
`on` = 1 restores the setting that the last `color` or `colortemp` command
wrote.

`power` on is therefore **not** a full state write. A caller that needs a known
state must send the color and the brightness after it. Do not assume a default.

## 2. Color and white temperature exclude each other

A `color` command clears the white temperature. A `colortemp` command clears
the color. A device holds one of the two, never both.

A status read shows this: the field that the last command did not write reports
zero. A zero temperature means color mode, and it is not a temperature of 0 K.

## 3. A read reports the stored setting

A read answers with the value that a write stored. It does not answer with what
the device renders. The two differ when the firmware accepts a value that it
renders nothing for.

A read that echoes a value is therefore not evidence that the device applied
it. To establish that a device applies a value, observe the device.

## 4. A per-segment brightness outlives the commands after it

The firmware keeps the per-segment brightness. A whole-device brightness
write, a color write and a power cycle all keep it. Only another per-segment
brightness write changes it.

A per-segment color behaves the other way: the next whole-device color write
clears it and paints every zone.

The level is invisible until a static color lights that zone, so a unit that
holds a low level on one zone looks like a hardware fault. A caller that dims
a zone must put the level back. Over `cloud` nothing reads the levels: the
state endpoint answers an empty value for the segment instances.

## 5. The zone interpolation is one setting, shared by every mode

The firmware keeps one interpolation setting. It decides whether the rendering
blends between two zones painted differently, and the blend wraps from the last
zone back to the first, so one lit zone at one end also lights the other.

Every mode reaches the same setting, by a different route. A mode carries it
inside the frame that paints, or in a command of its own. Where a mode carries
it alone, a caller changes the setting and leaves the colors as they are. Where
a mode carries it in the painting frame, a caller cannot: nothing outside the
firmware holds what the device shows, so the colors cannot be painted again
under the other setting.

The setting is not a fade over time. Two colors sent one after the other cut to
each other under either value.

## 6. A command over one mode can end the raw segment channel

The raw segment channel of `lan` renders until something else takes the
rendering back. A command over another mode is such a thing: the device then
shows the color that its stored setting holds, and the zones the channel
painted are gone.

Re-arm the channel and paint again to get the zones back. Do not hold a paint
over one mode and a setting over another: the second ends the first.
