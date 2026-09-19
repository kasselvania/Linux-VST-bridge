#!/usr/bin/env python3
"""Seal the reviewed exact K8I1 adapter.

The package plan is accepted only as the output embedded in the exact
source-owned compiler result.  Production sealing is additionally disabled in
source until the reviewed compiler result, disposable proof, and final archive
digests are pinned by a later evidence-only promotion.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/"bridge-manager/runtime"))
import kontakt8 as k


def plain(path: pathlib.Path, maximum: int) -> bytes:
    original = path
    metadata = original.lstat()
    if original.is_symlink() or not original.is_file() or metadata.st_nlink != 1:
        raise ValueError("k8i1_package_input")
    path = original.resolve(strict=True)
    data = path.read_bytes()
    after = path.stat()
    if (not data or len(data) > maximum
            or (metadata.st_dev, metadata.st_ino, metadata.st_size, metadata.st_mtime_ns)
            != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)):
        raise ValueError("k8i1_package_input")
    return data


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def main() -> None:
    parser=argparse.ArgumentParser()
    parser.add_argument("--shim",type=pathlib.Path,required=True)
    parser.add_argument("--real-msi",type=pathlib.Path,required=True)
    parser.add_argument("--registry",type=pathlib.Path,required=True)
    parser.add_argument("--setup-watch",type=pathlib.Path,required=True)
    parser.add_argument("--compiler-result",type=pathlib.Path,required=True)
    parser.add_argument("--disposable-proof",type=pathlib.Path,required=True)
    parser.add_argument("--setup",type=pathlib.Path,required=True)
    parser.add_argument("--pristine-msi",type=pathlib.Path,required=True)
    parser.add_argument("--cached-msi",type=pathlib.Path,required=True)
    parser.add_argument("--source",type=pathlib.Path,required=True)
    parser.add_argument("--third-party",type=pathlib.Path,default=ROOT/"tools/k8i1/THIRD_PARTY.md")
    parser.add_argument("--output",type=pathlib.Path,required=True)
    args=parser.parse_args()
    if (k.APPROVED_ADAPTER_SHA256 is None or k.APPROVED_COMPILER_RESULT_SHA256 is None
            or k.APPROVED_DISPOSABLE_PROOF_SHA256 is None):
        raise ValueError("k8i1_production_sealing_not_qualified")
    if args.output.exists():raise ValueError("k8i1_output_exists")
    compiler_data=plain(args.compiler_result,48*1024*1024)
    compiler=k.read_json_bytes(compiler_data,48*1024*1024)
    proof_data=plain(args.disposable_proof,2*1024*1024)
    proof=k.read_json_bytes(proof_data,2*1024*1024)
    if (hashlib.sha256(compiler_data).hexdigest()!=k.APPROVED_COMPILER_RESULT_SHA256
        or hashlib.sha256(proof_data).hexdigest()!=k.APPROVED_DISPOSABLE_PROOF_SHA256
        or compiler.get("schema")!=1 or compiler.get("kind")!="k8i1-package-compiler-result"):
        raise ValueError("k8i1_qualification_binding")
    plan=compiler.get("plan")
    if (not isinstance(plan,dict) or compiler.get("plan_sha256")!=hashlib.sha256(k.canonical(plan)).hexdigest()):
        raise ValueError("k8i1_compiler_result_binding")
    payload={"msi.dll":plain(args.shim,16*1024*1024),
             "msi_lvb_real.dll":plain(args.real_msi,48*1024*1024),
             "k8i1-registry.exe":plain(args.registry,16*1024*1024),
             "k8i1-setup-watch.exe":plain(args.setup_watch,16*1024*1024),
             "package-plan.json":k.canonical(plan),
             "compiler-result.json":compiler_data,
             "disposable-proof.json":proof_data,
             "SOURCE.json":plain(args.source,2*1024*1024),
             "THIRD_PARTY.txt":plain(args.third_party,2*1024*1024),
             "LICENSE.ni-wine-MIT":plain(ROOT/"tools/k8i1/LICENSE.ni-wine-MIT",2*1024*1024)}
    k.validate_disposable_proof(proof,k.APPROVED_COMPILER_RESULT_SHA256,payload)
    if (k.pe_machine(payload["msi.dll"])!=0x14c or k.pe_machine(payload["msi_lvb_real.dll"])!=0x14c
            or k.pe_machine(payload["k8i1-registry.exe"])!=0x8664
            or k.pe_machine(payload["k8i1-setup-watch.exe"])!=0x8664):
        raise ValueError("k8i1_package_architecture")
    source=k.read_json_bytes(payload["SOURCE.json"],2*1024*1024)
    if (set(source)!={"schema","wine_base","patches","msi_spec_sha256","corresponding_source_sha256","build_recipe"}
        or source["schema"]!=1 or source["wine_base"]!=k.WINE_BASE or source["patches"]!=k.WINE_PATCHES
        or source["msi_spec_sha256"]!="5a6085ce66f541d3c52552164ade070129ea05a0f8a8b1ed2c660132366aceb0"
        or not k.HEX64.fullmatch(str(source["corresponding_source_sha256"]))
        or source["build_recipe"]!="exact-proton-wine-msi-32bit-two-upstream-patches-v1"):
        raise ValueError("k8i1_corresponding_source")
    notices=payload["THIRD_PARTY.txt"].decode("utf-8")
    if "MIT" not in notices or "LGPL" not in notices or any(commit not in notices for commit in k.WINE_PATCHES):
        raise ValueError("k8i1_third_party_notice")
    authority={"product":{"name":k.PRODUCT,"version":k.VERSION},
               "setup":{"basename":k.SETUP_BASENAME,"sha256":k.SETUP_SHA256,"size":k.SETUP_SIZE},
               "pristine_msi":{"basename":k.MSI_BASENAME,"sha256":k.PRISTINE_MSI_SHA256,"size":k.PRISTINE_MSI_SIZE}}
    manifest={"schema":1,"kind":"k8i1-exact-package-adapter","product":authority["product"],
              "setup":authority["setup"],"pristine_msi":authority["pristine_msi"],
              "cached_msi":plan.get("cached_msi"),"wine":{"base":k.WINE_BASE,"patches":k.WINE_PATCHES},
              "exports":{"count":296,"ordinal_first":5,"ordinal_last":300,
                         "intercepted":["MsiInstallProductA","MsiInstallProductW"],
                         "spec_sha256":"5a6085ce66f541d3c52552164ade070129ea05a0f8a8b1ed2c660132366aceb0"},
              "artifacts":{},"plan_sha256":hashlib.sha256(k.canonical(plan)).hexdigest(),
              "compiler_result_sha256":hashlib.sha256(compiler_data).hexdigest(),
              "disposable_proof_sha256":hashlib.sha256(proof_data).hexdigest(),
              "source":{"head":git("rev-parse","HEAD"),"tree":git("rev-parse","HEAD^{tree}")}}
    payload["manifest.json"]=b""
    manifest["artifacts"]={name:({"sha256":hashlib.sha256(data).hexdigest(),"size":len(data)}
        if name!="manifest.json" else {"sha256":"0"*64,"size":0}) for name,data in payload.items()}
    k.Adapter._manifest(manifest,{**authority,"wine":{"base":k.WINE_BASE,"patches":k.WINE_PATCHES}})
    k.validate_plan(plan,authority,manifest)
    for path,expected,error in ((args.setup,{"sha256":k.SETUP_SHA256,"size":k.SETUP_SIZE},"k8i1_setup_identity"),
        (args.pristine_msi,{"sha256":k.PRISTINE_MSI_SHA256,"size":k.PRISTINE_MSI_SIZE},"k8i1_pristine_msi_identity"),
        (args.cached_msi,{"sha256":plan["cached_msi"]["sha256"],"size":plan["cached_msi"]["size"]},"k8i1_cached_msi_identity")):
        data=plain(path,2*1024*1024*1024)
        if {"sha256":hashlib.sha256(data).hexdigest(),"size":len(data)}!=expected:raise ValueError(error)
    payload["manifest.json"]=k.canonical(manifest)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=args.output.parent,delete=False) as raw:temporary=pathlib.Path(raw.name)
    try:
        with zipfile.ZipFile(temporary,"w",compression=zipfile.ZIP_STORED) as archive:
            for name in sorted(payload):archive.writestr(name,payload[name])
        temporary.chmod(0o400);temporary.replace(args.output)
    finally:temporary.unlink(missing_ok=True)
    if k.digest(args.output)!=k.APPROVED_ADAPTER_SHA256:
        args.output.unlink(missing_ok=True);raise ValueError("k8i1_adapter_not_qualified")
    print(json.dumps({"schema":1,"output":str(args.output.resolve()),"sha256":k.digest(args.output),
                      "plan_sha256":manifest["plan_sha256"],"source":manifest["source"]},sort_keys=True))


if __name__=="__main__":main()
