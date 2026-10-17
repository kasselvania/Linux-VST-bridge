#!/usr/bin/env python3
"""Acquire exact upstream redist packages and corresponding release source.

This prepares private build inputs. It does not publish a runtime or discharge
the remaining notice/source distribution obligations.
"""
import argparse
import ast
from contextlib import nullcontext
import hashlib
import io
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tarfile
import tempfile
from types import SimpleNamespace
from unittest.mock import patch as replace
import urllib.request

ROOT = Path(__file__).resolve().parent
INPUTS = json.loads((ROOT / "redist-inputs.json").read_bytes())
RECIPE = json.loads((ROOT / "source.json").read_bytes())
SOURCES = {
    "Makefile.in": (
        "25d1250ece9fe6d406784237242e1f01548ac7449ba67e316d314356f3b5d1f5",
        "11757e1aeb6be60e3d0722a1a514221a35832cd8a6bbe5327fdf342337a83d59"),
    "proton": (
        "787504a79bacf6b303984a9a846cf47463f36599248e8306b26d5faf78267aad",
        "93a5031cfa3ce203822d3cb87848c1345e1327677fafef3a0b0ecc593187a277"),
    "make/rules-cargo.mk": (
        "178b08f2877677a0c2c404682af7b6fe1492f1b6bcaab0a95aa8a35ad84b5861",
        "d2fc3ed4bd31e81e474da31bcae0d55c655431eca52ec7d66e744a53999ad5be"),
    "default_pfx.py": (
        "474c182df7ea0fa5c6b0a6495728b24655cc504848c45b638fb48c3b25a5b794",
        "14a5a426ad497b2624d1fbbef16b0f7ac011124f40a3ff9726c832cb51f42b90"),
}


def source_state(source):
    values = []
    for name, hashes in SOURCES.items():
        path = source / name
        if path.is_symlink() or not path.is_file():
            raise ValueError("redist source missing or linked: " + name)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest not in hashes:
            raise ValueError("redist source identity differs: " + name)
        values.append(hashes.index(digest))
    if len(set(values)) != 1:
        raise ValueError("redist source preparation is incomplete")
    return values[0]


def patch_source(source):
    patches = (
        ("proton-redist-inputs.patch", "d7948496aa7f68aafd6009bdbd18dd6a93319640dd3b824e2baa3ee1b9c30e08"),
        ("proton-audio-composition.patch", "36d899c358766ac48b0ebba8766ef3d96a63745511309b0064cb17396d1e665a"),
    )
    for name, expected in patches:
        if hashlib.sha256((ROOT / "patches" / name).read_bytes()).hexdigest() != expected:
            raise ValueError("redist build patch changed: " + name)
    if source_state(source) == 0:
        for name, _ in patches:
            patch = ROOT / "patches" / name
            subprocess.run(["git", "apply", "--check", str(patch)], cwd=source, check=True)
            subprocess.run(["git", "apply", str(patch)], cwd=source, check=True)
    if source_state(source) != 1:
        raise ValueError("redist source postimages differ")


def acquire(path, spec):
    if path.exists():
        if path.is_symlink() or path.stat().st_size != spec["size"]:
            raise ValueError("redist input changed: " + path.name)
    else:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=path.name + ".", delete=False) as out:
            temporary = Path(out.name)
            try:
                with urllib.request.urlopen(spec["url"], timeout=60) as response:
                    count = 0
                    digest = hashlib.sha256()
                    while block := response.read(1024 * 1024):
                        count += len(block)
                        if count > spec["size"]:
                            raise ValueError("redist input exceeds pinned extent: " + path.name)
                        out.write(block)
                        digest.update(block)
                if count != spec["size"] or digest.hexdigest() != spec["sha256"]:
                    raise ValueError("redist input identity differs: " + path.name)
                out.close()
                os.replace(temporary, path)
            finally:
                temporary.unlink(missing_ok=True)
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while block := source.read(1024 * 1024):
            digest.update(block)
    if path.stat().st_size != spec["size"] or digest.hexdigest() != spec["sha256"]:
        raise ValueError("redist input identity differs: " + path.name)


def notices(output):
    destination = output / "third-party-notices"
    for name, wanted in INPUTS["source_notices"].items():
        found = set()
        with tarfile.open(output / "corresponding-source" / name, mode="r|xz") as archive:
            for member in archive:
                if member.name not in wanted:
                    continue
                spec = wanted[member.name]
                if not member.isfile() or member.size != spec["size"] or member.name in found:
                    raise ValueError("source notice member differs: " + member.name)
                data = archive.extractfile(member).read(spec["size"] + 1)
                if hashlib.sha256(data).hexdigest() != spec["sha256"]:
                    raise ValueError("source notice identity differs: " + member.name)
                (destination / spec["name"]).write_bytes(data)
                found.add(member.name)
        if found != wanted.keys():
            raise ValueError("source notice missing: " + name)
    # The verified ONNX auxiliary archive carries these original notices.
    onnx = output / "proton/external-inputs/onnxruntime.tgz"
    spec = json.loads((ROOT / "build-inputs.json").read_bytes())["onnxruntime.tgz"]
    if onnx.stat().st_size != spec["size"] or hashlib.sha256(onnx.read_bytes()).hexdigest() != spec["sha256"]:
        raise ValueError("ONNX notice archive identity differs")
    wanted = {"LICENSE", "ThirdPartyNotices.txt", "Privacy.md"}
    with tarfile.open(onnx, mode="r|gz") as archive:
        for member in archive:
            name = Path(member.name).name
            if member.name != "onnxruntime-linux-x64-1.14.1/" + name or name not in wanted:
                continue
            if not member.isfile() or member.size > 1024 * 1024:
                raise ValueError("ONNX notice member differs")
            (destination / ("onnxruntime-" + name)).write_bytes(archive.extractfile(member).read())
            wanted.remove(name)
    if wanted:
        raise ValueError("ONNX auxiliary notices missing")
    for name, spec in INPUTS["local_notices"].items():
        path = output / "proton" / name
        if (path.is_symlink() or not path.is_file() or path.stat().st_size != spec["size"]
                or hashlib.sha256(path.read_bytes()).hexdigest() != spec["sha256"]):
            raise ValueError("local component notice differs: " + name)
        shutil.copyfile(path, destination / spec["name"])
    companion = runpy.run_path(str(ROOT / "source-companion.py"))
    companion["copy_component_notices"](output)
    build_inputs = json.loads((ROOT / "build-inputs.json").read_bytes())
    for name in ("fmt.zip", "spdlog.zip", "phonemize.zip", "espeak.zip"):
        path = output / "proton/external-inputs" / name
        spec = build_inputs[name]
        if path.stat().st_size != spec["size"] or companion["sha"](path) != spec["sha256"]:
            raise ValueError("external source notice archive differs: " + name)
        companion["copy_archive_notices"](path, destination / "external" / name)
    shutil.copyfile(ROOT / "redist-inputs.json", destination / "inputs.json")


def verify_commits(source):
    for name in ("proton", "wine"):
        path = source if name == "proton" else source / "wine"
        actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=path, text=True).strip()
        if actual != RECIPE[name]["commit"]:
            raise ValueError("redist source commit differs: " + name)


def prepare(output):
    source = output / "proton"
    verify_commits(source)
    status = subprocess.check_output(["git", "submodule", "status", "--recursive"],
                                     cwd=source, text=True)
    for line in status.splitlines():
        path = line[1:].split()[1]
        if path == "wine":  # Independently pinned and checked above.
            continue
        if path == "FEX" and line[0] == "-":  # Not used by this x86_64 build.
            continue
        if line[0] != " ":
            raise ValueError("redist auxiliary Git input differs: " + path)
    source_state(source)
    for group, directory in (("packages", source / "contrib"),
                             ("corresponding_source", output / "corresponding-source"),
                             ("notices", output / "third-party-notices")):
        directory.mkdir(exist_ok=True)
        for name, spec in INPUTS[group].items():
            # Remaining release source is acquired only by the explicit source
            # companion command, after compilation. Keep ordinary build inputs
            # unchanged when that optional source collection is not requested.
            if group == "corresponding_source" and name not in INPUTS["source_notices"]:
                continue
            acquire(directory / name, spec)
    patch_source(source)
    notices(output)
    print("Matching Mono, Gecko, Xalia and release-source inputs verified")


def check_launch_composition(source):
    """Execute the actual patched launcher dispatch without starting Wine.

    This checks argument/exit forwarding and initialization/wait ordering.
    Real prefix creation and descendant custody still require binary tests.
    """
    tree = ast.parse((source / "proton").read_text())
    session = next(node for node in tree.body if isinstance(node, ast.ClassDef) and node.name == "Session")
    run = next(node for node in session.body if isinstance(node, ast.FunctionDef) and node.name == "run")
    main = next(node for node in tree.body if isinstance(node, ast.If)
                and ast.unparse(node.test) == "__name__ == '__main__'")
    for anchored in (False, True):
        for verb in ("run", "waitforexitandrun", "runinprefix", "getcompatpath", "getnativepath"):
            calls = []
            target = [r"C:\Program Files\fixture.exe", "two words", '--quoted="value"']
            def proc(argv):
                calls.append(("command", argv))
                return 17
            proton = SimpleNamespace(
                wine_bin="/same-source/bin/wine", wineserver_bin="/same-source/bin/wineserver",
                cleanup_legacy_dist=lambda: calls.append("cleanup"),
                do_steampipe_fixups=lambda: calls.append("fixups"),
                missing_default_prefix=lambda: True,
                make_default_prefix=lambda: calls.append("default-prefix"))
            current = SimpleNamespace(
                remote_debug_cmd=None, env={}, cmdlineappend=["appended argument"], log_file=None,
                run_proc=proc, init_wine=lambda: calls.append("init-wine"),
                init_session=lambda prepare: calls.append(("init-session", prepare)))
            def exit_code(code):
                raise SystemExit(code)
            def winepath(argv, **kwargs):
                calls.append(("path", argv))
                return b"converted path"
            stdout = SimpleNamespace(buffer=io.BytesIO())
            namespace = {
                "__name__": "__main__",
                "sys": SimpleNamespace(argv=[str(source / "proton"), verb, *target],
                                       exit=exit_code, stdout=stdout),
                "os": SimpleNamespace(environ={"STEAM_COMPAT_DATA_PATH": "/new-private-prefix"}, path=os.path),
                "shutil": SimpleNamespace(which=lambda _: "anchor" if anchored else None),
                "subprocess": SimpleNamespace(check_output=winepath),
                "Proton": lambda _: proton, "CompatData": lambda _: SimpleNamespace(),
                "Session": lambda: current,
                "setup_game_dir_drive": lambda: calls.append("drive-setup"),
            }
            exec(compile(ast.Module(body=[run], type_ignores=[]), "patched-proton-run", "exec"), namespace)
            current.run = lambda: namespace["run"](current)
            try:
                exec(compile(ast.Module(body=main.body, type_ignores=[]), "patched-proton-main", "exec"), namespace)
            except SystemExit as result:
                assert result.code == (0 if verb.endswith("path") else 17)
            else:
                raise ValueError("launcher did not return an exit status")
            assert calls[:5] == ["cleanup", "fixups", "init-wine", "default-prefix",
                                 ("init-session", verb != "runinprefix")]
            remaining = calls[5:]
            if verb in ("run", "waitforexitandrun"):
                expected = ["drive-setup"] if verb == "run" else [("command", [proton.wineserver_bin, "-w"])]
                adverb = ["steam-runtime-launcher-interface-0", "proton"] if anchored else []
                expected.append(("command", adverb + [proton.wine_bin, *target, "appended argument"]))
                assert remaining == expected
            elif verb == "runinprefix":
                assert remaining == [("command", [proton.wine_bin, *target])]
            else:
                conversion = "-w" if verb == "getcompatpath" else ""
                args = [proton.wine_bin, "winepath"] + ([conversion] if conversion else []) + [target[0]]
                assert remaining == [("path", args)] and stdout.buffer.getvalue() == b"converted path"
            calls.clear()
            def refuse_preparation(_):
                raise RuntimeError("fixture prefix preparation failed")
            current.init_session = refuse_preparation
            try:
                exec(compile(ast.Module(body=main.body, type_ignores=[]), "patched-proton-main", "exec"), namespace)
            except RuntimeError as failure:
                assert str(failure) == "fixture prefix preparation failed"
            else:
                raise ValueError("launcher continued after preparation failed")
            assert not any(isinstance(call, tuple) and call[0] in ("command", "path") for call in calls)


def check_prefix_composition(source):
    """Run actual file preparation with synthetic ordinary payloads, no Wine.

    Locking and the Linux-only casefold ioctl are outside this temporary test.
    Prefix copying/tracking, registry identity, fonts and graphics remain real.
    """
    tree = ast.parse((source / "proton").read_text())
    tree.body = [node for node in tree.body if not
                 (isinstance(node, ast.ImportFrom) and node.module == "filelock")]
    namespace = {"__name__": "prefix-composition-test", "FileLock": lambda *a, **k: nullcontext()}
    exec(compile(tree, str(source / "proton"), "exec"), namespace)
    namespace["set_dir_casefold_bit"] = lambda _: None
    with tempfile.TemporaryDirectory(prefix="lvb-audio-prefix-") as directory:
        root = Path(directory).resolve()
        engine = root / "engine"
        proton = namespace["Proton"](str(engine))
        namespace["g_proton"] = proton
        for path in (proton.fonts_dir, proton.wine_fonts_dir, proton.default_pfx_dir + "dosdevices"):
            Path(path).mkdir(parents=True)
        Path(proton.fonts_dir, "fixture.ttf").write_text("font fixture")
        Path(proton.wine_inf).write_text("Wine metadata fixture")
        default = Path(proton.default_pfx_dir)
        (default / "system.reg").write_text('WINE REGISTRY Version 2\n\n'
                                             '[Software\\\\Microsoft\\\\Cryptography] 0\n'
                                             '"MachineGuid"="builder-template"\n')
        (default / "user.reg").write_text("WINE REGISTRY Version 2\n")
        graphics = ("d3d8", "d3d9", "d3d10", "d3d10_1", "d3d10core", "d3d11", "d3d12", "dxgi")
        for arch, folder in (("x86_64", "system32"), ("i386", "syswow64")):
            destination = default / "drive_c/windows" / folder
            destination.mkdir(parents=True)
            for name in graphics:
                (destination / (name + ".dll")).write_text(arch + " wine " + name)
            for component, names in (("wine/dxvk", ("d3d9", "d3d10core", "d3d11", "dxgi")),
                                     ("wine/vkd3d-proton", ("d3d12", "d3d12core")),
                                     ("wine/icu", ("icuin68", "icuuc68", "icudt68"))):
                destination = Path(proton.arch_pe_dir(component, arch == "i386"))
                destination.mkdir(parents=True, exist_ok=True)
                for name in names:
                    (destination / (name + ".dll")).write_text(arch + " " + component + " " + name)
        for renderer in ("dxvk", "wined3d"):
            state = root / renderer
            state.mkdir()
            compat = namespace["CompatData"](str(state))
            namespace["g_compatdata"] = compat
            options = {"disablenvapi"} | ({"wined3d"} if renderer == "wined3d" else set())
            namespace["g_session"] = SimpleNamespace(env={}, compat_config=options,
                                                      dlloverrides={}, nvidia_wine_dll_dir=None)
            with replace.dict(os.environ, {}, clear=True):
                compat.setup_prefix()
                prefix = Path(compat.prefix_dir)
                assert (prefix / "dosdevices/c:").is_symlink()
                assert (prefix / "drive_c/windows/Fonts/fixture.ttf").resolve() == Path(proton.fonts_dir, "fixture.ttf")
                assert not any((prefix / p).exists() for p in
                               ("drive_c/vrclient", "drive_c/openxr", "drive_c/Program Files (x86)/Steam"))
                for arch, folder in (("x86_64", "system32"), ("i386", "syswow64")):
                    contents = (prefix / "drive_c/windows" / folder / "d3d11.dll").read_text()
                    assert contents == arch + (" wine d3d11" if renderer == "wined3d" else " wine/dxvk d3d11")
                guid = namespace["get_replace_reg_value"](str(prefix / "system.reg"),
                                                          r"Software\\Microsoft\\Cryptography", "MachineGuid")
                assert guid is not None and guid != '"builder-template"'
                owned = prefix / "drive_c/keep.txt"
                owned.write_text("preserved caller data")
                compat.setup_prefix()
                assert owned.read_text() == "preserved caller data"
                assert namespace["get_replace_reg_value"](str(prefix / "system.reg"),
                                                           r"Software\\Microsoft\\Cryptography", "MachineGuid") == guid
                tracked = Path(compat.tracked_files_file).read_text()
                assert "drive_c/windows/system32/d3d11.dll" in tracked
                assert "drive_c/windows/syswow64/d3d11.dll" in tracked
                required = (default / "drive_c/windows/system32/d3d11.dll" if renderer == "wined3d"
                            else Path(proton.arch_pe_dir("wine/dxvk", False)) / "d3d11.dll")
                payload = required.read_bytes()
                required.unlink()
                try:
                    compat.setup_prefix()
                except FileNotFoundError:
                    pass
                else:
                    raise ValueError("missing required graphics payload was accepted")
                finally:
                    required.write_bytes(payload)
                assert owned.read_text() == "preserved caller data"


def check():
    with tempfile.TemporaryDirectory(prefix="lvb-redist-source-") as directory:
        source = Path(directory).resolve()
        for name in SOURCES:
            url = ("https://raw.githubusercontent.com/ValveSoftware/Proton/"
                   + RECIPE["proton"]["commit"] + "/" + name)
            target = source / name
            target.parent.mkdir(parents=True, exist_ok=True)
            with urllib.request.urlopen(url, timeout=30) as response:
                data = response.read(1024 * 1024 + 1)
            if len(data) > 1024 * 1024:
                raise ValueError("redist source exceeds check bound")
            target.write_bytes(data)
        if source_state(source) != 0:
            raise ValueError("downloaded source is not the pinned pristine input")
        try:
            check_prefix_composition(source)
        except KeyError as failure:
            if failure.args != ("STEAM_COMPAT_CLIENT_INSTALL_PATH",):
                raise
        else:
            raise ValueError("prefix regression did not detect the original Steam dependency")
        patch_source(source)
        patch_source(source)  # Composition can resume after an acquisition failure.
        check_launch_composition(source)
        check_prefix_composition(source)
        # Execute the pinned x64 post-install recipe with only c27's real file
        # layout. Obsolete wine64 aliases must not be required or cross-copied.
        installed = source / "installed-x64"
        (installed / "bin").mkdir(parents=True)
        (installed / "share/wine").mkdir(parents=True)
        for name in ("wine", "wineserver", "msidb"):
            (installed / "bin" / name).write_text("x86_64 " + name)
        (installed / "share/wine/wine.inf").write_text("same-source metadata")
        distribution = source / "distribution"
        (source / "post-obj").mkdir()
        target = "$(OBJ)/.wine-x86_64-post-build:"
        make_source = (source / "Makefile.in").read_text()
        recipe = make_source[make_source.index(target):make_source.index("$(OBJ)/.wine-aarch64-post-build:")]
        post_makefile = source / "post-install.mk"
        post_makefile.write_text("OBJ := " + str(source / "post-obj") + "\n"
                                "WINE_x86_64_DST := " + str(installed) + "\n"
                                "DST_DIR := " + str(distribution) + "\n"
                                "install-strip = cp $(1) $(2)/\n" + recipe)
        subprocess.run(["make", "--no-print-directory", "-f", str(post_makefile)],
                       cwd=source, env={"PATH": os.defpath}, check=True)
        if (distribution / "bin/wine").read_text() != "x86_64 wine" or (distribution / "lib/wine/i386-unix").exists():
            raise ValueError("native loader post-install layout differs")
        package = source / "package"
        package.mkdir()
        obj = source / "obj"
        obj.mkdir()
        binary = source / "bin"
        binary.mkdir()
        cargo = binary / "cargo"
        cargo.write_text("#!" + sys.executable + "\n"
                         "import os,sys\n"
                         "assert os.getcwd() == os.environ['LVB_EXPECT_PACKAGE']\n"
                         "assert os.environ['LVB_CARGO_TEST'] == 'target-environment'\n"
                         "assert sys.argv[1:3] == ['build', '--frozen']\n"
                         "assert '--target' in sys.argv and 'x86_64-unknown-linux-gnu' in sys.argv\n")
        cargo.chmod(0o755)
        makefile = source / "fixture.mk"
        makefile.write_text("OBJ := " + str(obj) + "\nARCHS := x86_64-unix\n"
                            "PKG_SRC := " + str(package) + "\n"
                            "PKG_x86_64_OBJ := " + str(obj / "target") + "\n"
                            "PKG_x86_64_ENV := LVB_CARGO_TEST=target-environment PATH="
                            + str(binary) + ":/usr/bin:/bin\n"
                            "include " + str(source / "make/rules-cargo.mk") + "\n"
                            "$(eval $(call create-rules-cargo,fixture,PKG,x86_64,unix))\n")
        subprocess.run(["make", "--no-print-directory", "-f", str(makefile),
                        str(obj / ".fixture-x86_64-build")], cwd=source,
                       env={"PATH": os.defpath, "LVB_EXPECT_PACKAGE": str(package)}, check=True)
        drives = source / "prefix/dosdevices"
        drives.mkdir(parents=True)
        (drives / "c:").symlink_to(source, target_is_directory=True)
        (drives / "com1:").symlink_to("/dev/ttyS0")
        (drives / "z:").symlink_to("/definitely-absent-lvb-build-path")
        (drives / "keep.txt").write_text("keep")
        (drives / "keep-link").symlink_to("keep.txt")
        namespace = runpy.run_path(str(source / "default_pfx.py"))
        namespace["fixup_drive_links"](str(drives.parent))
        if sorted(path.name for path in drives.iterdir()) != ["keep-link", "keep.txt"]:
            raise ValueError("generated prefix retains host drive/device links")
        (source / "Makefile.in").write_text("changed")
        try:
            patch_source(source)
        except ValueError:
            pass
        else:
            raise ValueError("modified redist source was accepted")
    print("Pinned source, audio launch/prefix composition, native loader, Cargo and generated-prefix regressions verified")
    runpy.run_path(str(ROOT / "source-companion.py"))["check"]()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--build-root", type=Path)
    mode.add_argument("--prepare-build-source", type=Path,
                      help="Apply pinned Proton integration before the Wine target")
    mode.add_argument("--check-upstream", action="store_true")
    args = parser.parse_args()
    if args.check_upstream:
        check()
    elif args.prepare_build_source:
        verify_commits(args.prepare_build_source.resolve())
        patch_source(args.prepare_build_source.resolve())
    else:
        prepare(args.build_root.resolve())
