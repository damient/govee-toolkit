---
title: Frequently asked questions
description: What Govee Toolkit is, which Govee lights it reaches, and what you need to control them over Wi-Fi, over Bluetooth or through the cloud.
---

## What is Govee Toolkit? <!-- menu: What it is -->

Govee Toolkit controls your Govee lights from your own computer. It turns
them on and off, and it sets the brightness, the color and the white
temperature. On a light strip it paints each zone on its own. It is free and
open source, under the MIT license.

## Is it an official Govee product? <!-- menu: Official or not -->

No. It is an independent project, not affiliated with, sponsored by or
endorsed by Govee. The name "Govee" identifies the lights it works with.

## Does my Govee light work with it? <!-- menu: Compatible lights -->

[The devices page]({{base}}devices/) lists every model the toolkit knows,
and what each one answers over Wi-Fi, over Bluetooth and through the cloud.
A model that nobody has probed yet shows a question mark: untested, which is
a long way from broken.

## My model is not on the list. What can I do? <!-- menu: A missing model -->

Two ways put it there. Send us the light and we add it for free, or follow
the guide and add it yourself. [Add a device]({{base}}devices/add/) walks
through both.

## Do I need to know how to code? <!-- menu: Coding skills -->

No. The command line takes one short command per action, such as
`govee on` or `govee color`. [Start here]({{base}}docs/start/) needs no code
at all. Developers can use the same engine from Rust, Python or Node.js.

## Does it work without the internet? <!-- menu: Without internet -->

Yes. Over Wi-Fi and over Bluetooth, each command goes from your computer
straight to the light, on your own network. The cloud is a third mode, which
you enable only if you want to reach your lights from anywhere.

## Wi-Fi, Bluetooth or cloud: which one do I pick? <!-- menu: Wi-Fi, Bluetooth or cloud -->

- **Wi-Fi** is the fastest. The light and your computer share one network.
- **Bluetooth** reaches a light in range, with no Wi-Fi at all.
- **Cloud** reaches your lights from anywhere with internet access.

You choose the modes for each light. [Modes]({{base}}docs/modes/) says what
each one needs.

## What is LAN Control, and how do I turn it on? <!-- menu: LAN Control -->

LAN Control is the switch that lets your own network reach the light over
Wi-Fi. Open the Govee Home app, select your light, open its settings and turn
on **LAN Control**. You do this once per light. Govee publishes
[the list of models that carry the switch]({{base}}devices/lan/).

## Do I need a Govee account or an API key? <!-- menu: Account and API key -->

Only for the cloud mode. Wi-Fi and Bluetooth need no account and no key. For
the cloud, ask for a key in the Govee Home app: profile, then settings, then
**Apply for API key**.

## Can I keep using the Govee Home app? <!-- menu: The Govee Home app -->

Yes. Over Bluetooth, a light holds one connection at a time: close the Govee
Home app when the toolkit connects over Bluetooth.

## Can I put a new light on my Wi-Fi without the Govee Home app? <!-- menu: Wi-Fi setup -->

Yes. `govee provision` sends the network name and the password to the light
over Bluetooth. Use a 2.4 GHz network: Govee lights do not join a 5 GHz one.
[Configure]({{base}}docs/configure/#wi-fi) gives the details.

## Which computers does it run on? <!-- menu: Supported systems -->

Linux, macOS and Windows. The Python package also carries builds for Linux
on ARM boards, `aarch64` and `armv7`.

## Can I control each segment of a light strip? <!-- menu: Segments -->

Yes, on a model that carries segments. Address one zone, a range or the
whole strip in one command. Each model page gives its zone count.

## Can my lights follow music or my screen? <!-- menu: Music and screen -->

The toolkit streams colors to a strip frame after frame, in real time, over
Wi-Fi and over Bluetooth. Your own program, or one you pick, supplies the
frames: from music, from the screen or from a sensor.

## Can I control my lights with DMX? <!-- menu: DMX -->

Yes. The DMX bridge, `govee-dmx`, receives Art-Net from a lighting desk, a
media server or a show application, and each light becomes a DMX fixture.
[DMX]({{base}}docs/dmx/) explains the setup.

## Does it work with Home Assistant, Homebridge or Matter? <!-- menu: Home Assistant, Matter -->

Not yet. A desktop app comes first, then Home Assistant, Homebridge and
Matter. [The roadmap]({{repo}}/blob/main/docs/roadmap.md) follows what
people ask for.

## My light does not answer. What do I check? <!-- menu: Troubleshooting -->

Check these first:

1. LAN Control is on for the light.
2. The light and your computer are on the same network. Guest Wi-Fi keeps
   them apart.
3. Your firewall lets multicast on UDP through.

[Troubleshooting]({{base}}docs/troubleshooting/) covers the rest.

## Where do I ask a question or report a problem? <!-- menu: Contact -->

Open an issue on [GitHub]({{repo}}/issues) with your model number and your
operating system. For a light you want to send us, write to
[contact@gvetk.com](mailto:contact@gvetk.com).
