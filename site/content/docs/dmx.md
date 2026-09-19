---
title: DMX
slug: dmx
order: 6
description: Drive your lights from a lighting desk over Art-Net, the way a show drives every other fixture.
---

# DMX

A lighting desk sends DMX over the network. `govee-dmx` receives it and writes
what it receives to each light over {{badge_lan}}. Your lights become fixtures
of the show.

## What you need

- A light that {{badge_lan}} reaches. [The devices page]({{base}}devices/) says
  which models do.
- A desk, a media server or a show application that sends Art-Net on your
  network.
- A patch file that says which light answers which channels.

## Personalities

A personality is one channel layout. Pick the one that fits the show, the way
you pick a mode on any other fixture.

| Personality | Channels | What you get |
| ----------- | -------- | ------------ |
| `full` | 6 | one color over the whole light, plus the white temperature |
| `segment` | 2 + 3 per zone | one color for each zone the phone controller shows |
| `pixel` | 2 + 3 per LED | one color for each LED the light renders |

Every personality starts the same way: channel 1 is the dimmer, channel 2 is
the mode channel. A cue therefore carries from one model to the next.

A model serves the personalities its hardware carries. A light whose every zone
is one LED serves `pixel` alone, because `segment` would give you the same
table. A light that renders no white temperature keeps channel 6 of `full` and
does nothing with it. The page of each model prints the tables, with every
channel and its number — [the H61A0]({{base}}devices/H61A0/#dmx) is one
example.

## Patch a fixture

Give the fixture a start address, the way you give one to any other fixture.
The channels follow it in order. Address 1 with `segment` on a light of 10
zones takes channels 1 to 32, and the next fixture starts at 33.

The first channel is always the dimmer. At 0 the light powers off, so a
blackout on the desk turns the rig off and needs no patch of its own. Above 0
the dimmer powers the light on and sets the brightness.

## What to expect

These are consumer lights, not theatre fixtures. Three things follow from that.

**A fade can look stepped.** The desk sends 255 values. A light that holds 100
brightness steps applies 100 of them. Each model page prints that count.

**The rate is the light's, not the desk's.** A desk sends up to 44 frames per
second, and a light accepts far less. The bridge holds the newest look and
sends that one, so the rig follows the desk rather than a queue.

**A static look costs nothing.** The bridge sends nothing where nothing
changed. It repeats the current look every 10 seconds, because a lost frame
would otherwise leave a zone at a stale color.

## When the desk stops

A fixture that receives no frame for 4 seconds does what you asked for: it
holds the last look, it goes black, or it powers off. The default is to hold.
A fixture that has taken no frame yet holds too, so a rig waits dark for the
desk rather than powering off at start.

One light that drops does not stop the show. The bridge reports the failure,
keeps every other light running, and retries that one.
