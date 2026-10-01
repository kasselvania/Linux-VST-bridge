"""Signing/wrapper checks; synthetic bytes are never installed or executed."""
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import unittest

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
