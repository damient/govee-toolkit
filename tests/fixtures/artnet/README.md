# Art-Net captures

Real Art-Net packets, recorded off the wire on UDP 6454. The parser tests read
every capture here — see
`packages/rust/crates/dmx/src/input/artnet/captures.rs`.

A conformance vector under `golden/` covers one mode, and DMX is not a mode.
Art-Net packets go here instead.

## What a capture is

[`../../../tools/record-artnet.py`](../../../tools/record-artnet.py) records
one. One datagram, and what the sender showed when it left:

```
<name>.bin      the UDP payload, from the `Art-Net\0` byte to the last slot
<name>.json     what the sender showed
```

An `ArtDmx` capture goes in this directory. An `ArtPoll` capture goes in
`poll/`, and its JSON carries `sender`, `talk_to_me` and `priority`.

The JSON:

```json
{
  "sender": "how the packet was produced, in your own words",
  "universe": 0,
  "sequence": 12,
  "physical": 0,
  "length": 512,
  "channels": { "1": 255, "2": 128 }
}
```

`channels` holds the values you read on the sender, keyed by DMX address,
counted from 1. List the channels you checked; the test reads the ones you
list. `sender` says what produced the packet: a desk and a media server
number their sequences differently, so both must be covered.

## Redaction

A capture is a recording of a network. The checklist in
[`../README.md`](../README.md) applies here too. An ArtDmx payload carries no
address of its own, so the work is in what goes beside it: name no household
in `sender`, and export the payload alone rather than a pcap that carries the
frames you did not open.

## What is recorded

| Capture | Sender |
| ------- | ------ |
| `qlcplus` | QLC+ 5.2.2, the Simple Desk |
| `poll/qlcplus` | QLC+ 5.2.2, the Art-Net output plugin |
| `touchdesigner` | TouchDesigner 2025.33230, a DMX Out CHOP |

The two senders differ in every field that is not the channel values:

| | QLC+ 5.2.2 | TouchDesigner 2025.33230 |
| - | ---------- | ------------------------ |
| Source port | 6454 | an ephemeral port |
| Rate | one packet every 2 seconds | 60 packets per second |
| Sequence | counts 1 to 255 | counts 1 to 255, then 0 |

The sequence of a wrap is what makes the pair worth holding: Art-Net reads a
sequence of 0 as the check turned off, so TouchDesigner turns it off once
every 256 packets.

QLC+ numbers its universes from 1, and its universe 1 sends the port-address 0.
