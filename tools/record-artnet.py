#!/usr/bin/env python3
r"""Record one ArtDmx packet and one ArtPoll packet off UDP 6454.

The payload alone is written, from the `Art-Net\0` byte to the last slot, the
way `tests/fixtures/artnet/README.md` asks. The JSON that goes beside a capture
is written as a template: `sender` and `channels` stay `TODO`, because only the
desk can say what it showed.

Stop `govee-dmx run` first, or the two programs compete for the port.
"""

import argparse
import json
import socket
import struct
import sys
from pathlib import Path

ID = b"Art-Net\0"
OP_DMX = 0x5000
OP_POLL = 0x2000
OP_POLL_REPLY = 0x2100
PORT = 6454


def opcode(payload):
    if len(payload) < 10 or not payload.startswith(ID):
        return None
    return struct.unpack_from("<H", payload, 8)[0]


def decode_dmx(payload):
    version = struct.unpack_from(">H", payload, 10)[0]
    sequence = payload[12]
    physical = payload[13]
    sub_uni = payload[14]
    net = payload[15]
    length = struct.unpack_from(">H", payload, 16)[0]
    data = payload[18 : 18 + length]
    return {
        "version": version,
        "sequence": sequence,
        "physical": physical,
        "universe": (net << 8) | sub_uni,
        "net": net,
        "sub_uni": sub_uni,
        "length": length,
        "data": data,
    }


def write_dmx(out, name, payload, source):
    frame = decode_dmx(payload)
    binary = out / f"{name}.bin"
    binary.write_bytes(payload)
    template = {
        "sender": "TODO: what produced the packet, in your own words",
        "universe": frame["universe"],
        "sequence": frame["sequence"],
        "physical": frame["physical"],
        "length": frame["length"],
        "channels": {},
    }
    (out / f"{name}.json").write_text(json.dumps(template, indent=2) + "\n")

    print(f"\nArtDmx from {source[0]}:{source[1]}  ->  {binary.name}")
    print(f"  protocol version {frame['version']}")
    print(f"  universe {frame['universe']}", end="")
    print(f" (net {frame['net']}, sub-uni {frame['sub_uni']})")
    print(f"  sequence {frame['sequence']}, physical {frame['physical']}", end="")
    print(f", length {frame['length']}")
    live = {i + 1: v for i, v in enumerate(frame["data"]) if v}
    shown = dict(list(live.items())[:24])
    print(f"  channels above 0, READ FROM THE BYTES, NOT EVIDENCE: {shown}")
    if len(live) > len(shown):
        print(f"  ... and {len(live) - len(shown)} more")
    print(f"  Fill `channels` in {name}.json with the values YOU set on the desk.")


def write_poll(out, name, payload, source):
    binary = out / f"{name}.bin"
    binary.write_bytes(payload)
    version = struct.unpack_from(">H", payload, 10)[0]
    flags = payload[12] if len(payload) > 12 else 0
    priority = payload[13] if len(payload) > 13 else 0
    print(f"\nArtPoll from {source[0]}:{source[1]}  ->  {binary.name}")
    print(f"  protocol version {version}, TalkToMe 0x{flags:02X}, priority {priority}")
    print("  The source address is printed here alone. Do not put it in a file.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=Path("tests/fixtures/artnet"))
    parser.add_argument(
        "--name", default="qlcplus", help="the capture name, without an extension"
    )
    parser.add_argument("--timeout", type=float, default=60.0, help="seconds to wait")
    parser.add_argument("--dmx-only", action="store_true")
    parser.add_argument("--poll-only", action="store_true")
    args = parser.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)
    want_dmx = not args.poll_only
    want_poll = not args.dmx_only

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    if hasattr(socket, "SO_REUSEPORT"):
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEPORT, 1)
    sock.bind(("0.0.0.0", PORT))
    sock.settimeout(args.timeout)

    print(
        f"Listening on UDP {PORT} for {args.timeout:.0f} s. Move a fader on the desk."
    )
    while want_dmx or want_poll:
        try:
            payload, source = sock.recvfrom(2048)
        except TimeoutError:
            print("\nTimeout. Nothing more arrived.", file=sys.stderr)
            break
        op = opcode(payload)
        if op == OP_DMX and want_dmx:
            write_dmx(args.out, args.name, payload, source)
            want_dmx = False
        elif op == OP_POLL and want_poll:
            write_poll(args.out, f"{args.name}-poll", payload, source)
            want_poll = False
        elif op is None:
            print(f"  a packet that is not Art-Net, from {source[0]}, dropped")
        elif op == OP_POLL_REPLY:
            print(f"  an ArtPollReply from {source[0]}, ignored")

    missing = [n for n, w in (("ArtDmx", want_dmx), ("ArtPoll", want_poll)) if w]
    if missing:
        print(f"Not recorded: {', '.join(missing)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
