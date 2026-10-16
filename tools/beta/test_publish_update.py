"""Feed publishing checks with synthetic installers and a throwaway key."""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import publish_update  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "portable"))


def new_key(path):
    subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(path)],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    path.chmod(0o600)
    public = subprocess.check_output(["openssl", "pkey", "-in", str(path), "-pubout", "-outform", "DER"])
    return public[12:].hex()


def openssl_sign(key, message, root):
    (root / "message").write_bytes(message)
    subprocess.run(["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", str(key), "-in", str(root / "message"),
                    "-out", str(root / "signature")], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return (root / "signature").read_bytes()


@unittest.skipUnless(shutil.which("openssl"), "OpenSSL builder dependency unavailable")
class PublishUpdate(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="lvb-publish-test-"))
        self.addCleanup(shutil.rmtree, self.root)
        self.key = self.root / "internal-test.pem"
        self.public = new_key(self.key)
        self.ready = self.root / "ready"
        self.ready.mkdir()
        self.notes = self.root / "notes.txt"
        self.notes.write_text("Test148\n- In-app updates.\n")

    def installer(self, version, system, key=None, folder=None, payload_suffix=b""):
        base = b"\x7fELF synthetic installer launcher"
        payload = f"payload {system}".encode()
        manifest = json.dumps({"schema": 1, "package": "linux-vst-bridge-beta", "version": version,
                               "payload_sha256": hashlib.sha256(payload).hexdigest()}).encode()
        signature = openssl_sign(key or self.key, publish_update.PACKAGE_DOMAIN + b"internal_test\0"
                                 + hashlib.sha256(base).digest() + manifest, self.root)
        payload += payload_suffix
        path = (folder or self.ready) / f"Linux-VST-Bridge-{version}-{system}-internal.install"
        path.write_bytes(base + manifest + signature + payload + b"LVBUSER1"
                         + struct.pack("<QQQQ", len(base), len(manifest), 64, len(payload)))
        return path

    def build(self, output="feed", minimum="0.12.0test146", published="2026-10-16T12:00:00Z", **extra):
        args = ["build", "--ready", str(self.ready), "--notes", str(self.notes), "--min-version", minimum,
                "--private-key", str(self.key), "--public-key", self.public, "--key-class", "internal_test",
                "--output", str(self.root / output), "--published", published]
        for alias in extra.get("aliases", []):
            args += ["--alias", alias]
        stderr = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(stderr):
            code = publish_update.main(args)
        return code, stderr.getvalue()

    def upload(self, dest, feed="feed", *extra):
        stderr = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(stderr):
            code = publish_update.main(["upload", "--feed-dir", str(self.root / feed), "--ready", str(self.ready),
                                        "--dest", str(dest), "--public-key", self.public,
                                        "--key-class", "internal_test", *extra])
        return code, stderr.getvalue()

    def test_feed_lists_real_installers_and_is_signed_over_the_feed_domain(self):
        cachy = self.installer("0.12.0test148", "cachyos")
        steam = self.installer("0.12.0test148", "steamos")
        self.assertEqual(self.build(aliases=["arch=cachyos"]), (0, ""))
        data = (self.root / "feed" / "latest.json").read_bytes()
        feed = json.loads(data)
        self.assertEqual(feed["version"], "0.12.0test148")
        self.assertEqual(feed["channel"], "beta")
        self.assertEqual(feed["min_version"], "0.12.0test146")
        self.assertEqual(feed["notes"], "Test148\n- In-app updates.\n")
        self.assertEqual(sorted(feed["packages"]), ["arch", "cachyos", "steamos"])
        for system, path in (("cachyos", cachy), ("steamos", steam), ("arch", cachy)):
            self.assertEqual(feed["packages"][system], {"file": path.name, "size": path.stat().st_size,
                                                        "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
        signature = (self.root / "feed" / "latest.json.sig").read_bytes()
        self.assertEqual(len(signature), 64)
        self.assertTrue(publish_update.openssl_verify(self.public, b"Linux VST Bridge update feed v1\0internal_test\0" + data, signature))
        self.assertFalse(publish_update.openssl_verify(self.public, b"Linux VST Bridge user package v1\0internal_test\0" + data, signature))
        import user_package
        self.assertEqual(publish_update.PACKAGE_DOMAIN, user_package.DOMAIN)
        # Output is never overwritten.
        code, error = self.build()
        self.assertEqual(code, 1)
        self.assertIn("already exists", error)

    def test_refuses_mixed_versions_foreign_installers_wrong_key_and_downgraded_minimum(self):
        self.installer("0.12.0test148", "cachyos")
        other = self.root / "other.pem"
        new_key(other)
        foreign = self.installer("0.12.0test148", "steamos", key=other)
        code, error = self.build()
        self.assertEqual(code, 1)
        self.assertIn("signature does not verify", error)
        foreign.unlink()
        self.installer("0.12.0test146", "steamos")
        code, error = self.build()
        self.assertIn("different versions", error)
        (self.ready / "Linux-VST-Bridge-0.12.0test146-steamos-internal.install").unlink()
        public = self.public
        self.public = "00" * 32
        code, error = self.build()
        self.assertIn("signature does not verify", error)
        self.public = new_key(self.root / "unrelated.pem")
        code, error = self.build()
        self.assertEqual(code, 1)
        self.public = public
        code, error = self.build(minimum="0.12.0test150")
        self.assertIn("min version", error)
        self.notes.write_text("x" * (17 * 1024))
        code, error = self.build()
        self.assertIn("16 KiB", error)
        # The manager refuses C1 control characters too, so the feed never carries them.
        self.notes.write_text("ok \u0085 hidden\n")
        code, error = self.build()
        self.assertIn("control characters", error)
        self.key.chmod(0o644)
        self.notes.write_text("ok\n")
        code, error = self.build()
        self.assertIn("private file", error)
        self.assertFalse((self.root / "feed" / "latest.json").exists())

    def test_upload_keeps_older_builds_and_refuses_to_replace_different_files(self):
        beta = self.root / "beta folder with spaces"
        beta.mkdir()
        (beta / "Linux-VST-Bridge-0.12.0test144-cachyos-internal.install").write_bytes(b"older build")
        (beta / "START HERE.txt").write_text("kept")
        self.installer("0.12.0test148", "cachyos")
        self.assertEqual(self.build(), (0, ""))
        self.assertEqual(self.upload(beta), (0, ""))
        names = sorted(p.name for p in beta.iterdir())
        self.assertEqual(names, ["Linux-VST-Bridge-0.12.0test144-cachyos-internal.install",
                                 "Linux-VST-Bridge-0.12.0test148-cachyos-internal.install",
                                 "START HERE.txt", "latest.json", "latest.json.sig"])
        self.assertEqual((beta / "latest.json").read_bytes(), (self.root / "feed" / "latest.json").read_bytes())
        self.assertEqual((beta / "Linux-VST-Bridge-0.12.0test144-cachyos-internal.install").read_bytes(), b"older build")
        self.assertEqual(os.stat(beta / "latest.json").st_mode & 0o777, 0o644)
        # Uploading again is a no-op for installers; the feed is replaced in place.
        self.assertEqual(self.upload(beta), (0, ""))
        # A different file under the same name is never replaced.
        (beta / "Linux-VST-Bridge-0.12.0test148-cachyos-internal.install").chmod(0o644)
        (beta / "Linux-VST-Bridge-0.12.0test148-cachyos-internal.install").write_bytes(b"something else")
        code, error = self.upload(beta)
        self.assertEqual(code, 1)
        self.assertIn("refusing to replace", error)
        # A feed changed after signing is not uploaded.
        feed = self.root / "feed" / "latest.json"
        feed.write_bytes(feed.read_bytes().replace(b"test146", b"test140"))
        code, error = self.upload(self.root / "unused")
        self.assertIn("feed signature does not verify", error)

    def test_build_checks_what_the_manager_checks(self):
        # Payload bytes changed after signing: the manager would refuse to stage it.
        self.installer("0.12.0test148", "cachyos", payload_suffix=b"!")
        code, error = self.build()
        self.assertEqual(code, 1)
        self.assertIn("payload differs", error)
        for path in self.ready.iterdir():
            path.unlink()
        path = self.installer("0.12.0test148", "cachyos")
        data = bytearray(path.read_bytes())
        data[-40 - 3] ^= 1
        path.write_bytes(bytes(data))
        code, error = self.build()
        self.assertIn("payload differs", error)
        path.write_bytes(b"MZ" + bytes(data[2:]))
        code, error = self.build()
        self.assertIn("not an ELF installer", error)
        self.installer("0.12.0test148", "cachyos")
        code, error = self.build(published="2026-10-16 12:00")
        self.assertIn("published", error)

    def test_upload_never_replaces_a_newer_feed_unless_forced(self):
        beta = self.root / "beta"
        beta.mkdir()
        self.installer("0.12.0test148", "cachyos")
        self.assertEqual(self.build(), (0, ""))
        (beta / "latest.json").write_text(json.dumps({"schema": 1, "version": "0.12.0test150"}))
        code, error = self.upload(beta)
        self.assertEqual(code, 1)
        self.assertIn("already offers 0.12.0test150", error)
        self.assertEqual(json.loads((beta / "latest.json").read_text())["version"], "0.12.0test150")
        self.assertEqual(self.upload(beta, "feed", "--force-older"), (0, ""))
        self.assertEqual(json.loads((beta / "latest.json").read_text())["version"], "0.12.0test148")
        with self.assertRaisesRegex(ValueError, "must not start"):
            publish_update.Destination("-oProxyCommand=evil:/beta")

    def test_versions_order_like_the_manager(self):
        key = publish_update.version_key
        self.assertLess(key("0.12.0test99"), key("0.12.0test100"))
        self.assertLess(key("0.12.0test144"), key("0.12.1"))
        self.assertEqual(key("0.12.0test146"), key("0.12.0test146"))


if __name__ == "__main__":
    unittest.main()
