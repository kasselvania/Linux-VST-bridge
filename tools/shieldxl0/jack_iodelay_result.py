#!/usr/bin/env python3

import argparse
import pathlib
import re


MEASUREMENT = re.compile(
    r"^\s*(?P<frames>[0-9]+(?:\.[0-9]+)?) frames\s+"
    r"(?P<milliseconds>[0-9]+(?:\.[0-9]+)?) ms total roundtrip latency\s*$",
    re.MULTILINE,
)


def measurements(text: str) -> list[tuple[float, float]]:
    return [
        (float(match.group("frames")), float(match.group("milliseconds")))
        for match in MEASUREMENT.finditer(text)
    ]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("log", type=pathlib.Path)
    args = parser.parse_args()
    values = measurements(args.log.read_text(errors="replace"))
    if not values:
        return 1
    frames, milliseconds = values[-1]
    print(f"{frames:.3f}\t{milliseconds:.3f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
