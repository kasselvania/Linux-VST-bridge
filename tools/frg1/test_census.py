#!/usr/bin/env python3
import hashlib
import importlib.util
import json
import pathlib
import stat
import subprocess
import sys
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
        self.assertEqual(lock["historical_profile"]["class"]["class_id"], "41727475415649536772616E50726F63")
        self.assertEqual(lock["historical_profile"]["parameter_count"], 2348)
        self.assertTrue(lock["successor_verification"]["required_class_continuity"])
        self.assertEqual(lock["retained_ubuntu_environment"]["execution"]["launcher_verb"], "runinprefix")
        self.assertEqual(lock["retained_ubuntu_environment"]["execution"]["synthetic_home"], "/home/ua1")
        self.assertFalse(lock["retained_ubuntu_environment"]["execution"]["network_shared"])
        self.assertEqual(lock["retained_ubuntu_environment"]["custody"]["pfx_mode"], "0775")
        self.assertEqual(census.READY_TIMEOUT, 180.0)

    def test_predecessor_profile_bytes_are_the_locked_starting_point(self) -> None:
        lock = census.load_lock(ROOT / "compatibility/frg1/input.lock.json")
        profile = ROOT / "compatibility/arturia-efx-fragments.json"
        self.assertEqual(census.sha256_file(profile), lock["historical_profile"]["profile_file_sha256"])
        value = json.loads(profile.read_text(encoding="utf-8"))
        self.assertEqual(value["module_sha256"], lock["historical_profile"]["module_sha256"])
        self.assertEqual(value["class"], lock["historical_profile"]["class"])
        self.assertEqual(value["capabilities"], lock["historical_profile"]["capabilities"])

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

    def test_streams_are_drained_before_synthetic_readiness(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            ready = pathlib.Path(directory) / "synthetic.ready"
            payload_size = 256 * 1024
            program = (
                "import os,pathlib,sys;"
                f"os.write(1,b'o'*{payload_size});"
                f"os.write(2,b'e'*{payload_size});"
                "pathlib.Path(sys.argv[1]).write_bytes(b'ready')"
            )
            child = subprocess.Popen(
                [sys.executable, "-c", program, str(ready)],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
            drain = census.BoundedDrain(
                child,
                stdout_limit=payload_size,
                stderr_limit=payload_size,
                label="synthetic",
            )
            census.wait_for_readiness(child, drain, ready, 5.0)
            census.wait_for_completion(child, drain, 5.0)
            output, error = drain.output()
            self.assertEqual(output, b"o" * payload_size)
            self.assertEqual(error, b"e" * payload_size)

    def test_handshake_binds_every_execution_input(self) -> None:
        lock = census.load_lock(ROOT / "compatibility/frg1/input.lock.json")
        handshake = census.expected_handshake(lock, "ab" * 16).decode()
        self.assertIn(lock["fixture"]["module"]["sha256"], handshake)
        self.assertIn(lock["windows_host"]["artifact"]["sha256"], handshake)
        self.assertIn(lock["windows_host"]["source_manifest"]["sha256"], handshake)
        self.assertIn(census.LOCK_SHA256, handshake)
        self.assertIn("component_case=class:41727475415649536772616E50726F63", handshake)

    def test_retained_prefix_custody_is_exact_and_snapshotted_without_source_mutation(self) -> None:
        lock = json.loads((ROOT / "compatibility/frg1/input.lock.json").read_text())
        module_bytes = b"successor-module"
        lock["fixture"]["module"]["byte_length"] = len(module_bytes)
        lock["fixture"]["module"]["sha256"] = hashlib.sha256(module_bytes).hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            home = pathlib.Path(directory)
            custody = lock["retained_ubuntu_environment"]["custody"]
            prefix = home / custody["prefix_home_relative"]
            module = prefix / lock["retained_ubuntu_environment"]["module_prefix_relative"]
            module.parent.mkdir(parents=True)
            (prefix / "pfx").chmod(0o775)
            module.write_bytes(module_bytes)
            owner = {
                "identity": custody["owner_identity"],
                "kind": "asc-prefix",
                "owner": "kasselvania/Linux-VST-bridge-ubuntu-lab",
                "schema": "linux-vst-bridge-ubuntu-lab-ua1-owner/v1",
                "slice": "UA1",
            }
            owner_bytes = census.canonical_json(owner) + b"\n"
            (prefix / census.OWNER_FILE).write_bytes(owner_bytes)
            custody["owner_file_sha256"] = hashlib.sha256(owner_bytes).hexdigest()
            for name in ("config_info", "pfx.lock", "tracked_files", "version"):
                (prefix / name).write_bytes(b"")
            source_lock = home / custody["source_lock_home_relative"]
            source_lock.parent.mkdir(parents=True)
            source_lock.write_bytes(b"locked-custody\n")
            custody["source_lock_byte_length"] = source_lock.stat().st_size
            custody["source_lock_sha256"] = census.sha256_file(source_lock)
            resolved_prefix = prefix.resolve()
            admitted = census.validate_retained_prefix(resolved_prefix, source_lock.resolve(), lock, home=home)
            state = home.resolve() / "operation-state"
            snapshot, receipt = census.snapshot_retained_prefix(resolved_prefix, state, lock)
            self.assertEqual(admitted["owner_identity"], custody["owner_identity"])
            self.assertEqual(census.sha256_file(module), lock["fixture"]["module"]["sha256"])
            self.assertEqual(census.sha256_file(snapshot / lock["retained_ubuntu_environment"]["module_prefix_relative"]), lock["fixture"]["module"]["sha256"])
            self.assertTrue(receipt["source_unchanged_after_snapshot"])

    def test_retained_prefix_custody_rejects_an_alias_or_foreign_path(self) -> None:
        lock = json.loads((ROOT / "compatibility/frg1/input.lock.json").read_text())
        with tempfile.TemporaryDirectory() as directory:
            home = pathlib.Path(directory)
            foreign = home / "foreign-prefix"
            foreign.mkdir()
            source_lock = home / "foreign-lock"
            source_lock.write_bytes(b"")
            with self.assertRaisesRegex(census.CensusError, "retained prefix path changed"):
                census.validate_retained_prefix(foreign, source_lock, lock, home=home)

    def test_successor_delta_requires_predecessor_class_continuity(self) -> None:
        lock = census.load_lock(ROOT / "compatibility/frg1/input.lock.json")
        expected = lock["successor_verification"]["expected_class"]
        records = [
            {"state": "ap12_class", **expected},
            {"state": "ap8_parameter_count", "count": 2400},
            {"state": "ap12_capabilities", "float32_result": 0, "float64_result": -1},
            {"state": "ap8_controller_association", "combined": False, "class_id": "00112233445566778899AABBCCDDEEFF"},
        ]
        delta = census.validate_successor_delta(records, lock)
        self.assertTrue(delta["class_continuity"])
        self.assertTrue(delta["parameter_count_changed"])
        records[0] = {**records[0], "class_id": "00" * 16}
        with self.assertRaisesRegex(census.CensusError, "successor class_id differs"):
            census.validate_successor_delta(records, lock)

    def test_scanner_namespace_projects_the_gpu_instead_of_repeating_headless_attempt(self) -> None:
        source = MODULE_PATH.read_text(encoding="utf-8")
        self.assertIn('"--dev-bind", "/dev/dri", "/dev/dri"', source)
        self.assertIn("argv.extend(exact_gpu_sysfs_argv())", source)
        self.assertIn('retained_execution["launcher_verb"]', source)
        self.assertNotIn('"/opt/frg1/runner/proton", "run",', source)

    def test_attempt_one_is_failure_not_profile_authority(self) -> None:
        result = json.loads((ROOT / "evidence/frg1/attempt-001/result.json").read_text())
        self.assertEqual(result["classification"], "FRG1_FACTORY_CENSUS_READINESS_TIMEOUT")
        self.assertFalse(result["execution"]["retry_performed"])
        self.assertEqual(result["cleanup"]["owned_process_count"], 0)
        self.assertFalse(result["retention"]["raw_stdout_retained"])
        self.assertFalse(result["scope"]["steam_deck_contacted"])
        self.assertFalse(result["scope"]["ubuntu_deployment_changed"])

    def test_attempt_two_is_post_gate_failure_not_profile_authority(self) -> None:
        result = json.loads((ROOT / "evidence/frg1/attempt-002/result.json").read_text())
        self.assertEqual(result["classification"], "FRG1_FACTORY_CENSUS_POST_GATE_EMPTY_OUTPUT")
        self.assertTrue(result["execution"]["readiness"]["exact_binding_verified"])
        self.assertTrue(result["execution"]["gate"]["present"])
        self.assertEqual(result["execution"]["structured_record_count"], 0)
        self.assertFalse(result["execution"]["retry_performed"])
        self.assertFalse(result["observations"]["factory_or_class_authority_obtained"])
        self.assertEqual(result["cleanup"]["frg1_environment_process_count"], 0)
        self.assertFalse(result["scope"]["steam_deck_contacted"])
        self.assertFalse(result["scope"]["ubuntu_publication_performed"])

    def test_attempt_three_is_retained_as_a_blank_prefix_mismatch(self) -> None:
        result = json.loads((ROOT / "evidence/frg1/attempt-003/result.json").read_text())
        self.assertEqual(result["classification"], "FRG1_FACTORY_CENSUS_POST_GATE_HOST_EXIT_90")
        self.assertEqual(result["execution"]["scanner_child_exit_status"], 90)
        self.assertEqual(result["execution"]["structured_record_count"], 0)
        self.assertFalse(result["scope"]["retained_asc_prefix_used"])
        self.assertFalse(result["observations"]["factory_or_class_authority_obtained"])
        self.assertFalse(result["scope"]["steam_deck_contacted"])
        self.assertFalse(result["scope"]["ubuntu_publication_performed"])


if __name__ == "__main__":
    unittest.main()
