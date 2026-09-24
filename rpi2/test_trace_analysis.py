import tempfile
from pathlib import Path
import unittest
import numpy as np
from trace_analysis import audio


class CaptureReduction(unittest.TestCase):
    def test_capture_alignment_separates_onset_delay_and_internal_zero_run(self):
        samples=np.zeros((960000,2),dtype='<f4')
        samples[100800:676800]=.125
        samples[200000:200256]=0
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'capture.f32le';samples.tofile(path)
            result=audio(path)
        self.assertEqual(result['first_nonzero_after_scheduled_note_on_seconds'],2.1)
        held=result['windows']['held_including_attack']
        self.assertEqual(held['initial_zero_frames'],4800)
        self.assertEqual(held['exact_zero_frames'],5056)
        self.assertEqual(len(held['internal_zero_runs']),1)
        self.assertEqual(held['internal_zero_runs'][0]['frames'],256)
        self.assertAlmostEqual(held['internal_zero_runs'][0]['start_seconds'],200000/48000)
        self.assertEqual(result['windows']['stable_hold']['exact_zero_frames'],256)

    def test_incomplete_capture_is_not_reduced_as_success(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'partial.f32le';path.write_bytes(b'\x00'*8)
            with self.assertRaises(ValueError):audio(path)


if __name__=='__main__':unittest.main()
