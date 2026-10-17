#!/usr/bin/env python3
"""Build the engine WholeArchive; the existing owner acquires SLR separately."""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import posixpath
import re
import runpy
import stat
import tarfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
ENGINE_ROOT = "Common-Proton-x86_64"
SLR = {
    "url": "https://repo.steampowered.com/steamrt4/images/4.0.20260805.254769/SteamLinuxRuntime_4.tar.xz",
    "size": 164895128,
    "sha256": "3226d8234e7c0542ee767837832bfb1dad5e5e2dc944ec97eb221b437f6b9349",
}
TOOLS = {
    "client_sha256": "SteamLinuxRuntime_4/pressure-vessel/bin/steam-runtime-launch-client",
    "service_sha256": "SteamLinuxRuntime_4/pressure-vessel/libexec/steam-runtime-tools-0/x86_64-linux-gnu-srt-launcher-service",
}
MAX_FILE = 512 * 1024 * 1024
MAX_TOTAL = 8 * 1024 * 1024 * 1024
UPSTREAM_INPUTS = json.loads((HERE / "redist-inputs.json").read_bytes())["corresponding_source"]
UPSTREAM_INPUTS.update({name: spec for name, spec in json.loads((HERE / "build-inputs.json").read_bytes()).items()
                        if name != "onnxruntime.tgz"})


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sha(path):
    if path.is_symlink() or not path.is_file():
        raise ValueError("Required regular file missing or linked: " + str(path))
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def identity(metadata):
    return (metadata.st_dev, metadata.st_ino, metadata.st_mode, metadata.st_size,
            metadata.st_mtime_ns, metadata.st_ctime_ns)


def inventory(root, links=False):
    if root.is_symlink() or not root.is_dir() or root.resolve() != root:
        raise ValueError("Input directory is missing or linked")
    result = {}
    for path in sorted(root.rglob("*")):
        name = path.relative_to(root).as_posix()
        metadata = path.lstat()
        if not (stat.S_ISREG(metadata.st_mode) or stat.S_ISDIR(metadata.st_mode)
                or links and stat.S_ISLNK(metadata.st_mode)):
            raise ValueError("Unsupported archive input: " + name)
        if path.is_symlink():
            target = str(path.readlink())
            normalized = posixpath.normpath(posixpath.join(posixpath.dirname(name), target))
            if (target.startswith("/") or normalized == ".." or normalized.startswith("../")
                    or not path.resolve().is_relative_to(root)):
                raise ValueError("Archive symlink escapes its input: " + name)
        result[name] = identity(metadata)
    return result


def file_manifest(root, entries):
    return {name: {"size": row[3], "sha256": sha(root / name)}
            for name, row in entries.items() if stat.S_ISREG(row[2])}


def read_declaration(data):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("Duplicate Wine declaration field")
            result[key] = value
        return result
    if len(data) > 65536:
        raise ValueError("Wine declaration extent differs")
    build = json.loads(data, object_pairs_hook=unique)
    fields = {"schema", "kind", "recipe_sha256", "wine_revision", "sdk_image",
              "configuration_sha256", "invocation", "helper_abi", "helper_sources", "winebuild_sha256"}
    if (type(build) is not dict or set(build) != fields or type(build["schema"]) is not int
            or build["schema"] != 1 or build["kind"] != "wine_build"
            or type(build["helper_abi"]) is not int or build["helper_abi"] != 1):
        raise ValueError("Unsupported Wine construction declaration")
    helpers = runpy.run_path(str(HERE / "build-identity.py"))["HELPER_SOURCES"]
    if (type(build["configuration_sha256"]) is not dict
            or set(build["configuration_sha256"]) != {"i386", "x86_64"}
            or type(build["helper_sources"]) is not dict or set(build["helper_sources"]) != set(helpers)):
        raise ValueError("Malformed Wine construction input identities")
    values = [build["recipe_sha256"], build["winebuild_sha256"],
              *build["configuration_sha256"].values(), *build["helper_sources"].values()]
    if any(type(value) is not str or not re.fullmatch("[0-9a-f]{64}", value) for value in values):
        raise ValueError("Malformed Wine construction digest")
    invocation = build["invocation"]
    if (type(invocation) is not dict or set(invocation) != {"target", "source_date_epoch",
            "base_source_date_epoch", "build_name", "make_arguments"} or invocation["target"] != "wine"
            or any(type(invocation[key]) is not int or invocation[key] <= 0
                   for key in ("source_date_epoch", "base_source_date_epoch"))
            or type(invocation["build_name"]) is not str
            or not re.fullmatch("[A-Za-z0-9._-]{1,128}", invocation["build_name"])
            or type(invocation["make_arguments"]) is not list
            or not 1 <= len(invocation["make_arguments"]) <= 16
            or any(type(arg) is not str or "/" in arg or not re.fullmatch("[!-~]{1,256}", arg)
                   for arg in invocation["make_arguments"])):
        raise ValueError("Malformed Wine construction invocation")
    return build


def member_bytes(path):
    result = {}
    total = 0
    with tarfile.open(path, "r|gz") as archive:
        for member in archive:
            name = member.name
            total += member.size
            if (not member.isfile() or member.size > 32 * 1024 * 1024
                    or total > 64 * 1024 * 1024 or len(result) >= 128 or name in result
                    or Path(name).is_absolute() or ".." in Path(name).parts):
                raise ValueError("Malformed construction source archive")
            result[name] = archive.extractfile(member).read()
    return result


def source_context(source, declaration, manifest):
    """Read original construction bytes, never reconstruct a declaration."""
    if {p.name for p in source.iterdir()} != {"README.txt", "prepared-source", "upstream-source",
                                            "cargo", "third-party-notices"}:
        raise ValueError("Source companion root roster differs")
    companion = runpy.run_path(str(HERE / "source-companion.py"))
    prepared = {p.name for p in (source / "prepared-source").iterdir()}
    expected = {name.replace("/", "_") + ".tar.gz" for name in companion["COMPONENTS"]}
    expected.update(name + ".tar.gz" for name in ("proton-build", "original-construction-recipes",
                    "collector-source", "wine-build-configuration", "piper-local-configuration"))
    if prepared != expected:
        raise ValueError("Source companion prepared-source roster differs")
    if {p.name for p in (source / "upstream-source").iterdir()} != set(UPSTREAM_INPUTS):
        raise ValueError("Source companion upstream-source roster differs")
    for name, spec in UPSTREAM_INPUTS.items():
        if manifest["upstream-source/" + name] != {"size": spec["size"], "sha256": spec["sha256"]}:
            raise ValueError("Source companion upstream input differs: " + name)
    cargo = list((source / "cargo").iterdir())
    if not cargo or any(not p.is_file() or not p.name.endswith((".crate", ".tar.gz")) for p in cargo):
        raise ValueError("Source companion compiler-selected Cargo inputs missing")
    config = member_bytes(source / "prepared-source/wine-build-configuration.tar.gz")
    if config.get("build/redist/wine-build.json") != declaration:
        raise ValueError("Source companion original declaration differs")
    build = read_declaration(declaration)
    builder = runpy.run_path(str(HERE / "build-identity.py"))
    original = member_bytes(source / "prepared-source/original-construction-recipes.tar.gz")
    recipe = json.loads(original["tools/runtime/source.json"])
    if recipe != json.loads((HERE / "source.json").read_bytes()):
        raise ValueError("Source companion engine source basis differs")
    names = {"tools/runtime/" + name for name in builder["RECIPE_FILES"]}
    names.update("tools/" + item["path"] for item in (*recipe["patches"], *recipe["fixtures"]))
    names.update("tools/runtime/patches/" + name for name in
                 ("proton-redist-inputs.patch", "proton-audio-composition.patch"))
    hashes = {name: hashlib.sha256(original[name]).hexdigest() for name in names}
    helpers = {name: hashlib.sha256(original[name]).hexdigest() for name in builder["HELPER_SOURCES"]}
    if (hashlib.sha256(canonical(hashes)).hexdigest() != build.get("recipe_sha256")
            or helpers != build.get("helper_sources")):
        raise ValueError("Original construction source binding differs")
    for name in companion["ORIGINAL_SUPPORT"]:
        if name not in original:
            raise ValueError("Original construction support missing")
    if (type(build.get("schema")) is not int or build["schema"] != 1
            or build.get("kind") != "wine_build" or type(build.get("helper_abi")) is not int
            or build["helper_abi"] != 1 or build.get("wine_revision") != recipe["wine"]["commit"]
            or build.get("sdk_image") != recipe["sdk"]
            or set(build.get("configuration_sha256", {})) != {"i386", "x86_64"}
            or build.get("invocation", {}).get("target") != "wine"):
        raise ValueError("Unsupported Wine construction declaration")
    for arch, digest in build["configuration_sha256"].items():
        if hashlib.sha256(config["build/obj-wine-" + arch + "/Makefile"]).hexdigest() != digest:
            raise ValueError("Original Wine configuration differs")
    epoch = build["invocation"]["source_date_epoch"]
    if type(epoch) is not int or epoch != recipe["proton"]["source_date_epoch"] or epoch <= 0:
        raise ValueError("Construction timestamp differs")
    return epoch


def command_marker(slr_archive, declaration_digest):
    before = identity(slr_archive.lstat())
    if slr_archive.stat().st_size != SLR["size"] or sha(slr_archive) != SLR["sha256"]:
        raise ValueError("SLR acquisition archive differs from the fixed catalog")
    digests = {}
    with tarfile.open(slr_archive, "r|xz") as archive:
        for member in archive:
            for role, name in TOOLS.items():
                if member.name != name:
                    continue
                if not member.isfile() or member.size > MAX_FILE or role in digests:
                    raise ValueError("SLR command tool is missing, duplicated or linked")
                digests[role] = hashlib.file_digest(archive.extractfile(member), "sha256").hexdigest()
    if set(digests) != set(TOOLS):
        raise ValueError("SLR command tool is missing")
    if identity(slr_archive.lstat()) != before:
        raise ValueError("SLR acquisition archive changed while reading")
    return canonical(dict(schema=2, kind="native_proton_command_session",
                          wine_build_sha256=declaration_digest, **digests))


def package(build_root, source, source_digest, slr_archive, output, declaration_digest):
    if build_root.is_relative_to(Path("/mnt/Storage2")) and not Path("/mnt/Storage2").is_mount():
        raise ValueError("Storage2 is not mounted")
    if output.exists() or output.is_symlink():
        raise ValueError("Archive output already exists")
    if output.parent.resolve() != output.parent or not output.parent.is_dir():
        raise ValueError("Output parent is missing or linked")
    for value in (source_digest, declaration_digest):
        if not re.fullmatch("[0-9a-f]{64}", value):
            raise ValueError("Expected source/declaration SHA256 is malformed")
    redist = build_root / "build/redist"
    if output.is_relative_to(redist) or output.is_relative_to(source):
        raise ValueError("Archive output must be outside its input trees")
    rows = inventory(redist, links=True)
    sources = inventory(source)
    if any(name.split("/")[0] in {"source-companion", "native-command-session.json"} for name in rows):
        raise ValueError("Redist already contains archive-owned additions")
    manifest = file_manifest(source, sources)
    if hashlib.sha256(canonical(manifest)).hexdigest() != source_digest:
        raise ValueError("Source companion identity differs")
    if sha(redist / "wine-build.json") != declaration_digest:
        raise ValueError("Original Wine declaration differs")
    declaration = (redist / "wine-build.json").read_bytes()
    epoch = source_context(source, declaration, manifest)
    if file_manifest(redist / "third-party-notices", inventory(redist / "third-party-notices")) != file_manifest(
            source / "third-party-notices", inventory(source / "third-party-notices")):
        raise ValueError("Runtime and source companion notices differ")
    for name in ("proton", "files/bin/wine"):
        if name not in rows or not stat.S_ISREG(rows[name][2]) or rows[name][2] & 0o111 == 0:
            raise ValueError("Required runtime executable missing")
    marker = command_marker(slr_archive, declaration_digest)
    entries = [(redist, name, name, row) for name, row in rows.items()]
    entries += [(source, name, "source-companion/" + name, row) for name, row in sources.items()]
    if (len(entries) + 3 >= 40000 or any(row[3] > MAX_FILE for *_, row in entries if stat.S_ISREG(row[2]))
            or sum(row[3] for *_, row in entries if stat.S_ISREG(row[2])) + len(marker) > MAX_TOTAL):
        raise ValueError("Archive exceeds the existing WholeArchive delivery bounds")
    raw = output.open("xb")
    try:
        with raw, gzip.GzipFile(fileobj=raw, filename="", mode="wb",
                                                    compresslevel=1, mtime=epoch) as compressed:
            with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.PAX_FORMAT) as archive:
                for name in (ENGINE_ROOT, ENGINE_ROOT + "/source-companion"):
                    info = tarfile.TarInfo(name)
                    info.type, info.mode, info.mtime = tarfile.DIRTYPE, 0o755, epoch
                    archive.addfile(info)
                for base, name, target, row in sorted(entries, key=lambda item: item[2]):
                    path = base / name
                    if identity(path.lstat()) != row:
                        raise ValueError("Archive input changed")
                    info = tarfile.TarInfo(ENGINE_ROOT + "/" + target)
                    info.mtime = epoch
                    info.mode = 0o755 if row[2] & 0o111 else 0o644
                    if stat.S_ISDIR(row[2]):
                        info.type = tarfile.DIRTYPE
                        archive.addfile(info)
                    elif stat.S_ISLNK(row[2]):
                        info.type, info.linkname = tarfile.SYMTYPE, str(path.readlink())
                        archive.addfile(info)
                    else:
                        info.size = row[3]
                        with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), "rb") as stream:
                            if identity(os.fstat(stream.fileno())) != row:
                                raise ValueError("Archive input replaced")
                            archive.addfile(info, stream)
                            if identity(os.fstat(stream.fileno())) != row:
                                raise ValueError("Archive input changed while reading")
                info = tarfile.TarInfo(ENGINE_ROOT + "/native-command-session.json")
                info.size, info.mode, info.mtime = len(marker), 0o644, epoch
                archive.addfile(info, io.BytesIO(marker))
        if inventory(redist, links=True) != rows or inventory(source) != sources:
            raise ValueError("Archive input roster changed")
    except BaseException:
        output.unlink(missing_ok=True)
        raise
    return dict(root=ENGINE_ROOT, compression="gzip", size=output.stat().st_size, sha256=sha(output),
                source_sha256=source_digest, wine_build_sha256=declaration_digest,
                command_session_sha256=hashlib.sha256(marker).hexdigest())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-root", type=Path, required=True)
    parser.add_argument("--source-companion", type=Path, required=True)
    parser.add_argument("--source-sha256", required=True, help="Verified collector file-manifest SHA256")
    parser.add_argument("--wine-build-sha256", required=True)
    parser.add_argument("--slr-archive", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(package(args.build_root.absolute(), args.source_companion.absolute(),
                             args.source_sha256, args.slr_archive.absolute(), args.output.absolute(),
                             args.wine_build_sha256), sort_keys=True))
