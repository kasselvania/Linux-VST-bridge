#!/usr/bin/env python3
"""Close the additional inputs used by the pinned Proton Wine build target."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent
INPUTS = json.loads((ROOT / "build-inputs.json").read_bytes())


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def replace_once(text, before, after):
    if text.count(before) != 1:
        raise ValueError("build source prerequisite differs: " + before)
    return text.replace(before, after, 1)


def prepare(source):
    expected = subprocess.check_output(["git", "rev-parse", "HEAD:piper"],
                                       cwd=source, text=True).strip()
    actual = subprocess.check_output(["git", "rev-parse", "HEAD"],
                                     cwd=source / "piper", text=True).strip()
    if actual != expected or actual != "9d06b74959570772e8bcbe7a3f696664d2421167":
        raise ValueError("Piper source identity differs")
    if subprocess.check_output(["git", "status", "--porcelain=v1"],
                               cwd=source / "piper", text=True):
        raise ValueError("Piper source is not pristine")
    inputs = source / "external-inputs"
    inputs.mkdir()
    for name, item in INPUTS.items():
        target = inputs / name
        with urllib.request.urlopen(item["url"], timeout=60) as response, target.open("xb") as out:
            count = 0
            while block := response.read(1024 * 1024):
                count += len(block)
                if count > item["size"]:
                    raise ValueError("build input exceeds pinned extent: " + name)
                out.write(block)
        if count != item["size"] or sha(target) != item["sha256"]:
            raise ValueError("build input identity differs: " + name)
    phonemize = inputs / "phonemize"
    phonemize.mkdir()
    with zipfile.ZipFile(inputs / "phonemize.zip") as archive:
        for name in archive.namelist():
            if Path(name).is_absolute() or ".." in Path(name).parts:
                raise ValueError("unsafe source archive path")
        archive.extractall(phonemize)
    phonemize = phonemize / "piper-phonemize-5ecbb9b59abcffe03a9775ae82491c9ac5037f4d"
    with tarfile.open(inputs / "onnxruntime.tgz") as archive:
        archive.extractall(inputs, filter="data")
    onnx = inputs / "onnxruntime-linux-x64-1.14.1"
    if not (onnx / "lib/libonnxruntime.so.1.14.1").is_file():
        raise ValueError("ONNXRuntime archive layout differs")
    # Nested CMake consumes verified local inputs; compilation has no network.
    piper_cmake = source / "piper/CMakeLists.txt"
    text = piper_cmake.read_text()
    for name, original in (("fmt.zip", "https://github.com/fmtlib/fmt/archive/refs/tags/${FMT_VERSION}.zip"),
                           ("spdlog.zip", "https://github.com/gabime/spdlog/archive/refs/tags/v${SPDLOG_VERSION}.zip")):
        text = replace_once(text, 'URL "' + original + '"',
                            'URL "/work/proton/external-inputs/' + name + '"\n'
                            '    URL_HASH SHA256=' + INPUTS[name]["sha256"])
    text = replace_once(text, 'URL "https://github.com/shaunren/piper-phonemize/archive/refs/heads/pic.zip"',
                        'SOURCE_DIR "/work/proton/external-inputs/phonemize/' + phonemize.name + '"')
    text = replace_once(text, 'CMAKE_ARGS -DCMAKE_INSTALL_PREFIX:PATH=${PIPER_PHONEMIZE_DIR}',
                        'CMAKE_ARGS -DCMAKE_INSTALL_PREFIX:PATH=${PIPER_PHONEMIZE_DIR}\n'
                        '    CMAKE_ARGS -DONNXRUNTIME_DIR:PATH=/work/proton/external-inputs/' + onnx.name)
    piper_cmake.write_text(text)
    cmake = phonemize / "CMakeLists.txt"
    text = replace_once(cmake.read_text(),
                        'URL "https://github.com/rhasspy/espeak-ng/archive/0f65aa301e0d6bae5e172cc74197d32a6182200f.zip"',
                        'URL "/work/proton/external-inputs/espeak.zip"\n'
                        '        URL_HASH SHA256=' + INPUTS["espeak.zip"]["sha256"])
    cmake.write_text(text)
    for name in ("vk.xml", "video.xml"):
        shutil.copyfile(inputs / name, source / "wine/dlls/winevulkan" / name)
    print("Nested Piper sources, ONNX auxiliary and Wine Vulkan registry inputs verified")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--proton-source", type=Path, required=True)
    args = parser.parse_args()
    prepare(args.proton_source.resolve())
