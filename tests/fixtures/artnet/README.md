# Art-Net captures

Real ArtDmx packets, recorded off the wire on UDP 6454. The parser tests read
every capture here — see `packages/rust/crates/dmx/src/input/artnet/tests.rs`.

A conformance vector under `golden/` covers one mode, and DMX is not a mode.
Art-Net packets go here instead.

## What a capture is

One datagram, and what the sender showed when it left:

```
<name>.bin      the UDP payload, from the `Art-Net\0` byte to the last slot
<name>.json     what the sender showed
```

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

Nothing yet. Both senders are still to be recorded.
