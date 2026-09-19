import hashlib
import json
import os
import pathlib
import tempfile
import unittest
import zipfile
import sys
from unittest.mock import patch

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import kontakt8 as k
ROOT = HERE.parents[1]
sys.path.insert(0,str(ROOT/"tools/k8i1"))
import compile_plan as compiler
sys.path.pop(0)


def pe32(payload=b"fixture",machine=0x14c):
    data = bytearray(256 + len(payload)); data[:2] = b"MZ"; data[60:64] = (128).to_bytes(4, "little")
    data[128:132] = b"PE\0\0"; data[132:134] = machine.to_bytes(2, "little"); data[256:] = payload
    return bytes(data)


def witness(data): return {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}
GENERATION={"windows_pid":123,"windows_created":456}


def write_idt(root,name,columns,rows):
    lines=["\t".join(columns),"\t".join("s255" for _ in columns),name+"\t"+"\t".join(columns[:1])]
    lines.extend("\t".join(str(row.get(column,"")) for column in columns) for row in rows)
    (root/f"{name}.idt").write_text("\n".join(lines)+"\n",encoding="utf-8")


def compiler_projection(root,offline):
    roster=sorted(k.COMPILER_REQUIRED_TABLES|{"CreateFolder","RemoveFile","Shortcut","Property"})
    (root/"_k8i1-tables.txt").write_text("\n".join(roster)+"\n",encoding="utf-8")
    tables={name:([],[]) for name in roster}
    tables["_Tables"]=(["Name"],[{"Name":name} for name in roster])
    tables["Directory"]=(["Directory","Directory_Parent","DefaultDir"],[
        {"Directory":"INSTALLDIR","Directory_Parent":"TARGETDIR","DefaultDir":".:OFFLINE"},
        {"Directory":"VST3DIR","Directory_Parent":"TARGETDIR","DefaultDir":".:OFFLINE"},
        {"Directory":"COMMONDIR","Directory_Parent":"TARGETDIR","DefaultDir":".:OFFLINE"}])
    tables["Component"]=(["Component","Directory_"],[
        {"Component":"app","Directory_":"INSTALLDIR"},{"Component":"vst3","Directory_":"VST3DIR"},
        {"Component":"common","Directory_":"COMMONDIR"}])
    tables["Feature"]=(["Feature"],[{"Feature":"f-app"},{"Feature":"f-vst3"},{"Feature":"f-common"}])
    tables["FeatureComponents"]=(["Feature_","Component_"],[
        {"Feature_":"f-app","Component_":"app"},{"Feature_":"f-vst3","Component_":"vst3"},
        {"Feature_":"f-common","Component_":"common"}])
    tables["File"]=(["File","Component_","FileName"],[
        {"File":"app-file","Component_":"app","FileName":"Kontakt 8.exe"},
        {"File":"vst-file","Component_":"vst3","FileName":"Kontakt 8.vst3"},
        {"File":"common-file","Component_":"common","FileName":"engine.dll"}])
    tables["Registry"]=(["Registry","Root","Key","Name","Value","Component_"],[
        {"Registry":"r"+str(index),"Root":"2","Key":r"Software\Native Instruments\Kontakt 8",
         "Name":name,"Value":value,"Component_":"common"}
        for index,(name,(_,value)) in enumerate(k.KONTAKT_REGISTRY.items())])
    tables["InstallExecuteSequence"]=(["Action","Condition","Sequence"],[
        {"Action":"PackageNotice","Condition":"NOT Installed","Sequence":"1200"}])
    tables["CustomAction"]=(["Action","Type","Source","Target"],[
        {"Action":"PackageNotice","Type":"19","Source":"","Target":"not executed by K8I1"}])
    tables["CreateFolder"]=(["Directory_","Component_"],[{"Directory_":"INSTALLDIR","Component_":"app"}])
    tables["RemoveFile"]=(["FileKey","Component_","FileName","DirProperty","InstallMode"],[])
    tables["Shortcut"]=(["Shortcut","Directory_","Name","Component_","Target","Arguments","Description",
                          "Hotkey","Icon_","IconIndex","ShowCmd","WkDir"],[
        {"Shortcut":"desktop","Directory_":"INSTALLDIR","Name":"Kontakt 8","Component_":"app",
         "Target":"[INSTALLDIR]Kontakt 8.exe","Arguments":"","Description":"","Hotkey":"0",
         "Icon_":"","IconIndex":"0","ShowCmd":"1","WkDir":"INSTALLDIR"}])
    tables["Property"]=(["Property","Value"],[
        {"Property":"ProductCode","Value":"{00000000-0000-0000-0000-000000000001}"},
        {"Property":"ProductVersion","Value":k.VERSION},
        {"Property":"ProductName","Value":"Kontakt 8 Player"}])
    for name,(columns,rows) in tables.items():write_idt(root,name,columns or ["Fixture"],rows)
    for name in ("Kontakt 8.exe","Kontakt 8.vst3","engine.dll"):
        path=offline/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(("payload:"+name).encode())
    return roster


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
        record=json.dumps({name:k.KONTAKT_REGISTRY[name][1]
                           for name in ("ContentDir","ContentVersion","InstallDir")},
                          sort_keys=True,separators=(",",":"))
        roster=sorted(k.COMPILER_REQUIRED_TABLES|{"CreateFolder","RemoveFile","Shortcut"})
        projections=[{"name":name,"sha256":hashlib.sha256(name.encode()).hexdigest(),"size":len(name),
                      "rows":0 if name in {"CustomAction","CreateFolder","RemoveFile","Shortcut"} else 1,
                      "columns":["fixture"]} for name in roster]
        properties={"INSTALLDIR":r"C:\Program Files\Native Instruments\Kontakt 8"}
        self.plan={"schema":2,"product":{"name":k.PRODUCT,"version":k.VERSION},
                   "setup":{"basename":k.SETUP_BASENAME,"sha256":k.SETUP_SHA256,"size":k.SETUP_SIZE},
                   "pristine_msi":{"basename":k.MSI_BASENAME,"sha256":k.PRISTINE_MSI_SHA256,"size":k.PRISTINE_MSI_SIZE},
                   "cached_msi":{"basename":k.MSI_BASENAME,"sha256":"2"*64,"size":4_222_976,"product_code":"{00000000-0000-0000-0000-000000000001}"},
                   "properties":{"observed":properties,"effective":properties,"sha256":hashlib.sha256(k.canonical(properties)).hexdigest()},
                   "tables":{"roster":[{"name":row["name"],"rows":row["rows"],
                              "disposition":"classified" if row["name"] in {"CustomAction","CreateFolder","RemoveFile","Shortcut"} else "derived"}
                              for row in projections],
                             "roster_sha256":"3"*64,"projection_sha256":hashlib.sha256(k.canonical(projections)).hexdigest(),"projections":projections},
                   "files":list(self.files.values()),
                   "state_files":[{"destination":"users/Public/Documents/Native Instruments/installed_products/Kontakt 8.json",
                                   "content_utf8":record,**witness(record.encode()),"class":"native_instruments_product_record",
                                   "derivation":{"registry_names":["ContentDir","ContentVersion","InstallDir"]}}],
                   "registry":[{"root":"HKLM","key":r"Software\Native Instruments\Kontakt 8","name":name,"type":kind,"value":value,"source_row":"r"+str(i),"component":"component-native_instruments_common"}
                               for i,(name,(kind,value)) in enumerate(k.KONTAKT_REGISTRY.items())],
                   "custom_actions":[],"mutations":{"create_folders":[],"remove_files":[],"shortcuts":[]},"omissions":[],
                   "source_tree_sha256":k.selected_source_tree_sha256(list(self.files.values()))}
        for row in self.plan["files"]:
            row.update(file_row="file-"+row["class"],directory="dir-"+row["class"],
                       feature_component_row=row["feature"]+"->"+row["component"])
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
                 "k8i1-registry.exe":pe32(b"registry",0x8664),"k8i1-setup-watch.exe":pe32(b"watch",0x8664),
                 "package-plan.json":k.canonical(self.plan),"THIRD_PARTY.txt":b"MIT and LGPL notices",
                 "LICENSE.ni-wine-MIT":b"MIT license fixture",
                 "SOURCE.json":k.canonical({"wine":k.WINE_BASE,"patches":k.WINE_PATCHES})}
        compiler={"schema":1,"kind":"k8i1-package-compiler-result","compiler":{"fixture":True},
                  "plan":self.plan,"plan_sha256":hashlib.sha256(k.canonical(self.plan)).hexdigest()}
        compiler_digest=hashlib.sha256(k.canonical(compiler)).hexdigest()
        proof={"schema":1,"kind":"k8i1-disposable-prefix-proof","compiler_result_sha256":compiler_digest,
               "candidate":{name:hashlib.sha256(entries[name]).hexdigest() for name in
                    ("msi.dll","msi_lvb_real.dll","k8i1-registry.exe","k8i1-setup-watch.exe","SOURCE.json")},
               "success":{"rewritten_tables":{"_Tables":True,"Directory":True,"File":True},
                    "intercept_count":1,"setup_exit":0,"payload_verified":True,"post_setup_state_verified":True,
                    "native_access_recognition_after_refresh":True,"native_access_recognition_after_cold_reopen":True,
                    "standalone_launch":True,"vst3_enumerated":True,"cleanup_confirmed":True},
               "forced_failure":{"intercept_count":1,"setup_exit":1603,"rollback_verified":True,
                    "payload_absent":True,"product_record_absent":True,"retryable":True}}
        proof_digest=hashlib.sha256(k.canonical(proof)).hexdigest()
        entries.update({"compiler-result.json":k.canonical(compiler),"disposable-proof.json":k.canonical(proof)})
        manifest={"schema":1,"kind":"k8i1-exact-package-adapter","product":self.plan["product"],"setup":self.plan["setup"],
                  "pristine_msi":self.plan["pristine_msi"],"cached_msi":self.plan["cached_msi"],
                  "wine":{"base":k.WINE_BASE,"patches":k.WINE_PATCHES},
                  "exports":{"count":296,"ordinal_first":5,"ordinal_last":300,"intercepted":["MsiInstallProductA","MsiInstallProductW"],
                             "spec_sha256":"5a6085ce66f541d3c52552164ade070129ea05a0f8a8b1ed2c660132366aceb0"},
                  "artifacts":{},"plan_sha256":hashlib.sha256(k.canonical(self.plan)).hexdigest(),
                  "compiler_result_sha256":compiler_digest,"disposable_proof_sha256":proof_digest,
                  "source":{"head":"d"*40,"tree":"e"*40}}
        entries["manifest.json"]=b""
        manifest["artifacts"]={name:(witness(data) if name!="manifest.json" else {"sha256":"0"*64,"size":0}) for name,data in entries.items()}
        entries["manifest.json"]=k.canonical(manifest)
        with zipfile.ZipFile(self.archive,"w",compression=zipfile.ZIP_STORED) as archive:
            for name,data in entries.items():archive.writestr(name,data)
        self.compiler_digest=compiler_digest;self.proof_digest=proof_digest

    def approved(self):
        return patch.multiple(k,APPROVED_ADAPTER_SHA256=k.digest(self.archive),
                              APPROVED_COMPILER_RESULT_SHA256=self.compiler_digest,
                              APPROVED_DISPOSABLE_PROOF_SHA256=self.proof_digest)


class Kontakt8Tests(unittest.TestCase):
    def test_package_compiler_derives_relations_complete_roster_actions_and_product_state(self):
        with tempfile.TemporaryDirectory() as raw:
            root=pathlib.Path(raw).resolve();projection=root/"tables";offline=root/"OFFLINE"
            projection.mkdir();offline.mkdir();roster=compiler_projection(projection,offline)
            properties=(r'INSTALLDIR="C:\Program Files\Native Instruments\Kontakt 8" '
                        r'VST3DIR="C:\Program Files\Common Files\VST3\Kontakt 8.vst3\Contents\x86_64-win" '
                        r'COMMONDIR="C:\Program Files\Common Files\Native Instruments\Kontakt 8"')
            plan=compiler.compile_projection(projection,offline,properties,
                {"basename":k.MSI_BASENAME,"sha256":"2"*64,"size":4_222_976})
            self.assertEqual(plan["cached_msi"]["product_code"],
                             "{00000000-0000-0000-0000-000000000001}")
            self.assertEqual([row["name"] for row in plan["tables"]["roster"]],roster)
            self.assertEqual({row["file_row"] for row in plan["files"]},{"app-file","vst-file","common-file"})
            self.assertEqual(plan["custom_actions"][0]["action"],"PackageNotice")
            self.assertEqual(plan["custom_actions"][0]["sequence"],1200)
            self.assertEqual(plan["mutations"]["create_folders"][0]["disposition"],"created_by_selected_file_plan")
            self.assertEqual(plan["mutations"]["shortcuts"][0]["disposition"],"omitted_nonessential_shortcut")
            state=json.loads(plan["state_files"][0]["content_utf8"])
            self.assertEqual(state,{name:k.KONTAKT_REGISTRY[name][1]
                                    for name in ("ContentDir","ContentVersion","InstallDir")})
            unknown="UnexpectedMutation";write_idt(projection,unknown,["Value"],[{"Value":"x"}])
            (projection/"_k8i1-tables.txt").write_text("\n".join(sorted(roster+[unknown]))+"\n")
            rows=[{"Name":name} for name in sorted(roster+[unknown])]
            write_idt(projection,"_Tables",["Name"],rows)
            with self.assertRaisesRegex(ValueError,"unclassified_table"):
                compiler.compile_projection(projection,offline,properties,
                    {"basename":k.MSI_BASENAME,"sha256":"2"*64,"size":4_222_976})

    def test_package_compiler_refuses_command_line_package_identity_override(self):
        with tempfile.TemporaryDirectory() as raw:
            root=pathlib.Path(raw).resolve();projection=root/"tables";offline=root/"OFFLINE"
            projection.mkdir();offline.mkdir();compiler_projection(projection,offline)
            with self.assertRaisesRegex(ValueError,"package_property_override"):
                compiler.compile_projection(projection,offline,
                    "PRODUCTCODE={11111111-1111-1111-1111-111111111111}",
                    {"basename":k.MSI_BASENAME,"sha256":"2"*64,"size":4_222_976})

    def test_production_adapter_is_source_gated_until_compiler_and_disposable_proof_are_pinned(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw)
            with self.assertRaisesRegex(ValueError,"adapter_identity"):k.validate_session(f.spec,f.authority)
            bad=json.loads(json.dumps(json.loads(zipfile.ZipFile(f.archive).read("disposable-proof.json"))))
            bad["success"]["native_access_recognition_after_cold_reopen"]=False
            with self.assertRaisesRegex(ValueError,"proof_success"):
                k.validate_disposable_proof(bad,f.compiler_digest,
                    {name:zipfile.ZipFile(f.archive).read(name) for name in
                     ("msi.dll","msi_lvb_real.dll","k8i1-registry.exe","k8i1-setup-watch.exe","SOURCE.json")})
            dummy=f.root/"dummy";dummy.write_bytes(b"x")
            command=[sys.executable,str(ROOT/"tools/k8i1/package.py")]
            for name in ("shim","real-msi","registry","setup-watch","compiler-result","disposable-proof",
                         "setup","pristine-msi","cached-msi","source"):
                command.extend(["--"+name,str(dummy)])
            command.extend(["--output",str(f.root/"must-not-exist.zip")])
            refused=__import__('subprocess').run(command,capture_output=True,text=True)
            self.assertNotEqual(refused.returncode,0);self.assertIn("production_sealing_not_qualified",refused.stderr)
            self.assertFalse((f.root/"must-not-exist.zip").exists())

    def test_closed_session_and_archive(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw)
            with f.approved():
                self.assertEqual(k.validate_session(f.spec,f.authority),f.authority)
                adapter=k.Adapter.extract(f.authority,f.operation);self.assertEqual(adapter.plan,f.plan)
                f.authority["setup"]["size"]+=1
                with self.assertRaises(ValueError):k.validate_session(f.spec,f.authority)

    def test_verified_transaction_and_existing_file_replacement(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);destination=f.drive/pathlib.Path(*pathlib.PurePosixPath(f.plan["files"][0]["destination"]).parts)
            destination.parent.mkdir(parents=True);destination.write_bytes(b"old")
            registry=k.MemoryRegistry();result=k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry,
                setup_generation=GENERATION)
            self.assertEqual(result["state"],"payload_deployment_verified");self.assertEqual(result["file_count"],4)
            terminal=k.finalize_transaction(f.plan,f.drive,f.operation,registry,
                {"generation":GENERATION,"wait":"signaled","query_error":0,"exit_code":0},True)
            self.assertEqual(terminal["state"],"completed")
            for row in f.plan["files"]:self.assertEqual(k.digest(f.drive/pathlib.Path(*pathlib.PurePosixPath(row["destination"]).parts)),row["sha256"])
            self.assertTrue(registry.verify(f.plan["registry"]))

    def test_failed_state_write_rolls_back_all_files_and_registry(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);destination=f.drive/pathlib.Path(*pathlib.PurePosixPath(f.plan["files"][0]["destination"]).parts)
            destination.parent.mkdir(parents=True);destination.write_bytes(b"old");old=k.digest(destination)
            registry=k.MemoryRegistry({("HKLM",r"Software\Native Instruments\Kontakt 8","InstallDir"):("REG_SZ","old")},fail_apply=True)
            with self.assertRaisesRegex(ValueError,"registry_apply_failed"):k.execute_transaction(
                f.plan,f.offline,f.drive,f.operation,registry,setup_generation=GENERATION)
            self.assertEqual(k.digest(destination),old)
            for row in f.plan["files"][1:]:self.assertFalse((f.drive/pathlib.Path(*pathlib.PurePosixPath(row["destination"]).parts)).exists())
            result=json.loads((f.operation/"k8i1-transaction.private.json").read_bytes())
            self.assertEqual(result["state"],"failed");self.assertTrue(result["rollback_verified"])

    def test_post_wrapper_change_to_existing_or_new_product_file_rolls_back(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);product=f.drive/"Program Files/Native Instruments/Kontakt 8"
            product.mkdir(parents=True);existing=product/"existing-from-prior-install.dat"
            existing.write_bytes(b"prior bytes");prior=k._metadata(existing)
            registry=k.MemoryRegistry()
            k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry,
                setup_generation=GENERATION)
            existing.write_bytes(b"wrapper changed this")
            unexpected=product/"wrapper-created.tmp";unexpected.write_bytes(b"unexpected")
            with self.assertRaisesRegex(ValueError,"surface_changed"):
                k.finalize_transaction(f.plan,f.drive,f.operation,registry,
                    {"generation":GENERATION,"wait":"signaled","query_error":0,"exit_code":0},True)
            self.assertTrue(k._same_file_identity(k._metadata(existing),prior))
            self.assertFalse(unexpected.exists())
            self.assertEqual(registry.values,{})

    def test_unpublished_success_rolls_back_instead_of_leaving_payload(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);registry=k.MemoryRegistry()
            with self.assertRaisesRegex(ValueError,"generated_publish_failure"):
                k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry,
                                      confirm=lambda _:(_ for _ in ()).throw(ValueError("generated_publish_failure")),
                                      setup_generation=GENERATION)
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
            f=Fixture(raw)
            with f.approved():adapter=k.Adapter.extract(f.authority,f.operation)
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
            with self.assertRaisesRegex(ValueError,"cancelled"):k.execute_transaction(
                f.plan,f.offline,f.drive,f.operation,k.MemoryRegistry(),cancel,setup_generation=GENERATION)
            self.assertEqual(k.digest(destination),prior)
            result=json.loads((f.operation/"k8i1-transaction.private.json").read_bytes());self.assertTrue(result["rollback_verified"])

    def test_transaction_fault_boundaries_restore_prior_state_and_recovery_requires_exact_setup_exit(self):
        points=("after_journal_prepare","after_file_intent","after_file_commit","after_registry_intent",
                "after_registry_commit","after_shim_success_intent")
        for point in points:
            with self.subTest(point=point),tempfile.TemporaryDirectory() as raw:
                f=Fixture(raw);target=f.drive/"Program Files/Native Instruments/Kontakt 8/Kontakt 8.exe"
                target.parent.mkdir(parents=True);target.write_bytes(b"old");target.chmod(0o440)
                prior=k._metadata(target);registry=k.MemoryRegistry();triggered=False
                def fault(observed):
                    nonlocal triggered
                    if not triggered and observed==point:triggered=True;raise ValueError("generated_fault_"+point)
                with self.assertRaisesRegex(ValueError,"generated_fault"):
                    k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry,fault=fault,
                                          setup_generation=GENERATION)
                self.assertTrue(triggered);self.assertEqual(target.lstat().st_atime_ns,prior["atime_ns"])
                self.assertTrue(k._same_file_identity(k._metadata(target),prior));self.assertEqual(registry.values,{})
                result=json.loads((f.operation/"k8i1-transaction.private.json").read_bytes())
                self.assertEqual(result["state"],"failed");self.assertTrue(result["rollback_verified"])
        class Crash(BaseException):pass
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);registry=k.MemoryRegistry()
            def crash(point):
                if point=="after_shim_result_publication":raise Crash()
            with self.assertRaises(Crash):
                k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry,
                    confirm=lambda _:{"receipt_sha256":"a"*64,"result_sha256":"b"*64},fault=crash,
                    setup_generation=GENERATION)
            with self.assertRaisesRegex(ValueError,"setup_not_terminal"):
                k.recover_interrupted_transaction(f.drive,f.operation,registry,
                    {"generation":{"windows_pid":999,"windows_created":456},"wait":"signaled","query_error":0,"exit_code":0},True)
            recovered=k.recover_interrupted_transaction(f.drive,f.operation,registry,
                {"generation":GENERATION,"wait":"signaled","query_error":0,"exit_code":1603},True)
            self.assertTrue(recovered["rollback_verified"]);self.assertEqual(registry.values,{})
            for row in f.plan["files"]:
                self.assertFalse((f.drive/pathlib.Path(*pathlib.PurePosixPath(row["destination"]).parts)).exists())

    def test_rollback_faults_leave_a_restartable_journal(self):
        for point in ("after_registry_restore","after_file_restore","after_surface_restore"):
            with self.subTest(point=point),tempfile.TemporaryDirectory() as raw:
                f=Fixture(raw);registry=k.MemoryRegistry()
                k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry,
                    confirm=lambda _:{"receipt_sha256":"a"*64,"result_sha256":"b"*64},
                    setup_generation=GENERATION)
                triggered=False
                def fault(observed):
                    nonlocal triggered
                    if not triggered and observed==point:triggered=True;raise ValueError("generated_rollback_fault")
                with self.assertRaisesRegex(ValueError,"setup_terminal_adverse"):
                    k.finalize_transaction(f.plan,f.drive,f.operation,registry,
                        {"generation":GENERATION,"wait":"signaled","query_error":0,"exit_code":1603},True,fault)
                self.assertTrue(triggered)
                journal=json.loads((k._transaction_root(f.drive,f.operation)/"journal.json").read_bytes())
                self.assertEqual(journal["state"],"rollback_unconfirmed")
                recovered=k.recover_interrupted_transaction(f.drive,f.operation,registry,
                    {"generation":GENERATION,"wait":"signaled","query_error":0,"exit_code":1603},True)
                self.assertTrue(recovered["rollback_verified"]);self.assertEqual(registry.values,{})

    def test_terminal_publication_faults_resume_to_rollback_or_exact_completion(self):
        class Crash(BaseException):pass
        for point in ("after_terminal_intent","after_terminal_commit"):
            with self.subTest(point=point),tempfile.TemporaryDirectory() as raw:
                f=Fixture(raw);registry=k.MemoryRegistry()
                k.execute_transaction(f.plan,f.offline,f.drive,f.operation,registry,
                    confirm=lambda _:{"receipt_sha256":"a"*64,"result_sha256":"b"*64},
                    setup_generation=GENERATION)
                def crash(observed):
                    if observed==point:raise Crash()
                with self.assertRaises(Crash):
                    k.finalize_transaction(f.plan,f.drive,f.operation,registry,
                        {"generation":GENERATION,"wait":"signaled","query_error":0,"exit_code":0},True,crash)
                recovered=k.recover_interrupted_transaction(f.drive,f.operation,registry,
                    {"generation":GENERATION,"wait":"signaled","query_error":0,"exit_code":0},True)
                if point=="after_terminal_commit":
                    self.assertEqual(recovered["state"],"completed")
                else:
                    self.assertEqual(recovered["state"],"rolled_back")

    def test_arm_fault_boundaries_are_restartably_reversible(self):
        points=("after_arm_journal_prepare","after_arm_intent","after_arm_commit",
                "after_arm_registry_intent","after_arm_registry_commit")
        for point in points:
            with self.subTest(point=point),tempfile.TemporaryDirectory() as raw:
                f=Fixture(raw)
                with f.approved():adapter=k.Adapter.extract(f.authority,f.operation)
                system=f.prefix/"drive_c/windows/syswow64";system.mkdir(parents=True)
                original=b"original";(system/"msi.dll").write_bytes(original);(system/"msi.dll").chmod(0o444)
                prior=k._metadata(system/"msi.dll")
                registry=k.MemoryRegistry({("HKCU",k.PrefixArm.OVERRIDE["key"],"msi"):("REG_SZ","builtin")})
                arm=k.PrefixArm(adapter,f.authority,f.prefix,f.operation,"a"*32,"b"*64,
                    f.operation/"request",f.operation/"result",registry)
                triggered=False
                def fault(observed):
                    nonlocal triggered
                    if not triggered and observed==point:triggered=True;raise ValueError("generated_arm_fault")
                with self.assertRaisesRegex(ValueError,"generated_arm_fault|k8i1_disarm_failed"):
                    arm.arm(lambda path:"Z:"+str(path).replace("/","\\"),fault)
                self.assertTrue(triggered)
                # A fresh owner can finish the same journal after interruption.
                recovered=k.PrefixArm(adapter,f.authority,f.prefix,f.operation,"a"*32,"b"*64,
                    f.operation/"request",f.operation/"result",registry)
                recovered.close()
                self.assertEqual((system/"msi.dll").lstat().st_atime_ns,prior["atime_ns"])
                self.assertTrue(k._same_file_identity(k._metadata(system/"msi.dll"),prior))
                self.assertEqual(registry.values[("HKCU",k.PrefixArm.OVERRIDE["key"],"msi")],("REG_SZ","builtin"))

    def test_disarm_fault_boundaries_resume_from_prefix_journal(self):
        for point in ("after_arm_registry_restore","after_arm_file_restore"):
            with self.subTest(point=point),tempfile.TemporaryDirectory() as raw:
                f=Fixture(raw)
                with f.approved():adapter=k.Adapter.extract(f.authority,f.operation)
                system=f.prefix/"drive_c/windows/syswow64";system.mkdir(parents=True)
                original=b"original";(system/"msi.dll").write_bytes(original);prior=k._metadata(system/"msi.dll")
                registry=k.MemoryRegistry();arm=k.PrefixArm(adapter,f.authority,f.prefix,f.operation,
                    "a"*32,"b"*64,f.operation/"request",f.operation/"result",registry)
                arm.arm(lambda path:"Z:"+str(path).replace("/","\\"));triggered=False
                def fault(observed):
                    nonlocal triggered
                    if not triggered and observed==point:triggered=True;raise ValueError("generated_disarm_fault")
                with self.assertRaisesRegex(ValueError,"k8i1_disarm_failed"):
                    arm.close(fault)
                self.assertTrue(triggered)
                recovered=k.PrefixArm(adapter,f.authority,f.prefix,f.operation,
                    "a"*32,"b"*64,f.operation/"request",f.operation/"result",registry)
                recovered.close();self.assertEqual((system/"msi.dll").lstat().st_atime_ns,prior["atime_ns"])
                self.assertTrue(k._same_file_identity(k._metadata(system/"msi.dll"),prior))
                self.assertFalse((system/"k8i1.private").exists())
        class Crash(BaseException):pass
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw)
            with f.approved():adapter=k.Adapter.extract(f.authority,f.operation)
            system=f.prefix/"drive_c/windows/syswow64";system.mkdir(parents=True)
            (system/"msi.dll").write_bytes(b"original");prior=k._metadata(system/"msi.dll")
            registry=k.MemoryRegistry();arm=k.PrefixArm(adapter,f.authority,f.prefix,f.operation,
                "a"*32,"b"*64,f.operation/"request",f.operation/"result",registry)
            arm.arm(lambda path:"Z:"+str(path).replace("/","\\"))
            with self.assertRaises(Crash):
                arm.close(lambda point:(_ for _ in ()).throw(Crash()) if point=="after_arm_disarm_result" else None)
            recovered=k.PrefixArm(adapter,f.authority,f.prefix,f.operation,
                "a"*32,"b"*64,f.operation/"request",f.operation/"result",registry)
            recovered.close();self.assertEqual((system/"msi.dll").lstat().st_atime_ns,prior["atime_ns"])
            self.assertTrue(k._same_file_identity(k._metadata(system/"msi.dll"),prior))

    def test_plan_refuses_incomplete_or_ambiguous_mapping(self):
        with tempfile.TemporaryDirectory() as raw:
            f=Fixture(raw);manifest=json.loads(zipfile.ZipFile(f.archive).read("manifest.json"))
            bad=json.loads(json.dumps(f.plan));bad["files"].append(dict(bad["files"][0]));manifest["plan_sha256"]=hashlib.sha256(k.canonical(bad)).hexdigest()
            with self.assertRaisesRegex(ValueError,"duplicate"):k.validate_plan(bad,f.authority,manifest)
            bad=json.loads(json.dumps(f.plan));bad["tables"]["roster"][0]["rows"]=-1;manifest["plan_sha256"]=hashlib.sha256(k.canonical(bad)).hexdigest()
            with self.assertRaisesRegex(ValueError,"tables"):k.validate_plan(bad,f.authority,manifest)

    def test_forwarder_roster_and_shim_have_no_hook_command(self):
        import subprocess,sys
        roster=ROOT/"tools/k8i1/pinned-msi-exports.txt";output=pathlib.Path(tempfile.mktemp());real=pathlib.Path(tempfile.mktemp())
        try:subprocess.run([sys.executable,str(ROOT/"tools/k8i1/generate_forwarders.py"),"--roster",str(roster),"--output",str(output),"--real-output",str(real)],check=True)
        finally:
            data=output.read_text() if output.exists() else "";real_data=real.read_text() if real.exists() else ""
            output.unlink(missing_ok=True);real.unlink(missing_ok=True)
        self.assertEqual(len([line for line in data.splitlines()[2:] if "=msi_lvb_real." in line]),294)
        self.assertIn("MsiInstallProductA=_MsiInstallProductA@8 @87",data)
        self.assertIn("MsiInstallProductW=_MsiInstallProductW@8 @88",data)
        self.assertEqual(real_data.splitlines()[:2],["LIBRARY msi_lvb_real.dll","EXPORTS"])
        self.assertEqual(len(real_data.splitlines()[2:]),296)
        source=(ROOT/"tools/k8i1/msi_shim.c").read_text()
        for forbidden in ("CreateProcess", "ShellExecute", "WinExec", "WinHttp", "InternetOpen", "start.exe"):
            self.assertNotIn(forbidden,source)

    def test_windows_export_audit_distinguishes_forwarders_from_intercepts(self):
        sys.path.insert(0,str(ROOT/"tools/k8i1"))
        try:import windows_test
        finally:sys.path.pop(0)
        observed=windows_test.export_rows("""
              5    0          MsiAdvertiseProductA (forwarded to msi_lvb_real.MsiAdvertiseProductA)
             87   52 00001000 MsiInstallProductA
        """)
        self.assertEqual(observed,{5:("MsiAdvertiseProductA","msi_lvb_real.MsiAdvertiseProductA"),87:("MsiInstallProductA",None)})


if __name__ == "__main__": unittest.main()
