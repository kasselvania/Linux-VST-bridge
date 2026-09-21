#!/usr/bin/env python3
"""Non-real-time Fates evdev reader with stable typed output."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import selectors
import struct
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import BinaryIO, Iterable, Iterator


EVENT = struct.Struct("=qqHHi")
EV_KEY = 0x01
EV_REL = 0x02
REL_X = 0x00
BTN_0 = 0x100
EVIOCSCLOCKID = 0x400445A0


@dataclass(frozen=True)
class Device:
    kind: str
    index: int
    path: Path
    code: int


DEVICES = tuple(
    Device("encoder", index, Path(f"/dev/input/fates-encoder-{index + 1}"), REL_X)
    for index in range(3)
) + tuple(
    Device("button", index, Path(f"/dev/input/fates-button-{index + 1}"), BTN_0 + index)
    for index in range(3)
)


def decode_events(data: bytes) -> Iterator[tuple[int, int, int, int]]:
    if len(data) % EVENT.size:
        raise ValueError(f"partial input_event record: {len(data)} bytes")
    for offset in range(0, len(data), EVENT.size):
        seconds, microseconds, event_type, code, value = EVENT.unpack_from(data, offset)
        monotonic_ns = seconds * 1_000_000_000 + microseconds * 1_000
        yield event_type, code, value, monotonic_ns


def typed_event(device: Device, event_type: int, code: int, value: int, monotonic_ns: int):
    if device.kind == "encoder" and event_type == EV_REL and code == device.code and value:
        return {
            "type": "encoder",
            "index": device.index,
            "delta": value,
            "monotonic_ns": monotonic_ns,
        }
    if device.kind == "button" and event_type == EV_KEY and code == device.code and value in (0, 1):
        return {
            "type": "button",
            "index": device.index,
            "pressed": bool(value),
            "monotonic_ns": monotonic_ns,
        }
    return None


def format_event(event: dict, output_format: str) -> str:
    if output_format == "json":
        return json.dumps(event, separators=(",", ":"), sort_keys=True)
    if event["type"] == "encoder":
        return (
            f"encoder(index={event['index']}, delta={event['delta']}, "
            f"monotonic_ns={event['monotonic_ns']})"
        )
    return (
        f"button(index={event['index']}, pressed={str(event['pressed']).lower()}, "
        f"monotonic_ns={event['monotonic_ns']})"
    )


def open_devices(devices: Iterable[Device]) -> dict[int, tuple[BinaryIO, Device]]:
    opened: dict[int, tuple[BinaryIO, Device]] = {}
    try:
        for device in devices:
            stream = device.path.open("rb", buffering=0)
            fcntl.ioctl(stream.fileno(), EVIOCSCLOCKID, struct.pack("=i", time.CLOCK_MONOTONIC))
            opened[stream.fileno()] = (stream, device)
    except Exception:
        for stream, _ in opened.values():
            stream.close()
        raise
    return opened


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--format", choices=("json", "text"), default="json")
    parser.add_argument("--count", type=int, help="exit after this many typed events")
    args = parser.parse_args()
    if args.count is not None and args.count < 1:
        parser.error("--count must be positive")

    missing = [str(device.path) for device in DEVICES if not device.path.exists()]
    if missing:
        print(f"missing stable control aliases: {', '.join(missing)}", file=sys.stderr)
        return 1

    try:
        opened = open_devices(DEVICES)
    except OSError as error:
        print(f"cannot open stable controls: {error}", file=sys.stderr)
        return 1

    selector = selectors.DefaultSelector()
    for fd, (stream, device) in opened.items():
        selector.register(fd, selectors.EVENT_READ, (stream, device))

    emitted = 0
    try:
        while True:
            for key, _ in selector.select():
                stream, device = key.data
                data = os.read(stream.fileno(), EVENT.size * 64)
                if not data:
                    raise RuntimeError(f"control disappeared: {device.path}")
                for raw in decode_events(data):
                    event = typed_event(device, *raw)
                    if event is None:
                        continue
                    print(format_event(event, args.format), flush=True)
                    emitted += 1
                    if args.count is not None and emitted >= args.count:
                        return 0
    except (OSError, RuntimeError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 1
    finally:
        selector.close()
        for stream, _ in opened.values():
            stream.close()


if __name__ == "__main__":
    sys.exit(main())
