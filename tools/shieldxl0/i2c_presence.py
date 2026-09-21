#!/usr/bin/env python3
"""Perform one bounded SMBus quick transaction to admit the CS4270 address."""

from __future__ import annotations

import argparse
import json
import sys
import time


RESET_CHIP = "/dev/gpiochip0"
RESET_LINE = 17
RESET_SETTLE_SECONDS = 0.01


def probe_codec(bus_number, address, gpio_module, smbus_type, sleep):
    settings = gpio_module.LineSettings(
        direction=gpio_module.line.Direction.OUTPUT,
        output_value=gpio_module.line.Value.ACTIVE,
    )
    with gpio_module.request_lines(
        RESET_CHIP,
        consumer="shieldxl0-i2c-admission",
        config={RESET_LINE: settings},
    ):
        sleep(RESET_SETTLE_SECONDS)
        with smbus_type(bus_number) as bus:
            bus.write_quick(address)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bus", type=int, default=1)
    parser.add_argument("--address", type=lambda value: int(value, 0), default=0x48)
    args = parser.parse_args()

    if args.address != 0x48:
        parser.error("SHIELDXL0 admits only the expected CS4270 address 0x48")

    try:
        import gpiod
        from smbus2 import SMBus

        probe_codec(args.bus, args.address, gpiod, SMBus, time.sleep)
    except Exception as error:  # an I/O error is the evidence this gate needs
        result = {
            "bus": args.bus,
            "address": "0x48",
            "present": False,
            "error": type(error).__name__,
        }
        if isinstance(error, OSError) and error.errno is not None:
            result["errno"] = error.errno
        print(json.dumps(result, sort_keys=True))
        return 1

    print(json.dumps({"bus": args.bus, "address": "0x48", "present": True}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
