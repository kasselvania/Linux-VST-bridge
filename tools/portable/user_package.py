#!/usr/bin/env python3
"""Wrap a canonical PKG0 payload in the native, user-space installer.

The private signing key stays outside the source tree and package. The verifier
uses the public key compiled into the installer/manager, never a bundled key.
Ed25519 wire signature: domain + key class + NUL + exact release manifest.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "pkg0"))
from assemble import validate_release_roster  # noqa: E402

DOMAIN = b"Linux VST Bridge user package v1\0"


def package(installer, staged, private_key, public_key, key_class, output):
    if key_class not in ("release", "internal_test"):
        raise ValueError("key class")
    if len(public_key) != 64 or any(c not in "0123456789abcdef" for c in public_key):
        raise ValueError("separate public key identity")
    for source in (installer, staged / "RELEASE_MANIFEST.json", staged / "payload.tar", private_key):
        if not source.is_file() or source.is_symlink() or source.stat().st_uid != os.getuid():
            raise ValueError("builder source owner/type")
    if private_key.stat().st_mode & 0o077:
        raise ValueError("signing key must remain private")
    if output.exists():
        raise ValueError("output already exists")
    base = installer.read_bytes()
    if not base.startswith(b"\x7fELF") or public_key.encode() not in base:
        raise ValueError("installer was not built with the supplied public trust root")
    manifest_bytes = (staged / "RELEASE_MANIFEST.json").read_bytes()
    if len(manifest_bytes) > 4 * 1024 * 1024:
        raise ValueError("manifest extent")
    manifest = json.loads(manifest_bytes)
    roster = validate_release_roster(manifest)
    payload = (staged / "payload.tar").read_bytes()
    if len(payload) > 2_000_000_000 or hashlib.sha256(payload).hexdigest() != manifest["payload_sha256"]:
        raise ValueError("payload differs")
    import tarfile
    with tarfile.open(staged / "payload.tar") as archive:
        for name in ("usr/bin/linux-vst-bridge", "usr/bin/linux-audio-compatibility-manager"):
            component = archive.extractfile(name).read()
            if public_key.encode() not in component:
                raise ValueError("manager/frontend was not built with the same public trust root")
    if key_class == "release":
        with tarfile.open(staged / "payload.tar") as archive:
            try:
                compliance = json.load(archive.extractfile("usr/share/doc/linux-vst-bridge-beta/COMPLIANCE_MANIFEST.json"))
            except (ValueError, TypeError) as error:
                raise ValueError("customer release obligations remain unselected") from error
        if (compliance.get("classification") != "customer_release"
                or any(compliance.get(k) is not True for k in ("release_authorized",
                           "third_party_notice_archive_complete", "customer_signing_key_selected"))):
            raise ValueError("customer release obligations or signing authority remain unselected")
    with tempfile.TemporaryDirectory(prefix="lvb-user-sign-") as directory:
        temporary = Path(directory)
        public = subprocess.check_output(["openssl", "pkey", "-in", str(private_key),
                                         "-pubout", "-outform", "DER"], stderr=subprocess.DEVNULL)
        # RFC 8410 Ed25519 SubjectPublicKeyInfo, exact algorithm/length.
        if public != bytes.fromhex("302a300506032b6570032100") + bytes.fromhex(public_key):
            raise ValueError("private signer differs from separately supplied public key")
        message = temporary / "message"
        message.write_bytes(DOMAIN + key_class.encode() + b"\0" + manifest_bytes)
        signature = temporary / "signature"
        subprocess.run(["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", str(private_key),
                        "-in", str(message), "-out", str(signature)],
                       check=True, timeout=10, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        signed = signature.read_bytes()
        if len(signed) != 64:
            raise ValueError("signature extent")
    # ELF loaders ignore the trailer; the source-owned installer interprets only
    # this bounded format, verifies the signature, then verifies the payload.
    with output.open("xb") as target:
        for part in (base, manifest_bytes, signed, payload,
                     b"LVBUSER1" + struct.pack("<QQQQ", len(base), len(manifest_bytes), 64, len(payload))):
            target.write(part)
        target.flush()
        os.fsync(target.fileno())
    output.chmod(0o555)
    return {"schema": 1, "format": "native_user_installer", "key_class": key_class,
            "public_key_sha256": hashlib.sha256(bytes.fromhex(public_key)).hexdigest(),
            "installer_sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
            "release_manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(),
            "source_head": manifest["source_head"], "source_tree": manifest["source_tree"],
            "version": manifest["version"], "files": len(roster)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--installer", type=Path, required=True)
    parser.add_argument("--staged", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--public-key", required=True)
    parser.add_argument("--key-class", choices=("release", "internal_test"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(package(args.installer, args.staged, args.private_key,
                             args.public_key, args.key_class, args.output), sort_keys=True))


if __name__ == "__main__":
    main()
