#!/usr/bin/env python3
import hashlib
import importlib.util
import json
import pathlib
import stat
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
MODULE_PATH = pathlib.Path(__file__).with_name("census.py")
SPEC = importlib.util.spec_from_file_location("frg1_census", MODULE_PATH)
assert SPEC and SPEC.loader
census = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(census)


class CensusTests(unittest.TestCase):
    def test_tracked_lock_is_exact_and_closed(self) -> None:
        lock = census.load_lock(ROOT / "compatibility/frg1/input.lock.json")
        self.assertEqual(lock["schema"], census.SCHEMA)
        self.assertEqual(lock["fixture"]["embedded_version"], "1.3.1.6566")
        self.assertFalse(lock["qualification_scope"]["ordinary_activation_authority"])
        self.assertEqual(lock["runner"]["installed_tree"]["entries"], 8900)
        self.assertEqual(lock["runner"]["runtime"]["installed_tree"]["entries"], 638)

    def test_changed_lock_is_refused(self) -> None:
        value = json.loads((ROOT / "compatibility/frg1/input.lock.json").read_text())
        value["fixture"]["module"]["sha256"] = "00" * 32
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "lock.json"
            path.write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaisesRegex(census.CensusError, "lock changed"):
                census.load_lock(path)

    def test_tree_identity_matches_ua1_newline_contract(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "file").write_bytes(b"fixture")
            (root / "directory").mkdir()
            (root / "directory/link").symlink_to("../file")
            observed = census.tree_identity(root)
            roster = [
                {"mode": stat.S_IMODE((root / "directory").stat().st_mode), "path": "directory", "type": "directory"},
                {"mode": stat.S_IMODE((root / "directory/link").lstat().st_mode), "path": "directory/link", "target": "../file", "type": "symlink"},
                {"byte_length": 7, "mode": stat.S_IMODE((root / "file").stat().st_mode), "path": "file", "sha256": hashlib.sha256(b"fixture").hexdigest(), "type": "file"},
            ]
            expected = hashlib.sha256(census.canonical_json(roster) + b"\n").hexdigest()
            self.assertEqual(observed, {"entries": 3, "sha256": expected})

    def test_escaping_symlink_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "escape").symlink_to("../../outside")
            with self.assertRaisesRegex(census.CensusError, "escaping symlink"):
                census.tree_identity(root)

    def test_private_failure_streams_are_bounded_and_mode_600(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            result = census.retain_private_streams(root, b"one\n", b"two\n")
            self.assertEqual(result["stdout_sha256"], hashlib.sha256(b"one\n").hexdigest())
            self.assertEqual(stat.S_IMODE((root / "raw.stdout").stat().st_mode), 0o600)
            self.assertEqual(stat.S_IMODE((root / "raw.stderr").stat().st_mode), 0o600)
            with self.assertRaisesRegex(census.CensusError, "stdout exceeded"):
                census.retain_private_streams(root, b"x" * (census.STDOUT_LIMIT + 1), b"")

    def test_handshake_binds_every_execution_input(self) -> None:
        lock = census.load_lock(ROOT / "compatibility/frg1/input.lock.json")
        handshake = census.expected_handshake(lock, "ab" * 16).decode()
        self.assertIn(lock["fixture"]["module"]["sha256"], handshake)
        self.assertIn(lock["windows_host"]["artifact"]["sha256"], handshake)
        self.assertIn(lock["windows_host"]["source_manifest"]["sha256"], handshake)
        self.assertIn(census.LOCK_SHA256, handshake)
        self.assertIn("component_case=first-audio", handshake)

    def test_attempt_one_is_failure_not_profile_authority(self) -> None:
        result = json.loads((ROOT / "evidence/frg1/attempt-001/result.json").read_text())
        self.assertEqual(result["classification"], "FRG1_FACTORY_CENSUS_READINESS_TIMEOUT")
        self.assertFalse(result["execution"]["retry_performed"])
        self.assertEqual(result["cleanup"]["owned_process_count"], 0)
        self.assertFalse(result["retention"]["raw_stdout_retained"])
        self.assertFalse(result["scope"]["steam_deck_contacted"])
        self.assertFalse(result["scope"]["ubuntu_deployment_changed"])


if __name__ == "__main__":
    unittest.main()
