#!/usr/bin/env python3
"""Build existing UIA and signed/tampered certificate cases as standalone PEs.

Only pinned LGPL source is acquired; no Wine binary or vendor payload is used.
--prepared-wine reuses the same verified source from the common engine build.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import urllib.request

HERE = Path(__file__).resolve().parent
RECIPE = json.loads((HERE / "source.json").read_bytes())
SUPPORT = {
    "include/wine/test.h": "2a8b0cb6dcf26a4affc21ffa6423e2ae79f5c2b2fa15a0e016d5b73d663bb81d",
    "include/wine/debug.h": "6b511fb8c5cd6374fef611afc1ed42839fb303c946e05f9855a2e5f9e30bb36a",
    "COPYING.LIB": "e237fa56668030e928551ddd60f05df5fe957f75eab874bbd017e085ed722e7c",
}
MESSAGE = "dlls/crypt32/tests/msg.c"


def checked(path, expected):
    if path.is_symlink() or not path.is_file():
        raise ValueError("Fixture source missing or linked: " + str(path))
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected:
        raise ValueError("Fixture source identity differs: " + str(path))
    return data


def acquire(stage, relative, expected, prepared):
    target = stage / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    if prepared is not None:
        target.write_bytes(checked(prepared / relative, expected))
    else:
        url = ("https://raw.githubusercontent.com/giang17/wine/"
               + RECIPE["wine"]["commit"] + "/" + relative)
        with urllib.request.urlopen(url, timeout=30) as response:
            data = response.read(2 * 1024 * 1024 + 1)
        if len(data) > 2 * 1024 * 1024:
            raise ValueError("Fixture source exceeds acquisition bound")
        target.write_bytes(data)
        checked(target, expected)
    return target


def sources(stage, prepared):
    for relative, expected in SUPPORT.items():
        acquire(stage, relative, expected, prepared)
    state = "after" if prepared is not None else "before"
    message = acquire(stage, MESSAGE, RECIPE["sources"][MESSAGE][state], prepared)
    if prepared is None:
        patch = next(item for item in RECIPE["patches"]
                     if item["path"] == "runtime/patches/dcomp-crypt32-regression.patch")
        path = HERE.parent / patch["path"]
        checked(path, patch["sha256"])
        subprocess.run(["git", "apply", "--check", str(path)], cwd=stage, check=True, timeout=30)
        subprocess.run(["git", "apply", str(path)], cwd=stage, check=True, timeout=30)
    text = checked(message, RECIPE["sources"][MESSAGE]["after"]).decode()
    start = text.index("/* A signed message whose signed attributes")
    end = text.index("\nSTART_TEST(msg)", start)
    # Retain the original LGPL/copyright header and complete positive/negative
    # test body. No test result or assertion is removed from the selected case.
    header = text[:text.index("*/") + 2]
    signature = stage / "signature_order.c"
    signature.write_text(header + "\n/* Isolated unchanged signed/tampered test case, 2026-10-16. */\n"
                         "#include <windows.h>\n#include <wincrypt.h>\n"
                         "#include <stdlib.h>\n#include <string.h>\n#include \"wine/test.h\"\n"
                         + text[start:end]
                         + "\nSTART_TEST(signature_order) { test_verify_unsorted_auth_attrs(); }\n")
    uia = HERE.parent / "runtime-uia-guard/guard_provider.c"
    fixture = next(item for item in RECIPE["fixtures"]
                   if item["path"] == "runtime-uia-guard/guard_provider.c")
    checked(uia, fixture["sha256"])
    shutil.copyfile(uia, stage / "guard_provider.c")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--prepared-wine", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    source_output = args.output / "source"
    if source_output.exists() or source_output.is_symlink():
        raise ValueError("Fixture source output already exists")
    with tempfile.TemporaryDirectory(prefix="fixture-source-", dir=args.output) as temporary:
        stage = Path(temporary)
        sources(stage, args.prepared_wine)
        for arch in ("i686", "x86_64"):
            for name, libraries in (
                    ("guard_provider", ("uiautomationcore", "ole32", "oleaut32", "uuid")),
                    ("signature_order", ("crypt32",))):
                target = args.output / (name + "-" + arch + ".exe")
                if target.exists() or target.is_symlink():
                    raise ValueError("Fixture output already exists: " + str(target))
                # The standalone tests export no Wine debug library. MinGW's
                # Windows headers omit Wine's internal DECLSPEC_EXPORT spelling.
                subprocess.run([arch + "-w64-mingw32-gcc", "-O2", "-static", "-DSTANDALONE",
                                "-DDECLSPEC_EXPORT=",
                                "-I" + str(stage / "include"), str(stage / (name + ".c")),
                                "-o", str(target), *["-l" + item for item in libraries]],
                               check=True, timeout=120)
                print(target.name + " " + hashlib.sha256(target.read_bytes()).hexdigest())
        # Accompany the LGPL standalone harness binaries with their exact
        # corresponding source, original notices, licence and rebuild commands.
        source_output.mkdir()
        for name in (*SUPPORT, "signature_order.c", "guard_provider.c"):
            target = source_output / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(stage / name, target)
        (source_output / "README.txt").write_text(
            "UIA and certificate regression fixture source\n"
            "Wine support headers and certificate case: " + RECIPE["wine"]["commit"] + "\n"
            "UIA case: Linux VST Bridge contributors, tools/runtime-uia-guard/guard_provider.c\n"
            "These sources and the included harness are LGPL 2.1 or later; see COPYING.LIB\n"
            "and the retained copyright/licence headers. The certificate case is extracted\n"
            "unchanged from the prepared Wine test, with a standalone entry point.\n\n"
            "Rebuild from this directory with MinGW Windows headers/import libraries:\n"
            "for arch in i686 x86_64; do\n"
            "  $arch-w64-mingw32-gcc -O2 -static -DSTANDALONE -DDECLSPEC_EXPORT= -Iinclude "
            "guard_provider.c -o guard_provider-$arch.exe -luiautomationcore -lole32 -loleaut32 -luuid\n"
            "  $arch-w64-mingw32-gcc -O2 -static -DSTANDALONE -DDECLSPEC_EXPORT= -Iinclude "
            "signature_order.c -o signature_order-$arch.exe -lcrypt32\n"
            "done\n")


if __name__ == "__main__":
    main()
