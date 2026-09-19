import hashlib
import json
import os
import pathlib
import tempfile
import unittest
import zipfile
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import kontakt8 as k
ROOT = HERE.parents[1]


def pe32(payload=b"fixture"):
    data = bytearray(256 + len(payload)); data[:2] = b"MZ"; data[60:64] = (128).to_bytes(4, "little")
    data[128:132] = b"PE\0\0"; data[132:134] = (0x14c).to_bytes(2, "little"); data[256:] = payload
    return bytes(data)


def witness(data): return {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}


class Fixture:
    def __init__(self, root):
        self.root = pathlib.Path(root).resolve(); self.environment = self.root / "environment"; self.prefix = self.environment / "compatdata/pfx"
        self.drive = self.prefix / "drive_c"; self.drive.mkdir(parents=True); self.operation = self.root / "operation"; self.operation.mkdir()
        self.offline = self.root / "OFFLINE"; self.offline.mkdir()
        self.files = {}
        for name, destination, cls in [
            ("a/Kontakt 8.exe", "Program Files/Native Instruments/Kontakt 8/Kontakt 8.exe", "standalone"),
            ("b/Kontakt 8.vst3", "Program Files/Common Files/VST3/Kontakt 8.vst3/Contents/x86_64-win/Kontakt 8.vst3", "vst3"),
            ("c/engine.dll", "Program Files/Common Files/Native Instruments/Kontakt 8/engine.dll", "native_instruments_common"),
        ]:
            data=("payload:"+name).encode(); source=self.offline/name; source.parent.mkdir(parents=True,exist_ok=True); source.write_bytes(data)
            self.files[name]={"source":name,"destination":destination,**witness(data),"component":"component-"+cls,
                              "feature":"feature-"+cls,"class":cls}
        record=json.dumps({"ContentDir":"C:\\Program Files\\Common Files\\Native Instruments\\Kontakt 8\\",
                           "ContentVersion":"4.0.0","InstallDir":"C:\\Program Files\\Native Instruments\\Kontakt 8\\"},
                          sort_keys=True,separators=(",",":"))
        row_counts={name:1 for name in k.REQUIRED_TABLES};row_counts["CustomAction"]=0
        row_counts.update({name:0 for name in k.MUTATION_TABLES})
        self.plan={"schema":1,"product":{"name":k.PRODUCT,"version":k.VERSION},
                   "setup":{"basename":k.SETUP_BASENAME,"sha256":k.SETUP_SHA256,"size":k.SETUP_SIZE},
                   "pristine_msi":{"basename":k.MSI_BASENAME,"sha256":k.PRISTINE_MSI_SHA256,"size":k.PRISTINE_MSI_SIZE},
                   "cached_msi":{"basename":k.MSI_BASENAME,"sha256":"2"*64,"size":4_222_976,"product_code":"{00000000-0000-0000-0000-000000000001}"},
                   "tables":{"snapshot_sha256":"3"*64,"row_counts":row_counts,"empty_or_absent":sorted(k.MUTATION_TABLES)},
                   "files":list(self.files.values()),
                   "state_files":[{"destination":"users/Public/Documents/Native Instruments/installed_products/Kontakt 8.json",
                                   "content_utf8":record,**witness(record.encode()),"class":"native_instruments_product_record"}],
                   "registry":[{"root":"HKLM","key":r"Software\Native Instruments\Kontakt 8","name":name,"type":kind,"value":value,"source_row":"r"+str(i)}
                               for i,(name,(kind,value)) in enumerate(k.KONTAKT_REGISTRY.items())],
                   "custom_actions":[],"omissions":[{"feature":"aax","disposition":"intentionally_omitted_nonselected_feature"}],
                   "source_tree_sha256":k.selected_source_tree_sha256(list(self.files.values()))}
        self.authority={"schema":1,"authority":"exact_kontakt8_8_13_1_msi_diversion_and_verified_deployment",
                        "application":"a"*64,"software_sha256":"b"*64,"environment":"c"*32,
                        "prefix":{"path_sha256":hashlib.sha256(os.fsencode(self.prefix)).hexdigest(),"device":self.prefix.stat().st_dev,"inode":self.prefix.stat().st_ino},
                        "adapter":{},"product":{"name":k.PRODUCT,"version":k.VERSION},
                        "setup":self.plan["setup"],"pristine_msi":self.plan["pristine_msi"],
                        "intercepts":["MsiInstallProductA","MsiInstallProductW"],"request_limit":1,"timeout_seconds":7200,
                        "wine":{"base":k.WINE_BASE,"patches":k.WINE_PATCHES},
                        "selected_features":["standalone","vst3","native_instruments_common"],"omitted_features":["aax"],
                        "licensing_state":"never_manufactured"}
        self.archive=self.root/"kontakt8-adapter.zip"; self.make_archive()
        self.authority["adapter"]={"path":str(self.archive),"sha256":k.digest(self.archive)}
        self.spec={"application_identity":"a"*64,"software_sha256":"b"*64,
                   "application":{"environment":{"id":"c"*32,"root":str(self.environment)}},"kontakt8_session":self.authority}

    def make_archive(self):
        entries={"msi.dll":pe32(b"shim"),"msi_lvb_real.dll":pe32(b"real"),
                 "k8i1-registry.exe":pe32(b"registry"),
                 "package-plan.json":k.canonical(self.plan),"THIRD_PARTY.txt":b"MIT and LGPL notices",
                 "LICENSE.ni-wine-MIT":b"MIT license fixture",
                 "SOURCE.json":k.canonical({"wine":k.WINE_BASE,"patches":k.WINE_PATCHES})}
        manifest={"schema":1,"kind":"k8i1-exact-package-adapter","product":self.plan["product"],"setup":self.plan["setup"],
                  "pristine_msi":self.plan["pristine_msi"],"cached_msi":self.plan["cached_msi"],
                  "wine":{"base":k.WINE_BASE,"patches":k.WINE_PATCHES},
                  "exports":{"count":296,"ordinal_first":5,"ordinal_last":300,"intercepted":["MsiInstallProductA","MsiInstallProductW"],
                             "spec_sha256":"5a6085ce66f541d3c52552164ade070129ea05a0f8a8b1ed2c660132366aceb0"},
                  "artifacts":{},"plan_sha256":hashlib.sha256(k.canonical(self.plan)).hexdigest(),
                  "source":{"head":"d"*40,"tree":"e"*40}}
        entries["manifest.json"]=b""
        manifest["artifacts"]={name:(witness(data) if name!="manifest.json" else {"sha256":"0"*64,"size":0}) for name,data in entries.items()}
        entries["manifest.json"]=k.canonical(manifest)
        with zipfile.ZipFile(self.archive,"w",compression=zipfile.ZIP_STORED) as archive:
            for name,data in entries.items():archive.writestr(name,data)


class Kontakt8Tests(unittest.TestCase):
    def test_private_table_projection_is_exact_and_mutation_tables_must_be_empty(self):
        with tempfile.TemporaryDirectory() as raw:
            root=pathlib.Path(raw).resolve()
            for name in k.REQUIRED_TABLES:
                rows=[] if name=="CustomAction" else ["row"]
                (root/(name+".idt")).write_text("Column\nType\nTable\n"+"\n".join(rows)+("\n" if rows else ""))
            snapshot=k.table_snapshot(root)
            self.assertEqual(set(snapshot["row_counts"]),k.REQUIRED_TABLES|k.MUTATION_TABLES)
            self.assertEqual(set(snapshot["empty_or_absent"]),k.MUTATION_TABLES)
            (root/"ServiceInstall.idt").write_text("Column\nType\nTable\nmutation\n")
            self.assertNotEqual(k.table_snapshot(root),snapshot)

    def test_closed_session_and_archive(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);self.assertEqual(k.validate_session(f.spec,f.authority),f.authority)
            adapter=k.Adapter.extract(f.authority,f.operation);self.assertEqual(adapter.plan,f.plan)
            f.authority["setup"]["size"]+=1
            with self.assertRaises(ValueError):k.validate_session(f.spec,f.authority)

    def test_verified_transaction_and_existing_file_replacement(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);destination=f.drive/pathlib.Path(*pathlib.PurePosixPath(f.plan["files"][0]["destination"]).parts)
            destination.parent.mkdir(parents=True);destination.write_bytes(b"old")
            registry=k.MemoryRegistry();result=k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry)
            self.assertEqual(result["state"],"verified");self.assertEqual(result["file_count"],4)
            for row in f.plan["files"]:self.assertEqual(k.digest(f.drive/pathlib.Path(*pathlib.PurePosixPath(row["destination"]).parts)),row["sha256"])
            self.assertTrue(registry.verify(f.plan["registry"]))

    def test_failed_state_write_rolls_back_all_files_and_registry(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);destination=f.drive/pathlib.Path(*pathlib.PurePosixPath(f.plan["files"][0]["destination"]).parts)
            destination.parent.mkdir(parents=True);destination.write_bytes(b"old");old=k.digest(destination)
            registry=k.MemoryRegistry({("HKLM",r"Software\Native Instruments\Kontakt 8","InstallDir"):("REG_SZ","old")},fail_apply=True)
            with self.assertRaisesRegex(ValueError,"registry_apply_failed"):k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry)
            self.assertEqual(k.digest(destination),old)
            for row in f.plan["files"][1:]:self.assertFalse((f.drive/pathlib.Path(*pathlib.PurePosixPath(row["destination"]).parts)).exists())
            result=json.loads((f.operation/"k8i1-transaction.private.json").read_bytes())
            self.assertEqual(result["state"],"failed");self.assertTrue(result["rollback_verified"])

    def test_unpublished_success_rolls_back_instead_of_leaving_payload(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);registry=k.MemoryRegistry()
            with self.assertRaisesRegex(ValueError,"generated_publish_failure"):
                k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry,
                                      confirm=lambda _:(_ for _ in ()).throw(ValueError("generated_publish_failure")))
            for row in f.plan["files"]:
                self.assertFalse((f.drive/pathlib.Path(*pathlib.PurePosixPath(row["destination"]).parts)).exists())
            self.assertEqual(registry.values,{})
            result=json.loads((f.operation/"k8i1-transaction.private.json").read_bytes())
            self.assertEqual(result["error"],"generated_publish_failure");self.assertTrue(result["rollback_verified"])

    def test_request_and_result_are_operation_bound(self):
        with tempfile.TemporaryDirectory() as raw:
            path=pathlib.Path(raw).resolve()/"request";op="a"*32;nonce="b"*64
            hx=lambda value:value.encode("utf-16le").hex()
            path.write_bytes(("\n".join(["K8I1_REQUEST_V1",op,nonce,"1","W","123","456",
                                         hx(r"C:\\Temp\\Kontakt 8 Setup PC.exe"),hx(r"C:\\Temp\\Kontakt 8 Setup PC.msi"),hx("PROPERTY=value"),""])).encode("utf-16le"))
            parsed=k.parse_request(path,op,nonce);self.assertEqual(parsed["windows_pid"],123);self.assertEqual(parsed["call"],"W")
            self.assertIn(b"v\x00e\x00r\x00i\x00f\x00i\x00e\x00d\x00",k.result_bytes(op,nonce,"c"*64,"d"*64))
            with self.assertRaises(ValueError):k.parse_request(path,"f"*32,nonce)

    def test_registry_protocol_and_runtime_port_are_closed(self):
        op="a"*32;nonce="b"*64;values={};calls=[]
        def invoke(action,row):
            calls.append((action,dict(row)));key=(row["root"],row["key"],row["name"])
            if action=="read":
                prior=values.get(key);return {"present":prior is not None,"type":prior[0] if prior else "none","value":prior[1] if prior else ""}
            if action=="write":values[key]=(row["type"],str(row["value"]))
            else:values.pop(key,None)
            return {"present":False,"type":"none","value":""}
        row={"root":"HKLM","key":r"Software\Native Instruments\Kontakt 8","name":"InstallDir",
             "type":"REG_SZ","value":"C:\\Program Files\\Native Instruments\\Kontakt 8\\","source_row":"1"}
        request=k.registry_request_bytes(op,nonce,"write",row).decode("utf-16le").splitlines()
        self.assertEqual(request[:4],["K8I1_REGISTRY_V1",op,nonce,"write"])
        empty=f"K8I1_REGISTRY_RESULT_V1 {op} {nonce} read 0 0 0 \n".encode()
        self.assertEqual(k.parse_registry_result(empty,op,nonce,"read"),{"present":False,"type":"none","value":""})
        port=k.RuntimeRegistry(invoke);snapshot=port.snapshot([row]);port.apply([row]);self.assertTrue(port.verify([row]))
        port.restore(snapshot);self.assertTrue(port.verify_snapshot(snapshot));self.assertEqual(values,{})
        self.assertEqual([call[0] for call in calls],["read","write","read","delete","read"])

    def test_prefix_arm_is_exact_and_reversible(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);adapter=k.Adapter.extract(f.authority,f.operation)
            system=f.prefix/"drive_c/windows/syswow64";system.mkdir(parents=True)
            original=b"original-wine-placeholder";(system/"msi.dll").write_bytes(original);(system/"msi.dll").chmod(0o444)
            registry=k.MemoryRegistry({("HKCU",k.PrefixArm.OVERRIDE["key"],"msi"):("REG_SZ","builtin")})
            arm=k.PrefixArm(adapter,f.authority,f.prefix,f.operation,"a"*32,"b"*64,
                            f.operation/"request",f.operation/"result",registry)
            arm.arm(lambda path:"Z:"+str(path).replace("/","\\"))
            self.assertEqual(k.digest(system/"msi.dll"),adapter.manifest["artifacts"]["msi.dll"]["sha256"])
            self.assertTrue(registry.verify([k.PrefixArm.OVERRIDE]));self.assertTrue((system/"k8i1.private").exists())
            arm.close();self.assertEqual((system/"msi.dll").read_bytes(),original)
            self.assertEqual((system/"msi.dll").stat().st_mode&0o777,0o444);self.assertFalse((system/"k8i1.private").exists())
            self.assertEqual(registry.values[("HKCU",k.PrefixArm.OVERRIDE["key"],"msi")],("REG_SZ","builtin"))
            arm.close();self.assertEqual((system/"msi.dll").read_bytes(),original)

    def test_exact_offline_requires_one_complete_package_tree(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);cache=f.root/"Temp/mia1";cache.mkdir(parents=True);msi=cache/k.MSI_BASENAME;msi.write_bytes(b"cached")
            target=f.root/"Temp/mia2/data/OFFLINE";target.parent.mkdir(parents=True);f.offline.rename(target)
            self.assertEqual(k.exact_offline(f.plan,msi),target)
            duplicate=f.root/"Temp/mia3/OFFLINE";duplicate.parent.mkdir(parents=True);__import__('shutil').copytree(target,duplicate)
            with self.assertRaisesRegex(ValueError,"ambiguous"):k.exact_offline(f.plan,msi)

    def test_cancelled_transaction_rolls_back(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);destination=f.drive/pathlib.Path(*pathlib.PurePosixPath(f.plan["files"][0]["destination"]).parts)
            destination.parent.mkdir(parents=True);destination.write_bytes(b"old");prior=k.digest(destination);calls=[0]
            def cancel():calls[0]+=1;return calls[0]>5
            with self.assertRaisesRegex(ValueError,"cancelled"):k.execute_transaction(f.plan,f.offline,f.drive,f.operation,k.MemoryRegistry(),cancel)
            self.assertEqual(k.digest(destination),prior)
            result=json.loads((f.operation/"k8i1-transaction.private.json").read_bytes());self.assertTrue(result["rollback_verified"])

    def test_plan_refuses_incomplete_or_ambiguous_mapping(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);manifest=json.loads(zipfile.ZipFile(f.archive).read("manifest.json"))
            bad=json.loads(json.dumps(f.plan));bad["files"].append(dict(bad["files"][0]));manifest["plan_sha256"]=hashlib.sha256(k.canonical(bad)).hexdigest()
            with self.assertRaisesRegex(ValueError,"duplicate"):k.validate_plan(bad,f.authority,manifest)
            bad=json.loads(json.dumps(f.plan));bad["tables"]["empty_or_absent"].remove("ServiceInstall");manifest["plan_sha256"]=hashlib.sha256(k.canonical(bad)).hexdigest()
            with self.assertRaisesRegex(ValueError,"tables"):k.validate_plan(bad,f.authority,manifest)

    def test_forwarder_roster_and_shim_have_no_hook_command(self):
        import subprocess,sys
        roster=ROOT/"tools/k8i1/pinned-msi-exports.txt";output=pathlib.Path(tempfile.mktemp())
        try:subprocess.run([sys.executable,str(ROOT/"tools/k8i1/generate_forwarders.py"),"--roster",str(roster),"--output",str(output)],check=True)
        finally:data=output.read_text() if output.exists() else "";output.unlink(missing_ok=True)
        self.assertEqual(len([line for line in data.splitlines()[2:] if "=msi_lvb_real." in line]),294)
        self.assertIn("MsiInstallProductA=_MsiInstallProductA@8 @87",data)
        self.assertIn("MsiInstallProductW=_MsiInstallProductW@8 @88",data)
        source=(ROOT/"tools/k8i1/msi_shim.c").read_text()
        for forbidden in ("CreateProcess", "ShellExecute", "WinExec", "WinHttp", "InternetOpen", "start.exe"):
            self.assertNotIn(forbidden,source)


if __name__ == "__main__": unittest.main()
