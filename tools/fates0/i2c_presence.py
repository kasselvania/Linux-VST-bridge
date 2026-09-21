#!/usr/bin/env python3
"""Perform one bounded SMBus quick transaction for the Fates WM8731 address."""

import argparse
import json
import sys


def probe_codec(bus_number, address, smbus_type):
    with smbus_type(bus_number) as bus:
        bus.write_quick(address)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bus", type=int, default=1)
    parser.add_argument("--address", type=lambda value: int(value, 0), default=0x1A)
    args = parser.parse_args()
    if args.address != 0x1A:
        parser.error("FATES0 admits only the expected WM8731 address 0x1a")
    try:
        from smbus2 import SMBus
        probe_codec(args.bus, args.address, SMBus)
    except Exception as error:
        result = {"bus": args.bus, "address": "0x1a", "present": False, "error": type(error).__name__}
        if isinstance(error, OSError) and error.errno is not None:
            result["errno"] = error.errno
        print(json.dumps(result, sort_keys=True))
        return 1
    print(json.dumps({"bus": args.bus, "address": "0x1a", "present": True}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
