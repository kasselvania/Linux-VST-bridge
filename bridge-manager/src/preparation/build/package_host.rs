//! Initial inspection uses the package's canonical host and its kit-owned support.
use super::*;
const HELPERS: [&str;4] = ["lvb-direct-wait.dll", "x86_64-windows/lvb-direct-wait.dll",
    "x86_64-unix/lvb-direct-wait.so", "direct-audio-helper.json"];
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HostRecipe {
    host_sha256:String,
    source_sha256:String,
    files:std::collections::BTreeMap<String,String>,
}

// One closed kit reader/unpacker serves preparation and initial package staging.
// Record mode reads only bounded control members; it never unpacks or repairs.
pub(super) fn unpack(kit:&Artifact, stage:Option<&Path>, purpose:&str) -> Result<Option<HostRecipe>> {
    let script=r#"import sys,json,zipfile,pathlib,hashlib,re
kit,out,purpose=sys.argv[1:];out=pathlib.Path(out)
with zipfile.ZipFile(kit) as z:
 assert z.getinfo('recipe.json').file_size<=65536
 recipe=json.loads(z.read('recipe.json'));assert recipe['schema'] in (1,2,3,4)
 sha=lambda b:hashlib.sha256(b).hexdigest()
 helpers=['lvb-direct-wait.dll','x86_64-windows/lvb-direct-wait.dll','x86_64-unix/lvb-direct-wait.so','direct-audio-helper.json']
 present=[('runtime/'+n) in recipe['files'] for n in helpers]
 assert not any(present) or all(present)
 result=None
 if purpose in ('package','package-record'):
  if not all(present):print('null');sys.exit(0)
  key='runtime/direct-audio-helper.json';assert z.getinfo(key).file_size<=65536
  data=z.read(key);assert sha(data)==recipe['files'][key]
  helper=json.loads(data);assert type(helper['schema']) is int and helper['schema'] in (1,2)
  if helper['schema']==1:print('null');sys.exit(0)
  assert helper['abi']==1 and re.fullmatch('[a-f0-9]{64}',helper['runner_build_sha256'])
  assert re.fullmatch('[a-f0-9]{40}',helper['runner_wine_revision'])
  assert set(helper['files'])==set(helpers[:3])
  assert all(helper['files'][n]==recipe['files']['runtime/'+n] for n in helpers[:3])
  result=dict(host_sha256=recipe['files']['runtime/host.exe'],source_sha256=recipe['files']['runtime/host-source-manifest.json'],files={n:recipe['files']['runtime/'+n] for n in helpers})
  assert helper['host_sha256']==result['host_sha256']
  assert all(re.fullmatch('[a-f0-9]{64}',v) for v in [result['host_sha256'],result['source_sha256'],*result['files'].values()])
  if purpose=='package-record':print(json.dumps(result));sys.exit(0)
  # The signed package host/source stay canonical. A kit cannot replace them.
  for name,key in [('host.exe','host_sha256'),('host-source-manifest.json','source_sha256')]:
   assert sha((out/name).read_bytes())==result[key]
  names=[('runtime/'+n,n) for n in helpers]
 else:
  assert purpose=='preparation'
  names=[('runtime/host.exe','host.exe'),('runtime/host-source-manifest.json','host-source-manifest.json')]
  if recipe['schema'] in (2,3,4):names += [('tools/mf3/native_builder.py','native_builder.py'),('tools/ap8_descriptor.py','ap8_descriptor.py')]
  if all(present):names += [('runtime/'+n,n) for n in helpers]
  launch=['installer-launch.exe','installer-launch-source.json']
  present=[('runtime/'+n) in recipe['files'] for n in launch]
  assert not any(present) or all(present)
  if all(present):names += [('runtime/'+n,n) for n in launch]
 for key,name in names:
  i=z.getinfo(key);assert not i.is_dir() and i.file_size<=64*1024*1024
  b=z.read(i);assert sha(b)==recipe['files'][key]
  (out/name).parent.mkdir(parents=True,exist_ok=True,mode=0o700)
  with (out/name).open('xb') as f:f.write(b)
 print(json.dumps(result))
"#;
    let mut child=Command::new("python3").args(["-I","-c",script]).arg(&kit.path)
        .arg(stage.unwrap_or(Path::new("-"))).arg(purpose)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let deadline=Instant::now()+Duration::from_secs(30);
    let status=loop {
        if let Some(status)=child.try_wait()? {break status;}
        if Instant::now()>=deadline {child.kill()?;child.wait()?;return Err("preparation_runtime_unpack_deadline".into());}
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut output=vec![];child.stdout.take().ok_or("preparation_runtime_output")?
        .take(8193).read_to_end(&mut output)?;
    require(status.success() && output.len()<=8192,"preparation_runtime_package_incomplete")?;
    Ok(serde_json::from_slice(&output)?)
}

fn package_runtime(kit:Artifact, host:Artifact, source_manifest:Artifact,
    recipe:HostRecipe) -> Result<Runtime> {
    let dir=host.path.parent().ok_or("package_host_runtime_path")?;
    require(source_manifest.path==dir.join("host-source-manifest.json")
        && host.path==dir.join("host.exe") && kit.path==dir.join("preparation-kit.zip")
        && host.sha256==recipe.host_sha256 && source_manifest.sha256==recipe.source_sha256
        && recipe.files.len()==HELPERS.len(),"package_host_runtime_binding")?;
    let direct_audio_helpers=HELPERS.into_iter().map(|name| {
        Ok(Artifact {path:dir.join(name),sha256:recipe.files.get(name)
            .ok_or("package_host_runtime_helper")?.clone()})
    }).collect::<Result<Vec<_>>>()?;
    Ok(Runtime {kit,host,source_manifest,builder:None,generator:None,direct_audio_helpers})
}

/// Called only while the package generation is still unpublished. The common
/// helper is derived from the sealed kit, never a second execution selection.
pub(crate) fn stage_package_host(kit:&Artifact, stage:&Path, final_root:&Path,
    host_sha:&str, source_sha:&str) -> Result<()> {
    let Some(recipe)=unpack(kit,Some(stage),"package")? else {return Ok(());};
    kit.verify()?;
    let runtime=package_runtime(Artifact {path:final_root.join("preparation-kit.zip"),sha256:kit.sha256.clone()},
        Artifact {path:final_root.join("host.exe"),sha256:host_sha.into()},
        Artifact {path:final_root.join("host-source-manifest.json"),sha256:source_sha.into()},recipe)?;
    for a in &runtime.direct_audio_helpers {
        let path=stage.join(a.path.strip_prefix(final_root)?);
        require(digest(&path)?==a.sha256,"package_host_helper_changed")?;
        fs::set_permissions(&path,fs::Permissions::from_mode(0o400))?;file(&path)?.sync_all()?;
    }
    for name in ["x86_64-windows","x86_64-unix"] {File::open(stage.join(name))?.sync_all()?;}
    immutable(&stage.join("runtime.json"),&runtime)
}

/// Recheck support against the sealed kit. Readback checks bounded records,
/// exact paths and modes without hashing/unpacking the runtime payload.
pub(crate) fn verify_package_host(software:&crate::catalogue::Software, full:bool) -> Result<()> {
    let Some(kit)=software.preparation_kit.as_ref() else {return Ok(());};
    let Some(recipe)=unpack(kit,None,"package-record")? else {return Ok(());};
    let expected=package_runtime(kit.clone(),software.host.clone(),software.source_manifest.clone(),recipe)?;
    let dir=software.host.path.parent().ok_or("package_host_runtime_path")?;
    let record=dir.join("runtime.json");
    require(file(&record)?.metadata()?.len()<=16384,"package_host_runtime_extent")?;
    let actual:Runtime=bounded(&record)?;
    require(actual==expected,"package_host_runtime_changed")?;
    for path in std::iter::once(&record).chain(actual.direct_audio_helpers.iter().map(|a|&a.path)) {
        require(path.canonicalize()?==*path && file(path)?.metadata()?.mode()&0o222==0,
            "package_host_runtime_mutability")?;
    }
    for a in actual.direct_audio_helpers {
        if full {a.verify()?;} else {a.validate_record()?;}
    }
    Ok(())
}
