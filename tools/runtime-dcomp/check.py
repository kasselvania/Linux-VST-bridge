#!/usr/bin/env python3
"""Check recovered source inputs, optionally applying patches in a temporary copy.

No downloads, compiler, runtime launch, prefix or installed setup access.
The supplied pristine Wine source is read only; only the two named source files
are copied into a temporary directory for Git's patch applicability check.
"""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent


def digest(value):
    return hashlib.sha256(value).hexdigest()


def check(wine_source=None):
    recipe = json.loads((ROOT / "recipe.json").read_bytes())
    for patch in recipe["patches"]:
        if digest((ROOT / patch["path"]).read_bytes()) != patch["sha256"]:
            raise ValueError("Recovered patch changed: " + patch["path"])
    bg1 = recipe["bg1"]
    template = (ROOT / bg1["launcher_template"]).read_bytes()
    if digest(template) != bg1["launcher_template_sha256"]:
        raise ValueError("Recovered launcher template changed")
    ast.parse(template)
    if template.count(b"@COMPONENTS_SHA256@") != 1:
        raise ValueError("Historical launcher substitution differs")
    rendered = template.replace(b"@COMPONENTS_SHA256@", bg1["components_manifest_sha256"].encode())
    if digest(rendered) != bg1["rendered_launcher_sha256"]:
        raise ValueError("Historical launcher does not reproduce the retained entry")
    marker = json.dumps(bg1["native_command_session"], sort_keys=True, separators=(",", ":")).encode() + b"\n"
    if digest(marker) != bg1["native_command_session_sha256"]:
        raise ValueError("Historical command-session marker differs")
    if wine_source is not None:
        with tempfile.TemporaryDirectory(prefix="lvb-dcomp-source-") as directory:
            stage = Path(directory)
            for patch in recipe["patches"]:
                source = (wine_source / patch["source"]).read_bytes()
                if digest(source) != patch["before_sha256"]:
                    raise ValueError("Pristine Wine source differs: " + patch["source"])
                target = stage / patch["source"]
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(source)
            for patch in recipe["patches"]:
                subprocess.run(["git", "apply", "--check", str(ROOT / patch["path"])],
                               cwd=stage, check=True, timeout=10, capture_output=True)
                subprocess.run(["git", "apply", str(ROOT / patch["path"])],
                               cwd=stage, check=True, timeout=10, capture_output=True)
                if digest((stage / patch["source"]).read_bytes()) != patch["after_sha256"]:
                    raise ValueError("Patched source differs from retained build: " + patch["source"])
    return recipe


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wine-source", type=Path, help="Pristine pinned Wine source; read only")
    arguments = parser.parse_args()
    check(arguments.wine_source)
    print("Recovered patch, launcher and marker identities match." +
          (" Both patches reproduce the retained source files." if arguments.wine_source else ""))
