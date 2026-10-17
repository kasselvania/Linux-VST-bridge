#!/usr/bin/env python3
"""Collect matching source beside a completed private common runtime.

Run after redist: source-companion.py --build-root OUTPUT --destination NEW_DIR.
This acquires the pinned source archives, without extracting or executing them.
Unpack the generated prepared-source tarballs into one directory; they retain
the proton/ and tools/ layout, applied changes, build recipes and configurations.
The source archives retain their original component notices and exceptions.
This creates source materials, not a publication or qualification declaration.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import posixpath
import re
import runpy
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent
INPUTS = json.loads((ROOT / "redist-inputs.json").read_bytes())
RECIPE = json.loads((ROOT / "source.json").read_bytes())
# Selected native x86 source/build graph. No Steam/VR relay, ARM/FEX, test media
# or unused nested submodule collection. Full selected Cargo Git workspaces keep
# their original manifests/lockfile; cache-only dependencies are excluded below.
COMPONENTS = (
    "wine", "dav1d", "libsoup", "gst-orc", "gstreamer", "graphene",
    "gst-plugins-rs", "Vulkan-Headers", "SPIRV-Headers", "glslang", "ffmpeg",
    "piper", "openfst", "kaldi", "vosk-api", "vkd3d", "dxvk", "dxvk-nvapi",
    "vkd3d-proton", "fonts/liberation-fonts",
    "dxvk/include/spirv", "dxvk/include/vulkan", "dxvk/subprojects/libdisplay-info",
    "dxvk/subprojects/dxbc-spirv", "dxvk/subprojects/dxbc-spirv/submodules/spirv_headers",
    "dxvk-nvapi/external/DirectX-Headers", "dxvk-nvapi/external/Vulkan-Headers",
    "dxvk-nvapi/external/nvapi", "dxvk-nvapi/external/vkroots",
    "vkd3d-proton/khronos/SPIRV-Headers", "vkd3d-proton/khronos/Vulkan-Headers",
    "vkd3d-proton/subprojects/dxil-spirv",
    "vkd3d-proton/subprojects/dxil-spirv/subprojects/dxbc-spirv",
    "vkd3d-proton/subprojects/dxil-spirv/subprojects/dxbc-spirv/submodules/spirv_headers",
    "vkd3d-proton/subprojects/dxil-spirv/third_party/SPIRV-Cross",
    "vkd3d-proton/subprojects/dxil-spirv/third_party/SPIRV-Tools",
    "vkd3d-proton/subprojects/dxil-spirv/third_party/spirv-headers",
)
GST_PATHS = tuple("subprojects/" + name for name in (
    "gstreamer", "gst-plugins-base", "gst-plugins-good", "gst-plugins-bad", "gst-libav"))
PROTON_PATHS = ("Makefile.in", "configure.sh", "make", "proton", "default_pfx.py",
                "filelock.py", "user_settings.sample.py", "proton_3.7_tracked_files",
                "compatibilitytool.vdf.template", "toolmanifest_x86_64.vdf",
                "dist.LICENSE", "LICENSE", ".gitmodules", "fonts", "media",
                "icu/README.md")
HELPER_SOURCES = ("tools/direct-audio-handoff/linux_wait.rs",
                  "tools/direct-audio-handoff/linux_wait.h",
                  "tools/direct-audio-handoff/shim.cpp")
HELPER_RECIPES = ("tools/direct-audio-handoff/CMakeLists.txt",
                  "tools/direct-audio-handoff/probe.cpp", "tools/mf3/prebuilt_kit.py",
                  "tools/mf3/native_builder.py", ".github/workflows/ap8-windows.yml")
NOTICE = re.compile(r"^(?:licen[cs]e|copying|notice|third.party.notices|patents|authors)(?:$|[._-])", re.I)


def is_notice(name):
    path = Path(name)
    return bool(NOTICE.match(path.name)) or any(part.lower() in ("licenses", "licences") for part in path.parts[:-1])


def component_paths(relative):
    if not relative:
        return PROTON_PATHS
    if relative == "gstreamer":
        return GST_PATHS
    if relative == "kaldi":
        return ("src", "cmake", "tools/CLAPACK", "CMakeLists.txt", "COPYING", "README.md", "INSTALL")
    return None


def git(source, *args):
    return subprocess.check_output(["git", *args], cwd=source, timeout=60)


def sha(path):
    if path.is_symlink() or not path.is_file():
        raise ValueError("source input missing or linked: " + str(path))
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def selected(name, paths):
    return paths is None or any(name == path or name.startswith(path + "/") for path in paths)


def tracked(source, paths=None):
    result = []
    for item in git(source, "ls-files", "--stage", "-z").split(b"\0"):
        if not item:
            continue
        metadata, name = item.split(b"\t", 1)
        mode, _, stage = metadata.split()
        name = name.decode()
        if stage != b"0":
            raise ValueError("unmerged source: " + name)
        if mode != b"160000" and selected(name, paths):
            result.append(name)
    return sorted(result)


def component_commit(proton, relative):
    if relative == "wine":
        expected = RECIPE["wine"]["commit"]
    else:
        parent = max((name for name in COMPONENTS if relative.startswith(name + "/")),
                     key=len, default="")
        path = relative[len(parent) + 1:] if parent else relative
        expected = git(proton / parent, "rev-parse", "HEAD:" + path).decode().strip()
    actual = git(proton / relative, "rev-parse", "HEAD").decode().strip()
    if actual != expected:
        raise ValueError("selected source commit differs: " + relative)


def copy_component_notices(output):
    """Preserve original texts, using the already-pinned Git objects."""
    proton = output / "proton"
    destination = output / "third-party-notices/components"
    for relative in COMPONENTS:
        component_commit(proton, relative)
        source = proton / relative
        names = [name for name in tracked(source, component_paths(relative)) if is_notice(name)]
        if not names:
            raise ValueError("selected component has no retained notice: " + relative)
        for name in names:
            target = destination / relative / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(git(source, "show", "HEAD:" + name))
        if relative == "dxvk-nvapi/external/nvapi":
            # License.txt addresses the SDK libraries. The used header carries
            # its own NVIDIA copyright and MIT permission block.
            for name in tracked(source):
                if not name.endswith(".h"):
                    continue
                header = git(source, "show", "HEAD:" + name)
                end = header.find(b"*/")
                if end < 0 or b"Permission is hereby granted" not in header[:end]:
                    raise ValueError("NVAPI header notice layout differs: " + name)
                target = destination / relative / (name + ".notice.txt")
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(header[:end + 2] + b"\n")


def archive_files(target, files, allow_links=False):
    """Only caller-selected source/configuration files, with stable metadata."""
    with target.open("xb") as raw, gzip.GzipFile(fileobj=raw, filename="", mode="wb",
                                               compresslevel=1, mtime=RECIPE["proton"]["source_date_epoch"]) as compressed:
        with tarfile.open(fileobj=compressed, mode="w|") as archive:
            for path, name in sorted(files, key=lambda item: item[1]):
                if (path.is_symlink() and not allow_links) or (not path.is_symlink() and not path.is_file()):
                    raise ValueError("collected source missing or linked: " + str(path))
                if Path(name).is_absolute() or ".." in Path(name).parts:
                    raise ValueError("unsafe source member: " + name)
                info = archive.gettarinfo(str(path), arcname=name)
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                info.mtime = RECIPE["proton"]["source_date_epoch"]
                info.mode = 0o755 if path.lstat().st_mode & 0o111 else 0o644
                if path.is_symlink():
                    link = str(path.readlink())
                    resolved = posixpath.normpath(posixpath.join(posixpath.dirname(name), link))
                    if link.startswith("/") or resolved.startswith("../") or resolved == "..":
                        raise ValueError("source link escapes the companion: " + name)
                    archive.addfile(info)
                    continue
                with path.open("rb") as stream:
                    archive.addfile(info, stream)


def copy_archive_notices(path, destination):
    """Copy notice texts only, never unpack source or executable payloads."""
    found = set()
    def save(name, size, read):
        if Path(name).is_absolute() or ".." in Path(name).parts or size > 1024 * 1024:
            raise ValueError("source archive notice member differs: " + name)
        target = destination / name
        if name in found or target.is_symlink():
            raise ValueError("duplicate source archive notice: " + name)
        data = read()
        if len(data) != size:
            raise ValueError("source archive notice extent differs: " + name)
        if target.exists() and target.read_bytes() != data:
            raise ValueError("retained source archive notice changed: " + name)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        found.add(name)
    if path.suffix == ".zip":
        with zipfile.ZipFile(path) as archive:
            for member in archive.infolist():
                if is_notice(member.filename) and not member.is_dir():
                    save(member.filename, member.file_size, lambda: archive.read(member))
    else:
        with tarfile.open(path, "r|*") as archive:
            for member in archive:
                if is_notice(member.name) and member.isfile():
                    save(member.name, member.size, lambda: archive.extractfile(member).read())


def cargo_sources(output, destination):
    """Compiler .d files select sources; Cargo.lock owns registry/Git identity."""
    lock = tomllib.loads((output / "proton/gst-plugins-rs/Cargo.lock").read_text())
    packages = lock["package"]
    registry = {(p["name"], p["version"]): p["checksum"] for p in packages
                if p.get("source", "").startswith("registry+")}
    revisions = {p["source"].rsplit("#", 1)[1] for p in packages
                 if p.get("source", "").startswith("git+")}
    roots = {}
    for arch in ("i386", "x86_64"):
        objects = output / ("build/obj-gst_plugins_rs-" + arch)
        dependencies = list(objects.rglob("*.d"))
        if not dependencies:
            raise ValueError("selected Cargo compiler dependencies missing: " + arch)
        for dependency in dependencies:
            for relative in re.findall(r"/work/cargo/((?:registry/src|git/checkouts)/[^\s:]+)", dependency.read_text()):
                parts = Path(relative).parts
                if len(parts) < 4:
                    raise ValueError("Cargo source path differs: " + relative)
                root = Path(*parts[:4])
                roots.setdefault(root, set()).add(str(Path(*parts[4:])))
    if not roots:
        raise ValueError("no compiler-selected Cargo sources")
    destination.mkdir()
    notices = output / "third-party-notices/cargo"
    for relative in sorted(roots):
        source = output / "cargo" / relative
        if source.is_symlink() or not source.resolve().is_relative_to((output / "cargo").resolve()):
            raise ValueError("Cargo source escapes its cache")
        if relative.parts[0] == "registry":
            package = tomllib.loads((source / "Cargo.toml").read_text())["package"]
            expected = registry[(package["name"], package["version"])]
            checksum = json.loads((source / ".cargo-checksum.json").read_bytes())
            if checksum["package"] != expected:
                raise ValueError("Cargo registry source differs: " + str(relative))
            cache = output / "cargo/registry/cache" / relative.parts[2] / (relative.name + ".crate")
            if sha(cache) != expected:
                raise ValueError("Cargo crate archive differs: " + relative.name)
            # Authenticate the extracted file map against the lock-verified
            # archive, rather than trusting mutable cache metadata by itself.
            original = {}
            with tarfile.open(cache, "r|gz") as archive:
                for member in archive:
                    if not member.isfile():
                        continue
                    parts = Path(member.name).parts
                    name = str(Path(*parts[1:]))
                    if not parts or parts[0] != relative.name or name in original or ".." in parts:
                        raise ValueError("Cargo source archive layout differs")
                    original[name] = hashlib.file_digest(archive.extractfile(member), "sha256").hexdigest()
            if original != checksum["files"] or not roots[relative].issubset(original):
                raise ValueError("Cargo source file inventory differs from its original archive")
            for name, digest in checksum["files"].items():
                if Path(name).is_absolute() or ".." in Path(name).parts or sha(source / name) != digest:
                    raise ValueError("Cargo registry source changed: " + str(relative / name))
            shutil.copyfile(cache, destination / cache.name)
            names = [name for name in checksum["files"] if is_notice(name)]
        else:
            revision = git(source, "rev-parse", "HEAD").decode().strip()
            if revision not in revisions or git(source, "diff", "HEAD", "--"):
                raise ValueError("Cargo Git source differs: " + str(relative))
            files = tracked(source)
            if not roots[relative].issubset(files):
                raise ValueError("Cargo compiler used untracked Git source: " + str(relative))
            archive_files(destination / (relative.parts[2] + "-" + revision + ".tar.gz"),
                          [(source / name, str(Path("cargo/git/checkouts") / relative.parts[2] / relative.parts[3] / name))
                           for name in files], allow_links=True)
            names = [name for name in files if is_notice(name)]
        if not names:
            raise ValueError("Cargo dependency notice missing: " + str(relative))
        for name in names:
            target = notices / relative.name / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source / name, target)


def build_context(output, repository):
    """Read existing construction facts; never regenerate them at collection."""
    path = output / "build/redist/wine-build.json"
    if path.stat().st_size > 65536 or path.is_symlink():
        raise ValueError("construction declaration missing or changed")
    declaration = json.loads(path.read_bytes())
    expected = {name: sha(repository / name) for name in HELPER_SOURCES}
    configurations = {arch: sha(output / ("build/obj-wine-" + arch + "/Makefile"))
                      for arch in ("i386", "x86_64")}
    if (declaration.get("wine_revision") != RECIPE["wine"]["commit"]
            or declaration.get("sdk_image") != RECIPE["sdk"]
            or declaration.get("helper_sources") != expected
            or declaration.get("configuration_sha256") != configurations):
        raise ValueError("construction source/configuration context differs")
    return path


def collect(output, destination):
    if destination.exists() or destination.parent != output:
        raise ValueError("source destination must be a new direct child of the build root")
    # A failed collection must not leave a complete-looking companion directory.
    with tempfile.TemporaryDirectory(dir=output, prefix=destination.name + ".") as temporary:
        stage = Path(temporary)
        collect_into(output, stage)
        stage.rename(destination)


def collect_into(output, destination):
    redist = runpy.run_path(str(ROOT / "redist-inputs.py"))
    proton = output / "proton"
    redist["verify_commits"](proton)
    if redist["source_state"](proton) != 1:
        raise ValueError("prepared Proton source required")
    runpy.run_path(str(ROOT / "prepare.py"))["verify_sources"](proton / "wine", "after")
    wine_changes = {name for name, spec in RECIPE["sources"].items()
                    if spec["before"] is not None and spec["before"] != spec["after"]}
    actual = set(git(proton / "wine", "diff", "--name-only", "HEAD").decode().splitlines())
    if actual != wine_changes:
        raise ValueError("prepared Wine source changes differ from the recorded patches")
    repository = ROOT.parent.parent
    construction = build_context(output, repository)
    for arch in ("i386", "x86_64"):
        if not (output / ("build/.wine-" + arch + "-build")).is_file():
            raise ValueError("Wine build incomplete: " + arch)
        if not (output / ("build/.gst_plugins_rs-" + arch + "-build")).is_file():
            raise ValueError("selected Rust build incomplete: " + arch)
    sources = output / "corresponding-source"
    sources.mkdir(exist_ok=True)
    for name, spec in INPUTS["corresponding_source"].items():
        redist["acquire"](sources / name, spec)
    # Prepare notices again so newly compiled Rust dependency notices join them.
    for name, spec in INPUTS["notices"].items():
        redist["acquire"](output / "third-party-notices" / name, spec)
    redist["notices"](output)
    prepared = destination / "prepared-source"
    prepared.mkdir()
    for relative in ("", *COMPONENTS):
        source = proton / relative
        if relative:
            component_commit(proton, relative)
            if relative not in ("wine", "piper") and git(source, "diff", "HEAD", "--"):
                raise ValueError("selected component source changed: " + relative)
        paths = component_paths(relative)
        names = tracked(source, paths)
        if relative == "wine":
            names += [name for name, spec in RECIPE["sources"].items() if spec["before"] is None]
            names += ["dlls/winevulkan/vk.xml", "dlls/winevulkan/video.xml"]
            inputs = json.loads((ROOT / "build-inputs.json").read_bytes())
            for name in ("vk.xml", "video.xml"):
                if sha(source / "dlls/winevulkan" / name) != inputs[name]["sha256"]:
                    raise ValueError("prepared Wine Vulkan registry differs: " + name)
        archive_files(prepared / ((relative or "proton-build").replace("/", "_") + ".tar.gz"),
                      [(source / name, str(Path("proton") / relative / name)) for name in sorted(set(names))],
                      allow_links=True)
    recipes = [(repository / name, name) for name in tracked(repository, ("tools/runtime",))]
    recipes += [(repository / name, name) for name in (*HELPER_SOURCES, *HELPER_RECIPES)]
    for item in (*RECIPE["patches"], *RECIPE["fixtures"]):
        path = ROOT.parent / item["path"]
        if sha(path) != item["sha256"]:
            raise ValueError("source recipe input differs: " + item["path"])
        if not path.is_relative_to(ROOT):
            recipes.append((path, "tools/" + item["path"]))
    archive_files(prepared / "bridge-build-recipes.tar.gz", recipes)
    configs = [(output / "build/Makefile", "build/Makefile"),
               (output / "build-inner.sh", "build-inner.sh"),
               (construction, "build/redist/wine-build.json")]
    for arch in ("i386", "x86_64"):
        for name in ("Makefile", "include/config.h"):
            relative = "build/obj-wine-" + arch + "/" + name
            configs.append((output / relative, relative))
    archive_files(prepared / "wine-build-configuration.tar.gz", configs)
    phonemize = "proton/external-inputs/phonemize/piper-phonemize-5ecbb9b59abcffe03a9775ae82491c9ac5037f4d/CMakeLists.txt"
    archive_files(prepared / "piper-local-configuration.tar.gz", [(output / phonemize, phonemize)])
    cargo_sources(output, destination / "cargo")
    archives = destination / "upstream-source"
    archives.mkdir()
    for name in INPUTS["corresponding_source"]:
        shutil.copyfile(sources / name, archives / name)
        if name not in INPUTS["source_notices"]:
            copy_archive_notices(sources / name, output / "third-party-notices/source-archives" / name)
    # Preserve all source-form external inputs, including the configure-only
    # Sonic input. ONNX's binary package is not a source companion input.
    build_inputs = json.loads((ROOT / "build-inputs.json").read_bytes())
    for name, spec in build_inputs.items():
        if name == "onnxruntime.tgz":
            continue
        path = proton / "external-inputs" / name
        if path.stat().st_size != spec["size"] or sha(path) != spec["sha256"]:
            raise ValueError("source build input differs: " + name)
        shutil.copyfile(path, archives / name)
    shutil.copytree(output / "third-party-notices", destination / "third-party-notices")
    collector_revision = git(repository, "rev-parse", "HEAD").decode().strip()
    (destination / "README.txt").write_text(
        "Matching source for the common Wine/audio/graphics/services build.\n\n"
        "Engine source basis: Proton " + RECIPE["proton"]["commit"] + ", Wine " + RECIPE["wine"]["commit"] + ".\n"
        "Engine SDK: " + RECIPE["sdk"] + ".\n"
        "Original construction declaration SHA256: " + sha(construction) + ".\n"
        "Collector/packaging Git base: " + collector_revision + ".\n"
        "The original declaration is retained verbatim; the later collector revision\n"
        "identifies packaging source and does not replace the engine construction facts.\n\n"
        "prepared-source/*.tar.gz retain the prepared proton/, tools/ and build/ paths.\n"
        "Unpack them together into one working directory. They contain the applied Wine\n"
        "and Proton changes, recipes/patches, both Wine configurations and local Piper\n"
        "CMake configuration. upstream-source/ retains the original source archives;\n"
        "the build-inputs.py recipe records their extraction locations and adaptations.\n"
        "cargo/ contains only compiler-selected registry packages and Git workspaces.\n"
        "Keep the original Cargo workspace manifests and lockfile unchanged.\n\n"
        "The original direct-audio helper source, CMake/MSVC recipe, paired native\n"
        "builder and Windows build workflow are included under their tools/ paths.\n"
        "No installed host or vendor payload is part of this source collection.\n\n"
        "tools/runtime/source.json pins the original Git bases and SDK. The source\n"
        "exports omit Git metadata; identity-checked build.sh acquisition starts from\n"
        "those original Git revisions. tools/runtime/build-inner.sh and proton/Makefile.in\n"
        "preserve the invocation and component build graph.\n"
        "The SDK/compiler and cache-only workspace dependencies are not this source\n"
        "payload. Third-party licenses, exceptions, notices and source availability\n"
        "requirements remain applicable; preserve third-party-notices/ with delivery.\n")
    print("Selected prepared source, build context, source archives and dependency notices collected")


def check():
    """Reject changed/missing selected inputs and avoid copying cache-only crates."""
    with tempfile.TemporaryDirectory(prefix="lvb-source-companion-") as temporary:
        output = Path(temporary)
        crate = output / "cargo/registry/src/test-registry/selected-1.0.0"
        crate.mkdir(parents=True)
        (crate / "Cargo.toml").write_text('[package]\nname="selected"\nversion="1.0.0"\n')
        (crate / "LICENSE").write_text("fixture notice")
        cache = output / "cargo/registry/cache/test-registry/selected-1.0.0.crate"
        cache.parent.mkdir(parents=True)
        with tarfile.open(cache, "w:gz") as archive:
            for name in ("Cargo.toml", "LICENSE"):
                archive.add(crate / name, arcname="selected-1.0.0/" + name)
        (crate / ".cargo-checksum.json").write_text(json.dumps({
            "package": sha(cache), "files": {name: sha(crate / name) for name in ("Cargo.toml", "LICENSE")}}))
        lock = output / "proton/gst-plugins-rs/Cargo.lock"
        lock.parent.mkdir(parents=True)
        lock.write_text('version=3\n[[package]]\nname="selected"\nversion="1.0.0"\n'
                        'source="registry+https://example.invalid"\nchecksum="' + sha(cache) + '"\n')
        (crate.parent / "unused-9.0.0").mkdir()
        for arch in ("i386", "x86_64"):
            objects = output / ("build/obj-gst_plugins_rs-" + arch)
            objects.mkdir(parents=True)
            (objects / "selected.d").write_text("target: /work/cargo/registry/src/test-registry/selected-1.0.0/LICENSE\n")
        destination = output / "selected"
        cargo_sources(output, destination)
        assert [path.name for path in destination.iterdir()] == [cache.name]
        assert (output / "third-party-notices/cargo/selected-1.0.0/LICENSE").read_text() == "fixture notice"
        (crate / "LICENSE").write_text("changed")
        checksums = json.loads((crate / ".cargo-checksum.json").read_bytes())
        checksums["files"]["LICENSE"] = sha(crate / "LICENSE")
        (crate / ".cargo-checksum.json").write_text(json.dumps(checksums))
        try:
            cargo_sources(output, output / "changed")
        except ValueError:
            pass
        else:
            raise AssertionError("changed Cargo source was accepted")
        source = output / "source.txt"
        source.write_text("source")
        archive_files(output / "one.tar.gz", [(source, "proton/source.txt")])
        archive_files(output / "two.tar.gz", [(source, "proton/source.txt")])
        assert sha(output / "one.tar.gz") == sha(output / "two.tar.gz")
        notice_zip = output / "notices.zip"
        with zipfile.ZipFile(notice_zip, "w") as archive:
            archive.writestr("source/LICENSE", "copyright fixture")
            archive.writestr("source/LICENSES/MIT.txt", "full license fixture")
            archive.writestr("source/payload.exe", "must not be copied")
        copy_archive_notices(notice_zip, output / "notices")
        copy_archive_notices(notice_zip, output / "notices")
        assert (output / "notices/source/LICENSE").read_text() == "copyright fixture"
        assert (output / "notices/source/LICENSES/MIT.txt").read_text() == "full license fixture"
        assert not (output / "notices/source/payload.exe").exists()
        repository = output / "helper-repository"
        for name in HELPER_SOURCES:
            path = repository / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("original helper source " + name)
        configs = {}
        for arch in ("i386", "x86_64"):
            path = output / ("build/obj-wine-" + arch + "/Makefile")
            path.parent.mkdir(parents=True)
            path.write_text("native " + arch)
            configs[arch] = sha(path)
        construction = output / "build/redist/wine-build.json"
        construction.parent.mkdir(parents=True)
        construction.write_text(json.dumps(dict(wine_revision=RECIPE["wine"]["commit"], sdk_image=RECIPE["sdk"],
            helper_sources={name: sha(repository / name) for name in HELPER_SOURCES}, configuration_sha256=configs)))
        original = construction.read_bytes()
        assert build_context(output, repository) == construction
        assert construction.read_bytes() == original
        (repository / HELPER_SOURCES[0]).write_text("different helper")
        try:
            build_context(output, repository)
        except ValueError:
            pass
        else:
            raise AssertionError("a different paired helper source was accepted")
        for arch in ("i386", "x86_64"):
            (output / ("build/obj-gst_plugins_rs-" + arch) / "selected.d").unlink()
        try:
            cargo_sources(output, output / "missing-dependencies")
        except ValueError:
            pass
        else:
            raise AssertionError("missing compiler dependency evidence was accepted")
        source.unlink()
        source.symlink_to(cache)
        try:
            archive_files(output / "linked.tar.gz", [(source, "proton/source.txt")])
        except ValueError:
            pass
        else:
            raise AssertionError("linked source was collected")
    print("Source companion selection, integrity and stable source-archive regressions verified")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-root", type=Path)
    parser.add_argument("--destination", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        check()
    elif args.build_root and args.destination:
        collect(args.build_root.resolve(strict=True), args.destination.resolve())
    else:
        parser.error("provide --build-root and --destination, or --self-test")
