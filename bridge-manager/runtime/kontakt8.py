"""K8I1 exact Kontakt 8 package transaction.

The module contains no discovery policy for arbitrary installers.  It accepts
only an immutable manager-bound adapter and its exact 8.13.1 manifest, then
applies a complete precomputed plan to the InstallAware OFFLINE payload.  A
transaction is successful only after byte-for-byte destination and product
state verification; every intermediate failure enters rollback.
"""
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import re
import shutil
import stat
import threading
import zipfile
from dataclasses import dataclass
from typing import Protocol

PRODUCT = "Kontakt 8 Player"
VERSION = "8.13.1"
SETUP_BASENAME = "Kontakt 8 Setup PC.exe"
SETUP_SHA256 = "5f7f26389337f3cd282319589549ade23f860023e6cb9a99f0f45baf67f7215d"
SETUP_SIZE = 1_188_804_208
MSI_BASENAME = "Kontakt 8 Setup PC.msi"
PRISTINE_MSI_SHA256 = "0f1caaac78c5ae718d849c3a2f5b9c7f1161b4326edf3f21dd7af2f01d95c929"
PRISTINE_MSI_SIZE = 4_104_192
WINE_BASE = "dc26e61847081a1b5cb0733dc30feba6ee575482"
WINE_PATCHES = [
    "24bbf46c6f055077df428341595873fe23475a60",
    "e0130972d5ffe578a4a25d75f4c1d0229c880a8c",
]
APPROVED_ADAPTER_SHA256 = None
APPROVED_COMPILER_RESULT_SHA256 = None
APPROVED_DISPOSABLE_PROOF_SHA256 = None
REQUIRED_ARCHIVE = {
    "manifest.json",
    "msi.dll",
    "msi_lvb_real.dll",
    "package-plan.json",
    "THIRD_PARTY.txt",
    "LICENSE.ni-wine-MIT",
    "SOURCE.json",
    "k8i1-registry.exe",
    "k8i1-setup-watch.exe",
    "compiler-result.json",
    "disposable-proof.json",
}
REQUIRED_TABLES = {
    "Directory", "Component", "File", "Feature", "FeatureComponents", "Registry",
    "InstallExecuteSequence", "CustomAction", "CreateFolder", "RemoveFile", "Shortcut",
}
COMPILER_REQUIRED_TABLES = {
    "_Tables", "Directory", "Component", "File", "Feature", "FeatureComponents",
    "Registry", "InstallExecuteSequence", "CustomAction",
}
MUTATION_TABLES = {
    "ServiceInstall", "ServiceControl", "Class", "TypeLib", "Environment", "IniFile",
    "MoveFiles", "DuplicateFile", "SelfReg", "ODBCDataSource", "ODBCDriver", "ODBCTranslator",
    "PublishComponent", "ProgId", "Extension", "MIME", "Verb",
}
HEX64 = re.compile(r"[0-9a-f]{64}\Z")
HEX32 = re.compile(r"[0-9a-f]{32}\Z")
MSI_PROPERTY = re.compile(r"[A-Z_][A-Z0-9_]{0,71}\Z")
ALLOWED_DESTINATIONS = (
    "Program Files/Native Instruments/Kontakt 8/",
    "Program Files/Common Files/Native Instruments/Kontakt 8/",
    "Program Files/Common Files/VST3/Kontakt 8.vst3/",
)
KONTAKT_REGISTRY = {
    "InstallDir": ("REG_SZ", "C:\\Program Files\\Native Instruments\\Kontakt 8\\"),
    "ContentDir": ("REG_SZ", "C:\\Program Files\\Common Files\\Native Instruments\\Kontakt 8\\"),
    "ContentVersion": ("REG_SZ", "4.0"),
    "InstallVST364Dir": ("REG_SZ", "C:\\Program Files\\Common Files\\VST3\\"),
}


def canonical(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def digest(path: pathlib.Path) -> str:
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def selected_source_tree_sha256(files: list[dict]) -> str:
    rows=[{"source":str(safe_relative(row["source"])),"sha256":row["sha256"],"size":row["size"]} for row in files]
    return digest_bytes(canonical(sorted(rows,key=lambda row:row["source"].casefold())))


def normalize_msi_properties(raw: str) -> dict[str, str]:
    tokens=[];current=[];quoted=False
    for char in raw.strip():
        if char=='"':quoted=not quoted;continue
        if char.isspace() and not quoted:
            if current:tokens.append("".join(current));current=[]
        else:current.append(char)
    if quoted:raise ValueError("k8i1_properties_quote")
    if current:tokens.append("".join(current))
    result={}
    for token in tokens:
        name,separator,value=token.partition("=");name=name.upper()
        if not separator or not MSI_PROPERTY.fullmatch(name) or name in result or "\0" in value:
            raise ValueError("k8i1_properties")
        result[name]=value
    return {name:result[name] for name in sorted(result)}


def unique(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("k8i1_duplicate_key")
        value[key] = item
    return value


def read_json_bytes(data: bytes, maximum: int):
    if not data or len(data) > maximum:
        raise ValueError("k8i1_record_extent")
    return json.loads(data, object_pairs_hook=unique)


def safe_relative(value: str) -> pathlib.PurePosixPath:
    if not isinstance(value, str) or not value or "\\" in value or "\0" in value:
        raise ValueError("k8i1_relative_path")
    path = pathlib.PurePosixPath(value)
    if path.is_absolute() or any(part in ("", ".", "..") for part in path.parts):
        raise ValueError("k8i1_relative_path")
    return path


def exact_file(path: pathlib.Path, *, maximum: int | None = None) -> os.stat_result:
    before = path.lstat()
    if (not stat.S_ISREG(before.st_mode) or before.st_uid != os.getuid() or before.st_nlink != 1
            or path.resolve() != path):
        raise ValueError("k8i1_file_identity")
    if maximum is not None and before.st_size > maximum:
        raise ValueError("k8i1_file_extent")
    return before


def pe_machine(data: bytes) -> int:
    if len(data) < 64 or data[:2] != b"MZ":
        raise ValueError("k8i1_pe_identity")
    offset = int.from_bytes(data[60:64], "little")
    if offset > len(data) - 6 or data[offset:offset + 4] != b"PE\0\0":
        raise ValueError("k8i1_pe_identity")
    return int.from_bytes(data[offset + 4:offset + 6], "little")


def validate_session(spec: dict, supplied: dict | None) -> dict | None:
    expected = spec.get("kontakt8_session")
    if expected is None:
        if supplied is not None:
            raise ValueError("k8i1_authority_unexpected")
        return None
    if supplied != expected:
        raise ValueError("k8i1_authority_lost")
    fields = {"schema", "authority", "application", "software_sha256", "environment", "prefix",
              "adapter", "product", "setup", "pristine_msi", "intercepts", "request_limit",
              "timeout_seconds", "wine", "selected_features", "omitted_features", "licensing_state"}
    if (not isinstance(expected, dict) or set(expected) != fields or expected["schema"] != 1
            or expected["authority"] != "exact_kontakt8_8_13_1_msi_diversion_and_verified_deployment"
            or expected["application"] != spec["application_identity"]
            or expected["software_sha256"] != spec["software_sha256"]
            or expected["environment"] != spec["application"]["environment"]["id"]
            or expected["product"] != {"name": PRODUCT, "version": VERSION}
            or expected["setup"] != {"basename": SETUP_BASENAME, "sha256": SETUP_SHA256, "size": SETUP_SIZE}
            or expected["pristine_msi"] != {"basename": MSI_BASENAME, "sha256": PRISTINE_MSI_SHA256,
                                             "size": PRISTINE_MSI_SIZE}
            or expected["intercepts"] != ["MsiInstallProductA", "MsiInstallProductW"]
            or expected["request_limit"] != 1 or expected["timeout_seconds"] != 7200
            or expected["wine"] != {"base": WINE_BASE, "patches": WINE_PATCHES}
            or expected["selected_features"] != ["standalone", "vst3", "native_instruments_common"]
            or expected["omitted_features"] != ["aax"]
            or expected["licensing_state"] != "never_manufactured"):
        raise ValueError("k8i1_authority_schema")
    prefix = pathlib.Path(spec["application"]["environment"]["root"]) / "compatdata/pfx"
    md = prefix.lstat()
    bound = expected["prefix"]
    if (not stat.S_ISDIR(md.st_mode) or prefix.resolve() != prefix
            or bound != {"path_sha256": digest_bytes(os.fsencode(prefix)), "device": md.st_dev, "inode": md.st_ino}):
        raise ValueError("k8i1_prefix_changed")
    adapter = expected["adapter"]
    if (not isinstance(adapter, dict) or set(adapter) != {"path", "sha256"}
            or not HEX64.fullmatch(str(adapter["sha256"]))
            or APPROVED_ADAPTER_SHA256 is None
            or adapter["sha256"] != APPROVED_ADAPTER_SHA256):
        raise ValueError("k8i1_adapter_identity")
    path = pathlib.Path(adapter["path"])
    exact_file(path, maximum=64 * 1024 * 1024)
    if path.name != "kontakt8-adapter.zip" or digest(path) != adapter["sha256"]:
        raise ValueError("k8i1_adapter_identity")
    return json.loads(json.dumps(expected))


def validate_disposable_proof(proof: dict, compiler_sha256: str, artifacts: dict[str, bytes]) -> None:
    fields={"schema","kind","compiler_result_sha256","candidate","success","forced_failure"}
    if (not isinstance(proof,dict) or set(proof)!=fields or proof["schema"]!=1
            or proof["kind"]!="k8i1-disposable-prefix-proof"
            or proof["compiler_result_sha256"]!=compiler_sha256):
        raise ValueError("k8i1_disposable_proof_binding")
    expected_candidate={name:digest_bytes(artifacts[name]) for name in
        ("msi.dll","msi_lvb_real.dll","k8i1-registry.exe","k8i1-setup-watch.exe","SOURCE.json")}
    if proof["candidate"]!=expected_candidate:raise ValueError("k8i1_disposable_proof_candidate")
    if proof["success"]!={"rewritten_tables":{"_Tables":True,"Directory":True,"File":True},
            "intercept_count":1,"setup_exit":0,"payload_verified":True,"post_setup_state_verified":True,
            "native_access_recognition_after_refresh":True,"native_access_recognition_after_cold_reopen":True,
            "standalone_launch":True,"vst3_enumerated":True,"cleanup_confirmed":True}:
        raise ValueError("k8i1_disposable_proof_success")
    if proof["forced_failure"]!={"intercept_count":1,"setup_exit":1603,"rollback_verified":True,
            "payload_absent":True,"product_record_absent":True,"retryable":True}:
        raise ValueError("k8i1_disposable_proof_failure")


@dataclass(frozen=True)
class Adapter:
    root: pathlib.Path
    manifest: dict
    plan: dict

    @classmethod
    def extract(cls, authority: dict, directory: pathlib.Path) -> "Adapter":
        archive = pathlib.Path(authority["adapter"]["path"])
        exact_file(archive, maximum=64 * 1024 * 1024)
        if digest(archive) != authority["adapter"]["sha256"]:
            raise ValueError("k8i1_adapter_changed")
        root = directory / "k8i1-adapter.private"
        root.mkdir(mode=0o700)
        with zipfile.ZipFile(archive) as package:
            infos = package.infolist()
            names = [item.filename for item in infos]
            if len(names) != len(set(names)) or set(names) != REQUIRED_ARCHIVE:
                raise ValueError("k8i1_adapter_archive_schema")
            for item in infos:
                if safe_relative(item.filename).parent != pathlib.PurePosixPath(".") or item.is_dir() or item.file_size > 48 * 1024 * 1024:
                    raise ValueError("k8i1_adapter_archive_entry")
            manifest_data = package.read("manifest.json")
            manifest = read_json_bytes(manifest_data, 2 * 1024 * 1024)
            cls._manifest(manifest, authority)
            for item in infos:
                data = package.read(item)
                witness = manifest["artifacts"][item.filename]
                if item.filename == "manifest.json":
                    if witness != {"sha256": "0" * 64, "size": 0}:
                        raise ValueError("k8i1_manifest_self_witness")
                elif witness != {"sha256": digest_bytes(data), "size": len(data)}:
                    raise ValueError("k8i1_adapter_artifact_identity")
                target = root / item.filename
                with target.open("xb") as handle:
                    handle.write(data); handle.flush(); os.fsync(handle.fileno())
                target.chmod(0o400)
        if pe_machine((root / "msi.dll").read_bytes()) != 0x14c or pe_machine((root / "msi_lvb_real.dll").read_bytes()) != 0x14c:
            raise ValueError("k8i1_adapter_architecture")
        if pe_machine((root / "k8i1-registry.exe").read_bytes()) != 0x8664 or pe_machine((root / "k8i1-setup-watch.exe").read_bytes()) != 0x8664:
            raise ValueError("k8i1_adapter_architecture")
        compiler = read_json_bytes((root / "compiler-result.json").read_bytes(), 48 * 1024 * 1024)
        proof = read_json_bytes((root / "disposable-proof.json").read_bytes(), 2 * 1024 * 1024)
        plan = read_json_bytes((root / "package-plan.json").read_bytes(), 48 * 1024 * 1024)
        if (compiler.get("schema") != 1 or compiler.get("kind") != "k8i1-package-compiler-result"
                or compiler.get("plan") != plan
                or compiler.get("plan_sha256") != digest_bytes(canonical(plan))
                or digest_bytes(canonical(compiler)) != manifest["compiler_result_sha256"]):
            raise ValueError("k8i1_compiler_result_binding")
        if digest_bytes(canonical(proof)) != manifest["disposable_proof_sha256"]:
            raise ValueError("k8i1_disposable_proof_binding")
        validate_disposable_proof(proof,manifest["compiler_result_sha256"],
            {name:(root/name).read_bytes() for name in
             ("msi.dll","msi_lvb_real.dll","k8i1-registry.exe","k8i1-setup-watch.exe","SOURCE.json")})
        validate_plan(plan, authority, manifest)
        return cls(root, manifest, plan)

    @staticmethod
    def _manifest(value: dict, authority: dict) -> None:
        fields = {"schema", "kind", "product", "setup", "pristine_msi", "cached_msi", "wine", "exports",
                  "artifacts", "plan_sha256", "compiler_result_sha256", "disposable_proof_sha256", "source"}
        if (not isinstance(value, dict) or set(value) != fields or value["schema"] != 1
                or value["kind"] != "k8i1-exact-package-adapter"
                or value["product"] != authority["product"] or value["setup"] != authority["setup"]
                or value["pristine_msi"] != authority["pristine_msi"]
                or value["wine"] != authority["wine"]
                or value["exports"] != {"count": 296, "ordinal_first": 5, "ordinal_last": 300,
                                          "intercepted": ["MsiInstallProductA", "MsiInstallProductW"],
                                          "spec_sha256": "5a6085ce66f541d3c52552164ade070129ea05a0f8a8b1ed2c660132366aceb0"}
                or set(value["artifacts"]) != REQUIRED_ARCHIVE
                or not HEX64.fullmatch(str(value["plan_sha256"]))
                or not HEX64.fullmatch(str(value["compiler_result_sha256"]))
                or not HEX64.fullmatch(str(value["disposable_proof_sha256"]))
                or APPROVED_COMPILER_RESULT_SHA256 is None
                or APPROVED_DISPOSABLE_PROOF_SHA256 is None
                or value["compiler_result_sha256"] != APPROVED_COMPILER_RESULT_SHA256
                or value["disposable_proof_sha256"] != APPROVED_DISPOSABLE_PROOF_SHA256):
            raise ValueError("k8i1_adapter_manifest")
        cached=value["cached_msi"]
        if (not isinstance(cached,dict) or set(cached)!={"basename","sha256","size","product_code"}
            or cached["basename"]!=MSI_BASENAME or not HEX64.fullmatch(str(cached["sha256"]))
            or type(cached["size"]) is not int or not 0<cached["size"]<=64*1024*1024
            or not re.fullmatch(r"\{[0-9A-F]{8}(?:-[0-9A-F]{4}){3}-[0-9A-F]{12}\}",str(cached["product_code"]))):
            raise ValueError("k8i1_cached_msi_manifest")
        # A self-digest cannot be represented without a detached envelope. The
        # installed Software archive digest binds manifest.json; the sentinel
        # makes that distinction explicit instead of pretending self-proof.
        if value["artifacts"]["manifest.json"] != {"sha256": "0" * 64, "size": 0}:
            raise ValueError("k8i1_manifest_self_witness")


def validate_plan(plan: dict, authority: dict, manifest: dict) -> None:
    fields = {"schema", "product", "setup", "pristine_msi", "cached_msi", "properties", "tables", "files",
              "state_files", "registry", "custom_actions", "mutations", "omissions", "source_tree_sha256"}
    if (not isinstance(plan, dict) or set(plan) != fields or plan["schema"] != 2
            or plan["product"] != authority["product"] or plan["setup"] != authority["setup"]
            or plan["pristine_msi"] != authority["pristine_msi"] or plan["cached_msi"] != manifest["cached_msi"]
            or digest_bytes(canonical(plan)) != manifest["plan_sha256"]):
        raise ValueError("k8i1_plan_binding")
    properties = plan["properties"]
    if (not isinstance(properties, dict) or set(properties) != {"observed", "effective", "sha256"}
            or not isinstance(properties["observed"], dict) or not isinstance(properties["effective"], dict)
            or properties["sha256"] != digest_bytes(canonical(properties["observed"]))
            or any(not isinstance(name, str) or not isinstance(value, str)
                   for mapping in (properties["observed"],properties["effective"]) for name,value in mapping.items())
            or any(properties["effective"].get(name)!=value for name,value in properties["observed"].items())):
        raise ValueError("k8i1_plan_properties")
    tables = plan["tables"]
    if (not isinstance(tables, dict) or set(tables) != {"roster", "roster_sha256", "projection_sha256", "projections"}
            or not HEX64.fullmatch(str(tables["roster_sha256"]))
            or not HEX64.fullmatch(str(tables["projection_sha256"]))
            or not isinstance(tables["roster"], list) or not isinstance(tables["projections"], list)
            or len(tables["roster"]) != len(tables["projections"]) or not tables["roster"]):
        raise ValueError("k8i1_plan_tables")
    roster_names = []
    projection_names = []
    for row in tables["roster"]:
        if (not isinstance(row, dict) or set(row) != {"name", "rows", "disposition"}
                or not isinstance(row["name"], str) or type(row["rows"]) is not int or row["rows"] < 0
                or row["disposition"] not in ("derived", "classified", "required_empty", "observed_non_deployment")
                or row["disposition"] == "required_empty" and row["rows"] != 0):
            raise ValueError("k8i1_plan_tables")
        roster_names.append(row["name"])
    for row in tables["projections"]:
        if (not isinstance(row, dict) or set(row) != {"name", "sha256", "size", "rows", "columns"}
                or not isinstance(row["name"], str) or not HEX64.fullmatch(str(row["sha256"]))
                or type(row["size"]) is not int or row["size"] <= 0
                or type(row["rows"]) is not int or row["rows"] < 0
                or not isinstance(row["columns"], list) or not row["columns"]):
            raise ValueError("k8i1_plan_tables")
        projection_names.append(row["name"])
    if (roster_names != sorted(set(roster_names)) or projection_names != roster_names
            or not COMPILER_REQUIRED_TABLES <= set(roster_names)
            or tables["projection_sha256"] != digest_bytes(canonical(tables["projections"]))):
        raise ValueError("k8i1_plan_tables")
    files = plan["files"]
    if not isinstance(files, list) or not files or len(files) > 20000:
        raise ValueError("k8i1_plan_files")
    sources, destinations, selected = set(), set(), set()
    total = 0
    for row in files:
        if (not isinstance(row, dict) or set(row) != {"source", "destination", "sha256", "size", "file_row",
                "component", "directory", "feature", "feature_component_row", "class"}
                or row["class"] not in ("standalone", "vst3", "native_instruments_common")
                or not HEX64.fullmatch(str(row["sha256"])) or type(row["size"]) is not int or row["size"] < 0
                or not isinstance(row["file_row"], str) or not row["file_row"]
                or not isinstance(row["component"], str) or not row["component"]
                or not isinstance(row["directory"], str) or not row["directory"]
                or not isinstance(row["feature"], str) or not row["feature"]
                or row["feature_component_row"] != f'{row["feature"]}->{row["component"]}'):
            raise ValueError("k8i1_plan_file")
        source, destination = str(safe_relative(row["source"])), str(safe_relative(row["destination"]))
        if not any(destination.startswith(root) for root in ALLOWED_DESTINATIONS):
            raise ValueError("k8i1_plan_destination_scope")
        source_key, destination_key = source.casefold(), destination.casefold()
        if source_key in sources or destination_key in destinations:
            raise ValueError("k8i1_plan_file_duplicate")
        sources.add(source_key); destinations.add(destination_key); selected.add(row["class"]); total += row["size"]
        if total > 8 * 1024 * 1024 * 1024:
            raise ValueError("k8i1_plan_extent")
    if selected != {"standalone", "vst3", "native_instruments_common"}:
        raise ValueError("k8i1_plan_features")
    required_destinations={value.casefold() for value in {
        "Program Files/Native Instruments/Kontakt 8/Kontakt 8.exe",
        "Program Files/Common Files/VST3/Kontakt 8.vst3/Contents/x86_64-win/Kontakt 8.vst3",
    }}
    if not required_destinations<=destinations:
        raise ValueError("k8i1_plan_required_payload")
    if plan["source_tree_sha256"]!=selected_source_tree_sha256(files):
        raise ValueError("k8i1_plan_source_tree")
    if len(plan["state_files"]) != 1:
        raise ValueError("k8i1_plan_state_file")
    for state in plan["state_files"]:
        if (not isinstance(state, dict) or set(state) !=
                {"destination", "content_utf8", "sha256", "size", "class", "derivation"}
                or state["class"] != "native_instruments_product_record"
                or state["derivation"] != {"registry_names":["ContentDir","ContentVersion","InstallDir"]}):
            raise ValueError("k8i1_plan_state_file")
        data = state["content_utf8"].encode()
        if state["size"] != len(data) or state["sha256"] != digest_bytes(data):
            raise ValueError("k8i1_plan_state_file")
        destination = str(safe_relative(state["destination"]))
        if destination != "users/Public/Documents/Native Instruments/installed_products/Kontakt 8.json":
            raise ValueError("k8i1_plan_state_file")
        try:
            product_state = json.loads(state["content_utf8"], object_pairs_hook=unique)
        except (TypeError, ValueError, json.JSONDecodeError) as exc:
            raise ValueError("k8i1_plan_state_file") from exc
        if (set(product_state) != {"ContentDir", "ContentVersion", "InstallDir"}
                or product_state != {name:KONTAKT_REGISTRY[name][1]
                    for name in ("ContentDir","ContentVersion","InstallDir")}):
            raise ValueError("k8i1_plan_state_file")
        if destination.casefold() in destinations:
            raise ValueError("k8i1_plan_file_duplicate")
        destinations.add(destination.casefold())
    registry_names = set()
    for row in plan["registry"]:
        if (not isinstance(row, dict) or set(row) != {"root", "key", "name", "type", "value", "source_row", "component"}
                or row["root"] != "HKLM" or row["type"] != "REG_SZ"
                or row["key"] != r"Software\Native Instruments\Kontakt 8"
                or not isinstance(row["name"], str) or not row["name"] or not isinstance(row["source_row"], str)
                or not row["source_row"] or not isinstance(row["component"], str) or not row["component"]):
            raise ValueError("k8i1_plan_registry")
        if ((row["type"]=="REG_SZ" and not isinstance(row["value"],str))
            or (row["type"]=="REG_DWORD" and (type(row["value"]) is not int or not 0<=row["value"]<=0xffffffff))):
            raise ValueError("k8i1_plan_registry")
        key = (row["root"], row["key"], row["name"])
        if key in registry_names:
            raise ValueError("k8i1_plan_registry_duplicate")
        registry_names.add(key)
        if KONTAKT_REGISTRY.get(row["name"]) != (row["type"], row["value"]):
            raise ValueError("k8i1_plan_registry")
    if {r["name"] for r in plan["registry"]} != set(KONTAKT_REGISTRY):
        raise ValueError("k8i1_plan_registry_incomplete")
    custom_count = next(row["rows"] for row in tables["roster"] if row["name"] == "CustomAction")
    if not isinstance(plan["custom_actions"],list) or len(plan["custom_actions"])!=custom_count:
        raise ValueError("k8i1_plan_custom_action")
    seen_actions=set()
    for row in plan["custom_actions"]:
        fields=("action","type","source","target","sequence","condition")
        if (not isinstance(row,dict) or set(row)!=set(fields)|{"row_sha256","disposition"}
                or not isinstance(row["action"],str) or not row["action"] or row["action"] in seen_actions
                or type(row["type"]) is not int or type(row["sequence"]) is not int
                or any(not isinstance(row[name],str) for name in ("source","target","condition"))
                or row["disposition"]!="omitted_by_exact_payload_deployment"
                or row["row_sha256"]!=digest_bytes(canonical({name:row[name] for name in fields}))):
            raise ValueError("k8i1_plan_custom_action")
        seen_actions.add(row["action"])
    mutations=plan["mutations"]
    if not isinstance(mutations,dict) or set(mutations)!={"create_folders","remove_files","shortcuts"}:
        raise ValueError("k8i1_plan_mutations")
    for row in mutations["create_folders"]:
        if (not isinstance(row,dict) or set(row)!={"directory","component","selected","disposition"}
                or not isinstance(row["directory"],str) or not row["directory"]
                or not isinstance(row["component"],str) or not row["component"]
                or type(row["selected"]) is not bool
                or row["disposition"]!=("created_by_selected_file_plan" if row["selected"] else "nonselected_component")):
            raise ValueError("k8i1_plan_mutations")
    for row in mutations["remove_files"]:
        if (not isinstance(row,dict) or set(row)!={"FileKey","Component_","FileName","DirProperty","InstallMode","disposition"}
                or row["disposition"]!="nonselected_component"
                or any(not isinstance(row[name],str) for name in row if name!="disposition")):
            raise ValueError("k8i1_plan_mutations")
    for row in mutations["shortcuts"]:
        if (not isinstance(row,dict) or set(row)!={"Shortcut","Directory_","Name","Component_","Target","Arguments",
                "Description","Hotkey","Icon_","IconIndex","ShowCmd","WkDir","disposition"}
                or row["disposition"] not in ("omitted_nonessential_shortcut","nonselected_component")
                or any(not isinstance(row[name],str) for name in row if name!="disposition")):
            raise ValueError("k8i1_plan_mutations")
    for table,key in (("CreateFolder","create_folders"),("RemoveFile","remove_files"),("Shortcut","shortcuts")):
        count=next((row["rows"] for row in tables["roster"] if row["name"]==table),0)
        if count!=len(mutations[key]):raise ValueError("k8i1_plan_mutations")
    if not isinstance(plan["omissions"], list):
        raise ValueError("k8i1_plan_omissions")
    for omission in plan["omissions"]:
        if (not isinstance(omission, dict) or omission.get("disposition") not in
                ("aax_not_selected", "nonselected_component")):
            raise ValueError("k8i1_plan_omissions")


class RegistryPort(Protocol):
    def snapshot(self, rows: list[dict]) -> object: ...
    def apply(self, rows: list[dict]) -> None: ...
    def restore(self, snapshot: object) -> None: ...
    def verify(self, rows: list[dict]) -> bool: ...
    def verify_snapshot(self, snapshot: object) -> bool: ...
    def begin_rollback(self) -> None: ...
    def end_rollback(self) -> None: ...


class MemoryRegistry:
    """Deterministic production-law fixture; production supplies a runtime port."""
    def __init__(self, values: dict | None = None, fail_apply: bool = False):
        self.values = dict(values or {}); self.fail_apply = fail_apply
    @staticmethod
    def key(row): return row["root"], row["key"], row["name"]
    def snapshot(self, rows):
        return [{"row": RuntimeRegistry._read_row(row),
                 "prior": ({"present": True, "type": self.values[self.key(row)][0], "value": str(self.values[self.key(row)][1])}
                           if self.key(row) in self.values else {"present": False, "type": "none", "value": ""})}
                for row in rows]
    def apply(self, rows):
        if self.fail_apply: raise ValueError("k8i1_registry_apply_failed")
        for row in rows: self.values[self.key(row)] = (row["type"], row["value"])
    def restore(self, snapshot):
        for item in reversed(snapshot):
            key=self.key(item["row"]);prior=item["prior"]
            if prior["present"]:self.values[key]=(prior["type"],prior["value"])
            else:self.values.pop(key,None)
    def verify(self, rows): return all(self.values.get(self.key(row)) == (row["type"], str(row["value"])) for row in rows)
    def verify_snapshot(self, snapshot):
        return all((item["prior"]["present"] and self.values.get(self.key(item["row"]))==(item["prior"]["type"],item["prior"]["value"]))
                   or (not item["prior"]["present"] and self.key(item["row"]) not in self.values) for item in snapshot)
    def begin_rollback(self):pass
    def end_rollback(self):pass


def registry_request_bytes(operation: str, nonce: str, action: str, row: dict) -> bytes:
    if (not HEX32.fullmatch(operation) or not HEX64.fullmatch(nonce)
            or action not in ("read", "write", "delete")):
        raise ValueError("k8i1_registry_request_identity")
    if row["root"] not in ("HKCU", "HKLM") or row["type"] not in ("REG_SZ", "REG_DWORD", "none"):
        raise ValueError("k8i1_registry_request_schema")
    encode = lambda value: str(value).encode("utf-16le").hex()
    return "\n".join(["K8I1_REGISTRY_V1", operation, nonce, action, encode(row["root"]),
                       encode(row["key"]), encode(row["name"]), row["type"],
                       encode(row.get("value", "")), ""]).encode("utf-16le")


def parse_registry_result(data: bytes, operation: str, nonce: str, action: str) -> dict:
    try:
        fields = data.decode("ascii").rstrip("\r\n").split(" ")
    except UnicodeDecodeError as exc:
        raise ValueError("k8i1_registry_result_encoding") from exc
    if (len(fields) != 8 or fields[:4] != ["K8I1_REGISTRY_RESULT_V1", operation, nonce, action]
            or not all(re.fullmatch(r"[0-9]{1,10}", value) for value in fields[4:7])
            or not re.fullmatch(r"(?:[0-9a-f]{4})*", fields[7])):
        raise ValueError("k8i1_registry_result_schema")
    status, present, kind = map(int, fields[4:7])
    if status or present not in (0, 1) or kind not in (0, 1, 4):
        raise ValueError("k8i1_registry_result_failure")
    try:
        value = bytes.fromhex(fields[7]).decode("utf-16le")
    except (ValueError, UnicodeDecodeError) as exc:
        raise ValueError("k8i1_registry_result_encoding") from exc
    return {"present": bool(present), "type": {0: "none", 1: "REG_SZ", 4: "REG_DWORD"}[kind], "value": value}


def setup_watch_request_bytes(operation: str, nonce: str, request: dict, setup: str,
                              setup_size: int, timeout_ms: int) -> bytes:
    if (not HEX32.fullmatch(operation) or not HEX64.fullmatch(nonce)
            or type(request.get("windows_pid")) is not int or request["windows_pid"] <= 0
            or type(request.get("windows_created")) is not int or request["windows_created"] <= 0
            or not isinstance(setup, str) or not setup
            or type(setup_size) is not int or setup_size <= 0
            or type(timeout_ms) is not int or not 1000 <= timeout_ms <= 7_200_000):
        raise ValueError("k8i1_setup_watch_request")
    return "\n".join(["K8I1_SETUP_WATCH_V1", operation, nonce, str(request["windows_pid"]),
                       str(request["windows_created"]), setup, str(setup_size), str(timeout_ms), ""]).encode("utf-16le")


def parse_setup_watch_result(data: bytes, operation: str, nonce: str, request: dict) -> dict:
    try: fields = data.decode("ascii").strip().split(" ")
    except UnicodeDecodeError as exc: raise ValueError("k8i1_setup_watch_result") from exc
    if (len(fields) != 7 or fields[:3] != ["K8I1_SETUP_WATCH_RESULT_V1", operation, nonce]
            or not all(re.fullmatch(r"[0-9]{1,20}", value) for value in fields[3:])):
        raise ValueError("k8i1_setup_watch_result")
    pid, created, error, exit_code = map(int, fields[3:])
    if pid != request["windows_pid"] or created != request["windows_created"] or error > 0xffffffff or exit_code > 0xffffffff:
        raise ValueError("k8i1_setup_watch_result")
    return {"schema": 1, "generation": {"windows_pid": pid, "windows_created": created},
            "wait": "signaled" if error == 0 else "unavailable", "query_error": error,
            "exit_code": exit_code if error == 0 else None}


class RuntimeRegistry:
    """Exact registry port backed by the source-owned fixed Windows helper."""
    def __init__(self, invoke, phase=lambda _:None):
        self.invoke = invoke;self.phase=phase;self.rolling_back=False

    @staticmethod
    def _read_row(row):
        return {"root": row["root"], "key": row["key"], "name": row["name"], "type": "none", "value": ""}

    def snapshot(self, rows):
        return [{"row": self._read_row(row), "prior": self.invoke("read", self._read_row(row))} for row in rows]

    def apply(self, rows):
        for row in rows:
            result = self.invoke("write", row)
            if result["present"]:
                raise ValueError("k8i1_registry_write_result")

    def restore(self, snapshot):
        for item in reversed(snapshot):
            prior = item["prior"]
            row = dict(item["row"])
            if prior["present"]:
                row.update(type=prior["type"], value=prior["value"])
                self.invoke("write", row)
            else:
                self.invoke("delete", row)

    def verify(self, rows):
        for row in rows:
            result = self.invoke("read", self._read_row(row))
            if not result["present"] or (result["type"], result["value"]) != (row["type"], str(row["value"])):
                return False
        return True

    def verify_snapshot(self, snapshot):
        for item in snapshot:
            result = self.invoke("read", item["row"])
            if result != item["prior"]:
                return False
        return True

    def begin_rollback(self):self.rolling_back=True;self.phase(True)
    def end_rollback(self):self.phase(False);self.rolling_back=False


def _fsync_directory(path: pathlib.Path) -> None:
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try: os.fsync(fd)
    finally: os.close(fd)


def _planned_destination(root: pathlib.Path, relative: str) -> tuple[pathlib.Path, list[pathlib.Path]]:
    """Resolve one destination without mutating its parent tree.

    Missing directories are returned as intended mutations so the complete
    rollback journal can be durable before the first directory is created.
    """
    target = root.joinpath(*safe_relative(relative).parts)
    current = root; missing = []
    for part in safe_relative(relative).parts[:-1]:
        current = current / part
        if current.exists():
            md = current.lstat()
            if not stat.S_ISDIR(md.st_mode) or current.resolve() != current:
                raise ValueError("k8i1_destination_alias")
        else:
            missing.append(current)
    if target.exists() and (not stat.S_ISREG(target.lstat().st_mode) or target.resolve() != target):
        raise ValueError("k8i1_destination_alias")
    return target, missing


def _create_planned_directories(root: pathlib.Path, relatives: list[str]) -> None:
    for relative in relatives:
        directory = root.joinpath(*safe_relative(relative).parts)
        if directory.exists():
            md = directory.lstat()
            if not stat.S_ISDIR(md.st_mode) or directory.resolve() != directory:
                raise ValueError("k8i1_destination_alias")
            continue
        directory.mkdir(mode=0o700)
        _fsync_directory(directory.parent)


def _copy(source, output, cancelled) -> None:
    while True:
        if cancelled():
            raise ValueError("k8i1_transaction_cancelled")
        data = source.read(1024 * 1024)
        if not data:
            return
        output.write(data)


def _metadata(path: pathlib.Path) -> dict:
    identity=digest(path)
    md = path.lstat()
    return {"mode": stat.S_IMODE(md.st_mode), "uid": md.st_uid, "gid": md.st_gid,
            "atime_ns": md.st_atime_ns, "mtime_ns": md.st_mtime_ns,
            "sha256": identity, "size": md.st_size}


def _restore_metadata(path: pathlib.Path, value: dict) -> None:
    os.chmod(path, value["mode"], follow_symlinks=False)
    try: os.chown(path, value["uid"], value["gid"], follow_symlinks=False)
    except PermissionError:
        current = path.lstat()
        if (current.st_uid, current.st_gid) != (value["uid"], value["gid"]): raise
    os.utime(path, ns=(value["atime_ns"], value["mtime_ns"]), follow_symlinks=False)


def _same_file_identity(left: dict, right: dict) -> bool:
    return all(left.get(name)==right.get(name) for name in
               ("mode","uid","gid","mtime_ns","sha256","size"))


def _atomic_record(path: pathlib.Path, value: dict) -> None:
    temporary = path.with_name(path.name + ".staging-" + os.urandom(8).hex())
    with temporary.open("xb") as handle:
        handle.write(canonical(value)); handle.flush(); os.fsync(handle.fileno())
    os.replace(temporary, path); _fsync_directory(path.parent)


def _transaction_root(drive: pathlib.Path, operation: pathlib.Path) -> pathlib.Path:
    key = digest_bytes(os.fsencode(operation.resolve()))[:32]
    return drive / ".linux-vst-bridge-private" / "k8i1-transactions" / key


def _surface(plan: dict, drive: pathlib.Path) -> list[dict]:
    roots = ["Program Files/Native Instruments/Kontakt 8",
             "Program Files/Common Files/Native Instruments/Kontakt 8",
             "Program Files/Common Files/VST3/Kontakt 8.vst3"]
    observed = []
    for relative in roots:
        root = drive.joinpath(*safe_relative(relative).parts)
        if not root.exists(): continue
        if not root.is_dir() or root.resolve() != root: raise ValueError("k8i1_surface_alias")
        for path in sorted(root.rglob("*")):
            if path.is_dir(): continue
            exact_file(path, maximum=2 * 1024 * 1024 * 1024)
            observed.append({"path": str(path.relative_to(drive)), **_metadata(path)})
    product_record=drive.joinpath(*safe_relative(
        "users/Public/Documents/Native Instruments/installed_products/Kontakt 8.json").parts)
    if product_record.exists():
        exact_file(product_record, maximum=2 * 1024 * 1024)
        observed.append({"path":str(product_record.relative_to(drive)),**_metadata(product_record)})
    return observed


def _load_journal(drive: pathlib.Path, operation: pathlib.Path) -> tuple[pathlib.Path, dict]:
    root = _transaction_root(drive, operation); path = root / "journal.json"
    exact_file(path, maximum=16 * 1024 * 1024)
    journal = read_json_bytes(path.read_bytes(), 16 * 1024 * 1024)
    if journal.get("schema") != 1 or journal.get("drive") != {"device": drive.stat().st_dev, "inode": drive.stat().st_ino}:
        raise ValueError("k8i1_journal_identity")
    return root, journal


def rollback_transaction(drive: pathlib.Path, operation: pathlib.Path, registry: RegistryPort,
                         fault=lambda point: None) -> dict:
    root, journal = _load_journal(drive, operation); errors = []
    try:
        registry.begin_rollback();registry.restore(journal["registry_snapshot"]); fault("after_registry_restore")
    except Exception as exc:
        errors.append(str(exc) if isinstance(exc,ValueError) else "k8i1_rollback_registry_"+type(exc).__name__)
    for row in reversed(journal["files"]):
        try:
            destination = drive.joinpath(*safe_relative(row["destination"]).parts)
            backup = root / row["backup"] if row["backup"] else None
            if backup is None:
                if destination.exists():
                    destination.unlink();_fsync_directory(destination.parent)
            else:
                if backup.exists():
                    exact_file(backup, maximum=2 * 1024 * 1024 * 1024);os.replace(backup, destination)
                elif not destination.exists() or not _same_file_identity(_metadata(destination),row["prior"]):
                    raise ValueError("k8i1_rollback_backup_missing")
                _restore_metadata(destination, row["prior"])
                if not _same_file_identity(_metadata(destination),row["prior"]):
                    raise ValueError("k8i1_rollback_file_changed")
                _restore_metadata(destination,row["prior"])
                _fsync_directory(destination.parent)
            fault("after_file_restore")
        except Exception as exc:
            errors.append(str(exc) if isinstance(exc,ValueError) else "k8i1_rollback_file_"+type(exc).__name__)
    baseline={row["path"].casefold():row for row in journal["baseline_surface"]}
    try:
        current={row["path"].casefold():row for row in _surface({},drive)}
        for path in sorted(set(current)-set(baseline)):
            destination=drive.joinpath(*safe_relative(current[path]["path"]).parts)
            destination.unlink();_fsync_directory(destination.parent)
        for row in journal["baseline_surface"]:
            backup=root/row["backup"] if row.get("backup") else None
            destination=drive.joinpath(*safe_relative(row["path"]).parts)
            if backup is not None:
                if backup.exists():
                    exact_file(backup,maximum=2*1024*1024*1024);os.replace(backup,destination)
                elif not destination.exists() or not _same_file_identity(_metadata(destination),row):
                    raise ValueError("k8i1_rollback_surface_backup_missing")
                _restore_metadata(destination,row)
                _fsync_directory(destination.parent)
        restored={row["path"].casefold():row for row in _surface({},drive)}
        if set(restored)!=set(baseline) or any(not _same_file_identity(restored[path],prior)
                                               for path,prior in baseline.items()):
            raise ValueError("k8i1_rollback_surface_changed")
        for prior in baseline.values():
            _restore_metadata(drive.joinpath(*safe_relative(prior["path"]).parts),prior)
        fault("after_surface_restore")
    except Exception as exc:
        errors.append(str(exc) if isinstance(exc,ValueError) else "k8i1_rollback_surface_"+type(exc).__name__)
    for relative in sorted(journal["created_directories"],key=lambda value:value.count("/"),reverse=True):
        try: drive.joinpath(*safe_relative(relative).parts).rmdir()
        except OSError: pass
    try:
        if not registry.verify_snapshot(journal["registry_snapshot"]):
            raise ValueError("k8i1_rollback_registry_changed")
    except Exception as exc:
        errors.append(str(exc) if isinstance(exc,ValueError) else "k8i1_rollback_verify_"+type(exc).__name__)
    finally:
        try:registry.end_rollback()
        except Exception as exc:errors.append("k8i1_rollback_phase_"+type(exc).__name__)
    if not errors:
        journal["state"] = "rolled_back"; journal["rollback_verified"] = True
        error=None
    else:
        error=errors[0]
        journal["state"] = "rollback_unconfirmed"; journal["rollback_verified"] = False
        journal["rollback_error"] = errors
    _atomic_record(root / "journal.json", journal); fault("after_rollback_result")
    return {"schema": 1, "state": journal["state"], "rollback_verified": journal["rollback_verified"],
            "rollback_error": error}


def recover_interrupted_transaction(drive: pathlib.Path, operation: pathlib.Path,
                                    registry: RegistryPort, setup_exit: dict,
                                    descendants_absent: bool, fault=lambda point: None) -> dict:
    """Resume rollback only after the exact retained setup generation is terminal."""
    _, journal = _load_journal(drive, operation)
    if journal.get("state") == "completed":
        terminal=journal.get("terminal")
        if not isinstance(terminal,dict) or terminal.get("state")!="completed":
            raise ValueError("k8i1_transaction_terminal_receipt")
        return terminal
    if journal.get("state") == "rolled_back":
        return {"schema": 1, "state": "rolled_back", "rollback_verified": True,
                "rollback_error": None}
    if (setup_exit.get("wait") != "signaled" or setup_exit.get("query_error") != 0
            or setup_exit.get("generation") != journal.get("setup_generation")
            or not descendants_absent):
        raise ValueError("k8i1_transaction_recovery_setup_not_terminal")
    return rollback_transaction(drive, operation, registry, fault)


def execute_transaction(plan: dict, offline: pathlib.Path, drive: pathlib.Path, operation: pathlib.Path,
                        registry: RegistryPort, cancelled=lambda: False, confirm=lambda receipt: None,
                        fault=lambda point: None, *, setup_generation: dict | None = None) -> dict:
    """Stage and verify payload, publish shim success, but retain rollback custody.

    Completion is intentionally deferred to ``finalize_transaction`` after the
    exact setup generation exits.
    """
    if offline.resolve() != offline or drive.resolve() != drive:
        raise ValueError("k8i1_transaction_root_alias")
    drive_md=drive.lstat()
    if (not isinstance(setup_generation, dict)
            or set(setup_generation) != {"windows_pid", "windows_created"}
            or any(type(setup_generation[name]) is not int or setup_generation[name] <= 0
                   for name in setup_generation)):
        raise ValueError("k8i1_transaction_setup_generation")
    root = _transaction_root(drive, operation)
    if root.exists():
        if (root/"journal.json").exists():raise ValueError("k8i1_transaction_recovery_required")
        raise ValueError("k8i1_transaction_journal_exists")
    root.mkdir(parents=True, mode=0o700); stage=root/"stage";rollback=root/"rollback"
    stage.mkdir(mode=0o700);rollback.mkdir(mode=0o700)
    if not (root.stat().st_dev == stage.stat().st_dev == rollback.stat().st_dev == drive_md.st_dev):
        raise ValueError("k8i1_transaction_cross_filesystem")
    created_dirs=[]; records=[]; registry_snapshot=registry.snapshot(plan["registry"])
    baseline_surface=_surface(plan,drive)
    planned_paths={row["destination"].casefold() for row in plan["files"]+plan["state_files"]}
    for ordinal,row in enumerate(baseline_surface,1):
        if row["path"].casefold() in planned_paths:continue
        source=drive.joinpath(*safe_relative(row["path"]).parts);backup=rollback/f"surface-{ordinal:05d}"
        with source.open("rb") as input_file,backup.open("xb") as output:
            _copy(input_file,output,cancelled);output.flush();os.fsync(output.fileno())
        _restore_metadata(source,row);_restore_metadata(backup,row);row["backup"]=str(backup.relative_to(root))
    journal={"schema":1,"state":"preparing","drive":{"device":drive_md.st_dev,"inode":drive_md.st_ino},
             "setup_generation":setup_generation,
             "files":records,"created_directories":created_dirs,"registry_snapshot":registry_snapshot,
             "registry_state":"prepared","baseline_surface":baseline_surface,"rollback_verified":None}
    try:
        required_bytes=sum(row["size"] for row in plan["files"])+sum(row["size"] for row in plan["state_files"])
        if shutil.disk_usage(drive).free < required_bytes*2+64*1024*1024:raise ValueError("k8i1_space_unavailable")
        material=[(row,offline/pathlib.Path(*safe_relative(row["source"]).parts),None) for row in plan["files"]]
        material += [(row,None,row["content_utf8"].encode()) for row in plan["state_files"]]
        for ordinal,(row,source,generated) in enumerate(material,1):
            if cancelled():raise ValueError("k8i1_transaction_cancelled")
            destination,planned_directories=_planned_destination(drive,row["destination"])
            for directory in planned_directories:
                relative=str(directory.relative_to(drive))
                if relative not in created_dirs:created_dirs.append(relative)
            if source is not None:
                exact_file(source,maximum=2*1024*1024*1024)
                if source.stat().st_size!=row["size"] or digest(source)!=row["sha256"]:raise ValueError("k8i1_source_changed")
            elif len(generated)!=row["size"] or digest_bytes(generated)!=row["sha256"]:raise ValueError("k8i1_generated_state_changed")
            staged=stage/f"{ordinal:05d}";backup=None;prior=None
            with staged.open("xb") as output:
                if source is None:output.write(generated)
                else:
                    with source.open("rb") as input_file:_copy(input_file,output,cancelled)
                output.flush();os.fsync(output.fileno())
            if staged.stat().st_size!=row["size"] or digest(staged)!=row["sha256"]:raise ValueError("k8i1_stage_changed")
            if destination.exists():
                exact_file(destination,maximum=2*1024*1024*1024);prior=_metadata(destination);backup=rollback/f"{ordinal:05d}"
                with destination.open("rb") as input_file,backup.open("xb") as output:
                    _copy(input_file,output,cancelled);output.flush();os.fsync(output.fileno())
                _restore_metadata(backup,prior)
            records.append({"destination":str(destination.relative_to(drive)),"staged":str(staged.relative_to(root)),
                            "backup":str(backup.relative_to(root)) if backup else None,"prior":prior,
                            "create_directories":[str(path.relative_to(drive)) for path in planned_directories],
                            "sha256":row["sha256"],"size":row["size"],"state":"prepared"})
        _fsync_directory(stage);_fsync_directory(rollback)
        journal["state"]="prepared";_atomic_record(root/"journal.json",journal);fault("after_journal_prepare")
        for record in records:
            if cancelled():raise ValueError("k8i1_transaction_cancelled")
            record["state"]="intent";_atomic_record(root/"journal.json",journal);fault("after_file_intent")
            _create_planned_directories(drive,record["create_directories"])
            destination=drive.joinpath(*safe_relative(record["destination"]).parts)
            os.replace(root/record["staged"],destination);_fsync_directory(destination.parent)
            record["committed_metadata"]=_metadata(destination)
            record["state"]="committed";_atomic_record(root/"journal.json",journal);fault("after_file_commit")
        journal["registry_state"]="intent";_atomic_record(root/"journal.json",journal);fault("after_registry_intent")
        registry.apply(plan["registry"]);journal["registry_state"]="committed"
        _atomic_record(root/"journal.json",journal);fault("after_registry_commit")
        if cancelled():raise ValueError("k8i1_transaction_cancelled")
        for record in records:
            destination=drive.joinpath(*safe_relative(record["destination"]).parts)
            if destination.stat().st_size!=record["size"] or digest(destination)!=record["sha256"]:raise ValueError("k8i1_installed_file_changed")
        if not registry.verify(plan["registry"]):raise ValueError("k8i1_registry_verification_failed")
        receipt={"schema":1,"state":"payload_deployment_verified","file_count":len(records),
                 "payload_sha256":digest_bytes(canonical([{"destination":r["destination"],"sha256":r["sha256"],"size":r["size"]} for r in records])),
                 "registry_count":len(plan["registry"]),"rollback_verified":None,
                 "journal_sha256":digest(root/"journal.json")}
        journal["state"]="shim_success_intent";_atomic_record(root/"journal.json",journal)
        fault("after_shim_success_intent")
        confirmation=confirm(receipt)
        journal["shim_result_identity"]=confirmation
        journal["state"]="shim_success_returned";_atomic_record(root/"journal.json",journal)
        fault("after_shim_result_publication");return receipt
    except Exception as exc:
        error=str(exc) if isinstance(exc,ValueError) else "k8i1_transaction_"+type(exc).__name__
        if not (root/"journal.json").exists():_atomic_record(root/"journal.json",journal)
        result=rollback_transaction(drive,operation,registry,fault)
        result.update(state="failed",error=error)
        _atomic_record(operation/"k8i1-transaction.private.json",result)
        raise


def finalize_transaction(plan: dict, drive: pathlib.Path, operation: pathlib.Path, registry: RegistryPort,
                         setup_exit: dict, descendants_absent: bool, fault=lambda point: None) -> dict:
    root,journal=_load_journal(drive,operation)
    try:
        if (journal["state"]!="shim_success_returned"
                or setup_exit.get("generation")!=journal.get("setup_generation")
                or setup_exit.get("wait")!="signaled"
                or setup_exit.get("query_error")!=0 or setup_exit.get("exit_code")!=0):
            raise ValueError("k8i1_setup_terminal_adverse")
        if not descendants_absent:raise ValueError("k8i1_installer_descendant_remains")
        for row in journal["files"]:
            destination=drive.joinpath(*safe_relative(row["destination"]).parts)
            current=_metadata(destination);expected=row.get("committed_metadata")
            stable=("mode","uid","gid","mtime_ns","sha256","size")
            if (not isinstance(expected,dict)
                    or any(current[name]!=expected[name] for name in stable)
                    or current["size"]!=row["size"] or current["sha256"]!=row["sha256"]):
                raise ValueError("k8i1_post_setup_file_changed")
        if not registry.verify(plan["registry"]):raise ValueError("k8i1_post_setup_registry_changed")
        planned={row["destination"].casefold() for row in journal["files"]}
        baseline={row["path"].casefold():row for row in journal["baseline_surface"]}
        current={row["path"].casefold():row for row in _surface(plan,drive)}
        if set(current) != set(baseline)|planned:
            raise ValueError("k8i1_post_setup_surface_changed")
        for path, prior in baseline.items():
            if path not in planned and current[path] != prior:
                raise ValueError("k8i1_post_setup_surface_changed")
        terminal={"schema":1,"state":"completed","payload_deployment_verified":True,
                  "shim_success_returned":True,"setup_exit_observed":True,"setup_exit":setup_exit,
                  "post_setup_state_verified":True,"file_count":len(journal["files"]),
                  "registry_count":len(plan["registry"]),"rollback_verified":None}
        journal["state"]="completion_intent";journal["terminal"]=terminal
        _atomic_record(root/"journal.json",journal);fault("after_terminal_intent")
        journal["state"]="completed";journal["rollback_verified"]=None
        _atomic_record(root/"journal.json",journal);fault("after_terminal_commit")
        return terminal
    except Exception as exc:
        error=str(exc) if isinstance(exc,ValueError) else "k8i1_post_setup_"+type(exc).__name__
        rollback=rollback_transaction(drive,operation,registry,fault)
        result={**rollback,"schema":1,"state":"failed","payload_deployment_verified":journal["state"] in ("shim_success_returned","completed"),
                "shim_success_returned":journal["state"] in ("shim_success_returned","completed"),
                "setup_exit_observed":setup_exit.get("wait")=="signaled","setup_exit":setup_exit,
                "post_setup_state_verified":False,"error":error}
        _atomic_record(operation/"k8i1-transaction.private.json",result);raise ValueError(error) from exc


class PrefixArm:
    """Temporary, exactly reversible per-application MSI override."""
    OVERRIDE = {"root": "HKCU", "key": r"Software\Wine\AppDefaults\Kontakt 8 Setup PC.exe\DllOverrides",
                "name": "msi", "type": "REG_SZ", "value": "native,builtin"}

    def __init__(self, adapter: Adapter, authority: dict, prefix: pathlib.Path, operation: pathlib.Path,
                 operation_id: str, nonce: str, request: pathlib.Path, result: pathlib.Path,
                 registry: RegistryPort):
        self.adapter=adapter;self.authority=authority;self.prefix=prefix;self.operation=operation
        self.operation_id=operation_id;self.nonce=nonce;self.request=request;self.result=result;self.registry=registry
        self.drive=prefix/"drive_c";self.system=self.drive/"windows/syswow64"
        self.root=self.drive/".linux-vst-bridge-private/k8i1-arm"/operation_id
        self.records=[];self.override_snapshot=None;self.armed=False

    def arm(self, windows_path, fault=lambda point: None) -> None:
        if self.armed: raise ValueError("k8i1_arm_reentry")
        md=self.system.lstat()
        if not stat.S_ISDIR(md.st_mode) or self.system.resolve()!=self.system: raise ValueError("k8i1_system_directory")
        if self.root.exists():raise ValueError("k8i1_arm_journal_exists")
        self.root.mkdir(parents=True,mode=0o700);stage=self.root/"stage";backup=self.root/"rollback"
        stage.mkdir(mode=0o700);backup.mkdir(mode=0o700)
        if not (self.drive.stat().st_dev==self.root.stat().st_dev==stage.stat().st_dev==backup.stat().st_dev):
            raise ValueError("k8i1_arm_cross_filesystem")
        try:
            material=[]
            for ordinal,name in enumerate(("msi.dll", "msi_lvb_real.dll", "k8i1.private"),1):
                target=self.system/name;prior=None
                if target.exists():
                    exact_file(target,maximum=64*1024*1024);prior=backup/f"{ordinal:02d}";prior_identity=_metadata(target)
                    with target.open("rb") as source,prior.open("xb") as output:
                        _copy(source,output,lambda:False);output.flush();os.fsync(output.fileno())
                    _restore_metadata(prior,prior_identity)
                else:prior_identity=None
                if name=="k8i1.private":
                    data=("\n".join(["K8I1_CONFIG_V1",self.operation_id,self.nonce,
                        self.authority["setup"]["sha256"],str(self.authority["setup"]["size"]),
                        self.adapter.manifest["cached_msi"]["sha256"],str(self.adapter.manifest["cached_msi"]["size"]),
                        windows_path(self.request),windows_path(self.result),self.adapter.manifest["plan_sha256"],
                        str(self.authority["timeout_seconds"]*1000),""])).encode("utf-16le")
                else:data=(self.adapter.root/name).read_bytes()
                staged=stage/f"{ordinal:02d}"
                with staged.open("xb") as output:output.write(data);output.flush();os.fsync(output.fileno())
                material.append({"name":name,"target":str(target.relative_to(self.drive)),
                                 "staged":str(staged.relative_to(self.root)),
                                 "backup":str(prior.relative_to(self.root)) if prior else None,
                                 "prior":prior_identity,"sha256":digest_bytes(data),"size":len(data),"state":"prepared"})
            self.override_snapshot=self.registry.snapshot([self.OVERRIDE])
            self.records=material
            journal={"schema":1,"operation":self.operation_id,
                     "drive":{"device":self.drive.stat().st_dev,"inode":self.drive.stat().st_ino},
                     "state":"prepared","files":self.records,"override_snapshot":self.override_snapshot,
                     "override_state":"prepared"}
            _atomic_record(self.root/"journal.json",journal);fault("after_arm_journal_prepare")
            for record in self.records:
                record["state"]="intent";_atomic_record(self.root/"journal.json",journal);fault("after_arm_intent")
                target=self.drive.joinpath(*safe_relative(record["target"]).parts)
                os.replace(self.root/record["staged"],target);target.chmod(0o400);_fsync_directory(target.parent)
                record["state"]="committed";_atomic_record(self.root/"journal.json",journal);fault("after_arm_commit")
            journal["override_state"]="intent";_atomic_record(self.root/"journal.json",journal);fault("after_arm_registry_intent")
            self.registry.apply([self.OVERRIDE]);journal["override_state"]="committed"
            _atomic_record(self.root/"journal.json",journal);fault("after_arm_registry_commit")
            if not self.registry.verify([self.OVERRIDE]):raise ValueError("k8i1_override_verification_failed")
            journal["state"]="armed";_atomic_record(self.root/"journal.json",journal);self.armed=True
        except Exception:
            if (self.root/"journal.json").exists():self.close()
            else:
                shutil.rmtree(self.root,ignore_errors=True);self.records.clear();self.override_snapshot=None
            raise

    def close(self, fault=lambda point: None) -> None:
        journal_path=self.root/"journal.json"
        if not journal_path.exists() and not self.records and self.override_snapshot is None:
            self.armed = False
            return
        error=None
        try:
            exact_file(journal_path,maximum=2*1024*1024);journal=read_json_bytes(journal_path.read_bytes(),2*1024*1024)
            if (journal.get("schema")!=1 or journal.get("operation")!=self.operation_id
                    or journal.get("drive")!={"device":self.drive.stat().st_dev,"inode":self.drive.stat().st_ino}):
                raise ValueError("k8i1_arm_journal_identity")
            if journal.get("state")=="disarmed":
                self.records.clear();self.override_snapshot=None;self.armed=False;return
            self.registry.restore(journal["override_snapshot"]);fault("after_arm_registry_restore")
            if not self.registry.verify_snapshot(journal["override_snapshot"]):raise ValueError("k8i1_override_rollback_failed")
        except Exception as exc:
            error=exc
            journal={"files":self.records}
        for record in reversed(journal.get("files",[])):
            try:
                target=self.drive.joinpath(*safe_relative(record["target"]).parts)
                prior=self.root/record["backup"] if record["backup"] else None
                if prior is None:target.unlink(missing_ok=True)
                else:
                    if prior.exists():os.replace(prior,target)
                    elif not target.exists() or _metadata(target)!=record["prior"]:raise ValueError("k8i1_arm_backup_missing")
                    _restore_metadata(target,record["prior"])
                    if _metadata(target)!=record["prior"]:raise ValueError("k8i1_arm_restore_changed")
                _fsync_directory(target.parent)
                fault("after_arm_file_restore")
            except Exception as exc:error=error or exc
        if not error:
            journal["state"]="disarmed";_atomic_record(journal_path,journal);fault("after_arm_disarm_result")
        self.records.clear()
        self.override_snapshot=None
        self.armed=False
        if error:raise ValueError("k8i1_disarm_failed") from error


def host_windows_path(value: str, prefix: pathlib.Path) -> pathlib.Path:
    path=pathlib.PureWindowsPath(value)
    if path.drive.lower()=="c:":
        candidate=prefix/"drive_c"/pathlib.Path(*path.parts[1:])
    elif path.drive.lower()=="z:":
        candidate=pathlib.Path("/").joinpath(*path.parts[1:])
    else:raise ValueError("k8i1_windows_path_drive")
    candidate=pathlib.Path(os.path.normpath(candidate))
    if path.drive.lower()=="c:" and prefix/"drive_c" not in (candidate,*candidate.parents):
        raise ValueError("k8i1_windows_path_escape")
    return candidate


def exact_offline(plan: dict, cached_msi: pathlib.Path) -> pathlib.Path:
    roots=[]
    search=cached_msi.parent.parent
    for candidate in sorted(search.glob("mia*")):
        for root in (candidate/"OFFLINE",candidate/"data/OFFLINE"):
            if not root.is_dir() or root.resolve()!=root:continue
            try:
                for row in plan["files"]:
                    source=root/pathlib.Path(*safe_relative(row["source"]).parts)
                    exact_file(source)
                    if source.stat().st_size!=row["size"] or digest(source)!=row["sha256"]:raise ValueError
            except (OSError,ValueError):continue
            roots.append(root)
    if len(roots)!=1:raise ValueError("k8i1_offline_payload_ambiguous_or_missing")
    return roots[0]


def parse_request(path: pathlib.Path, operation: str, nonce: str) -> dict:
    exact_file(path, maximum=196608)
    data = path.read_bytes()
    if len(data) & 1:
        raise ValueError("k8i1_request_encoding")
    try: lines = data.decode("utf-16le").splitlines()
    except UnicodeDecodeError as exc: raise ValueError("k8i1_request_encoding") from exc
    if (len(lines) != 10 or lines[:4] != ["K8I1_REQUEST_V1", operation, nonce, "1"]
            or lines[4] not in ("A", "W") or not re.fullmatch(r"[1-9][0-9]{0,9}", lines[5])
            or not re.fullmatch(r"[1-9][0-9]{0,19}", lines[6])
            or any(len(value) > 131072 or len(value) % 4 for value in lines[7:10])):
        raise ValueError("k8i1_request_schema")
    def decode(value):
        try: return bytes.fromhex(value).decode("utf-16le")
        except (ValueError, UnicodeDecodeError) as exc: raise ValueError("k8i1_request_encoding") from exc
    setup, package, properties = decode(lines[7]), decode(lines[8]), decode(lines[9])
    if pathlib.PureWindowsPath(setup).name.lower() != SETUP_BASENAME.lower():
        raise ValueError("k8i1_request_setup")
    if pathlib.PureWindowsPath(package).name.lower() != MSI_BASENAME.lower():
        raise ValueError("k8i1_request_package")
    return {"schema": 1, "operation": operation, "nonce": nonce, "ordinal": 1,
            "call": lines[4], "windows_pid": int(lines[5]), "windows_created": int(lines[6]),
            "setup": setup, "package": package,
            "properties": properties,"properties_sha256": digest_bytes(properties.encode("utf-16le"))}


def result_bytes(operation: str, nonce: str, plan_sha256: str, receipt_sha256: str) -> bytes:
    if not HEX32.fullmatch(operation) or not HEX64.fullmatch(nonce) or not HEX64.fullmatch(plan_sha256) or not HEX64.fullmatch(receipt_sha256):
        raise ValueError("k8i1_result_identity")
    return ("\n".join(["K8I1_RESULT_V1", operation, nonce, "1", "verified", plan_sha256, receipt_sha256, ""])).encode("utf-16le")


def failure_result_bytes(operation: str, nonce: str, plan_sha256: str) -> bytes:
    if not HEX32.fullmatch(operation) or not HEX64.fullmatch(nonce) or not HEX64.fullmatch(plan_sha256):
        raise ValueError("k8i1_result_identity")
    return ("\n".join(["K8I1_RESULT_V1", operation, nonce, "1", "failed", plan_sha256, "0" * 64, ""])).encode("utf-16le")
