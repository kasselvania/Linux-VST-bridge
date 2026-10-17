#!/usr/bin/env python3
"""Collect matching source beside a completed private common runtime.

Run after redist: source-companion.py --build-root OUTPUT --destination NEW_DIR
    --construction-source ORIGINAL_PROJECT_CHECKOUT_OR_SNAPSHOT
The construction input defaults to this checkout and must match the retained
declaration recipe hash. A later collector checkout does not replace that input.
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
ORIGINAL_SUPPORT = ("tools/runtime/build.sh", "tools/runtime/complete.sh", "tools/runtime/README.md")
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


def cargo_dependency_path(source, name):
    """Rust include_str! paths may retain src/../ beside tracked source names."""
    normalized = posixpath.normpath(name)
    if (Path(name).is_absolute() or normalized in (".", "..")
            or normalized.startswith("../")):
        raise ValueError("Cargo compiler source path escapes its checkout: " + name)
    if normalized != name:
        actual = (source / name).resolve(strict=True)
        if (actual != (source / normalized).resolve(strict=True)
                or not actual.is_relative_to(source.resolve(strict=True))):
            raise ValueError("Cargo compiler source path changes through a link: " + name)
    return normalized


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
            make_dependencies = "\n".join(line for line in dependency.read_text().splitlines()
                                          if not line.lstrip().startswith("#"))
            for relative in re.findall(r"/work/cargo/((?:registry/src|git/checkouts)/[^\s:]+)", make_dependencies):
                parts = Path(relative).parts
                if len(parts) < 4:
                    raise ValueError("Cargo source path differs: " + relative)
                root = Path(*parts[:4])
                name = cargo_dependency_path(output / "cargo" / root, str(Path(*parts[4:])))
                roots.setdefault(root, set()).add(name)
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
            if relative.name != package["name"] + "-" + package["version"]:
                raise ValueError("Cargo registry source path differs: " + str(relative))
            cache = output / "cargo/registry/cache" / relative.parts[2] / (relative.name + ".crate")
            if sha(cache) != expected:
                raise ValueError("Cargo crate archive differs: " + relative.name)
            # Use the lock-verified archive itself. Current Cargo caches need
            # not contain .cargo-checksum.json; mutable cache metadata is not
            # evidence that the compiled sources match the original package.
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
            if not roots[relative].issubset(original):
                raise ValueError("Cargo source file inventory differs from its original archive")
            for name, digest in original.items():
                if Path(name).is_absolute() or ".." in Path(name).parts or sha(source / name) != digest:
                    raise ValueError("Cargo registry source changed: " + str(relative / name))
            shutil.copyfile(cache, destination / cache.name)
            names = [name for name in original if is_notice(name)]
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


def construction_recipe(repository):
    """Use the builder's canonical hash over the explicit original file roster."""
    repository = repository.resolve(strict=True)
    builder = runpy.run_path(str(ROOT / "build-identity.py"))
    recipe = json.loads((repository / "tools/runtime/source.json").read_bytes())
    names = {"tools/runtime/" + name for name in builder["RECIPE_FILES"]}
    names.update("tools/" + item["path"] for item in (*recipe["patches"], *recipe["fixtures"]))
    names.update("tools/runtime/patches/" + name for name in
                 ("proton-redist-inputs.patch", "proton-audio-composition.patch"))
    for name in names:
        path = repository / name
        if Path(name).is_absolute() or ".." in Path(name).parts or not path.resolve().is_relative_to(repository):
            raise ValueError("construction recipe path escapes its snapshot: " + name)
    identity = builder["recipe_identity"]
    identity.__globals__.update(ROOT=repository, HERE=repository / "tools/runtime")
    return identity(), sorted(names), recipe


def build_context(output, repository):
    """Read existing construction facts; never regenerate them at collection."""
    path = output / "build/redist/wine-build.json"
    if path.stat().st_size > 65536 or path.is_symlink():
        raise ValueError("construction declaration missing or changed")
    declaration = json.loads(path.read_bytes())
    recipe_sha256, _, recipe = construction_recipe(repository)
    if declaration.get("recipe_sha256") != recipe_sha256:
        raise ValueError("original construction recipe hash differs from retained declaration")
    if recipe != RECIPE:
        raise ValueError("collector and construction source basis differ")
    expected = {name: sha(repository / name) for name in HELPER_SOURCES}
    configurations = {arch: sha(output / ("build/obj-wine-" + arch + "/Makefile"))
                      for arch in ("i386", "x86_64")}
    if (declaration.get("schema") != 1 or declaration.get("kind") != "wine_build"
            or declaration.get("wine_revision") != RECIPE["wine"]["commit"]
            or declaration.get("sdk_image") != RECIPE["sdk"]
            or declaration.get("helper_sources") != expected
            or declaration.get("configuration_sha256") != configurations):
        raise ValueError("construction source/configuration context differs")
    return path


def archive_construction(target, repository, declaration):
    recipe_hash, names, _ = construction_recipe(repository)
    if recipe_hash != declaration["recipe_sha256"]:
        raise ValueError("original construction recipe changed before collection")
    original_names = sorted(set(names) | set(HELPER_SOURCES) | set(HELPER_RECIPES) | set(ORIGINAL_SUPPORT))
    archive_files(target, [(repository / name, name) for name in original_names])
    with tarfile.open(target) as archive:
        archived = {member.name: hashlib.sha256(archive.extractfile(member).read()).hexdigest()
                    for member in archive if member.isfile()}
    canonical = runpy.run_path(str(ROOT / "build-identity.py"))["canonical"]
    recipe_hash = hashlib.sha256(canonical({name: archived[name] for name in names})).hexdigest()
    if (recipe_hash != declaration["recipe_sha256"]
            or {name: archived[name] for name in HELPER_SOURCES} != declaration["helper_sources"]):
        raise ValueError("archived original construction bytes differ from retained declaration")


def collect(output, destination, construction_source=None):
    storage = Path("/mnt/Storage2")
    if output.is_relative_to(storage) and not storage.is_mount():
        raise ValueError("Storage2 must be mounted before source acquisition/collection")
    if destination.exists() or destination.parent != output:
        raise ValueError("source destination must be a new direct child of the build root")
    # A failed collection must not leave a complete-looking companion directory.
    with tempfile.TemporaryDirectory(dir=output, prefix=destination.name + ".") as temporary:
        stage = Path(temporary)
        collect_into(output, stage, construction_source or ROOT.parent.parent)
        stage.rename(destination)


def collect_into(output, destination, construction_source):
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
    construction = build_context(output, construction_source)
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
    original_declaration = json.loads(construction.read_bytes())
    archive_construction(prepared / "original-construction-recipes.tar.gz", construction_source, original_declaration)
    # The later collector is useful packaging source, with a distinct member
    # prefix so unpacking it cannot overwrite the original construction files.
    _, collector_names, _ = construction_recipe(repository)
    collector_names = sorted(set(collector_names) | set(tracked(repository, ("tools/runtime",)))
                             | set(HELPER_SOURCES) | set(HELPER_RECIPES))
    collector_files = [(repository / name, "collector/" + name) for name in collector_names]
    archive_files(prepared / "collector-source.tar.gz", collector_files)
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
        "Original construction recipe SHA256: " + original_declaration["recipe_sha256"] + ".\n"
        "Collector/packaging Git base: " + collector_revision + ".\n"
        "The original declaration is retained verbatim; the later collector revision\n"
        "identifies packaging source and does not replace the engine construction facts.\n\n"
        "prepared-source/*.tar.gz retain the prepared proton/, tools/ and build/ paths.\n"
        "original-construction-recipes.tar.gz contains the recipe bytes authenticated\n"
        "by the retained declaration. collector-source.tar.gz is separately prefixed\n"
        "under collector/ and contains the later source/notice packaging code.\n"
        "Unpack them together into one working directory. They contain the applied Wine\n"
        "and Proton changes, recipes/patches, both Wine configurations and local Piper\n"
        "CMake configuration. upstream-source/ retains the original source archives;\n"
        "the original build-inputs.py records their extraction locations and adaptations.\n"
        "cargo/ contains only compiler-selected registry packages and Git workspaces.\n"
        "Keep the original Cargo workspace manifests and lockfile unchanged.\n\n"
        "The original direct-audio helper source, CMake/MSVC recipe, paired native\n"
        "builder and Windows build workflow are included under their tools/ paths.\n"
        "No installed host or vendor payload is part of this source collection.\n\n"
        "The original tools/runtime/source.json pins the Git bases and SDK. The source\n"
        "exports omit Git metadata; identity-checked build.sh acquisition starts from\n"
        "those original Git revisions. tools/runtime/build-inner.sh and proton/Makefile.in\n"
        "preserve the invocation and component build graph.\n"
        "The SDK/compiler and cache-only workspace dependencies are not this source\n"
        "payload. Third-party licenses, exceptions, notices and source availability\n"
        "requirements remain applicable; preserve third-party-notices/ with delivery.\n")
    print("Selected prepared source, build context, source archives and dependency notices collected")


def check_cargo_git_paths(output):
    source = output / "cargo/git/checkouts/fixture/revision"
    (source / "library/src").mkdir(parents=True)
    (source / "library/README.md").write_text("tracked package documentation")
    (source / "library/src/lib.rs").write_text('#![doc = include_str!("../README.md")]\n')
    (source / "LICENSE").write_text("fixture Git notice")
    git(source, "init", "-q")
    git(source, "add", ".")
    git(source, "-c", "user.name=Source fixture", "-c", "user.email=fixture@example.invalid",
        "commit", "-qm", "Pinned source fixture")
    revision = git(source, "rev-parse", "HEAD").decode().strip()
    lock = output / "proton/gst-plugins-rs/Cargo.lock"
    lock.parent.mkdir(parents=True)
    lock.write_text('version=3\n[[package]]\nname="fixture"\nversion="1.0.0"\n'
                    'source="git+https://example.invalid/fixture#' + revision + '"\n')
    dependencies = []
    for arch in ("i386", "x86_64"):
        path = output / ("build/obj-gst_plugins_rs-" + arch + "/fixture.d")
        path.parent.mkdir(parents=True)
        dependencies.append(path)
    def compiler_path(name):
        for path in dependencies:
            path.write_text("target: /work/cargo/git/checkouts/fixture/revision/" + name + "\n")
    def reject(name, message):
        try:
            cargo_sources(output, output / name)
        except ValueError as failure:
            assert message in str(failure), str(failure)
        else:
            raise AssertionError("invalid Cargo Git source was collected: " + name)
    compiler_path("library/src/../README.md")
    cargo_sources(output, output / "selected")
    with tarfile.open(output / ("selected/fixture-" + revision + ".tar.gz")) as archive:
        assert archive.extractfile("cargo/git/checkouts/fixture/revision/library/README.md").read() == b"tracked package documentation"
    (source / "library/README.md").write_text("changed source")
    reject("changed", "Cargo Git source differs")
    git(source, "checkout", "--", "library/README.md")
    (source / "library/untracked.rs").write_text("untracked code")
    compiler_path("library/src/../untracked.rs")
    reject("untracked", "Cargo compiler used untracked Git source")
    compiler_path("library/src/../../../LICENSE")
    reject("escape", "Cargo compiler source path escapes")
    external = output / "external"
    (external / "child").mkdir(parents=True)
    (external / "LICENSE").write_text("outside source")
    (source / "redirect").symlink_to(external / "child", target_is_directory=True)
    compiler_path("redirect/../LICENSE")
    reject("linked", "Cargo compiler source path changes through a link")
    compiler_path("library/src/../README.md")
    lock.write_text(lock.read_text().replace(revision, "0" * 40))
    reject("unlocked", "Cargo Git source differs")


def check():
    """Reject changed/missing selected inputs and avoid copying cache-only crates."""
    with tempfile.TemporaryDirectory(prefix="lvb-source-companion-") as temporary:
        output = Path(temporary)
        check_cargo_git_paths(output / "git-paths")
        crate = output / "cargo/registry/src/test-registry/selected-1.0.0"
        crate.mkdir(parents=True)
        (crate / "src").mkdir()
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
            (objects / "selected.d").write_text("target: /work/cargo/registry/src/test-registry/selected-1.0.0/src/../LICENSE\n"
                "# env-dep:CARGO_MANIFEST_DIR=/work/cargo/registry/src/test-registry/selected-1.0.0\n")
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
        _, recipe_names, _ = construction_recipe(ROOT.parent.parent)
        for name in sorted(set(recipe_names) | set(HELPER_RECIPES) | set(ORIGINAL_SUPPORT)):
            path = repository / name
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT.parent.parent / name, path)
        original_recipe = repository / "tools/runtime/redist-inputs.json"
        original_recipe.write_bytes(original_recipe.read_bytes() + b"\n")
        configs = {}
        for arch in ("i386", "x86_64"):
            path = output / ("build/obj-wine-" + arch + "/Makefile")
            path.parent.mkdir(parents=True)
            path.write_text("native " + arch)
            configs[arch] = sha(path)
        construction = output / "build/redist/wine-build.json"
        construction.parent.mkdir(parents=True)
        construction.write_text(json.dumps(dict(schema=1, kind="wine_build", wine_revision=RECIPE["wine"]["commit"], sdk_image=RECIPE["sdk"],
            recipe_sha256=construction_recipe(repository)[0],
            helper_sources={name: sha(repository / name) for name in HELPER_SOURCES}, configuration_sha256=configs)))
        original = construction.read_bytes()
        assert build_context(output, repository) == construction
        assert construction.read_bytes() == original
        later = output / "later-collector"
        shutil.copytree(repository, later)
        later_recipe = later / "tools/runtime/redist-inputs.json"
        later_recipe.write_bytes(later_recipe.read_bytes() + b"\n")
        for name in ORIGINAL_SUPPORT:
            path = later / name
            path.write_bytes(path.read_bytes() + b"\nlater collector support revision\n")
        for mismatched in (later, ROOT.parent.parent):
            try:
                build_context(output, mismatched)
            except ValueError as failure:
                assert "construction recipe hash" in str(failure)
            else:
                raise AssertionError("later/default collector recipes replaced the original construction")
        archive_construction(output / "original-context.tar.gz", repository, json.loads(original))
        with tarfile.open(output / "original-context.tar.gz") as archive:
            assert archive.extractfile("tools/runtime/redist-inputs.json").read() == original_recipe.read_bytes()
            assert archive.extractfile("tools/runtime/redist-inputs.json").read() != later_recipe.read_bytes()
            for name in ORIGINAL_SUPPORT:
                assert archive.extractfile(name).read() == (repository / name).read_bytes()
                assert archive.extractfile(name).read() != (later / name).read_bytes()
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
    parser.add_argument("--construction-source", type=Path,
                        help="Original project checkout/snapshot matching wine-build.json recipe_sha256; defaults to this checkout")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        check()
    elif args.build_root and args.destination:
        construction_source = args.construction_source.resolve(strict=True) if args.construction_source else None
        collect(args.build_root.resolve(strict=True), args.destination.resolve(), construction_source)
    else:
        parser.error("provide --build-root and --destination, or --self-test")
