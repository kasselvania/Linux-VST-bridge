#!/usr/bin/env python3
"""Verify a PKG0 release against a separately supplied trusted public key.

The public key inside a release bundle is a convenience copy, never a trust
root. The caller must provide the expected fingerprint and trusted key file.
"""
import argparse
import json
from pathlib import Path
import re
import tempfile

from sign_release import gpg, sha_file, verify as verify_signature
from assemble import KIT_DESTINATION
from verify_package import verify as verify_package


def verify(bundle, trusted_key, fingerprint, key_class="release", structure_only=False):
    fingerprint = fingerprint.upper()
    if not re.fullmatch(r"[0-9A-F]{40}", fingerprint):
        raise ValueError("trusted fingerprint syntax")
    if key_class not in ("release", "internal_test"):
        raise ValueError("signing key class")
    if not trusted_key.is_file() or trusted_key.is_symlink():
        raise ValueError("separate trusted key required")
    identity = json.loads((bundle / "SIGNING_IDENTITY.json").read_bytes())
    if (identity.get("schema") != 1 or identity.get("key_class") != key_class
            or identity.get("signer_fingerprint") != fingerprint):
        raise ValueError("release signing identity")
    package = bundle / identity["package"]
    manifest = bundle / "RELEASE_MANIFEST.json"
    sums = bundle / "SHA256SUMS"
    if (package.parent != bundle or not re.fullmatch(
            r"linux-vst-bridge-beta-[0-9][A-Za-z0-9.]*-1-x86_64\.pkg\.tar\.zst",
            package.name)):
        raise ValueError("release package name")
    for path, field in ((package, "package_sha256"),
                        (manifest, "release_manifest_sha256"),
                        (sums, "sha256sums_sha256")):
        if not path.is_file() or path.is_symlink() or sha_file(path) != identity[field]:
            raise ValueError("release signed input differs")
    expected_sums = (f"{identity['package_sha256']}  {package.name}\n"
                     f"{identity['release_manifest_sha256']}  RELEASE_MANIFEST.json\n")
    if sums.read_text() != expected_sums:
        raise ValueError("release SHA256SUMS differs")
    with tempfile.TemporaryDirectory(prefix="lvb-pkg0-gpg-") as tmp:
        home = Path(tmp)
        gpg(home, "--import", str(trusted_key))
        listing = gpg(home, "--with-colons", "--list-keys", fingerprint).decode()
        if not any(row.startswith("fpr:") and row.split(":")[9].upper() == fingerprint
                   for row in listing.splitlines()):
            raise ValueError("trusted key fingerprint differs")
        for path in (package, manifest, sums):
            verify_signature(home, bundle / (path.name + ".sig"), path, fingerprint)
    roster = json.loads(manifest.read_bytes())
    if key_class == "release" and not any(
            row.get("destination") == KIT_DESTINATION for row in roster.get("files", [])):
        raise ValueError("release preparation kit absent")
    return verify_package(package, roster, structure_only=structure_only)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--trusted-key", type=Path, required=True)
    parser.add_argument("--fingerprint", required=True)
    parser.add_argument("--key-class", choices=("release", "internal_test"), default="release")
    parser.add_argument("--structure-only", action="store_true")
    args = parser.parse_args()
    print(json.dumps(verify(args.bundle, args.trusted_key, args.fingerprint,
                            args.key_class, args.structure_only), sort_keys=True))
