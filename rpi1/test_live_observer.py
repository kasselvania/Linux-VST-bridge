import json
import unittest

from live_observer import Model


def health(flags, temperature):
    return "RPI1_HEALTH " + json.dumps({
        "monotonic_seconds": 4527.756,
        "temperature_millidegrees": str(temperature),
        "throttled": f"throttled=0x{flags:x}",
        "voltage": "EXT5V_V volt(24)=4.83874000V",
        "mem_available": "3840288 kB",
        "cohorts": {"lvb-rpi1-0123456789abcdef0123456789abcdef.service": {
            "processes": 15, "memory.current": "3889369088"}},
    })


class LiveObserverTests(unittest.TestCase):
    def test_historical_thermal_flags_do_not_become_current_undervoltage(self):
        model = Model()
        model.ingest_health(health(0x80008, 83700))
        snapshot = model.snapshot()
        self.assertTrue(snapshot["health"]["soft_temperature_limit_now"])
        self.assertFalse(snapshot["health"]["undervoltage_now"])
        model.ingest_health(health(0xe0000, 76550))
        snapshot = model.snapshot()
        self.assertFalse(snapshot["health"]["soft_temperature_limit_now"])
        self.assertTrue(snapshot["health"]["thermal_limit_seen"])
        self.assertFalse(snapshot["health"]["undervoltage_seen"])

    def test_gap_is_separate_from_editor_and_viewer_state(self):
        model = Model()
        model.ingest_journal("RPI1_READY jack_frames=512 bridge_frames=2048")
        model.ingest_journal("RPI1_CALLBACK_LIVE callbacks=100 missing_frames=2560 gaps=1 deadline_misses=0")
        model.ingest_journal("RPI1_EDITOR native_view=1 epoch=1 lifecycle=3 result=0")
        model.screen_probe("connected", "responding", True, 21)
        model.ingest_journal("RPI1_CALLBACK_LIVE callbacks=200 missing_frames=197120 gaps=3 deadline_misses=0")
        snapshot = model.snapshot()
        self.assertEqual(snapshot["callback"]["missing_frames"], 197120)
        self.assertEqual(snapshot["editor"]["meaning"], "awaiting focus")
        self.assertEqual(snapshot["screen"]["tunnel"], "connected")
        self.assertTrue(any("194560 newly missing frames" in item["message"]
                            for item in snapshot["events"]))
        model.ingest_journal("RPI1_CLEAN_SHUTDOWN session=0123456789abcdef0123456789abcdef")
        self.assertEqual(model.snapshot()["host"]["state"], "clean shutdown")
        model.ingest_journal("RPI1_READY jack_frames=512 bridge_frames=2048")
        self.assertIsNone(model.snapshot()["callback"])


if __name__ == "__main__":
    unittest.main()
