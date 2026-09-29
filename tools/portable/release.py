#!/usr/bin/env python3
"""Sign or verify one exact Debian beta package with an external GPG key.

The public key copied into a bundle is informational. Verification requires a
separately supplied trusted key and expected primary fingerprint. Release
signing additionally rebuilds the PKG1 backend from a clean exact source tree.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "pkg0"))
import sign_release  # noqa: E402
import deb  # noqa: E402
from assemble import KIT_DESTINATION  # noqa: E402


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def digest(path):
    with path.open("rb") as source:
        return deb.sha_file(source)


def identity_name(manifest):
    return (f"linux-vst-bridge-beta_{manifest['version']}-{manifest['pkgrel']}"
            "_amd64.deb")


def manifest_at(staged):
    path = staged / "RELEASE_MANIFEST.json"
    manifest, raw = deb.read_manifest(path)
    return path, manifest, raw


def sign(package, staged, home, fingerprint, key_class, output, source_root=None):
    package = Path(package).absolute()
    staged = Path(staged).resolve(strict=True)
    home = Path(home)
    output = Path(output).absolute()
    fingerprint = fingerprint.upper()
    require(re.fullmatch(r"[0-9A-F]{40}", fingerprint) is not None,
            "signing fingerprint syntax")
    require(key_class in ("internal_test", "release"), "signing key class")
    require(home.is_dir() and not home.is_symlink(), "isolated GPG home required")
    require(not output.exists() and not output.is_symlink(), "release output exists")
    manifest_path, manifest, raw = manifest_at(staged)
    require(package.name == identity_name(manifest) and package.is_file()
            and not package.is_symlink(), "Debian release package identity")
    if key_class == "release":
        require(source_root is not None and any(
            row["destination"] == KIT_DESTINATION for row in manifest["files"]),
            "release signing requires exact source-backed preparation kit")
    deb.verify(package, staged, source_root=source_root if key_class == "release" else None,
               rebuild_backend=key_class == "release")
    sign_release.secret_fingerprint(home, fingerprint)
    package_sha = digest(package)
    manifest_sha = hashlib.sha256(raw).hexdigest()
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="lvb-deb-sign-", dir=output.parent) as tmp:
        temporary = Path(tmp) / "bundle"
        temporary.mkdir(mode=0o700)
        for source in (package, manifest_path):
            target = temporary / source.name
            shutil.copyfile(source, target)
            target.chmod(0o444)
        require(digest(package) == package_sha and digest(manifest_path) == manifest_sha
                and digest(temporary / package.name) == package_sha
                and digest(temporary / manifest_path.name) == manifest_sha,
                "Debian release input changed while staging")
        deb.verify(temporary / package.name, temporary)
        sums = temporary / "SHA256SUMS"
        sums.write_text(f"{package_sha}  {package.name}\n"
                        f"{manifest_sha}  RELEASE_MANIFEST.json\n")
        public = sign_release.gpg(home, "--armor", "--export", fingerprint)
        require(public.startswith(b"-----BEGIN PGP PUBLIC KEY BLOCK-----"),
                "signing public key export")
        (temporary / "SIGNING_PUBLIC_KEY.asc").write_bytes(public)
        for target in (temporary / package.name, temporary / manifest_path.name, sums):
            signature = temporary / (target.name + ".sig")
            sign_release.gpg(home, "--yes", "--local-user", fingerprint,
                             "--digest-algo", "SHA256", "--output", str(signature),
                             "--detach-sign", str(target))
            sign_release.verify(home, signature, target, fingerprint)
        identity = {"schema": 1, "format": "deb", "key_class": key_class,
                    "signer_fingerprint": fingerprint, "package": package.name,
                    "package_sha256": package_sha,
                    "release_manifest_sha256": manifest_sha,
                    "sha256sums_sha256": digest(sums)}
        (temporary / "SIGNING_IDENTITY.json").write_text(
            json.dumps(identity, sort_keys=True, indent=2) + "\n")
        for path in temporary.iterdir():
            path.chmod(0o444)
        temporary.rename(output)
    return identity


def verify(bundle, trusted_key, fingerprint, key_class="release"):
    bundle = Path(bundle).resolve(strict=True)
    trusted_key = Path(trusted_key)
    fingerprint = fingerprint.upper()
    require(re.fullmatch(r"[0-9A-F]{40}", fingerprint) is not None,
            "trusted fingerprint syntax")
    require(key_class in ("internal_test", "release"), "signing key class")
    require(trusted_key.is_file() and not trusted_key.is_symlink(),
            "separate trusted key required")
    identity = json.loads((bundle / "SIGNING_IDENTITY.json").read_bytes())
    require(set(identity) == {"schema", "format", "key_class", "signer_fingerprint",
                             "package", "package_sha256", "release_manifest_sha256",
                             "sha256sums_sha256"}
            and identity["schema"] == 1 and identity["format"] == "deb"
            and identity["key_class"] == key_class
            and identity["signer_fingerprint"] == fingerprint,
            "Debian signing identity")
    manifest_path, manifest, raw = manifest_at(bundle)
    package = bundle / identity["package"]
    require(package.name == identity_name(manifest) and package.parent == bundle,
            "Debian signed package identity")
    sums = bundle / "SHA256SUMS"
    expected = {"SIGNING_IDENTITY.json", "SIGNING_PUBLIC_KEY.asc",
                package.name, package.name + ".sig", "RELEASE_MANIFEST.json",
                "RELEASE_MANIFEST.json.sig", "SHA256SUMS", "SHA256SUMS.sig"}
    require({path.name for path in bundle.iterdir()} == expected,
            "Debian signed bundle roster")
    for path, sha in ((package, identity["package_sha256"]),
                      (manifest_path, identity["release_manifest_sha256"]),
                      (sums, identity["sha256sums_sha256"])):
        require(path.is_file() and not path.is_symlink()
                and re.fullmatch(r"[0-9a-f]{64}", sha) is not None
                and digest(path) == sha, "Debian signed input differs")
    require(hashlib.sha256(raw).hexdigest() == identity["release_manifest_sha256"]
            and sums.read_text() == (
                f"{identity['package_sha256']}  {package.name}\n"
                f"{identity['release_manifest_sha256']}  RELEASE_MANIFEST.json\n"),
            "Debian signed checksums differ")
    if key_class == "release":
        require(any(row["destination"] == KIT_DESTINATION for row in manifest["files"]),
                "release preparation kit absent")
    with tempfile.TemporaryDirectory(prefix="lvb-deb-gpg-") as tmp:
        home = Path(tmp)
        sign_release.gpg(home, "--import", str(trusted_key))
        listing = sign_release.gpg(home, "--with-colons", "--list-keys", fingerprint).decode()
        require(any(row.startswith("fpr:") and row.split(":")[9].upper() == fingerprint
                    for row in listing.splitlines()), "trusted key fingerprint differs")
        for path in (package, manifest_path, sums):
            sign_release.verify(home, bundle / (path.name + ".sig"), path, fingerprint)
    return deb.verify(package, bundle)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    writer = sub.add_parser("sign")
    writer.add_argument("--package", type=Path, required=True)
    writer.add_argument("--staged", type=Path, required=True)
    writer.add_argument("--gpg-home", type=Path, required=True)
    writer.add_argument("--fingerprint", required=True)
    writer.add_argument("--key-class", choices=("internal_test", "release"), required=True)
    writer.add_argument("--source", type=Path)
    writer.add_argument("--output", type=Path, required=True)
    reader = sub.add_parser("verify")
    reader.add_argument("--bundle", type=Path, required=True)
    reader.add_argument("--trusted-key", type=Path, required=True)
    reader.add_argument("--fingerprint", required=True)
    reader.add_argument("--key-class", choices=("internal_test", "release"), default="release")
    args = parser.parse_args()
    if args.command == "sign":
        print(json.dumps(sign(args.package, args.staged, args.gpg_home,
                              args.fingerprint, args.key_class, args.output,
                              args.source), sort_keys=True))
    else:
        print(json.dumps(verify(args.bundle, args.trusted_key,
                                args.fingerprint, args.key_class), sort_keys=True))


if __name__ == "__main__":
    main()
