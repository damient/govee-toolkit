# Devices with LAN control (official Govee list)

Govee publishes the list of models that expose the **LAN Control** switch, along
with the LAN API specification, in its WLAN guide:

<!-- generated: lan-source -->
**Source:** <https://app-h5.govee.com/user-manual/wlan-guide>
**Retrieved:** 2026-09-22
**Entries:** 276
<!-- /generated -->

> ⚠️ This page is a **copy** of Govee's list, kept for offline reference and to
> make the SKUs searchable and linkable. The data is
> [`lan-supported-devices.json`](lan-supported-devices.json), and the table
> below and the model list of the site are generated from it. Govee updates its guide
> without notice: the source above wins, and a model missing here may have
> been added after the retrieval date. Re-check the source before concluding a
> device is unsupported.

Being on this list means the device can expose the LAN switch — it says nothing
about which *undocumented* commands it accepts. What this project has actually
verified is tracked separately in [`compatibility.md`](compatibility.md), and
declared per SKU in [`../devices/`](../devices/).

A model with no device file resolves to
[`../devices/listed/govee-lan.yaml`](../devices/listed/govee-lan.yaml): power,
brightness and color over `lan` and `cloud`, not verified on that model. The
`Definition` column says which file describes each model.

## Enabling LAN control

1. Add the device to the Govee Home app and make sure its Wi-Fi is connected.
2. Turn on the **LAN** switch on the device's Settings page.

If the device is on the list below but the LAN switch is not visible, Govee's
instruction is: unplug the device, power it on again, wait about 30 minutes and
check again; if it still does not appear, contact Govee support.

## Supported models

Sorted by SKU. Govee's own list is grouped by product family and unsorted; the
order here is for lookup. The category is the group that Govee's list puts the
model in.

<!-- generated: lan-models -->
| SKU | Model | Category | Definition |
| --- | ----- | -------- | ---------- |
| `H1232` | Govee Ceiling Light Pro | Ceiling Light | listed, not verified |
| `H1252` | Govee Ceiling Light Pro | Ceiling Light | listed, not verified |
| `H1270` | Govee Ceiling Light Ultra | Ceiling Light | listed, not verified |
| `H12D0` | Govee Sky Ceiling Light | Ceiling Light | listed, not verified |
| `H1310` | Govee Ceiling Fan with Lights | Ceiling Fan Light | listed, not verified |
| `H1370` | Govee Bladeless Ceiling Fan | Ceiling Fan Light | listed, not verified |
| `H1401` | Govee Smart Bulb | LED Bulbs | listed, not verified |
| `H14A1` | Govee Smart Bulb | LED Bulbs | listed, not verified |
| `H14B1` | Govee Smart Bulb | LED Bulbs | listed, not verified |
| `H14C2` | Govee Edison Bulb | LED Bulbs | listed, not verified |
| `H1630` | Govee Lantern Floor Lamp | Floor Lamps | listed, not verified |
| `H16B0` | Govee Floor Lamp 3 | Floor Lamps | listed, not verified |
| `H16C0` | Govee Floor Lamp 3 Lite | Floor Lamps | listed, not verified |
| `H1771` | Govee Wallwash Table Lamp | Table Lamps | listed, not verified |
| `H1811` | Govee Bubble Projector Light | Projector Light | listed, not verified |
| `H1880` | Govee Tri-Zone Galaxy Light Projector | Projector Light | listed, not verified |
| `H1A42` | Govee LED Strip Light 2 | RGBIC Strip Lights | listed, not verified |
| `H1A43` | Govee LED Strip Light 2 | RGBIC Strip Lights | listed, not verified |
| `H1A44` | Govee LED Strip Light 2 | RGBIC Strip Lights | listed, not verified |
| `H1A45` | Govee LED Strip Light 2 | RGBIC Strip Lights | listed, not verified |
| `H1AB1` | Govee COB Strip Light 2 | RGBIC Strip Lights | listed, not verified |
| `H1AB2` | Govee COB Strip Light 2 | RGBIC Strip Lights | listed, not verified |
| `H1AB3` | Govee COB Strip Light 2 | RGBIC Strip Lights | listed, not verified |
| `H1B6A` | Govee Strip Light with Tower Kit | RGBIC Strip Lights | listed, not verified |
| `H2800` | Govee Monitor Light Bar | Monitor Lights | listed, not verified |
| `H3010` | Govee Outdoor String Lights 3 | String Lights | listed, not verified |
| `H3011` | Govee Outdoor String Lights 3 | String Lights | listed, not verified |
| `H3012` | Govee Outdoor String Lights 3 | String Lights | listed, not verified |
| `H3013` | Govee Outdoor String Lights 3 | String Lights | listed, not verified |
| `H30D0` | Govee Outdoor Filament String Lights | String Lights | listed, not verified |
| `H30D1` | Govee Outdoor Filament String Lights | String Lights | listed, not verified |
| `H3200` | Govee Outdoor Spotlights Lite | Landscape Lighting | listed, not verified |
| `H3401` | Govee Outdoor Deck Lights 2 | Landscape Lighting | listed, not verified |
| `H3500` | Govee Outdoor Pathway Lights 2 Lite | Landscape Lighting | listed, not verified |
| `H3501` | Govee Outdoor Pathway Lights 2 Lite | Landscape Lighting | listed, not verified |
| `H3510` | Govee Outdoor Cylinder Pathway Lights | Landscape Lighting | listed, not verified |
| `H3511` | Govee Outdoor Cylinder Pathway Lights | Landscape Lighting | listed, not verified |
| `H3700` | Govee Outdoor Classic Wall Light | Landscape Lighting | listed, not verified |
| `H3751` | Govee Outdoor Wall Light Pro | Landscape Lighting | listed, not verified |
| `H3860` | Govee Holiday Pattern Projector Light | Projector Light | listed, not verified |
| `H3A51` | 30m Permanent Outdoor Lights 2 Pro | Landscape Lighting | listed, not verified |
| `H3A52` | 45m Permanent Outdoor Lights 2 Pro | Landscape Lighting | listed, not verified |
| `H3A53` | 60m Permanent Outdoor Lights 2 Pro | Landscape Lighting | listed, not verified |
| `H6003` | Wi-Fi RGBWW LED Bulb | Discontinued | listed, not verified |
| `H6004` | Wi-Fi+BLE RGBWW LED Bulb | LED Bulbs | listed, not verified |
| `H6006` | Wi-Fi+BLE RGBWW LED Bulb | LED Bulbs | listed, not verified |
| `H6008` | Wi-Fi+BLE RGBWW LED Bulb | LED Bulbs | [`H6008`](../devices/H6008.yaml) |
| `H6009` | Wi-Fi+BLE RGBWW LED Bulb | LED Bulbs | listed, not verified |
| `H600A` | Wi-Fi+BLE RGBWW LED Bulb | LED Bulbs | listed, not verified |
| `H6010` | Wi-Fi+BLE RGBWW LED Bulb | LED Bulbs | listed, not verified |
| `H601A` | 6inch Smart RGBWW LED Downlight | Downlight | listed, not verified |
| `H601B` | 4inch Smart RGBWW LED Downlight | Downlight | listed, not verified |
| `H601C` | 6inch Recessed Downlight (Can Light) | Downlight | listed, not verified |
| `H6020` | Govee Table Lamp 2 Pro x Sound by JBL | Table Lamps | listed, not verified |
| `H6022` | Govee Table Lamp 2 | Table Lamps | [`H6022`](../devices/H6022.yaml) |
| `H6038` | Govee Wall Sconce | Wall Lights | listed, not verified |
| `H6039` | Govee Wall Sconce | Wall Lights | listed, not verified |
| `H6042` | Govee TV Light Bar 2 | Discontinued | listed, not verified |
| `H6043` | Govee TV Light Bars 2 | Discontinued | listed, not verified |
| `H6046` | Govee RGBIC TV Light Bars | Table Lamps | listed, not verified |
| `H6047` | Govee RGBIC Gaming Light Bars | Table Lamps | listed, not verified |
| `H6048` | Govee RGBIC Gaming Light Bars Pro | Table Lamps | listed, not verified |
| `H6051` | Govee Table Lamp Lite | Discontinued | listed, not verified |
| `H6052` | Govee RGBICWW Table Lamp | Discontinued | listed, not verified |
| `H6056` | Govee RGBICWW Light Bars | Table Lamps | listed, not verified |
| `H6059` | Govee Night Light Mini | Discontinued | listed, not verified |
| `H6061` | Govee Glide Hexa Light Panels | Wall Lights | listed, not verified |
| `H6062` | Govee Glide Wall Light | Discontinued | listed, not verified |
| `H6063` | Govee Gaming Wall Light | Wall Lights | listed, not verified |
| `H6065` | Govee Glide Y Lights | Wall Lights | listed, not verified |
| `H6066` | Govee Glide Hexa Pro Light Panels | Discontinued | listed, not verified |
| `H6067` | Govee Glide Tri Light Panels | Discontinued | listed, not verified |
| `H6069` | Govee Mini Panel Lights | Wall Lights | listed, not verified |
| `H606A` | Govee Glide Hexa Light Panel Ultra | Wall Lights | listed, not verified |
| `H6072` | Govee RGBICWW Floor Lamp | Floor Lamps | listed, not verified |
| `H6073` | Smart RGB LED Floor Lamp | Discontinued | listed, not verified |
| `H6076` | Govee Floor Lamp (H6076A) | Floor Lamps | listed, not verified |
| `H6078` | Govee RGBICWW Cylinder Floor Lamp | Floor Lamps | listed, not verified |
| `H607C` | Govee RGBICWW Floor Lamp 2 | Floor Lamps | listed, not verified |
| `H6087` | Govee RGBIC Fixture Lights | Discontinued | listed, not verified |
| `H6088` | Govee RGBIC Cube Wall Sconces | Wall Lights | listed, not verified |
| `H608A` | 5m String Downlights | Wall Lights | listed, not verified |
| `H6093` | Star Light Projector (Aurora) | Projector Light | listed, not verified |
| `H6094` | Star Light Projector (Ocean Wave) | Projector Light | listed, not verified |
| `H6095` | Star Light Projector (Nebula) | Projector Light | listed, not verified |
| `H609D` | Govee Galaxy Light Projector 2 Pro | Projector Light | listed, not verified |
| `H60A0` | 12inch Ceiling Light | Discontinued | listed, not verified |
| `H60A1` | 12inch Ceiling Light | Ceiling Light | listed, not verified |
| `H60A4` | Govee Square Ceiling Light | Ceiling Light | listed, not verified |
| `H60A6` | Govee Ceiling Light Pro | Ceiling Light | listed, not verified |
| `H60B0` | Govee Uplighter Floor Lamp | Floor Lamps | listed, not verified |
| `H60B1` | Govee Torchiere Floor Lamp | Floor Lamps | listed, not verified |
| `H60B2` | Govee Tree Floor Lamp | Floor Lamps | listed, not verified |
| `H60B3` | Govee Uplighter Floor Lamp | Floor Lamps | listed, not verified |
| `H60C1` | Govee Pendant Light | Pendant Light | listed, not verified |
| `H610A` | Govee Glide Lively Wall Light | Wall Lights | listed, not verified |
| `H610B` | Govee Glide Music Wall Light | Discontinued | listed, not verified |
| `H6110` | 2*5m MultiColor with Alexa | RGB Strip Lights | listed, not verified |
| `H6117` | 2*5m RGBIC with Alexa | RGBIC Strip Lights | listed, not verified |
| `H6141` | 5m Smart MultiColor Strip Light | Discontinued | listed, not verified |
| `H6143` | 5m RGBIC Strip Light | Discontinued | listed, not verified |
| `H6144` | 2*5m RGBIC Strip Light | Discontinued | listed, not verified |
| `H6159` | 5m MultiColor with Alexa | RGB Strip Lights | listed, not verified |
| `H615A` | 5m RGB Strip Light with Alexa | RGB Strip Lights | listed, not verified |
| `H615B` | 10m RGB Strip Light with Alexa | RGB Strip Lights | listed, not verified |
| `H615C` | 15m RGB Strip Light with Alexa | RGB Strip Lights | listed, not verified |
| `H615D` | 20m RGB Strip Light with Alexa | RGB Strip Lights | listed, not verified |
| `H615E` | 30m RGB Strip Light with Alexa | RGB Strip Lights | listed, not verified |
| `H6163` | 5m RGBIC with Alexa | Discontinued | listed, not verified |
| `H6167` | 2.4m RGBIC TV Backlight | TV Backlights | listed, not verified |
| `H616C` | 10m RGBIC Outdoor Strip Lights 2 | Strip Lights | listed, not verified |
| `H616D` | 2*7.5m RGBIC Outdoor Strip Lights 2 | Strip Lights | listed, not verified |
| `H616E` | 2*10m RGBIC Outdoor Strip Lights 2 | Strip Lights | listed, not verified |
| `H6173` | 2*10m Outdoor RGBIC LED Strip Lights | Strip Lights | listed, not verified |
| `H6176` | 30m RGBIC Outdoor Strip Lights | Strip Lights | listed, not verified |
| `H6182` | Wi-Fi MultiColor TV Strip Light | TV Backlights | listed, not verified |
| `H618A` | 5m RGBIC Basic Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H618C` | 10m RGBIC Basic Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H618E` | 2*10m RGBIC Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H618F` | 2*15m RGBIC LED Strip Light With Alexa | RGBIC Strip Lights | listed, not verified |
| `H618G` | 2*20m RGBIC LED Strip Light with Alexa | RGBIC Strip Lights | listed, not verified |
| `H619A` | 5m RGBIC Pro Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H619B` | 7.5m RGBIC Pro Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H619C` | 10m RGBIC Pro Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H619D` | 2*7.5m RGBIC Pro Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H619E` | 2*10m RGBIC Pro Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H619Z` | 3m RGBIC Pro Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H61A0` | 3m RGBIC LED Neon Rope Lights | Neon Rope Light | [`H61A0`](../devices/H61A0.yaml) |
| `H61A1` | 2m RGBIC LED Neon Rope Lights | Neon Rope Light | listed, not verified |
| `H61A2` | 5m RGBIC LED Neon Rope Lights | Neon Rope Light | listed, not verified |
| `H61A3` | 4m RGBIC LED Neon Rope Lights | Neon Rope Light | listed, not verified |
| `H61A5` | 10m RGBIC LED Neon Rope Lights | Neon Rope Light | listed, not verified |
| `H61A8` | 10m RGBIC Outdoor Neon Rope Light | Neon Rope Light | listed, not verified |
| `H61A9` | 20m RGBIC Outdoor Neon Rope Light | Neon Rope Light | listed, not verified |
| `H61B1` | 5m RGBICW LED Strip Light with cover | RGBIC Strip Lights | listed, not verified |
| `H61B3` | 5m RGBIC LED Strip Light with Cover | RGBIC Strip Lights | listed, not verified |
| `H61B6` | 10m RGBIC LED Strip Light with Cover | RGBIC Strip Lights | listed, not verified |
| `H61B8` | Govee Strip Light with Skyline Kit | RGBIC Strip Lights | listed, not verified |
| `H61B9` | Govee Strip Light with Skyline Kit | RGBIC Strip Lights | listed, not verified |
| `H61BA` | 5m RGBICW LED Strip Light | Discontinued | listed, not verified |
| `H61BC` | 10m RGBICW LED Strip Light | Discontinued | listed, not verified |
| `H61BE` | 2*10m RGBICW LED Strip Light | Discontinued | listed, not verified |
| `H61C2` | 2m RGBIC Neon Rope Light for desks | Neon Rope Light | listed, not verified |
| `H61C3` | 3m RGBIC Neon Rope Light for gaming desk | Neon Rope Light | listed, not verified |
| `H61C5` | 5m RGBIC Neon Rope Light for desks | Neon Rope Light | listed, not verified |
| `H61D3` | 3m Govee Neon Rope Light 2 | Neon Rope Light | listed, not verified |
| `H61D5` | 5m Govee Neon Rope Light 2 | Neon Rope Light | listed, not verified |
| `H61D6` | 10m Govee Neon Rope Light 2 | Neon Rope Light | listed, not verified |
| `H61E0` | Govee LED Strip Light M1 | RGBIC Strip Lights | listed, not verified |
| `H61E1` | Govee LED Strip Light M1 | RGBIC Strip Lights | listed, not verified |
| `H61F2` | Govee Strip Light 2 Pro | RGBIC Strip Lights | listed, not verified |
| `H61F5` | Govee Strip Light 2 Pro | RGBIC Strip Lights | listed, not verified |
| `H61F6` | Govee Strip Light 2 Pro | RGBIC Strip Lights | listed, not verified |
| `H6350` | Sphere Net Lights | Holiday Lights | listed, not verified |
| `H6351` | Sphere Net Lights | Holiday Lights | listed, not verified |
| `H6630` | Govee Gaming Pixel Light | Pixel Light | listed, not verified |
| `H6631` | Govee Gaming Pixel Light | Pixel Light | listed, not verified |
| `H6671` | Govee RGBWIC TV Backlight 2 | TV Backlights | listed, not verified |
| `H6672` | Govee RGBWIC TV Backlight 2 | TV Backlights | listed, not verified |
| `H6800` | Christmas Tree Lights | Holiday Lights | listed, not verified |
| `H6810` | 240LED Net Lights | Holiday Lights | listed, not verified |
| `H6811` | 480LED Net Lights | Holiday Lights | listed, not verified |
| `H6840` | Cone Tree Lights | Holiday Lights | listed, not verified |
| `H6841` | Cone Tree Lights | Holiday Lights | listed, not verified |
| `H6842` | Cone Tree Lights | Holiday Lights | listed, not verified |
| `H6843` | Cone Tree Lights | Holiday Lights | listed, not verified |
| `H6850` | Govee Ball Lights | Holiday Lights | listed, not verified |
| `H6860` | C9 String Lights | Holiday Lights | listed, not verified |
| `H6861` | C9 String Lights | Holiday Lights | listed, not verified |
| `H6870` | Govee Christmas Sparkle String Lights | Holiday Lights | listed, not verified |
| `H6871` | Govee Christmas Sparkle String Lights | Holiday Lights | listed, not verified |
| `H7012` | LED Bulb String Lights | Discontinued | listed, not verified |
| `H7013` | LED Bulb String Lights | Discontinued | listed, not verified |
| `H7021` | LED Bulb String Lights | String Lights | listed, not verified |
| `H7025` | LED Bulb String Lights | String Lights | listed, not verified |
| `H7026` | Govee Outdoor Clear Bulb String Lights | String Lights | listed, not verified |
| `H7027` | Govee Outdoor Clear Bulb String Lights | String Lights | listed, not verified |
| `H7028` | LED Bulb String Lights | String Lights | listed, not verified |
| `H702A` | Govee Outdoor S14 Bulb String Lights 2 | String Lights | listed, not verified |
| `H702B` | Govee Outdoor S14 Bulb String Lights 2 | String Lights | listed, not verified |
| `H702C` | Govee Outdoor S14 Bulb String Lights 2 | String Lights | listed, not verified |
| `H7030` | LED Bulb String Lights | String Lights | listed, not verified |
| `H7033` | LED Bulb String Lights | Discontinued | listed, not verified |
| `H7037` | LED Bulb String Lights | String Lights | listed, not verified |
| `H7038` | LED Bulb String Lights | String Lights | listed, not verified |
| `H7039` | LED Bulb String Lights | String Lights | listed, not verified |
| `H703A` | 15m Dots String Lights | Landscape Lighting | listed, not verified |
| `H703B` | 30m Dots String Lights | Landscape Lighting | listed, not verified |
| `H7041` | LED Bulb String Lights | Discontinued | listed, not verified |
| `H7042` | LED Bulb String Lights | Discontinued | listed, not verified |
| `H7045` | Govee Outdoor Chromatic String Lights | String Lights | listed, not verified |
| `H7046` | Govee Outdoor Chromatic String Lights | String Lights | listed, not verified |
| `H7050` | Smart Ground Lights | Discontinued | listed, not verified |
| `H7051` | Smart Ground Lights | Discontinued | listed, not verified |
| `H7055` | Govee RGBIC Path Lights | Discontinued | listed, not verified |
| `H7056` | Govee Outdoor Pathway Lights 2 | Landscape Lighting | listed, not verified |
| `H7057` | Outdoor Flood Lights 2 | Landscape Lighting | listed, not verified |
| `H7058` | Outdoor Flood Lights 2 | Landscape Lighting | listed, not verified |
| `H705A` | 30m Permanent Outdoor Lights | Landscape Lighting | listed, not verified |
| `H705B` | 15m Permanent Outdoor Lights | Landscape Lighting | listed, not verified |
| `H705D` | 15m Permanent Outdoor Lights 2 | Landscape Lighting | listed, not verified |
| `H705E` | 30m Permanent Outdoor Lights 2 | Landscape Lighting | listed, not verified |
| `H705F` | 45m Permanent Outdoor Lights 2 | Landscape Lighting | listed, not verified |
| `H7060` | Govee RGBIC Flood Lights | Discontinued | listed, not verified |
| `H7061` | 2 Pack RGBIC Flood Lights | Discontinued | listed, not verified |
| `H7062` | 6 Pack RGBIC Flood Lights | Discontinued | listed, not verified |
| `H7063` | Govee Outdoor Flood Light | Landscape Lighting | listed, not verified |
| `H7065` | Govee RGBIC Spotlights | Discontinued | listed, not verified |
| `H7066` | Govee RGBIC Spotlights | Discontinued | listed, not verified |
| `H706A` | 30m Permanent Outdoor Lights Pro | Landscape Lighting | listed, not verified |
| `H706B` | 45m Permanent Outdoor Lights Pro | Landscape Lighting | listed, not verified |
| `H706C` | 60m Permanent Outdoor Lights Pro | Landscape Lighting | listed, not verified |
| `H7070` | Govee Outdoor Projector Light | Discontinued | listed, not verified |
| `H7071` | Govee Outdoor Decoration Projector Light | Projector Light | listed, not verified |
| `H7072` | Govee Outdoor Lamp Post Lights | Landscape Lighting | listed, not verified |
| `H7073` | Govee Outdoor Projector Light | Projector Light | listed, not verified |
| `H7076` | Govee Outdoor UpDown Wall Light | Landscape Lighting | listed, not verified |
| `H707A` | 30m Permanent Outdoor Lights Prism | Landscape Lighting | listed, not verified |
| `H707B` | 45m Permanent Outdoor Lights Prism | Landscape Lighting | listed, not verified |
| `H707C` | 60m Permanent Outdoor Lights Prism | Landscape Lighting | listed, not verified |
| `H7086` | Govee Outdoor Garden Lights | Landscape Lighting | listed, not verified |
| `H7087` | Govee Outdoor Tree Lights | Landscape Lighting | listed, not verified |
| `H7093` | Govee Outdoor Spotlights 2 | Landscape Lighting | listed, not verified |
| `H7094` | Govee Outdoor Spotlights 2 | Landscape Lighting | listed, not verified |
| `H70B1` | 1.5*2m 520LED Curtain Lights | Landscape Lighting | listed, not verified |
| `H70B6` | Govee Curtain Lights Pro | Holiday Lights | listed, not verified |
| `H70B8` | Govee Lightwall | Holiday Lights | listed, not verified |
| `H70BC` | 400LED Curtain Lights | Landscape Lighting | listed, not verified |
| `H70C1` | 10m RGBIC String Light | Strip Lights | listed, not verified |
| `H70C2` | 20m RGBIC String Light | Strip Lights | listed, not verified |
| `H70C4` | 20m Christmas String Lights 2 | Holiday Lights | listed, not verified |
| `H70C5` | 30m Christmas String Lights 2 | Holiday Lights | listed, not verified |
| `H70C7` | 50m Christmas String Lights 2 | Holiday Lights | listed, not verified |
| `H70C9` | 100m Christmas String Lights 2 | Holiday Lights | listed, not verified |
| `H70D1` | 10m RGBIC Outdoor Icicle Lights | Holiday Lights | listed, not verified |
| `H70D2` | 20m RGBIC Outdoor Icicle Lights | Holiday Lights | listed, not verified |
| `H70D3` | 30m RGBIC Outdoor Icicle Lights | Holiday Lights | listed, not verified |
| `H8015` | Govee Smart Bulb | LED Bulbs | listed, not verified |
| `H801A` | Govee Recessed Downlight | Downlight | listed, not verified |
| `H801B` | 15m Permanent Outdoor Lights | Landscape Lighting | listed, not verified |
| `H801D` | Govee Recessed Downlight | Downlight | listed, not verified |
| `H8022` | Govee Table Lamp 2S | Table Lamps | listed, not verified |
| `H8025` | Govee Outdoor Clear Bulb String Lights | String Lights | listed, not verified |
| `H8026` | Govee Outdoor Clear Bulb String Lights | String Lights | listed, not verified |
| `H802A` | 5m RGBIC Basic Strip Lights with Alexa | RGBIC Strip Lights | listed, not verified |
| `H8048` | Govee RGBIC Gaming Light Bars Pro | Table Lamps | listed, not verified |
| `H8057` | Outdoor Flood Lights 2 | Landscape Lighting | listed, not verified |
| `H805A` | 30m Permanent Outdoor Lights Elite | Landscape Lighting | listed, not verified |
| `H805B` | 15m Permanent Outdoor Lights Elite | Landscape Lighting | listed, not verified |
| `H805C` | 45m Permanent Outdoor Lights Elite | Landscape Lighting | listed, not verified |
| `H8066` | Govee Smart Outdoor Spotlight | Landscape Lighting | listed, not verified |
| `H8067` | Govee Smart Outdoor Spotlights | Landscape Lighting | listed, not verified |
| `H8069` | Govee Mini Panel Lights | Wall Lights | listed, not verified |
| `H806C` | 60m Permanent Outdoor Lights Pro | Landscape Lighting | listed, not verified |
| `H8076` | Govee Floor Lamp (H8076A) | Floor Lamps | listed, not verified |
| `H807C` | Govee Corner Floor Lamp 2 | Floor Lamps | listed, not verified |
| `H80A1` | 12inch Ceiling Light | Ceiling Light | listed, not verified |
| `H80A4` | Govee Square Ceiling Light | Ceiling Light | listed, not verified |
| `H80C4` | 20m Christmas String Lights 2 | Holiday Lights | listed, not verified |
| `H80C5` | 30m Christmas String Lights 2 | Holiday Lights | listed, not verified |
| `H80D1` | 10m RGBIC Outdoor Icicle Lights | Holiday Lights | listed, not verified |
| `H8630` | Govee Gaming Pixel Light | Pixel Light | listed, not verified |
| `H8811` | 480LED Net Lights | Holiday Lights | listed, not verified |
| `H8840` | Cone Tree Lights | Holiday Lights | listed, not verified |
| `H8841` | Cone Tree Lights | Holiday Lights | listed, not verified |
| `R1250` | Govee Ceiling Light Pro | Ceiling Light | listed, not verified |
| `R1310` | Govee Ceiling Fan with Lights | Ceiling Fan Light | listed, not verified |
| `R14B1` | Govee Smart Bulb | LED Bulbs | listed, not verified |
| `R14C2` | Govee Edison Bulb | LED Bulbs | listed, not verified |
| `R1501` | Govee Smart Downlight | Downlight | listed, not verified |
| `R1810` | Govee Star Light Projector (Cosmos) | Projector Light | listed, not verified |
| `R3A51` | 30m Permanent Outdoor Lights Pro Dual | Landscape Lighting | listed, not verified |
| `R60B3` | Govee Uplighter Floor Lamp | Floor Lamps | listed, not verified |
| `R618D` | 15m RGBIC LED Strip Light | RGBIC Strip Lights | listed, not verified |
| `R6842` | Cone Tree Lights | Holiday Lights | listed, not verified |
| `R6861` | C9 String Lights | Holiday Lights | listed, not verified |
<!-- /generated -->
