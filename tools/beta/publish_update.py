#!/usr/bin/env python3
"""Publish a verified build to the beta update feed the manager checks.

Two steps, so the signing key never has to travel:

  build   where the signing key is (the build container for internal test
          builds): read the signed installers in a build's ready/ folder,
          verify each one's own signature with the public key, write
          latest.json from their real versions, sizes and SHA-256, and sign it
          with the same key and key class (detached, latest.json.sig).
  upload  from anywhere with SSH to the beta host: verify the feed signature
          and every listed installer again, then copy the installers and the
          feed into the beta folder. Nothing there is deleted; an existing
          file with the same name and different bytes is refused.

The feed signature is Ed25519 over: domain + key class + NUL + feed bytes,
with the domain "Linux VST Bridge update feed v1\\0". Only openssl reads the
private key; this script never opens, prints or copies it.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import struct
import subprocess
import sys
import tempfile
import unicodedata

FEED_DOMAIN = b"Linux VST Bridge update feed v1\0"
PACKAGE_DOMAIN = b"Linux VST Bridge user package v1\0"
FEED_FILE = "latest.json"
SIGNATURE_FILE = "latest.json.sig"
CHANNEL = "beta"
NOTES_MAX = 16 * 1024
SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")
# The installer's own bounds (portable_package.rs).
MANIFEST_MAX = 4 * 1024 * 1024
PAYLOAD_MAX = 2_000_000_000
LAUNCHER_MAX = 256 * 1024 * 1024
PUBLISHED = re.compile(r"[!-~]{1,64}")
VERSION = re.compile(r"[0-9][A-Za-z0-9.]{0,79}")
SYSTEM = re.compile(r"[a-z0-9._-]{1,32}")
FILE = re.compile(r"[A-Za-z0-9_][A-Za-z0-9._-]{0,150}\.install")


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def version_key(version):
    """The manager's ordering: numeric runs as numbers, 0.12.0test99 < 0.12.0test100."""
    parts = []
    for token in re.findall(r"[0-9]+|[A-Za-z]+", version):
        parts.append((1, int(token), "") if token.isdigit() else (0, 0, token.lower()))
    return parts


def public_key_hex(value):
    require(len(value) == 64 and all(c in "0123456789abcdef" for c in value), "public key must be 64 lower-case hex digits")
    return value


def openssl_verify(public_key, message, signature):
    with tempfile.TemporaryDirectory(prefix="lvb-feed-verify-") as directory:
        temporary = Path(directory)
        (temporary / "public.der").write_bytes(SPKI_PREFIX + bytes.fromhex(public_key))
        (temporary / "message").write_bytes(message)
        (temporary / "signature").write_bytes(signature)
        result = subprocess.run(["openssl", "pkeyutl", "-verify", "-pubin", "-keyform", "DER",
                                 "-inkey", str(temporary / "public.der"), "-rawin",
                                 "-in", str(temporary / "message"), "-sigfile", str(temporary / "signature")],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=20)
        return result.returncode == 0


def hash_range(source, length, name):
    digest = hashlib.sha256()
    while length:
        block = source.read(min(length, 1 << 20))
        require(block, f"{name}: installer layout")
        digest.update(block)
        length -= len(block)
    return digest


def installer_release(path, public_key, key_class):
    """The installer's signed release manifest, after the checks the manager
    makes before staging it: ELF launcher, footer and layout bounds, the
    manifest signature over the launcher hash, and the payload hash."""
    size = path.stat().st_size
    require(40 < size <= PAYLOAD_MAX + LAUNCHER_MAX, f"{path.name}: not a signed installer")
    with open(path, "rb") as source:
        require(source.read(4) == b"\x7fELF", f"{path.name}: not an ELF installer")
        source.seek(size - 40)
        footer = source.read(40)
        require(footer[:8] == b"LVBUSER1", f"{path.name}: not a signed installer")
        offset, manifest_size, signature_size, payload_size = struct.unpack("<QQQQ", footer[8:])
        require(4 <= offset <= LAUNCHER_MAX and manifest_size <= MANIFEST_MAX and signature_size == 64
                and payload_size <= PAYLOAD_MAX
                and offset + manifest_size + 64 + payload_size + 40 == size, f"{path.name}: installer layout")
        source.seek(0)
        base = hash_range(source, offset, path.name)
        manifest = source.read(manifest_size)
        signature = source.read(64)
        payload = hash_range(source, payload_size, path.name).hexdigest()
    require(openssl_verify(public_key, PACKAGE_DOMAIN + key_class.encode() + b"\0" + base.digest() + manifest,
                           signature), f"{path.name}: installer signature does not verify with this public key and key class")
    release = json.loads(manifest)
    require(release.get("package") == "linux-vst-bridge-beta" and VERSION.fullmatch(release.get("version", "")),
            f"{path.name}: release manifest")
    require(release.get("payload_sha256") == payload, f"{path.name}: payload differs from its signed manifest")
    return release


def read_notes(path):
    text = Path(path).read_text(encoding="utf-8").strip("\n") + "\n"
    require(len(text.encode()) <= NOTES_MAX, "notes are longer than 16 KiB")
    # The manager refuses every Unicode control character but these three (C0, DEL and C1).
    require(all(c in "\n\r\t" or unicodedata.category(c) != "Cc" for c in text), "notes contain control characters")
    return text


def feed_from_ready(ready, notes, min_version, public_key, key_class, published=None, aliases=()):
    installers = sorted(p for p in Path(ready).iterdir() if p.is_file() and p.name.endswith(".install"))
    require(installers, "the ready folder has no .install files")
    version = None
    packages = {}
    for path in installers:
        require(FILE.fullmatch(path.name), f"{path.name}: unsafe file name")
        release = installer_release(path, public_key, key_class)
        require(version in (None, release["version"]), "installers in the ready folder have different versions")
        version = release["version"]
        prefix = f"Linux-VST-Bridge-{version}-"
        require(path.name.startswith(prefix), f"{path.name}: name does not carry its version {version}")
        middle = path.name[len(prefix):-len(".install")]
        system = middle.rsplit("-", 1)[0] if middle.endswith(("-internal", "-release")) else middle
        require(SYSTEM.fullmatch(system) and system not in packages, f"{path.name}: system name")
        packages[system] = {"file": path.name, "size": path.stat().st_size, "sha256": sha256(path)}
    for alias in aliases:
        name, _, target = alias.partition("=")
        require(SYSTEM.fullmatch(name) and name not in packages and target in packages,
                f"alias {alias}: use NAME=SYSTEM with an existing system")
        packages[name] = dict(packages[target])
    require(1 <= len(packages) <= 16, "a feed lists 1 to 16 packages")
    require(VERSION.fullmatch(min_version) and version_key(min_version) <= version_key(version),
            "min version must be a version no newer than the build")
    published = published or datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    require(PUBLISHED.fullmatch(published), "published must be 1 to 64 printable ASCII characters without spaces")
    return {"schema": 1, "channel": CHANNEL, "version": version, "published": published,
            "min_version": min_version, "notes": read_notes(notes), "packages": packages}


def feed_bytes(feed):
    return json.dumps(feed, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()


def sign(data, private_key, public_key, key_class):
    require(key_class in ("internal_test", "release"), "key class")
    key = Path(private_key)
    status = key.stat()
    require(key.is_file() and not key.is_symlink() and status.st_uid == os.getuid()
            and status.st_mode & 0o077 == 0, "the signing key must be a private file owned by this user")
    public = subprocess.check_output(["openssl", "pkey", "-in", str(key), "-pubout", "-outform", "DER"],
                                     stderr=subprocess.DEVNULL, timeout=20)
    require(public == SPKI_PREFIX + bytes.fromhex(public_key), "the signing key differs from the given public key")
    with tempfile.TemporaryDirectory(prefix="lvb-feed-sign-") as directory:
        temporary = Path(directory)
        (temporary / "message").write_bytes(FEED_DOMAIN + key_class.encode() + b"\0" + data)
        subprocess.run(["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", str(key),
                        "-in", str(temporary / "message"), "-out", str(temporary / "signature")],
                       check=True, timeout=20, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        signature = (temporary / "signature").read_bytes()
    require(len(signature) == 64, "signature extent")
    require(openssl_verify(public_key, FEED_DOMAIN + key_class.encode() + b"\0" + data, signature),
            "the new feed signature does not verify")
    return signature


def build(args):
    public_key = public_key_hex(args.public_key)
    feed = feed_from_ready(args.ready, args.notes, args.min_version, public_key, args.key_class,
                           args.published, args.alias)
    data = feed_bytes(feed)
    signature = sign(data, args.private_key, public_key, args.key_class)
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    for name in (FEED_FILE, SIGNATURE_FILE):
        require(not (output / name).exists(), f"{output / name} already exists")
    (output / FEED_FILE).write_bytes(data)
    (output / SIGNATURE_FILE).write_bytes(signature)
    return {"version": feed["version"], "packages": sorted(feed["packages"]), "feed_sha256": hashlib.sha256(data).hexdigest()}


def verified_feed(feed_dir, ready, public_key, key_class):
    data = (Path(feed_dir) / FEED_FILE).read_bytes()
    signature = (Path(feed_dir) / SIGNATURE_FILE).read_bytes()
    require(len(signature) == 64 and openssl_verify(public_key, FEED_DOMAIN + key_class.encode() + b"\0" + data, signature),
            "the feed signature does not verify with this public key and key class")
    feed = json.loads(data)
    require(feed.get("schema") == 1 and feed.get("channel") == CHANNEL, "feed format")
    files = {}
    for system, package in feed["packages"].items():
        path = Path(ready) / package["file"]
        require(FILE.fullmatch(package["file"]) and path.is_file(), f"{package['file']} is not in the ready folder")
        require(path.stat().st_size == package["size"] and sha256(path) == package["sha256"],
                f"{package['file']} differs from the signed feed")
        files[package["file"]] = (path, package["sha256"])
    return feed, data, signature, files


class Destination:
    """The beta folder, local or `host:path` over SSH. Never deletes anything."""

    def __init__(self, spec):
        host, sep, path = spec.partition(":")
        self.host = host if sep and "/" not in host else None
        self.path = path if self.host else spec
        require(not (self.host or "").startswith("-"), "the beta host must not start with '-'")

    def run(self, command, data=None, stdin=None):
        """`data` is sent as bytes; `stdin` streams an open file."""
        argv = ["ssh", "-o", "BatchMode=yes", "--", self.host, command] if self.host else ["sh", "-c", command]
        if stdin is not None:
            return subprocess.run(argv, stdin=stdin, capture_output=True, timeout=1800)
        return subprocess.run(argv, input=data, capture_output=True, timeout=1800)

    def read(self, name):
        result = self.run(f"if [ -f {self.target(name)} ]; then cat {self.target(name)}; fi")
        require(result.returncode == 0, f"could not read the beta folder: {result.stderr.decode(errors='replace').strip()}")
        return result.stdout

    def target(self, name):
        return shlex.quote(str(Path(self.path) / name))

    def existing_sha256(self, name):
        result = self.run(f"if [ -e {self.target(name)} ]; then sha256sum {self.target(name)}; fi")
        require(result.returncode == 0, f"could not read the beta folder: {result.stderr.decode(errors='replace').strip()}")
        text = result.stdout.decode().strip()
        return text.split()[0] if text else None

    def put(self, source, name, mode, replace=False):
        temporary = f".{name}.uploading"
        rename = "mv -f" if replace else "mv -n"
        command = (f"test -d {shlex.quote(self.path)} && cat > {self.target(temporary)} && chmod {mode} {self.target(temporary)}"
                   f" && {rename} {self.target(temporary)} {self.target(name)} && test ! -e {self.target(temporary)}")
        with open(source, "rb") as data:
            result = self.run(command, stdin=data)
        require(result.returncode == 0, f"copying {name} failed: {result.stderr.decode(errors='replace').strip()}")


def upload(args):
    public_key = public_key_hex(args.public_key)
    feed, data, signature, files = verified_feed(args.feed_dir, args.ready, public_key, args.key_class)
    destination = Destination(args.dest)
    published = destination.read(FEED_FILE)
    if published:
        try:
            current = json.loads(published).get("version", "")
        except ValueError:
            current = ""
        require(args.force_older or not VERSION.fullmatch(current) or version_key(current) <= version_key(feed["version"]),
                f"the beta folder already offers {current}, newer than {feed['version']}; use --force-older to replace it")
    copied, kept = [], []
    for name, (path, digest) in sorted(files.items()):
        existing = destination.existing_sha256(name)
        if existing == digest:
            kept.append(name)
            continue
        require(existing is None, f"{name} already exists in the beta folder with different contents; refusing to replace it")
        destination.put(path, name, "0755")
        require(destination.existing_sha256(name) == digest, f"{name} did not arrive intact")
        copied.append(name)
    # The signature goes first; the manager retries once if it reads a feed
    # and signature from either side of this swap.
    with tempfile.TemporaryDirectory(prefix="lvb-feed-upload-") as directory:
        for name, content in ((SIGNATURE_FILE, signature), (FEED_FILE, data)):
            local = Path(directory) / name
            local.write_bytes(content)
            destination.put(local, name, "0644", replace=True)
    require(destination.existing_sha256(FEED_FILE) == hashlib.sha256(data).hexdigest(), "the feed did not arrive intact")
    return {"version": feed["version"], "copied": copied, "already_present": kept}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    writer = commands.add_parser("build", help="write and sign latest.json from a ready/ folder")
    writer.add_argument("--ready", required=True)
    writer.add_argument("--notes", required=True, help="plain-text What's new file")
    writer.add_argument("--min-version", required=True,
                        help="oldest installed version that may update in the app (it must have the updater)")
    writer.add_argument("--private-key", required=True)
    writer.add_argument("--public-key", required=True)
    writer.add_argument("--key-class", choices=("internal_test", "release"), required=True)
    writer.add_argument("--output", required=True)
    writer.add_argument("--published", help="UTC publish time; default now")
    writer.add_argument("--alias", action="append", default=[], help="NAME=SYSTEM, e.g. arch=cachyos")
    sender = commands.add_parser("upload", help="verify, then copy installers and feed into the beta folder")
    sender.add_argument("--feed-dir", required=True)
    sender.add_argument("--ready", required=True)
    sender.add_argument("--dest", required=True, help="beta folder: PATH or HOST:PATH")
    sender.add_argument("--public-key", required=True)
    sender.add_argument("--key-class", choices=("internal_test", "release"), required=True)
    sender.add_argument("--force-older", action="store_true",
                        help="replace a newer published feed with this older one")
    args = parser.parse_args(argv)
    try:
        result = build(args) if args.command == "build" else upload(args)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"publish_update: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
