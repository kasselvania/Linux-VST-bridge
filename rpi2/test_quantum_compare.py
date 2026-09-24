import unittest
from recorder_compare import outstanding_frames
class QuantumCounters(unittest.TestCase):
    def test_completed_frames_not_call_count_drive_drain(self):
        for q in (256,512):
            c=dict(callbacks=10,paused_frames=512,bridge_processed=100,
                   bridge_processed_frames=4097,processing_quantum=q)
            self.assertEqual(outstanding_frames(c,q),511)
            c['bridge_processed']+=1 # zero-frame flush must not fake progress
            self.assertEqual(outstanding_frames(c,q),511)
            with self.assertRaises(ValueError):outstanding_frames(c,768-q)
            del c['bridge_processed_frames']
            with self.assertRaises(KeyError):outstanding_frames(c,q)
