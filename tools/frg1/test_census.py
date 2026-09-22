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
        self.assertEqual(lock["retained_ubuntu_environment"]["execution"]["prefix_initialization"]["verb"], "getcompatpath")
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

    def test_retained_prefix_is_initialized_before_runinprefix(self) -> None:
        lock = census.load_lock(ROOT / "compatibility/frg1/input.lock.json")
        scanner = ["bwrap", "--clearenv", "--", "/opt/frg1/runner/proton", "runinprefix", "host.exe"]
        bootstrap = census.prefix_initialization_argv(scanner, lock)
        self.assertEqual(bootstrap, ["bwrap", "--clearenv", "--", "/opt/frg1/runner/proton", "getcompatpath", "/"])
        self.assertEqual(census.PREFIX_INITIALIZATION_TIMEOUT, 120.0)

    def test_direct_proton_entrypoint_is_closed_and_forwards_exact_arguments(self) -> None:
        entrypoint = ROOT / "tools/frg1/direct-proton-entrypoint.sh"
        with tempfile.TemporaryDirectory() as directory:
            proton = pathlib.Path(directory) / "proton"
            proton.write_text("#!/bin/sh\nprintf '%s\\n' \"$@\"\n", encoding="utf-8")
            proton.chmod(0o700)
            result = subprocess.run(
                [str(entrypoint), "--verb=run", "--", str(proton), "runinprefix", "C:\\host.exe"],
                check=True,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.stdout.splitlines(), ["runinprefix", "C:\\host.exe"])
            refused = subprocess.run(
                [str(entrypoint), "--verb=inspect", "--", str(proton)],
                check=False,
                capture_output=True,
            )
            self.assertEqual(refused.returncode, 64)

    def test_review_candidate_closes_census_runner_host_descriptor_and_native(self) -> None:
        profile_path = ROOT / "compatibility/frg1/arturia-efx-fragments.json"
        candidate_path = ROOT / "evidence/frg1/candidate.json"
        entrypoint = ROOT / "tools/frg1/direct-proton-entrypoint.sh"
        profile = json.loads(profile_path.read_text())
        candidate = json.loads(candidate_path.read_text())
        self.assertEqual(profile["claim"], "review_candidate")
        self.assertEqual(profile["revision"], 11)
        self.assertEqual(profile["module_sha256"], candidate["fixture"]["module_sha256"])
        self.assertEqual(profile["requirements"]["host_sha256"], candidate["windows_host"]["artifact_sha256"])
        self.assertEqual(profile["requirements"]["host_source_sha256"], candidate["windows_host"]["source_manifest_sha256"])
        self.assertEqual(profile["requirements"]["native_sha256"], candidate["native_proxy"]["sha256"])
        self.assertEqual(profile["requirements"]["descriptor_sha256"], candidate["descriptor"]["sha256"])
        self.assertEqual(census.sha256_file(profile_path), candidate["profile"]["file_sha256"])
        self.assertEqual(census.sha256_file(entrypoint), profile["requirements"]["runner"]["entry_point_sha256"])
        self.assertEqual(
            candidate["census"]["validator_replay_sha256"],
            census.sha256_file(ROOT / "evidence/frg1/attempt-005/validator-replay.json"),
        )
        for receipt in ("runner_receipt", "runtime_receipt"):
            self.assertIn(candidate["runner"][receipt]["sha256"], profile["requirements"]["runner"]["file_sha256"])

    def test_registered_successor_changes_only_revision_and_native_identity(self) -> None:
        historical = json.loads((ROOT / "compatibility/frg1/arturia-efx-fragments.json").read_text())
        successor = json.loads((ROOT / "compatibility/frg1/revision-12/arturia-efx-fragments.json").read_text())
        manifest = json.loads((ROOT / "compatibility/frg1/revision-12/qualification-package.json").read_text())
        receipt = json.loads((ROOT / "evidence/frg1/registered-proxy-repair.json").read_text())
        normalized = json.loads(json.dumps(successor))
        normalized["revision"] = historical["revision"]
        normalized["requirements"]["native_sha256"] = historical["requirements"]["native_sha256"]
        self.assertEqual(normalized["evidence"].pop(), "evidence/frg1/registered-proxy-repair.json")
        self.assertEqual(normalized, historical)
        self.assertEqual(successor["revision"], 12)
        self.assertEqual(receipt["historical_native_sha256"], historical["requirements"]["native_sha256"])
        self.assertEqual(receipt["native_proxy"]["sha256"], successor["requirements"]["native_sha256"])
        self.assertEqual(receipt["descriptor_sha256"], successor["requirements"]["descriptor_sha256"])
        self.assertIn("registered", receipt["build"]["cargo_flags"])
        self.assertFalse(receipt["build"]["network_shared"])
        self.assertTrue(receipt["native_proxy"]["managed_runtime_path_present"])
        self.assertTrue(receipt["native_proxy"]["legacy_ap9_performance_path_absent"])
        fingerprint = hashlib.sha256(json.dumps(successor, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()
        self.assertEqual(manifest["profile_fingerprint"], fingerprint)
        self.assertEqual(manifest["native_sha256"], receipt["native_proxy"]["sha256"])

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
            {"state": "ap8_factory", "factory": {"vendor_hex": "Arturia".encode().hex()}},
            {"state": "ap12_class", **expected},
            {"state": "ap8_parameter_count", "count": 2400},
            {"state": "ap12_capabilities", "float32_result": 0, "float64_result": -1},
            {"state": "ap8_controller_association", "combined": False, "class_id": "00112233445566778899AABBCCDDEEFF"},
        ]
        delta = census.validate_successor_delta(records, lock)
        self.assertTrue(delta["class_continuity"])
        self.assertTrue(delta["parameter_count_changed"])
        records[1] = {**records[1], "class_id": "00" * 16}
        with self.assertRaisesRegex(census.CensusError, "successor class_id differs"):
            census.validate_successor_delta(records, lock)

    def test_successor_delta_refuses_changed_factory_vendor(self) -> None:
        lock = census.load_lock(ROOT / "compatibility/frg1/input.lock.json")
        expected = lock["successor_verification"]["expected_class"]
        records = [
            {"state": "ap8_factory", "factory": {"vendor_hex": "Other".encode().hex()}},
            {"state": "ap12_class", **expected},
            {"state": "ap8_parameter_count", "count": 2415},
            {"state": "ap12_capabilities", "float32_result": 0, "float64_result": 1},
            {"state": "ap8_controller_association", "combined": True},
        ]
        with self.assertRaisesRegex(census.CensusError, "successor factory vendor differs"):
            census.validate_successor_delta(records, lock)

    def test_successor_delta_refuses_float64_support(self) -> None:
        lock = census.load_lock(ROOT / "compatibility/frg1/input.lock.json")
        expected = lock["successor_verification"]["expected_class"]
        records = [
            {"state": "ap8_factory", "factory": {"vendor_hex": "Arturia".encode().hex()}},
            {"state": "ap12_class", **expected},
            {"state": "ap8_parameter_count", "count": 2415},
            {"state": "ap12_capabilities", "float32_result": 0, "float64_result": 0},
            {"state": "ap8_controller_association", "combined": True},
        ]
        with self.assertRaisesRegex(census.CensusError, "successor gained unsupported float64 capability"):
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

    def test_attempt_four_identifies_missing_product_prefix_initialization(self) -> None:
        result = json.loads((ROOT / "evidence/frg1/attempt-004/result.json").read_text())
        self.assertEqual(result["classification"], "FRG1_RETAINED_PREFIX_RUNINPREFIX_INITIALIZATION_MISSING")
        self.assertEqual(result["execution"]["scanner_child_exit_status"], 53)
        self.assertTrue(result["retained_ubuntu_environment"]["source_unchanged_after_failure"])
        self.assertTrue(result["scope"]["retained_asc_prefix_snapshot_used"])
        self.assertFalse(result["observations"]["module_loading_reached"])
        self.assertFalse(result["scope"]["steam_deck_contacted"])

    def test_attempt_five_establishes_the_exact_successor_census(self) -> None:
        result = json.loads((ROOT / "evidence/frg1/attempt-005/result.json").read_text())
        self.assertEqual(result["classification"], "FRG1_SUCCESSOR_FACTORY_CENSUS_CONFIRMED")
        self.assertEqual(result["inspection"]["class"]["class_id"], "41727475415649536772616E50726F63")
        self.assertEqual(result["inspection"]["class"]["version"], "1.3.1.6566")
        self.assertEqual(result["inspection"]["parameter_count"], 2415)
        self.assertTrue(result["predecessor_delta"]["class_continuity"])
        self.assertTrue(result["execution"]["cleanup_confirmed"])
        self.assertTrue(result["retained_ubuntu_environment"]["source_unchanged_after_execution"])
        self.assertFalse(result["scope"]["steam_deck_contacted"])
        self.assertFalse(result["scope"]["ubuntu_publication_performed"])

    def test_attempt_five_validator_replay_closes_factory_and_precision(self) -> None:
        replay = json.loads((ROOT / "evidence/frg1/attempt-005/validator-replay.json").read_text())
        attempt = json.loads((ROOT / "evidence/frg1/attempt-005/result.json").read_text())
        self.assertEqual(replay["disposition"], "FRG1_SUCCESSOR_VALIDATOR_REPLAY_PASSED")
        self.assertEqual(replay["basis"]["input_lock_canonical_sha256"], census.LOCK_SHA256)
        self.assertEqual(
            replay["basis"]["input_lock_file_sha256"],
            census.sha256_file(ROOT / "compatibility/frg1/input.lock.json"),
        )
        self.assertEqual(replay["basis"]["validator_source_sha256"], census.sha256_file(MODULE_PATH))
        self.assertEqual(
            replay["basis"]["attempt_result_sha256"],
            census.sha256_file(ROOT / "evidence/frg1/attempt-005/result.json"),
        )
        self.assertEqual(
            replay["basis"]["private_report_sha256"],
            attempt["retention"]["private_report_sha256"],
        )
        self.assertEqual(replay["result"]["factory_vendor"], attempt["factory"]["vendor"])
        self.assertEqual(replay["result"]["float32_result"], attempt["inspection"]["float32_result"])
        self.assertEqual(replay["result"]["float64_result"], attempt["inspection"]["float64_result"])
        self.assertFalse(replay["scope"]["physical_execution_performed"])
        self.assertFalse(replay["scope"]["attempt_005_historical_identity_rewritten"])

    def test_integration_inputs_bind_both_exact_product_lines_without_execution(self) -> None:
        record = json.loads((ROOT / "evidence/frg1/integration-inputs.json").read_text())
        self.assertEqual(record["schema"], "linux-vst-bridge-fci1-integration-inputs/v1")
        self.assertEqual(record["frg1"]["head_commit"], "78f1e390b7690fdd41fb89ff783b1d6f156d0243")
        self.assertEqual(
            record["lead_platform"]["head_commit"],
            "c576787dcb7338c460d7fcdd2f8a5cf54aeadc79",
        )
        self.assertEqual(
            record["integration_merge"]["parents"],
            [
                "55a7aae032787d3e0501863b21348e17c73f854a",
                "c576787dcb7338c460d7fcdd2f8a5cf54aeadc79",
            ],
        )
        self.assertFalse(record["qualification_scope"]["physical_execution_authorized"])


if __name__ == "__main__":
    unittest.main()
