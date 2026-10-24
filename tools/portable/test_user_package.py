"""Signing/wrapper checks; synthetic bytes are never installed or executed."""
import hashlib
import io
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tarfile
import unittest
from unittest.mock import patch

import user_package
import test_assemble
import assemble


@unittest.skipUnless(shutil.which("openssl"), "OpenSSL builder dependency unavailable")
class UserPackage(unittest.TestCase):
    def setUp(self):
        self.fixture = test_assemble.PackageAssembly()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root = self.fixture.root
        self.key = self.root / "internal-test.pem"
        subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(self.key)],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.key.chmod(0o600)
        public = subprocess.check_output(["openssl", "pkey", "-in", str(self.key),
                                          "-pubout", "-outform", "DER"])
        self.public = public[12:].hex()
        for row in self.fixture.files:
            if row["kind"] in ("manager", "frontend"):
                p = Path(row["source"])
                p.write_bytes(p.read_bytes() + self.public.encode())
                row["sha256"] = hashlib.sha256(p.read_bytes()).hexdigest()
            elif row['kind'] == 'compliance':
                # A real historical declaration, with no native build fields.
                data = json.dumps(dict(classification='internal_test_only', release_authorized=False)).encode()
                Path(row['source']).write_bytes(data)
                row['sha256'] = hashlib.sha256(data).hexdigest()
        self.fixture.add_kit()
        self.stage = self.root / "staged"
        assemble.build(self.fixture.spec, self.stage, 1234567890)
        self.installer = self.root / "source-owned-installer-fixture"
        self.installer.write_bytes(b"\x7fELF synthetic verifier fixture " + self.public.encode())

    def wrap(self, name="candidate.run", key_class="internal_test", public=None):
        return user_package.package(self.installer, self.stage, self.key,
                                    public or self.public, key_class, self.root / name)

    def test_wire_signature_and_canonical_payload_identity(self):
        identity = self.wrap()
        data = (self.root / "candidate.run").read_bytes()
        self.assertEqual(data[-40:-32], b"LVBUSER1")
        offset, extent, sig_size, payload_size = struct.unpack("<QQQQ", data[-32:])
        self.assertEqual(sig_size, 64)
        self.assertEqual(offset, self.installer.stat().st_size)
        manifest = data[offset:offset + extent]
        self.assertEqual(manifest, (self.stage / "RELEASE_MANIFEST.json").read_bytes())
        signature = data[offset + extent:offset + extent + 64]
        payload = data[offset + extent + 64:-40]
        self.assertEqual(len(payload), payload_size)
        self.assertEqual(hashlib.sha256(payload).hexdigest(), json.loads(manifest)["payload_sha256"])
        self.assertEqual(identity["key_class"], "internal_test")
        self.assertEqual((self.root / "candidate.run").stat().st_mode & 0o777, 0o555)
        message = self.root / "message"
        message.write_bytes(user_package.DOMAIN + b"internal_test\0"
                            + hashlib.sha256(data[:offset]).digest() + manifest)
        signed = self.root / "signature"
        signed.write_bytes(signature)
        subprocess.run(["openssl", "pkeyutl", "-verify", "-rawin", "-inkey", str(self.key),
                        "-in", str(message), "-sigfile", str(signed)], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        with self.assertRaisesRegex(ValueError, "already exists"):
            self.wrap()

    def test_wrong_trust_changed_payload_and_unapproved_release_refuse(self):
        with self.assertRaisesRegex(ValueError, "trust root"):
            self.wrap(public="00" * 32)
        with self.assertRaisesRegex(ValueError, "release obligations"):
            self.wrap(key_class="release")
        payload = self.stage / "payload.tar"
        payload.write_bytes(payload.read_bytes() + b"changed")
        with self.assertRaisesRegex(ValueError, "payload differs"):
            self.wrap()
        self.assertFalse((self.root / "candidate.run").exists())

    def native_pair(self):
        frontend = next(row for row in self.fixture.files if row['kind'] == 'frontend')
        public = {key: self.fixture.spec['source_' + key] for key in ('head', 'tree')}
        manager = dict(schema=1, bridge_source=public,
            frontend_source={**public, 'manifest':'crates/window/Cargo.toml',
                             'manifest_sha256':'1'*64, 'cargo_lock_sha256':'2'*64},
            target='x86_64-unknown-linux-gnu', binary='linux-audio-compatibility-manager',
            features=['live'], portable_public_key=self.public, portable_key_class='internal_test',
            artifact_sha256=frontend['sha256'])
        frontend.pop('build_head', None)
        frontend.pop('build_tree', None)
        frontend['frontend_build'] = manager
        installer = {**manager, 'binary':'linux-vst-bridge-install'}
        self.embed_installer(installer)
        return dict(classification='internal_test_only', release_authorized=False,
                    frontend_build=manager, installer_build=installer)

    def embed_installer(self, record):
        metadata = {key: value for key, value in record.items() if key != 'artifact_sha256'}
        data = b'\x7fELF native fixture ' + self.public.encode() + json.dumps(
            metadata, sort_keys=True, separators=(',', ':')).encode()
        self.installer.write_bytes(data)
        record['artifact_sha256'] = hashlib.sha256(data).hexdigest()

    def stage_compliance(self, compliance):
        row = next(row for row in self.fixture.files if row['kind'] == 'compliance')
        data = json.dumps(compliance, sort_keys=True).encode()
        Path(row['source']).write_bytes(data)
        row['sha256'] = hashlib.sha256(data).hexdigest()
        self.stage = self.root / 'native-staged'
        assemble.build(self.fixture.spec, self.stage, 1234567890)

    def test_native_pair_is_bound_before_signing_without_changing_wire_format(self):
        compliance = self.native_pair()
        self.stage_compliance(compliance)
        self.wrap()
        data = (self.root / 'candidate.run').read_bytes()
        offset, _, _, _ = struct.unpack('<QQQQ', data[-32:])
        self.assertEqual(data[:offset], self.installer.read_bytes())
        self.assertEqual(hashlib.sha256(data[:offset]).hexdigest(),
                         compliance['installer_build']['artifact_sha256'])
        self.assertEqual(data[-40:-32], b'LVBUSER1')

    def test_valid_historical_private_frontend_keeps_its_legacy_installer_route(self):
        compliance = self.native_pair()
        compliance['frontend_build']['frontend_source'].update(
            manifest='apps/bridge/Cargo.toml', head='d'*40, tree='e'*40)
        del compliance['installer_build']
        self.installer.write_bytes(b'\x7fELF historical installer fixture ' + self.public.encode())
        self.stage_compliance(compliance)
        self.wrap()
        self.assertTrue((self.root/'candidate.run').is_file())

    def test_native_payload_refuses_egui_base_before_signer_invocation(self):
        self.stage_compliance(self.native_pair())
        self.installer.write_bytes(b'\x7fELF historical egui fixture ' + self.public.encode())
        with patch.object(user_package.subprocess, 'run') as signer:
            with self.assertRaisesRegex(ValueError, 'artifact/build identity'):
                self.wrap()
        signer.assert_not_called()
        self.assertFalse((self.root / 'candidate.run').exists())

    def test_native_payload_requires_installer_record(self):
        compliance = self.native_pair()
        del compliance['installer_build']
        self.stage_compliance(compliance)
        with self.assertRaisesRegex(ValueError, 'exact native installer build'):
            self.wrap()
        self.assertFalse((self.root / 'candidate.run').exists())

    def test_native_pair_refuses_another_locked_dependency_identity(self):
        compliance = self.native_pair()
        record = compliance['installer_build']
        record['frontend_source'] = {**record['frontend_source'], 'cargo_lock_sha256':'4'*64}
        self.embed_installer(record)
        self.stage_compliance(compliance)
        with self.assertRaisesRegex(ValueError, 'source or trust identities differ'):
            self.wrap()
        self.assertFalse((self.root / 'candidate.run').exists())

    def test_native_pair_requires_embedded_installer_declaration_even_with_updated_hash(self):
        compliance = self.native_pair()
        self.installer.write_bytes(b'\x7fELF unbound fixture ' + self.public.encode())
        compliance['installer_build']['artifact_sha256'] = hashlib.sha256(self.installer.read_bytes()).hexdigest()
        self.stage_compliance(compliance)
        with self.assertRaisesRegex(ValueError, 'embedded build identity differs'):
            self.wrap()
        self.assertFalse((self.root / 'candidate.run').exists())

    def test_invalid_or_missing_compliance_cannot_fall_back_to_legacy_signing(self):
        original = (self.stage/'payload.tar').read_bytes()
        original_manifest = json.loads((self.stage/'RELEASE_MANIFEST.json').read_bytes())
        name = 'usr/share/doc/linux-vst-bridge-beta/COMPLIANCE_MANIFEST.json'
        for invalid, refusal in ((b'not-json', 'compliance declaration is missing or invalid'),
                                 (b'[]', 'compliance declaration is missing or invalid'),
                                 (None, 'compliance declaration is missing or invalid'),
                                 (b'{"frontend_build":null}', 'frontend build schema'),
                                 (b'{"installer_build":null}', 'exact native installer build')):
            with self.subTest(compliance=invalid):
                manifest = json.loads(json.dumps(original_manifest))
                with tarfile.open(fileobj=io.BytesIO(original)) as source, tarfile.open(self.stage/'payload.tar', 'w') as target:
                    for entry in source:
                        data = source.extractfile(entry).read()
                        if entry.name == name:
                            if invalid is None:
                                continue
                            data = invalid
                            entry.size = len(data)
                            row = next(row for row in manifest['files'] if row['destination'] == name)
                            row.update(size=len(data), sha256=hashlib.sha256(data).hexdigest())
                        target.addfile(entry, io.BytesIO(data))
                manifest['payload_sha256'] = hashlib.sha256((self.stage/'payload.tar').read_bytes()).hexdigest()
                (self.stage/'RELEASE_MANIFEST.json').write_text(json.dumps(manifest))
                with patch.object(user_package.subprocess, 'run') as signer:
                    with self.assertRaisesRegex(ValueError, refusal):
                        self.wrap()
                signer.assert_not_called()
                self.assertFalse((self.root/'candidate.run').exists())
