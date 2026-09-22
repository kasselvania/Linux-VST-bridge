import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "rpi1_validate_transfer", ROOT / "rpi1" / "validate_transfer.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class TransferBaselineTests(unittest.TestCase):
    def test_repository_baseline_passes(self):
        result = MODULE.validate(ROOT, ROOT / "rpi1" / "transfer-baseline.json")
        self.assertEqual(result["gates"]["transfer_baseline"], "PASSED")

    def test_changed_record_digest_refuses(self):
        baseline = json.loads((ROOT / "rpi1" / "transfer-baseline.json").read_text())
        changed = copy.deepcopy(baseline)
        changed["repository_records"][0]["sha256"] = "00" * 32
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "transfer-baseline.json"
            manifest.write_text(json.dumps(changed))
            with self.assertRaisesRegex(MODULE.BaselineError, "record digest"):
                MODULE.validate(ROOT, manifest)

    def test_changed_runner_refuses(self):
        baseline = json.loads((ROOT / "rpi1" / "transfer-baseline.json").read_text())
        changed = copy.deepcopy(baseline)
        changed["deck_runner"]["id"] = "ambient-latest"
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "transfer-baseline.json"
            manifest.write_text(json.dumps(changed))
            with self.assertRaisesRegex(MODULE.BaselineError, "runner id"):
                MODULE.validate(ROOT, manifest)


if __name__ == "__main__":
    unittest.main()
