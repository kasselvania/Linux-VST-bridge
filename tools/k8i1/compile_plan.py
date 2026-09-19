#!/usr/bin/env python3
"""Compile the exact Kontakt 8.13.1 plan from a freshly opened pristine MSI.

This is deliberately not a plan sealer.  It runs the source-owned table
exporter itself, enumerates the MSI's complete table roster, and derives every
selected file through the package's relational tables.  Unsupported table
effects or custom actions are a compiler error, not prose for a caller to fill
in later.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "bridge-manager/runtime"))
import kontakt8 as k

DERIVED_TABLES = {
    "_Tables", "_Columns", "_Validation", "Directory", "Component", "File",
    "Feature", "FeatureComponents", "Registry", "Property", "InstallExecuteSequence",
}
CLASSIFIED_TABLES = {"CustomAction", "CreateFolder", "RemoveFile", "Shortcut"}
EMPTY_MUTATION_TABLES = k.MUTATION_TABLES | {
    "AppId", "BindImage", "Condition", "CreateFolder", "DuplicateFile", "Environment",
    "Extension", "Font", "IniFile", "IniLocator", "IsolatedComponent", "LaunchCondition",
    "LockPermissions", "MIME", "MoveFiles", "ODBCAttribute", "ODBCDataSource",
    "ODBCDriver", "ODBCTranslator", "ProgId", "PublishComponent", "RemoveFile",
    "RemoveIniFile", "ReserveCost", "SelfReg", "ServiceControl", "ServiceInstall",
    "Shortcut", "TypeLib", "Verb", "MsiAssembly", "MsiAssemblyName",
}
OBSERVED_TABLES = {
    "AdminExecuteSequence", "AdminUISequence", "AdvtExecuteSequence", "AdvtUISequence",
    "ActionText", "BBControl", "Billboard", "Binary", "CheckBox", "ComboBox", "Control",
    "ControlCondition", "ControlEvent", "Dialog", "DrLocator", "Error", "EventMapping",
    "FeatureComponents", "Feature", "Icon", "InstallUISequence", "ListBox", "ListView",
    "Media", "MsiDigitalCertificate", "MsiDigitalSignature", "MsiFileHash",
    "MsiPatchCertificate", "Patch", "PatchPackage",
    "Property", "RadioButton", "RegLocator", "Signature", "TextStyle", "UIText",
    "Upgrade", "_Streams", "_Storages", "_TransformView",
}
STANDARD_ROOTS = {
    "ProgramFiles64Folder": r"C:\Program Files",
    "ProgramFilesFolder": r"C:\Program Files (x86)",
    "CommonFiles64Folder": r"C:\Program Files\Common Files",
    "CommonFilesFolder": r"C:\Program Files (x86)\Common Files",
    "CommonAppDataFolder": r"C:\ProgramData",
    "WindowsFolder": r"C:\windows",
    "System64Folder": r"C:\windows\system32",
    "SystemFolder": r"C:\windows\syswow64",
}
PROPERTY = re.compile(r"[A-Z_][A-Z0-9_]{0,71}\Z")


def exact(path: pathlib.Path, maximum: int) -> dict:
    md = path.lstat()
    if (not path.is_file() or path.is_symlink() or md.st_nlink != 1
            or path.resolve() != path or not 0 < md.st_size <= maximum):
        raise ValueError("k8i1_compiler_input")
    return {"sha256": k.digest(path), "size": md.st_size}


def decode_idt(data: bytes) -> str:
    if data.startswith((b"\xff\xfe", b"\xfe\xff")):
        return data.decode("utf-16")
    return data.decode("utf-8", errors="strict")


def read_idt(path: pathlib.Path) -> tuple[list[str], list[dict[str, str]], dict]:
    identity = exact(path, 64 * 1024 * 1024)
    lines = decode_idt(path.read_bytes()).splitlines()
    if len(lines) < 3:
        raise ValueError("k8i1_compiler_idt_shape")
    columns = lines[0].split("\t")
    if not columns or len(columns) != len(set(columns)) or any(not value for value in columns):
        raise ValueError("k8i1_compiler_idt_columns")
    rows = []
    for line in lines[3:]:
        if not line:
            continue
        fields = line.split("\t")
        if len(fields) != len(columns):
            raise ValueError("k8i1_compiler_idt_row")
        rows.append(dict(zip(columns, fields, strict=True)))
    return columns, rows, {**identity, "rows": len(rows), "columns": columns}


def long_name(value: str) -> str:
    target = value.split(":", 1)[0]
    return target.split("|", 1)[-1]


def source_name(value: str) -> str:
    source = value.split(":", 1)[1] if ":" in value else value
    return source.split("|", 1)[-1]


def win_relative(value: str) -> str:
    normalized = value.replace("/", "\\")
    if not re.match(r"(?i)^c:\\", normalized):
        raise ValueError("k8i1_compiler_non_c_destination")
    parts = [part for part in normalized[3:].split("\\") if part]
    return str(k.safe_relative("/".join(parts)))


def substitute(value: str, properties: dict[str, str]) -> str:
    def replacement(match: re.Match[str]) -> str:
        name = match.group(1).upper()
        if name not in properties:
            raise ValueError("k8i1_compiler_unresolved_property")
        return properties[name]
    result = re.sub(r"\[([A-Za-z_][A-Za-z0-9_]*)\]", replacement, value)
    if "[" in result or "]" in result:
        raise ValueError("k8i1_compiler_unresolved_property")
    return result


def compile_projection(projection: pathlib.Path, offline: pathlib.Path, properties_raw: str,
                       package: dict) -> dict:
    roster_path = projection / "_k8i1-tables.txt"
    identity = exact(roster_path, 1024 * 1024)
    roster_text = decode_idt(roster_path.read_bytes())
    roster = [line for line in roster_text.splitlines() if line]
    if (not roster or roster != sorted(set(roster))
            or any(not re.fullmatch(r"[_A-Za-z][_A-Za-z0-9]{0,71}", name) for name in roster)):
        raise ValueError("k8i1_compiler_table_roster")
    projected = sorted(path.stem for path in projection.glob("*.idt"))
    if projected != roster:
        raise ValueError("k8i1_compiler_projection_roster")
    tables: dict[str, list[dict[str, str]]] = {}
    witnesses = []
    dispositions = []
    for name in roster:
        columns, rows, witness = read_idt(projection / f"{name}.idt")
        tables[name] = rows
        witnesses.append({"name": name, **witness})
        if name in DERIVED_TABLES:
            disposition = "derived"
        elif name in CLASSIFIED_TABLES:
            disposition = "classified"
        elif name in EMPTY_MUTATION_TABLES:
            if rows:
                raise ValueError("k8i1_compiler_unsupported_mutation_table")
            disposition = "required_empty"
        elif name in OBSERVED_TABLES:
            disposition = "observed_non_deployment"
        else:
            raise ValueError("k8i1_compiler_unclassified_table")
        dispositions.append({"name": name, "rows": len(rows), "disposition": disposition})
    names = {row.get("Name") for row in tables.get("_Tables", [])}
    if names != set(roster):
        raise ValueError("k8i1_compiler_tables_self_roster")

    required = {"Directory", "Component", "File", "Feature", "FeatureComponents", "Registry",
                "InstallExecuteSequence", "CustomAction"}
    if not required <= set(roster):
        raise ValueError("k8i1_compiler_required_table")
    observed_properties = k.normalize_msi_properties(properties_raw)
    if {"PRODUCTCODE", "PRODUCTVERSION", "PRODUCTNAME"} & set(observed_properties):
        raise ValueError("k8i1_compiler_package_property_override")
    properties = {**STANDARD_ROOTS, **observed_properties}
    properties = {name:properties[name] for name in sorted(properties)}
    package_properties = {}
    for row in tables.get("Property", []):
        name = row.get("Property", "").upper()
        value = row.get("Value", "")
        if not PROPERTY.fullmatch(name) or name in package_properties:
            raise ValueError("k8i1_compiler_property_row")
        package_properties[name] = value
        properties.setdefault(name, value)
    product_code = package_properties.get("PRODUCTCODE", "").upper()
    if (not re.fullmatch(r"\{[0-9A-F]{8}(?:-[0-9A-F]{4}){3}-[0-9A-F]{12}\}", product_code)
            or package_properties.get("PRODUCTVERSION") != k.VERSION):
        raise ValueError("k8i1_compiler_package_identity")
    package = {**package, "product_code": product_code}

    directories = {}
    for row in tables["Directory"]:
        key = row.get("Directory", "")
        if not key or key in directories:
            raise ValueError("k8i1_compiler_directory")
        directories[key] = row

    def chain(key: str) -> list[dict[str, str]]:
        result = []
        seen = set()
        while key and key not in ("TARGETDIR", "SourceDir"):
            if key in seen or key not in directories:
                raise ValueError("k8i1_compiler_directory_graph")
            seen.add(key); row = directories[key]; result.append(row); key = row.get("Directory_Parent", "")
        result.reverse()
        return result

    directory_windows: dict[str, str] = {}
    directory_sources: dict[str, str] = {}
    for key in directories:
        rows = chain(key)
        root_index = None
        root_value = None
        for index, row in enumerate(rows):
            candidate = properties.get(row["Directory"].upper())
            if candidate and re.match(r"(?i)^c:\\", candidate):
                root_index, root_value = index, candidate.rstrip("\\/")
        if root_index is not None:
            value = root_value
            for row in rows[root_index + 1:]:
                part = long_name(row.get("DefaultDir", ""))
                if part not in ("", "."):
                    value += "\\" + part
            directory_windows[key] = value
        offline_index = next((i for i, row in enumerate(rows)
                              if source_name(row.get("DefaultDir", "")).casefold() == "offline"), None)
        if offline_index is not None:
            parts = []
            for row in rows[offline_index + 1:]:
                part = source_name(row.get("DefaultDir", ""))
                if part not in ("", "."):
                    parts.append(part)
            directory_sources[key] = "/".join(parts)

    components = {}
    for row in tables["Component"]:
        key = row.get("Component", "")
        if not key or key in components or not row.get("Directory_"):
            raise ValueError("k8i1_compiler_component")
        components[key] = row
    component_features: dict[str, list[str]] = {}
    feature_rows = {row.get("Feature") for row in tables["Feature"]}
    for row in tables["FeatureComponents"]:
        feature, component = row.get("Feature_", ""), row.get("Component_", "")
        if feature not in feature_rows or component not in components:
            raise ValueError("k8i1_compiler_feature_component")
        component_features.setdefault(component, []).append(feature)

    files = []
    omissions = []
    seen_file_keys, seen_sources, seen_destinations = set(), set(), set()
    for row in tables["File"]:
        file_key, component = row.get("File", ""), row.get("Component_", "")
        if not file_key or file_key in seen_file_keys or component not in components:
            raise ValueError("k8i1_compiler_file_row")
        seen_file_keys.add(file_key)
        features = sorted(set(component_features.get(component, [])))
        if len(features) != 1:
            raise ValueError("k8i1_compiler_file_feature_ambiguous")
        directory = components[component]["Directory_"]
        if directory not in directory_windows or directory not in directory_sources:
            raise ValueError("k8i1_compiler_file_directory_unresolved")
        filename = long_name(row.get("FileName", ""))
        if not filename or "/" in filename or "\\" in filename:
            raise ValueError("k8i1_compiler_file_name")
        source_rel = "/".join(part for part in (directory_sources[directory], filename) if part)
        destination_rel = win_relative(directory_windows[directory] + "\\" + filename)
        source = offline.joinpath(*k.safe_relative(source_rel).parts)
        source_identity = exact(source, 2 * 1024 * 1024 * 1024)
        if source_rel.casefold() in seen_sources or destination_rel.casefold() in seen_destinations:
            raise ValueError("k8i1_compiler_file_duplicate")
        seen_sources.add(source_rel.casefold()); seen_destinations.add(destination_rel.casefold())
        if destination_rel.startswith("Program Files/Common Files/Avid/Audio/Plug-Ins/"):
            omissions.append({"file": file_key, "component": component, "feature": features[0],
                              "destination": destination_rel, "disposition": "aax_not_selected"})
            continue
        if destination_rel.startswith("Program Files/Native Instruments/Kontakt 8/"):
            file_class = "standalone"
        elif destination_rel.startswith("Program Files/Common Files/VST3/Kontakt 8.vst3/"):
            file_class = "vst3"
        elif destination_rel.startswith("Program Files/Common Files/Native Instruments/Kontakt 8/"):
            file_class = "native_instruments_common"
        else:
            raise ValueError("k8i1_compiler_file_destination_unclassified")
        files.append({"source": source_rel, "destination": destination_rel,
                      "sha256": source_identity["sha256"], "size": source_identity["size"],
                      "file_row": file_key, "component": component, "directory": directory,
                      "feature": features[0], "feature_component_row": f"{features[0]}->{component}",
                      "class": file_class})
    offline_files = sorted(path for path in offline.rglob("*") if path.is_file())
    if len(offline_files) != len(seen_sources):
        raise ValueError("k8i1_compiler_unclassified_payload_file")

    registry = []
    selected_components = {row["component"] for row in files}
    for row in tables["Registry"]:
        component = row.get("Component_", "")
        if component not in selected_components:
            omissions.append({"registry_row": row.get("Registry", ""), "component": component,
                              "disposition": "nonselected_component"})
            continue
        if (row.get("Root") != "2" or row.get("Key") != r"Software\Native Instruments\Kontakt 8"
                or row.get("Name") not in k.KONTAKT_REGISTRY):
            raise ValueError("k8i1_compiler_registry_scope")
        value = substitute(row.get("Value", ""), {**properties, **{key.upper(): value for key, value in directory_windows.items()}})
        if value.startswith("#"):
            raise ValueError("k8i1_compiler_registry_type")
        expected = k.KONTAKT_REGISTRY[row["Name"]]
        if expected != ("REG_SZ", value):
            raise ValueError("k8i1_compiler_registry_value")
        registry.append({"root": "HKLM", "key": row["Key"], "name": row["Name"],
                         "type": "REG_SZ", "value": value, "source_row": row["Registry"],
                         "component": component})
    if {row["name"] for row in registry} != set(k.KONTAKT_REGISTRY):
        raise ValueError("k8i1_compiler_registry_incomplete")

    sequence = {}
    for row in tables["InstallExecuteSequence"]:
        action = row.get("Action", "")
        if not action or action in sequence:
            raise ValueError("k8i1_compiler_execute_sequence")
        sequence[action] = row
    custom_actions = []
    for row in tables["CustomAction"]:
        action = row.get("Action", "")
        if not action or action not in sequence:
            raise ValueError("k8i1_compiler_custom_action_sequence")
        item={"action": action, "type": int(row.get("Type", "-1")),
              "source": row.get("Source", ""), "target": row.get("Target", ""),
              "sequence": int(sequence[action].get("Sequence", "-1")),
              "condition": sequence[action].get("Condition", ""),
              "disposition": "omitted_by_exact_payload_deployment"}
        item["row_sha256"]=k.digest_bytes(k.canonical({name:item[name] for name in
            ("action","type","source","target","sequence","condition")}))
        custom_actions.append(item)

    create_folders=[]
    for row in tables.get("CreateFolder",[]):
        component=row.get("Component_","");directory=row.get("Directory_","")
        if component not in components or directory not in directories:
            raise ValueError("k8i1_compiler_create_folder")
        selected=component in selected_components
        create_folders.append({"directory":directory,"component":component,"selected":selected,
            "disposition":"created_by_selected_file_plan" if selected else "nonselected_component"})
    remove_files=[]
    for row in tables.get("RemoveFile",[]):
        component=row.get("Component_","")
        if component not in components:raise ValueError("k8i1_compiler_remove_file")
        if component in selected_components:raise ValueError("k8i1_compiler_selected_remove_file_unsupported")
        item={name:row.get(name,"") for name in ("FileKey","Component_","FileName","DirProperty","InstallMode")}
        item["disposition"]="nonselected_component";remove_files.append(item)
    shortcuts=[]
    for row in tables.get("Shortcut",[]):
        component=row.get("Component_","")
        if component not in components:raise ValueError("k8i1_compiler_shortcut")
        item={name:row.get(name,"") for name in
              ("Shortcut","Directory_","Name","Component_","Target","Arguments","Description","Hotkey","Icon_","IconIndex","ShowCmd","WkDir")}
        item["disposition"]=("omitted_nonessential_shortcut" if component in selected_components
                             else "nonselected_component")
        shortcuts.append(item)

    registry_values={row["name"]:row["value"] for row in registry}
    state = json.dumps({"ContentDir": registry_values["ContentDir"],
                        "ContentVersion": registry_values["ContentVersion"],
                        "InstallDir": registry_values["InstallDir"]},
                       sort_keys=True, separators=(",", ":"))
    table_record = {"roster": dispositions, "roster_sha256": identity["sha256"],
                    "projection_sha256": k.digest_bytes(k.canonical(witnesses)), "projections": witnesses}
    plan = {"schema": 2, "product": {"name": k.PRODUCT, "version": k.VERSION},
            "setup": {"basename": k.SETUP_BASENAME, "sha256": k.SETUP_SHA256, "size": k.SETUP_SIZE},
            "pristine_msi": {"basename": k.MSI_BASENAME, "sha256": k.PRISTINE_MSI_SHA256,
                             "size": k.PRISTINE_MSI_SIZE},
            "cached_msi": package, "properties": {"observed": observed_properties, "effective": properties,
                "sha256": k.digest_bytes(k.canonical(observed_properties))},
            "tables": table_record, "files": files,
            "state_files": [{"destination": "users/Public/Documents/Native Instruments/installed_products/Kontakt 8.json",
                "content_utf8": state, **{"sha256": k.digest_bytes(state.encode()), "size": len(state.encode())},
                "class": "native_instruments_product_record",
                "derivation":{"registry_names":["ContentDir","ContentVersion","InstallDir"]}}],
            "registry": registry, "custom_actions": custom_actions,
            "mutations":{"create_folders":create_folders,"remove_files":remove_files,"shortcuts":shortcuts},
            "omissions": omissions,
            "source_tree_sha256": k.selected_source_tree_sha256(files)}
    return plan


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--wine", type=pathlib.Path, required=True)
    parser.add_argument("--exporter", type=pathlib.Path, required=True)
    parser.add_argument("--setup", type=pathlib.Path, required=True)
    parser.add_argument("--pristine-msi", type=pathlib.Path, required=True)
    parser.add_argument("--cached-msi", type=pathlib.Path, required=True)
    parser.add_argument("--offline", type=pathlib.Path, required=True)
    parser.add_argument("--properties", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("k8i1_compiler_output_exists")
    setup = exact(args.setup.resolve(), 2 * 1024 * 1024 * 1024)
    pristine = exact(args.pristine_msi.resolve(), 64 * 1024 * 1024)
    cached = exact(args.cached_msi.resolve(), 64 * 1024 * 1024)
    if setup != {"sha256": k.SETUP_SHA256, "size": k.SETUP_SIZE}:
        raise ValueError("k8i1_setup_identity")
    if pristine != {"sha256": k.PRISTINE_MSI_SHA256, "size": k.PRISTINE_MSI_SIZE}:
        raise ValueError("k8i1_pristine_msi_identity")
    exporter = exact(args.exporter.resolve(), 16 * 1024 * 1024)
    wine = exact(args.wine.resolve(), 64 * 1024 * 1024)
    raw_properties = args.properties.resolve().read_text(encoding="utf-8", errors="strict")
    with tempfile.TemporaryDirectory(prefix="k8i1-projection-") as raw:
        projection = pathlib.Path(raw).resolve()
        completed = subprocess.run([str(args.wine.resolve()), str(args.exporter.resolve()),
                                    str(args.pristine_msi.resolve()), str(projection)],
                                   stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   timeout=120, check=False)
        if completed.returncode != 0 or len(completed.stdout) > 65536 or len(completed.stderr) > 65536:
            raise ValueError("k8i1_compiler_projection_failed")
        plan = compile_projection(projection, args.offline.resolve(), raw_properties,
                                  {"basename": k.MSI_BASENAME, **cached})
        result = {"schema": 1, "kind": "k8i1-package-compiler-result",
                  "compiler": {"source_sha256": k.digest(pathlib.Path(__file__).resolve()),
                               "exporter": exporter, "wine": wine},
                  "plan": plan, "plan_sha256": k.digest_bytes(k.canonical(plan))}
    data = k.canonical(result)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("xb") as handle:
        handle.write(data); handle.flush(); os.fsync(handle.fileno())
    print(json.dumps({"schema": 1, "output": str(args.output.resolve()),
                      "sha256": k.digest(args.output), "plan_sha256": result["plan_sha256"]}, sort_keys=True))


if __name__ == "__main__":
    main()
