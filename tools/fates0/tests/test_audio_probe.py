import importlib.util
import math
import struct
import sys
import tempfile
import unittest
import wave
from pathlib import Path


MODULE_PATH = Path(__file__).parents[1] / "audio_probe.py"
SPEC = importlib.util.spec_from_file_location("fates0_audio_probe", MODULE_PATH)
audio = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = audio
SPEC.loader.exec_module(audio)


class AudioProbeTest(unittest.TestCase):
    def test_generated_wave_has_bounded_level_and_shape(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "tone.wav"
            audio.generate(path, 440.0, 1.0)
            result = audio.analyze(path, 440.0)
            self.assertTrue(result["pass"])
            self.assertEqual(result["format"], "S16_LE")
            self.assertLess(result["channels"][0]["peak_dbfs"], -29)

    def test_silent_capture_fails_loopback_admission(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "silence.wav"
            audio.generate(path, 440.0, 0.5, silent=True)
            self.assertFalse(audio.analyze(path, 440.0)["pass"])

    def test_leading_and_trailing_silence_do_not_corrupt_frequency(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "delayed.wav"
            with wave.open(str(path), "wb") as output:
                output.setnchannels(2)
                output.setsampwidth(2)
                output.setframerate(audio.RATE)
                frames = bytearray()
                for index in range(audio.RATE * 2):
                    active = audio.RATE // 4 <= index < audio.RATE + audio.RATE // 4
                    sample = int(math.sin(2 * math.pi * 440 * index / audio.RATE) * 2000) if active else 0
                    frames.extend(struct.pack("<hh", sample, sample))
                output.writeframes(frames)
            self.assertTrue(audio.analyze(path, 440.0)["pass"])

    def test_channel_imbalance_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "imbalanced.wav"
            with wave.open(str(path), "wb") as output:
                output.setnchannels(2)
                output.setsampwidth(2)
                output.setframerate(audio.RATE)
                frames = bytearray()
                for index in range(audio.RATE):
                    left = int(math.sin(2 * math.pi * 440 * index / audio.RATE) * 4000)
                    right = int(left / 8)
                    frames.extend(struct.pack("<hh", left, right))
                output.writeframes(frames)
            self.assertFalse(audio.analyze(path, 440.0)["pass"])


if __name__ == "__main__":
    unittest.main()
