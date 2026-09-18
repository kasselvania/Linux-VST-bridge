#!/usr/bin/env python3
"""Perform one bounded SMBus quick transaction to admit the CS4270 address."""

from __future__ import annotations

import argparse
import json
import sys


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bus", type=int, default=1)
    parser.add_argument("--address", type=lambda value: int(value, 0), default=0x48)
    args = parser.parse_args()

    if args.address != 0x48:
        parser.error("SHIELDXL0 admits only the expected CS4270 address 0x48")

    try:
        from smbus2 import SMBus

        with SMBus(args.bus) as bus:
            bus.write_quick(args.address)
    except Exception as error:  # an I/O error is the evidence this gate needs
        print(
            json.dumps(
                {
                    "bus": args.bus,
                    "address": "0x48",
                    "present": False,
                    "error": type(error).__name__,
                },
                sort_keys=True,
            )
        )
        return 1

    print(json.dumps({"bus": args.bus, "address": "0x48", "present": True}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
