#!/usr/bin/env python3
"""Generate safe stereo test WAVs and analyze Fates loopback captures."""

from __future__ import annotations

import argparse
import json
import math
import struct
import sys
import wave
from pathlib import Path


RATE = 48_000
CHANNELS = 2
AMPLITUDE = 0.03125  # approximately -30 dBFS; mixer state remains deterministic


def generate(path: Path, frequency: float, seconds: float, silent: bool = False) -> None:
    frame_count = int(RATE * seconds)
    with wave.open(str(path), "wb") as output:
        output.setnchannels(CHANNELS)
        output.setsampwidth(2)
        output.setframerate(RATE)
        frames = bytearray()
        for index in range(frame_count):
            sample = 0 if silent else round(math.sin(2 * math.pi * frequency * index / RATE) * 32767 * AMPLITUDE)
            frames.extend(struct.pack("<hh", sample, sample))
        output.writeframes(frames)


def analyze(path: Path, expected_frequency: float) -> dict:
    with wave.open(str(path), "rb") as source:
        if source.getnchannels() != CHANNELS or source.getsampwidth() != 2 or source.getframerate() != RATE:
            raise ValueError("capture must be stereo S16_LE WAV at 48 kHz")
        raw = source.readframes(source.getnframes())
    samples = struct.unpack(f"<{len(raw) // 2}h", raw)
    channels = [samples[0::2], samples[1::2]]
    results = []
    for channel in channels:
        if not channel:
            raise ValueError("capture contains no frames")
        raw_peak = max(abs(value) for value in channel)
        threshold = max(raw_peak * 0.1, 8)
        active_indices = [index for index, value in enumerate(channel) if abs(value) >= threshold]
        active = channel[active_indices[0] : active_indices[-1] + 1] if active_indices else channel
        rms = math.sqrt(sum(value * value for value in active) / len(active)) / 32768.0
        peak = raw_peak / 32768.0
        crossings = sum(1 for left, right in zip(active, active[1:]) if left <= 0 < right)
        frequency = crossings * RATE / len(active)
        results.append({"rms_dbfs": 20 * math.log10(max(rms, 1e-12)), "peak_dbfs": 20 * math.log10(max(peak, 1e-12)), "estimated_hz": frequency})
    balance_db = abs(results[0]["rms_dbfs"] - results[1]["rms_dbfs"])
    passed = all(result["rms_dbfs"] > -70 and abs(result["estimated_hz"] - expected_frequency) <= 15 for result in results) and balance_db <= 3
    return {
        "path": str(path),
        "format": "S16_LE",
        "rate_hz": RATE,
        "channels": results,
        "channel_balance_db": balance_db,
        "expected_hz": expected_frequency,
        "pass": passed,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    generator = subparsers.add_parser("generate")
    generator.add_argument("path", type=Path)
    generator.add_argument("--frequency", type=float, default=440.0)
    generator.add_argument("--seconds", type=float, default=3.0)
    generator.add_argument("--silent", action="store_true")
    analyzer = subparsers.add_parser("analyze")
    analyzer.add_argument("path", type=Path)
    analyzer.add_argument("--expected-frequency", type=float, default=440.0)
    args = parser.parse_args()
    if args.command == "generate":
        if not 0.1 <= args.seconds <= 10:
            parser.error("duration must be in 0.1..10 seconds")
        if not args.silent and not 20 <= args.frequency <= 2000:
            parser.error("frequency must be in 20..2000 Hz")
        generate(args.path, args.frequency, args.seconds, args.silent)
        return 0
    result = analyze(args.path, args.expected_frequency)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
