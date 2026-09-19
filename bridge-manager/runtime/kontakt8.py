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
REQUIRED_ARCHIVE = {
    "manifest.json",
    "msi.dll",
    "msi_lvb_real.dll",
    "package-plan.json",
    "THIRD_PARTY.txt",
    "LICENSE.ni-wine-MIT",
    "SOURCE.json",
    "k8i1-registry.exe",
}
REQUIRED_TABLES = {
    "Directory", "Component", "File", "Feature", "FeatureComponents", "Registry",
    "InstallExecuteSequence", "CustomAction", "CreateFolder", "RemoveFile", "Shortcut",
}
MUTATION_TABLES = {
    "ServiceInstall", "ServiceControl", "Class", "TypeLib", "Environment", "IniFile",
    "MoveFiles", "DuplicateFile", "SelfReg", "ODBCDataSource", "ODBCDriver", "ODBCTranslator",
    "PublishComponent", "ProgId", "Extension", "MIME", "Verb",
}
HEX64 = re.compile(r"[0-9a-f]{64}\Z")
HEX32 = re.compile(r"[0-9a-f]{32}\Z")
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


def table_snapshot(directory: pathlib.Path) -> dict:
    """Bind private msidump IDT projections without retaining proprietary rows."""
    md=directory.lstat()
    if not stat.S_ISDIR(md.st_mode) or directory.resolve()!=directory:
        raise ValueError("k8i1_table_directory")
    rows={};witnesses=[];empty=[]
    for name in sorted(REQUIRED_TABLES|MUTATION_TABLES):
        path=directory/(name+".idt")
        if not path.exists():
            if name in REQUIRED_TABLES:raise ValueError("k8i1_table_missing")
            count=0;data=b""
        else:
            exact_file(path,maximum=64*1024*1024);data=path.read_bytes()
            try:lines=data.decode("utf-8",errors="strict").splitlines()
            except UnicodeDecodeError as exc:raise ValueError("k8i1_table_encoding") from exc
            if len(lines)<3:raise ValueError("k8i1_table_shape")
            count=sum(bool(line) for line in lines[3:])
        rows[name]=count
        if name in MUTATION_TABLES and count==0:empty.append(name)
        witnesses.append({"name":name,"sha256":digest_bytes(data),"size":len(data),"rows":count})
    return {"snapshot_sha256":digest_bytes(canonical(witnesses)),"row_counts":rows,
            "empty_or_absent":empty}


def validate_private_package_inputs(plan: dict, setup: pathlib.Path, pristine: pathlib.Path,
                                    cached: pathlib.Path, offline: pathlib.Path,
                                    tables: pathlib.Path) -> None:
    for path,expected_size,expected_sha,error in (
        (setup,SETUP_SIZE,SETUP_SHA256,"k8i1_setup_identity"),
        (pristine,PRISTINE_MSI_SIZE,PRISTINE_MSI_SHA256,"k8i1_pristine_msi_identity"),
        (cached,plan["cached_msi"]["size"],plan["cached_msi"]["sha256"],"k8i1_cached_msi_identity"),
    ):
        metadata=exact_file(path,maximum=2*1024*1024*1024)
        if metadata.st_size!=expected_size or digest(path)!=expected_sha:raise ValueError(error)
    omd=offline.lstat()
    if not stat.S_ISDIR(omd.st_mode) or offline.resolve()!=offline:raise ValueError("k8i1_offline_identity")
    for row in plan["files"]:
        source=offline/pathlib.Path(*safe_relative(row["source"]).parts)
        metadata=exact_file(source,maximum=2*1024*1024*1024)
        if metadata.st_size!=row["size"] or digest(source)!=row["sha256"]:
            raise ValueError("k8i1_source_changed")
    if table_snapshot(tables)!=plan["tables"]:raise ValueError("k8i1_table_snapshot")


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
            or not HEX64.fullmatch(str(adapter["sha256"]))):
        raise ValueError("k8i1_adapter_identity")
    path = pathlib.Path(adapter["path"])
    exact_file(path, maximum=64 * 1024 * 1024)
    if path.name != "kontakt8-adapter.zip" or digest(path) != adapter["sha256"]:
        raise ValueError("k8i1_adapter_identity")
    return json.loads(json.dumps(expected))


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
        plan = read_json_bytes((root / "package-plan.json").read_bytes(), 48 * 1024 * 1024)
        validate_plan(plan, authority, manifest)
        return cls(root, manifest, plan)

    @staticmethod
    def _manifest(value: dict, authority: dict) -> None:
        fields = {"schema", "kind", "product", "setup", "pristine_msi", "cached_msi", "wine", "exports",
                  "artifacts", "plan_sha256", "source"}
        if (not isinstance(value, dict) or set(value) != fields or value["schema"] != 1
                or value["kind"] != "k8i1-exact-package-adapter"
                or value["product"] != authority["product"] or value["setup"] != authority["setup"]
                or value["pristine_msi"] != authority["pristine_msi"]
                or value["wine"] != authority["wine"]
                or value["exports"] != {"count": 296, "ordinal_first": 5, "ordinal_last": 300,
                                          "intercepted": ["MsiInstallProductA", "MsiInstallProductW"],
                                          "spec_sha256": "5a6085ce66f541d3c52552164ade070129ea05a0f8a8b1ed2c660132366aceb0"}
                or set(value["artifacts"]) != REQUIRED_ARCHIVE
                or not HEX64.fullmatch(str(value["plan_sha256"]))):
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
    fields = {"schema", "product", "setup", "pristine_msi", "cached_msi", "tables", "files", "state_files",
              "registry", "custom_actions", "omissions", "source_tree_sha256"}
    if (not isinstance(plan, dict) or set(plan) != fields or plan["schema"] != 1
            or plan["product"] != authority["product"] or plan["setup"] != authority["setup"]
            or plan["pristine_msi"] != authority["pristine_msi"] or plan["cached_msi"] != manifest["cached_msi"]
            or digest_bytes(canonical(plan)) != manifest["plan_sha256"]):
        raise ValueError("k8i1_plan_binding")
    tables = plan["tables"]
    if (not isinstance(tables, dict) or set(tables) != {"snapshot_sha256", "row_counts", "empty_or_absent"}
            or not HEX64.fullmatch(str(tables["snapshot_sha256"]))
            or set(tables["row_counts"])!=REQUIRED_TABLES|MUTATION_TABLES
            or set(tables["empty_or_absent"])!=MUTATION_TABLES):
        raise ValueError("k8i1_plan_tables")
    if any(type(v) is not int or v < 0 for v in tables["row_counts"].values()):
        raise ValueError("k8i1_plan_tables")
    files = plan["files"]
    if not isinstance(files, list) or not files or len(files) > 20000:
        raise ValueError("k8i1_plan_files")
    sources, destinations, selected = set(), set(), set()
    total = 0
    for row in files:
        if (not isinstance(row, dict) or set(row) != {"source", "destination", "sha256", "size", "component", "feature", "class"}
                or row["class"] not in ("standalone", "vst3", "native_instruments_common")
                or not HEX64.fullmatch(str(row["sha256"])) or type(row["size"]) is not int or row["size"] < 0
                or not isinstance(row["component"], str) or not row["component"]
                or not isinstance(row["feature"], str) or not row["feature"]):
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
        if (not isinstance(state, dict) or set(state) != {"destination", "content_utf8", "sha256", "size", "class"}
                or state["class"] != "native_instruments_product_record"):
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
                or product_state["InstallDir"] != "C:\\Program Files\\Native Instruments\\Kontakt 8\\"
                or product_state["ContentDir"] != "C:\\Program Files\\Common Files\\Native Instruments\\Kontakt 8\\"
                or not isinstance(product_state["ContentVersion"], str)
                or not product_state["ContentVersion"]):
            raise ValueError("k8i1_plan_state_file")
        if destination.casefold() in destinations:
            raise ValueError("k8i1_plan_file_duplicate")
        destinations.add(destination.casefold())
    registry_names = set()
    for row in plan["registry"]:
        if (not isinstance(row, dict) or set(row) != {"root", "key", "name", "type", "value", "source_row"}
                or row["root"] != "HKLM" or row["type"] != "REG_SZ"
                or row["key"] != r"Software\Native Instruments\Kontakt 8"
                or not isinstance(row["name"], str) or not row["name"] or not isinstance(row["source_row"], str)):
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
    if len(plan["custom_actions"]) != tables["row_counts"]["CustomAction"]:
        raise ValueError("k8i1_plan_custom_action")
    actions = set()
    for action in plan["custom_actions"]:
        if (not isinstance(action, dict) or set(action) != {"action", "sequence", "disposition", "basis"}
                or not isinstance(action["action"],str) or not action["action"]
                or type(action["sequence"]) is not int or action["sequence"]<0
                or action["disposition"] not in ("covered_by_verified_payload_and_state", "intentionally_omitted_nonselected_feature")
                or not isinstance(action["basis"],str) or not action["basis"]):
            raise ValueError("k8i1_plan_custom_action")
        if action["action"] in actions:
            raise ValueError("k8i1_plan_custom_action")
        actions.add(action["action"])
    if plan["omissions"] != [{"feature": "aax", "disposition": "intentionally_omitted_nonselected_feature"}]:
        raise ValueError("k8i1_plan_omissions")


class RegistryPort(Protocol):
    def snapshot(self, rows: list[dict]) -> object: ...
    def apply(self, rows: list[dict]) -> None: ...
    def restore(self, snapshot: object) -> None: ...
    def verify(self, rows: list[dict]) -> bool: ...
    def verify_snapshot(self, snapshot: object) -> bool: ...


class MemoryRegistry:
    """Deterministic production-law fixture; production supplies a runtime port."""
    def __init__(self, values: dict | None = None, fail_apply: bool = False):
        self.values = dict(values or {}); self.fail_apply = fail_apply
    @staticmethod
    def key(row): return row["root"], row["key"], row["name"]
    def snapshot(self, rows): return {self.key(row): self.values.get(self.key(row), None) for row in rows}
    def apply(self, rows):
        if self.fail_apply: raise ValueError("k8i1_registry_apply_failed")
        for row in rows: self.values[self.key(row)] = (row["type"], row["value"])
    def restore(self, snapshot):
        for key, value in snapshot.items():
            if value is None: self.values.pop(key, None)
            else: self.values[key] = value
    def verify(self, rows): return all(self.values.get(self.key(row)) == (row["type"], row["value"]) for row in rows)
    def verify_snapshot(self, snapshot): return all((value is None and key not in self.values) or self.values.get(key) == value for key, value in snapshot.items())


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


class RuntimeRegistry:
    """Exact registry port backed by the source-owned fixed Windows helper."""
    def __init__(self, invoke):
        self.invoke = invoke

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


def _fsync_directory(path: pathlib.Path) -> None:
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try: os.fsync(fd)
    finally: os.close(fd)


def _safe_destination(root: pathlib.Path, relative: str, created: list[pathlib.Path]) -> pathlib.Path:
    target = root.joinpath(*safe_relative(relative).parts)
    current = root
    for part in safe_relative(relative).parts[:-1]:
        current = current / part
        if current.exists():
            md = current.lstat()
            if not stat.S_ISDIR(md.st_mode) or current.resolve() != current:
                raise ValueError("k8i1_destination_alias")
        else:
            current.mkdir(mode=0o700); created.append(current)
    if target.exists() and (not stat.S_ISREG(target.lstat().st_mode) or target.resolve() != target):
        raise ValueError("k8i1_destination_alias")
    return target


def _copy(source, output, cancelled) -> None:
    while True:
        if cancelled():
            raise ValueError("k8i1_transaction_cancelled")
        data = source.read(1024 * 1024)
        if not data:
            return
        output.write(data)


def execute_transaction(plan: dict, offline: pathlib.Path, drive: pathlib.Path, operation: pathlib.Path,
                        registry: RegistryPort, cancelled=lambda: False, confirm=lambda receipt: None) -> dict:
    """Apply one already validated exact plan, or restore its complete prior state."""
    if offline.resolve() != offline or drive.resolve() != drive:
        raise ValueError("k8i1_transaction_root_alias")
    drive_md=drive.lstat();drive_identity=(drive_md.st_dev,drive_md.st_ino)
    def stable_drive():
        current=drive.lstat()
        if (not stat.S_ISDIR(current.st_mode) or drive.resolve()!=drive
            or (current.st_dev,current.st_ino)!=drive_identity):raise ValueError("k8i1_transaction_root_changed")
    stage = operation / "k8i1-stage.private"; rollback = operation / "k8i1-rollback.private"
    stage.mkdir(mode=0o700); rollback.mkdir(mode=0o700)
    created_dirs: list[pathlib.Path] = []
    records: list[dict] = []
    registry_snapshot = registry.snapshot(plan["registry"])
    committed = False
    error = None
    try:
        required_bytes = sum(row["size"] for row in plan["files"]) + sum(row["size"] for row in plan["state_files"])
        if shutil.disk_usage(drive).free < required_bytes * 2 + 64 * 1024 * 1024:
            raise ValueError("k8i1_space_unavailable")
        material = [(row, offline / pathlib.Path(*safe_relative(row["source"]).parts), None) for row in plan["files"]]
        material += [(row, None, row["content_utf8"].encode()) for row in plan["state_files"]]
        for ordinal, (row, source, generated) in enumerate(material, 1):
            stable_drive()
            if cancelled(): raise ValueError("k8i1_transaction_cancelled")
            destination = _safe_destination(drive, row["destination"], created_dirs)
            if source is not None:
                exact_file(source)
                if source.stat().st_size != row["size"] or digest(source) != row["sha256"]:
                    raise ValueError("k8i1_source_changed")
            else:
                if len(generated) != row["size"] or digest_bytes(generated) != row["sha256"]:
                    raise ValueError("k8i1_generated_state_changed")
            staged = stage / f"{ordinal:05d}"
            with staged.open("xb") as output:
                if source is not None:
                    with source.open("rb") as input_file: _copy(input_file, output, cancelled)
                else: output.write(generated)
                output.flush(); os.fsync(output.fileno())
            if staged.stat().st_size != row["size"] or digest(staged) != row["sha256"]:
                raise ValueError("k8i1_stage_changed")
            prior = None
            if destination.exists():
                exact_file(destination)
                prior = rollback / f"{ordinal:05d}"
                with destination.open("rb") as input_file, prior.open("xb") as output:
                    _copy(input_file, output, cancelled); output.flush(); os.fsync(output.fileno())
            records.append({"destination": destination, "staged": staged, "prior": prior,
                            "prior_sha256": digest(prior) if prior else None,
                            "sha256": row["sha256"], "size": row["size"]})
        _fsync_directory(stage); _fsync_directory(rollback)
        stable_drive()
        if cancelled(): raise ValueError("k8i1_transaction_cancelled")
        for record in records:
            os.replace(record["staged"], record["destination"]); _fsync_directory(record["destination"].parent)
        if cancelled(): raise ValueError("k8i1_transaction_cancelled")
        registry.apply(plan["registry"])
        stable_drive()
        for record in records:
            if record["destination"].stat().st_size != record["size"] or digest(record["destination"]) != record["sha256"]:
                raise ValueError("k8i1_installed_file_changed")
        if not registry.verify(plan["registry"]):
            raise ValueError("k8i1_registry_verification_failed")
        stable_drive()
        receipt = {"schema": 1, "state": "verified", "file_count": len(records),
                   "payload_sha256": digest_bytes(canonical([{"destination": str(r["destination"].relative_to(drive)),
                                                              "sha256": r["sha256"], "size": r["size"]} for r in records])),
                   "registry_count": len(plan["registry"]), "rollback_verified": None}
        # The shim success handoff is part of commit. If it cannot be published,
        # the transaction remains uncommitted and the finally block rolls back.
        confirm(receipt)
        committed = True
        return receipt
    except Exception as exc:
        error = str(exc) if isinstance(exc, ValueError) else "k8i1_transaction_" + type(exc).__name__
        raise
    finally:
        if not committed:
            rollback_error = None
            try:
                registry.restore(registry_snapshot)
                for record in reversed(records):
                    destination, prior = record["destination"], record["prior"]
                    if prior is None:
                        destination.unlink(missing_ok=True)
                    else:
                        os.replace(prior, destination)
                        if digest(destination) != record["prior_sha256"]:
                            raise ValueError("k8i1_rollback_file_changed")
                    _fsync_directory(destination.parent)
                for directory in reversed(created_dirs):
                    try: directory.rmdir()
                    except OSError: pass
                if not registry.verify_snapshot(registry_snapshot):
                    raise ValueError("k8i1_rollback_registry_changed")
            except Exception as exc:
                rollback_error = str(exc) if isinstance(exc, ValueError) else "k8i1_rollback_" + type(exc).__name__
            result = {"schema": 1, "state": "failed", "error": error,
                      "rollback_verified": rollback_error is None, "rollback_error": rollback_error}
            target = operation / "k8i1-transaction.private.json"
            data = canonical(result)
            with target.open("xb") as handle: handle.write(data); handle.flush(); os.fsync(handle.fileno())


class PrefixArm:
    """Temporary, exactly reversible per-application MSI override."""
    OVERRIDE = {"root": "HKCU", "key": r"Software\Wine\AppDefaults\Kontakt 8 Setup PC.exe\DllOverrides",
                "name": "msi", "type": "REG_SZ", "value": "native,builtin"}

    def __init__(self, adapter: Adapter, authority: dict, prefix: pathlib.Path, operation: pathlib.Path,
                 operation_id: str, nonce: str, request: pathlib.Path, result: pathlib.Path,
                 registry: RegistryPort):
        self.adapter=adapter;self.authority=authority;self.prefix=prefix;self.operation=operation
        self.operation_id=operation_id;self.nonce=nonce;self.request=request;self.result=result;self.registry=registry
        self.system=prefix/"drive_c/windows/syswow64";self.backup=operation/"k8i1-arm-rollback.private"
        self.records=[];self.override_snapshot=None;self.armed=False

    def arm(self, windows_path) -> None:
        if self.armed: raise ValueError("k8i1_arm_reentry")
        md=self.system.lstat()
        if not stat.S_ISDIR(md.st_mode) or self.system.resolve()!=self.system: raise ValueError("k8i1_system_directory")
        self.backup.mkdir(mode=0o700)
        try:
            for name in ("msi.dll", "msi_lvb_real.dll", "k8i1.private"):
                target=self.system/name;prior=None;prior_mode=None
                if target.exists():
                    prior_md=exact_file(target,maximum=64*1024*1024);prior=self.backup/name;prior_mode=stat.S_IMODE(prior_md.st_mode)
                    with target.open("rb") as source,prior.open("xb") as output:
                        _copy(source,output,lambda:False);output.flush();os.fsync(output.fileno())
                if name=="k8i1.private":
                    data=("\n".join(["K8I1_CONFIG_V1",self.operation_id,self.nonce,
                        self.authority["setup"]["sha256"],str(self.authority["setup"]["size"]),
                        self.adapter.manifest["cached_msi"]["sha256"],str(self.adapter.manifest["cached_msi"]["size"]),
                        windows_path(self.request),windows_path(self.result),self.adapter.manifest["plan_sha256"],
                        str(self.authority["timeout_seconds"]*1000),""])).encode("utf-16le")
                else:data=(self.adapter.root/name).read_bytes()
                staged=self.operation/(name+".arm.private")
                with staged.open("xb") as output:output.write(data);output.flush();os.fsync(output.fileno())
                os.replace(staged,target);_fsync_directory(target.parent);target.chmod(0o400)
                self.records.append((target,prior,prior_mode,digest(target)))
            self.override_snapshot=self.registry.snapshot([self.OVERRIDE])
            self.registry.apply([self.OVERRIDE])
            if not self.registry.verify([self.OVERRIDE]):raise ValueError("k8i1_override_verification_failed")
            self.armed=True
        except Exception:
            self.close()
            raise

    def close(self) -> None:
        if not self.records and self.override_snapshot is None:
            self.armed = False
            return
        error=None
        try:
            if self.override_snapshot is not None:
                self.registry.restore(self.override_snapshot)
                if not self.registry.verify_snapshot(self.override_snapshot):raise ValueError("k8i1_override_rollback_failed")
        except Exception as exc:error=exc
        for target,prior,prior_mode,_ in reversed(self.records):
            try:
                if prior is None:target.unlink(missing_ok=True)
                else:os.replace(prior,target);target.chmod(prior_mode)
                _fsync_directory(target.parent)
            except Exception as exc:error=error or exc
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
