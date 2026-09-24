---
title: DMX
slug: dmx
order: 7
description: Drive your lights from a lighting desk over DMX, the way a show drives every other fixture.
---

# DMX

`govee-dmx` takes the Art-Net your desk already sends and writes it to each
light over {{badge_lan}}. Your lights become fixtures of the show.

## What you need

- A light that {{badge_lan}} reaches. [The devices page]({{base}}devices/)
  says which models do.
- A desk, a media server or a show application that sends Art-Net.
- A patch file that says which light answers which channels.

## Pick a channel layout

Each light answers one of three layouts. A desk calls this a personality.

| Layout | Channels | What you get |
| ------ | -------- | ------------ |
| `full` | 6 | one color for the whole light |
| `segment` | 3 + 3 per zone | one color per zone |
| `pixel` | 3 + 3 per LED | one color per LED |

Channel 1 is always the dimmer, channel 2 the mode and channel 3 the white
temperature, so a cue carries from one model to the next. The white covers the
whole light, whatever the layout. A model serves only the layouts its hardware carries.
Each model page prints its tables, channel by channel —
[the H61A0]({{base}}devices/H61A0/#dmx) is one example.

## Patch a fixture

Two commands set the rig up for you.

```sh
govee-dmx patch      # find the lights and write the patch file
govee-dmx identify   # light one fixture at a time
```

`patch` looks for the lights on your network and gives each one a free start
address. It leaves alone every address you already set, so you can run it again
after you add a light.

`identify` walks the rig. It takes every fixture off, then lights them one by
one and names the line of the patch file that drives each one. That is how you
label the fixtures on the desk.

The first channel is always the dimmer. At 0 it powers the light off, so a
blackout on the desk takes the rig down.

## Run the rig

```sh
govee-dmx run        # receive Art-Net and drive the lights
```

`run` listens for Art-Net and writes each frame to the lights the patch file
names. Your desk lists the node as soon as it answers a poll, and you patch it
there the way you patch any other node. Add `--scan` to look for the lights
first, so one command takes a rig from nothing to a node the desk sees.

`--dry-run` prints every packet and what each fixture reads out of it, and
writes to no light. That is how you check a patch before the rig moves.

The run takes every light off at the start, so the rig comes up dark. A light
that drops off the network stops no other one: the run reports it and carries
on.
