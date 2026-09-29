"""Internal-key Debian signature fixture; no customer release key is selected."""

import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "pkg0"))
import assemble
import deb
import release
import test_assemble


@unittest.skipUnless(shutil.which("gpg") and shutil.which("git"),
                     "GPG and Git are required for this fixture")
class SignedDebianRelease(unittest.TestCase):
    def test_exact_signature_kit_boundary_and_changed_inputs(self):
        fixture = test_assemble.PackageAssembly("test_binary_roster_and_external_runtime_contract")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        staged = fixture.root / "staged"
        assemble.build(fixture.spec, staged, 1_234_567_890)
        package = fixture.root / "linux-vst-bridge-beta_0.1.0beta1-1_amd64.deb"
        deb.build(staged, package, 1_234_567_890)
        home = fixture.root / "gpg"
        home.mkdir(mode=0o700)

        def gpg(*args):
            return subprocess.check_output(["gpg", "--homedir", str(home), "--batch",
                                            "--pinentry-mode", "loopback", "--no-tty", *args])

        gpg("--passphrase", "", "--quick-gen-key",
            "Portable Internal Test <portable@example.invalid>", "ed25519", "sign", "1d")
        rows = gpg("--with-colons", "--list-secret-keys").decode().splitlines()
        fingerprint = next(row.split(":")[9] for row in rows if row.startswith("fpr:"))
        trusted = fixture.root / "trusted.asc"
        trusted.write_bytes(gpg("--armor", "--export", fingerprint))
        with self.assertRaisesRegex(ValueError, "source-backed preparation kit"):
            release.sign(package, staged, home, fingerprint, "release", fixture.root / "not-release")
        bundle = fixture.root / "signed"
        release.sign(package, staged, home, fingerprint, "internal_test", bundle)
        self.assertEqual(release.verify(bundle, trusted, fingerprint, "internal_test")["files"],
                         len(fixture.files) + 2)
        with self.assertRaises(ValueError):
            release.verify(bundle, trusted, "0" * 40, "internal_test")
        signed_package = bundle / package.name
        signed_package.chmod(0o644)
        signed_package.write_bytes(signed_package.read_bytes() + b"changed")
        with self.assertRaises(ValueError):
            release.verify(bundle, trusted, fingerprint, "internal_test")

        fixture.add_kit()
        kit_staged = fixture.root / "kit-staged"
        assemble.build(fixture.spec, kit_staged, 1_234_567_890)
        kit_package = fixture.root / "kit" / package.name
        kit_package.parent.mkdir()
        deb.build(kit_staged, kit_package, 1_234_567_890)
        with self.assertRaisesRegex(ValueError, "source-backed preparation kit"):
            release.sign(kit_package, kit_staged, home, fingerprint, "release",
                         fixture.root / "missing-source")
        subprocess.run(["git", "init", "-q", str(fixture.root)], check=True)
        (fixture.root / "README").write_text("wrong source generation\n")
        subprocess.run(["git", "-C", str(fixture.root), "add", "README"], check=True)
        subprocess.run(["git", "-C", str(fixture.root), "-c", "user.name=Fixture",
                        "-c", "user.email=fixture@example.invalid", "commit", "-qm",
                        "wrong source"], check=True)
        with self.assertRaisesRegex(ValueError, "source head/tree differs"):
            release.sign(kit_package, kit_staged, home, fingerprint, "release",
                         fixture.root / "wrong-source", source_root=fixture.root)
        kit_bundle = fixture.root / "kit-signed"
        release.sign(kit_package, kit_staged, home, fingerprint,
                     "internal_test", kit_bundle)
        self.assertEqual(release.verify(kit_bundle, trusted, fingerprint,
                                        "internal_test")["files"], len(fixture.files) + 2)
        identity = json.loads((kit_bundle / "SIGNING_IDENTITY.json").read_text())
        self.assertEqual(identity["format"], "deb")
        with self.assertRaisesRegex(ValueError, "signing identity"):
            release.verify(kit_bundle, trusted, fingerprint, "release")


if __name__ == "__main__":
    unittest.main()
