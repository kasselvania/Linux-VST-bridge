#!/usr/bin/env python3
"""Correlate generated stereo f32le with its physical return; requires NumPy."""
import json
import sys
import numpy as np


def correlate(sent, returned, rate=48000):
    size = 1 << (len(sent) + len(returned) - 1).bit_length()
    maximum = min(rate, len(returned) - 1)
    cross = np.fft.irfft(np.fft.rfft(returned, size) *
                         np.conj(np.fft.rfft(sent, size)), size)[:maximum + 1]
    lag = int(np.argmax(np.abs(cross)))
    energy = float(np.linalg.norm(sent) * np.linalg.norm(returned))
    confidence = float(abs(cross[lag]) / energy) if energy else 0.0
    valid = confidence >= 0.25 and np.max(np.abs(returned)) >= 1e-5 and lag < maximum
    return {"valid": bool(valid), "delay_frames": lag if valid else None,
            "delay_ms": lag * 1000 / rate if valid else None,
            "correlation": confidence,
            "polarity": int(np.sign(cross[lag])) if valid else None}


def main():
    if sys.argv[1:] == ["--self-test"]:
        rng = np.random.default_rng(417)
        sent = np.zeros(144000)
        sent[24000:72000] = rng.uniform(-0.01, 0.01, 48000)
        for lag, gain in [(534, 0.5), (18384, -0.3)]:
            returned = np.zeros_like(sent)
            returned[lag:] = sent[:-lag] * gain
            returned += rng.normal(0, 1e-5, len(returned))
            result = correlate(sent, returned)
            assert result["valid"] and result["delay_frames"] == lag, result
        assert not correlate(sent, np.zeros_like(sent))["valid"]
        assert not correlate(sent, rng.normal(0, 0.001, len(sent)))["valid"]
        print("known delays, inversion, noise, and absent return: passed")
        return
    if len(sys.argv) != 2:
        raise SystemExit("usage: loopback_analysis.py RETURN.f32le | --self-test")
    received = np.fromfile(sys.argv[1], dtype="<f4").reshape(-1, 2).astype(float)
    sent = np.fromfile(sys.argv[1] + ".input.f32le", dtype="<f4").reshape(-1, 2).astype(float)
    if sent.shape != (144000, 2) or received.shape != sent.shape:
        raise SystemExit("expected complete 3-second stereo capture at 48 kHz")
    if not np.isfinite(sent).all() or not np.isfinite(received).all():
        raise SystemExit("nonfinite capture")
    result = {"rate": 48000, "frames": len(sent),
              "sent_peak": np.max(np.abs(sent), axis=0).tolist(),
              "received_peak": np.max(np.abs(received), axis=0).tolist(),
              "paths": []}
    for tx in range(2):
        for rx in range(2):
            result["paths"].append({"output": tx + 1, "input": rx + 1,
                                    **correlate(sent[:, tx], received[:, rx])})
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
