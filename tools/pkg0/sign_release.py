#!/usr/bin/env python3
"""Sign a verified PKG0 package using a supplied, isolated GPG key.

This tool does not create or persist a private key. The release fingerprint is
an operator-approved input; an internal test key must never be shipped.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
from verify_package import verify as verify_package
from assemble import PKGREL


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def gpg(home, *args):
    return subprocess.run(
        ["gpg", "--homedir", str(home), "--batch", "--no-tty", *args],
        check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout


def secret_fingerprint(home, expected):
    listing = gpg(home, "--with-colons", "--list-secret-keys", expected).decode()
    lines = [line.split(":") for line in listing.splitlines()]
    primaries = [index for index, row in enumerate(lines) if row[0] in ("sec", "sec#")]
    require(len(primaries) == 1, "one exact signing primary required")
    following = lines[primaries[0] + 1]
    require(following[0] == "fpr" and following[9].upper() == expected,
            "signing secret-key fingerprint differs")


def verify(home, signature, original, expected):
    result = gpg(home, "--status-fd", "1", "--verify", str(signature), str(original))
    signatures = [line.split() for line in result.decode().splitlines()
                  if line.startswith("[GNUPG:] VALIDSIG ")]
    require(len(signatures) == 1 and signatures[0][2].upper() == expected,
            "release signature fingerprint differs")


def regular(path):
    require(path.is_file() and not path.is_symlink(), "release input regular file required")


def sign(package, manifest, home, fingerprint, key_class, output, structure_only=False):
    fingerprint = fingerprint.upper()
    require(re.fullmatch(r"[0-9A-F]{40}", fingerprint) is not None,
            "signing fingerprint syntax")
    require(key_class in ("internal_test", "release"), "signing key class")
    for path in (package, manifest):
        regular(path)
    require(home.is_dir() and not home.is_symlink(), "isolated GPG home required")
    require(not output.exists() and not output.is_symlink(), "release output exists")
    data = json.loads(manifest.read_bytes())
    require(data.get("schema") == 1 and data.get("package") == "linux-vst-bridge-beta"
            and data.get("pkgrel") == PKGREL
            and re.fullmatch(r"[0-9][A-Za-z0-9.]*", data.get("version", "")),
            "release manifest identity")
    expected_name = f"linux-vst-bridge-beta-{data['version']}-{data['pkgrel']}-x86_64.pkg.tar.zst"
    require(package.name == expected_name and manifest.name == "RELEASE_MANIFEST.json",
            "release filename identity")
    package_sha = sha_file(package)
    manifest_sha = sha_file(manifest)
    verify_package(package, data, structure_only=structure_only)
    secret_fingerprint(home, fingerprint)
    temporary = output.with_name(output.name + f".stage-{os.getpid()}")
    require(not temporary.exists(), "release stage exists")
    temporary.mkdir(mode=0o700, parents=False)
    try:
        for source in (package, manifest):
            target = temporary / source.name
            shutil.copyfile(source, target)
            target.chmod(0o444)
        require(sha_file(package) == package_sha and sha_file(manifest) == manifest_sha
                and sha_file(temporary / package.name) == package_sha
                and sha_file(temporary / manifest.name) == manifest_sha,
                "release bytes changed while staging")
        sums = temporary / "SHA256SUMS"
        sums.write_text(f"{package_sha}  {package.name}\n"
                        f"{manifest_sha}  RELEASE_MANIFEST.json\n")
        public = gpg(home, "--armor", "--export", fingerprint)
        require(public.startswith(b"-----BEGIN PGP PUBLIC KEY BLOCK-----"),
                "signing public key export")
        (temporary / "SIGNING_PUBLIC_KEY.asc").write_bytes(public)
        for target in (temporary / package.name, temporary / manifest.name, sums):
            signature = temporary / (target.name + ".sig")
            gpg(home, "--yes", "--local-user", fingerprint, "--digest-algo", "SHA256",
                "--output", str(signature), "--detach-sign", str(target))
            verify(home, signature, target, fingerprint)
        identity = {"schema": 1, "key_class": key_class, "signer_fingerprint": fingerprint,
                    "package": package.name, "package_sha256": package_sha,
                    "release_manifest_sha256": manifest_sha,
                    "sha256sums_sha256": sha_file(sums)}
        (temporary / "SIGNING_IDENTITY.json").write_text(
            json.dumps(identity, sort_keys=True, indent=2) + "\n")
        for path in temporary.iterdir():
            path.chmod(0o444)
        temporary.rename(output)
        return identity
    except BaseException:
        shutil.rmtree(temporary)
        raise


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--gpg-home", type=Path, required=True)
    parser.add_argument("--fingerprint", required=True)
    parser.add_argument("--key-class", choices=("internal_test", "release"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(sign(args.package, args.manifest, args.gpg_home,
                          args.fingerprint, args.key_class, args.output), sort_keys=True))
