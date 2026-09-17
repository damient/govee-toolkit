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
| `basic` | 4 | a dimmer and one color over the whole light |
| `full` | 6 | the same, plus the white temperature and a control channel |
| `pixel` | 1 + 3 per zone | one color for each zone the phone controller shows |
| `pixel-native` | 1 + 3 per LED | one color for each LED the light renders |

A model serves the personalities its hardware carries. The page of each model
prints them, with every channel and its number —
[the H61A0]({{base}}devices/H61A0/#dmx) is one example.

## Patch a fixture

Give the fixture a start address, the way you give one to any other fixture.
The channels follow it in order. Address 1 with `pixel` on a light of 10 zones
takes channels 1 to 31, and the next fixture starts at 32.

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
