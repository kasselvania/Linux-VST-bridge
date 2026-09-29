"""Private internal-key test; it does not designate a product release key."""
import json
from pathlib import Path
import shutil
import subprocess
import unittest

import assemble
import sign_release
import verify_release
import test_assemble


@unittest.skipUnless(shutil.which("gpg"), "GPG unavailable on this host")
class SignedRelease(unittest.TestCase):
    def test_exact_signature_and_changed_inputs(self):
        fixture = test_assemble.PackageAssembly("test_binary_roster_and_external_runtime_contract")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        root = fixture.root
        output = root / "staged"
        assemble.build(fixture.spec, output, 1234567890)
        package = root / "linux-vst-bridge-beta-0.1.0beta1-1-x86_64.pkg.tar.zst"
        test_assemble.package_archive(output, package)
        home = root / "gpg"
        home.mkdir(mode=0o700)
        def gpg(*args):
            return subprocess.check_output(["gpg", "--homedir", str(home), "--batch",
                                            "--pinentry-mode", "loopback", "--no-tty", *args])
        gpg("--passphrase", "", "--quick-gen-key", "PKG0 Internal Test <pkg0@example.invalid>",
            "ed25519", "sign", "1d")
        rows = gpg("--with-colons", "--list-secret-keys").decode().splitlines()
        fingerprint = next(row.split(":")[9] for row in rows if row.startswith("fpr:"))
        bundle = root / "signed"
        sign_release.sign(package, output / "RELEASE_MANIFEST.json", home, fingerprint,
                          "internal_test", bundle, structure_only=True)
        with self.assertRaisesRegex(ValueError, "packaged preparation kit"):
            sign_release.sign(package, output / "RELEASE_MANIFEST.json", home, fingerprint,
                              "release", root / "old-package-release", source_root=root)
        trusted = root / "trusted.asc"
        trusted.write_bytes(gpg("--armor", "--export", fingerprint))
        self.assertEqual(verify_release.verify(bundle, trusted, fingerprint,
                                               "internal_test", True)["files"], len(fixture.files) + 2)
        changed = bundle / "SHA256SUMS"
        changed.chmod(0o644)
        changed.write_text(changed.read_text() + "extra\n")
        with self.assertRaises(ValueError):
            verify_release.verify(bundle, trusted, fingerprint, "internal_test", True)
        changed.write_text(f"{json.loads((bundle / 'SIGNING_IDENTITY.json').read_text())['package_sha256']}  {package.name}\n"
                           f"{json.loads((bundle / 'SIGNING_IDENTITY.json').read_text())['release_manifest_sha256']}  RELEASE_MANIFEST.json\n")
        wrong = "0" * 40
        with self.assertRaises(ValueError):
            verify_release.verify(bundle, trusted, wrong, "internal_test", True)
        signature = bundle / (package.name + ".sig")
        signature.chmod(0o644)
        signature.write_bytes(b"not a release signature")
        with self.assertRaises(subprocess.CalledProcessError):
            verify_release.verify(bundle, trusted, fingerprint, "internal_test", True)

        fixture.add_kit()
        kit_output = root / "kit-staged"
        assemble.build(fixture.spec, kit_output, 1234567890)
        kit_package = root / "kit-package" / package.name
        kit_package.parent.mkdir()
        test_assemble.package_archive(kit_output, kit_package)
        with self.assertRaisesRegex(ValueError, "source-backed"):
            sign_release.sign(kit_package, kit_output / "RELEASE_MANIFEST.json", home,
                              fingerprint, "release", root / "unverified-release")
        kit_bundle = root / "kit-signed"
        sign_release.sign(kit_package, kit_output / "RELEASE_MANIFEST.json", home,
                          fingerprint, "internal_test", kit_bundle, structure_only=True)
        self.assertEqual(verify_release.verify(kit_bundle, trusted, fingerprint,
                                               "internal_test", True)["files"], len(fixture.files) + 2)


if __name__ == "__main__":
    unittest.main()
