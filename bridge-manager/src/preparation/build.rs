//! Offline preparation from the installed kit, outside the audio path and registry lock.
use super::*;
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};
mod package_host;
pub use package_host::{stage_package_host, verify_package_host};
pub fn recipe_available(m: &Manager) -> Result<Artifact> {
    let sw: crate::catalogue::Software = read_json(&m.root.join("software.json"))?;
    let kit=sw.preparation_kit.ok_or("Preparation support is missing. Install the complete manager package")?;
    require(
        kit.path.starts_with(m.root.join("software"))
            && kit.path.canonicalize()? == kit.path
            && file(&kit.path)?.metadata()?.mode() & 0o222 == 0,
        "preparation_kit_authority",
    )?;
    require(
        file(&kit.path)?.metadata()?.len() <= 256 * 1024 * 1024,
        "preparation_kit_size",
    )?;
    Ok(kit)
}
pub fn recipe(m: &Manager) -> Result<Artifact> {
    let kit = recipe_available(m)?;
    kit.verify()?;
    Ok(kit)
}
pub fn recipe_for_software(m: &Manager, software: &crate::catalogue::Software) -> Result<Artifact> {
    let kit = software.preparation_kit.clone()
        .ok_or("Preparation support is missing. Install the complete manager package")?;
    require(kit.path.starts_with(m.root.join("software"))
        && kit.path.canonicalize()? == kit.path
        && file(&kit.path)?.metadata()?.mode() & 0o222 == 0,
        "preparation_kit_authority")?;
    require(file(&kit.path)?.metadata()?.len() <= 256 * 1024 * 1024,
        "preparation_kit_size")?;
    kit.verify()?;
    Ok(kit)
}
/// Resolve this selected generation's complete launch execution set. The signed
/// kit owns optional launch bytes; legacy Runtime and selected Software stay intact.
pub fn launch_software(m:&Manager)->Result<crate::catalogue::Software> {
    let mut selected:crate::catalogue::Software=read_json(&m.root.join("software.json"))?;
    selected.validate_record()?;
    for artifact in [&selected.manager,&selected.supervisor,&selected.ownership,
        &selected.host,&selected.source_manifest] {artifact.verify()?;}
    if let Some(adapter)=&selected.installer_launch {adapter.verify()?;return Ok(selected);}
    let kit=recipe_for_software(m,&selected)?;
    let script=r#"import sys,json,zipfile,re
with zipfile.ZipFile(sys.argv[1]) as z:
 assert z.getinfo('recipe.json').file_size<=65536
 r=json.loads(z.read('recipe.json'));assert r['schema']==4
 name='runtime/installer-launch.exe'
 value=r['files'].get(name)
 if value is not None:
  value={n:r['files']['runtime/'+n] for n in ['installer-launch.exe','installer-launch-source.json']}
  assert all(re.fullmatch('[a-f0-9]{64}',v) for v in value.values())
 print(json.dumps(value))
"#;
    let mut child=Command::new("python3").args(["-I","-c",script])
        .arg(&kit.path).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let deadline=Instant::now()+Duration::from_secs(3);
    let status=loop {
        if let Some(status)=child.try_wait()? {break status;}
        if Instant::now()>=deadline {child.kill()?;child.wait()?;return Err("launch_component_read_deadline".into());}
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut bytes=vec![];child.stdout.take().ok_or("launch_component_output")?.take(513).read_to_end(&mut bytes)?;
    require(status.success() && bytes.len()<=512,"launch_component_recipe_invalid")?;
    let hashes:Option<std::collections::BTreeMap<String,String>>=serde_json::from_slice(&bytes)?;
    let hashes=hashes.ok_or("The managed application launch component is missing. Install the complete bridge update.")?;
    let runtime=stage_runtime_for_software(m,&selected)?;
    stage_launch_files(m,&kit,&hashes)?;
    let adapter=Artifact {path:runtime.host.path.with_file_name("installer-launch.exe"),sha256:hashes["installer-launch.exe"].clone()};
    require(adapter.path.canonicalize()?==adapter.path
        && file(&adapter.path)?.metadata()?.mode()&0o222==0,"launch_component_not_immutable")?;
    adapter.verify()?;selected.installer_launch=Some(adapter);Ok(selected)
}

// Old readers can stage this same kit without knowing its optional launch
// members. Complete only those missing files under the existing kit owner;
// never rewrite runtime.json or replace a changed retained image.
fn stage_launch_files(m:&Manager,kit:&Artifact,hashes:&std::collections::BTreeMap<String,String>)->Result<()> {
    let _lock=m.lock("preparation-kit.lock")?;
    let dir=runtime_dir(m,&kit.sha256)?;
    require(dir.canonicalize()?==dir,"launch_component_directory_alias")?;
    let mut missing=vec![];
    for (name,sha) in hashes {
        require(matches!(name.as_str(),"installer-launch.exe"|"installer-launch-source.json")
            && valid_hex(sha,64),"launch_component_recipe_invalid")?;
        let path=dir.join(name);
        if path.try_exists()? {
            require(path.canonicalize()?==path && file(&path)?.metadata()?.mode()&0o222==0,
                "launch_component_not_immutable")?;
            Artifact{path,sha256:sha.clone()}.verify()?;
        } else {missing.push(name.clone());}
    }
    if missing.is_empty(){return Ok(());}
    let stage=dir.with_file_name(format!(".launch-{}",random_id()?));private_dir(&stage)?;
    let script=r#"import sys,json,zipfile,pathlib,hashlib
kit,out,names=sys.argv[1:];out=pathlib.Path(out)
with zipfile.ZipFile(kit) as z:
 assert z.getinfo('recipe.json').file_size<=65536
 r=json.loads(z.read('recipe.json'));assert r['schema']==4
 for name in json.loads(names):
  key='runtime/'+name;i=z.getinfo(key);assert not i.is_dir() and i.file_size<=64*1024*1024
  b=z.read(i);assert hashlib.sha256(b).hexdigest()==r['files'][key]
  (out/name).write_bytes(b)
"#;
    let result=(||->Result<()> {
        let mut child=Command::new("python3").args(["-I","-c",script]).arg(&kit.path).arg(&stage)
            .arg(serde_json::to_string(&missing)?).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
        let deadline=Instant::now()+Duration::from_secs(30);
        let status=loop {
            if let Some(status)=child.try_wait()?{break status;}
            if Instant::now()>=deadline{child.kill()?;child.wait()?;return Err("launch_component_unpack_deadline".into());}
            std::thread::sleep(Duration::from_millis(20));
        };
        require(status.success(),"launch_component_package_incomplete")?;kit.verify()?;
        for name in &missing {
            let path=stage.join(name);Artifact{path:path.clone(),sha256:hashes[name].clone()}.verify()?;
            fs::set_permissions(&path,fs::Permissions::from_mode(0o400))?;file(&path)?.sync_all()?;
            crate::publication::rename_link(&path,&dir.join(name),false)?;
        }
        File::open(&dir)?.sync_all()?;Ok(())
    })();
    fs::remove_dir_all(&stage)?;result
}

/// Historical preparation readback only. Mutation independently verifies the
/// signed kit and every launch byte through launch_software.
pub fn launch_software_record(m:&Manager)->Result<crate::catalogue::Software> {
    let mut selected:crate::catalogue::Software=read_json(&m.root.join("software.json"))?;
    selected.validate_record()?;
    if selected.installer_launch.is_some(){return Ok(selected);}
    let kit=selected.preparation_kit.as_ref().ok_or("launch_component_record_missing")?;
    let dir=runtime_dir(m,&kit.sha256)?;
    let path=dir.join("installer-launch-source.json");
    require(path.canonicalize()?==path && file(&path)?.metadata()?.mode()&0o222==0,"launch_component_record_authority")?;
    let source:Value=bounded(&path)?;
    require(source["schema"]==1 && source["component"]=="installer_launch","launch_component_record_identity")?;
    let sha=source["artifact_sha256"].as_str().filter(|s|valid_hex(s,64)).ok_or("launch_component_record_identity")?;
    let adapter=Artifact{path:dir.join("installer-launch.exe"),sha256:sha.into()};adapter.validate_record()?;
    require(adapter.path.canonicalize()?==adapter.path && file(&adapter.path)?.metadata()?.mode()&0o222==0,
        "launch_component_record_authority")?;
    selected.installer_launch=Some(adapter);Ok(selected)
}
/// A verified kit must bind this exact native binary as well as module/class.
/// An older proxy cannot acquire a larger envelope from a newer manager alone.
pub fn maximum_bridge_frames(m: &Manager, r: &Registration) -> Result<Option<u32>> {
    if let Some(maximum) = current_kit_maximum(m, r)? { return Ok(Some(maximum)); }
    // A changed kit describes its own proxies, not the capacity of a retained
    // publication. Follow that publication's exact candidate recipe, never an
    // ambient kit search or the successor's module/class match alone.
    let registry = m.registry()?;
    let Some(entry) = registry.classes.get(&r.metadata.class_id)
        .filter(|entry| entry.registration == *r) else { return Ok(None); };
    let Some(reference) = &entry.managed_revision else { return Ok(None); };
    let revision = m.load_revision(&r.metadata.class_id, reference)?;
    retained_maximum(m, &revision)
}
/// Package refresh validates capacity against the explicit staged runtime.
/// The selected package may still describe the predecessor while preparation
/// is in progress, so it cannot answer for the candidate native image.
pub(crate) fn candidate_maximum_bridge_frames(
    m: &Manager,
    candidate: &Candidate,
) -> Result<Option<u32>> {
    let registration = super::configuration::registration(candidate)?;
    let runtime = existing_runtime(m, &candidate.recipe_sha256)?;
    require(
        candidate.host == runtime.host && candidate.source_manifest == runtime.source_manifest,
        "candidate_runtime_changed",
    )?;
    capability_from_kit(&runtime.kit, &registration, "maximum_bridge_frames")
}
pub(crate) fn candidate_supports_audio_completion(
    m: &Manager,
    candidate: &Candidate,
) -> Result<bool> {
    let registration = super::configuration::registration(candidate)?;
    let runtime = existing_runtime(m, &candidate.recipe_sha256)?;
    require(
        candidate.host == runtime.host && candidate.source_manifest == runtime.source_manifest,
        "candidate_runtime_changed",
    )?;
    Ok(capability_from_kit(&runtime.kit, &registration,
        "audio_completion_contract")? == Some(1))
}
/// Rollback validates the target ancestor's capacity before selecting it. The
/// currently selected revision cannot stand in for that ancestor's recipe.
pub fn revision_maximum_bridge_frames(m: &Manager, r: &Revision) -> Result<Option<u32>> {
    if let Some(maximum) = current_kit_maximum(m, &r.registration)? { return Ok(Some(maximum)); }
    retained_maximum(m, r)
}
fn current_kit_maximum(m: &Manager, r: &Registration) -> Result<Option<u32>> {
    let software: crate::catalogue::Software = read_json(&m.root.join("software.json"))?;
    r.native.verify()?;
    if software.preparation_kit.is_some() { capability_from_kit(&recipe(m)?, r, "maximum_bridge_frames") }
    else { Ok(None) }
}
fn retained_maximum(m: &Manager, r: &Revision) -> Result<Option<u32>> {
    let candidate = super::publication_candidate(m, &r.profile, &r.registration)?;
    if !valid_hex(&candidate.recipe_sha256, 64) { return Ok(None); }
    let retained = existing_runtime(m, &candidate.recipe_sha256)?;
    require(candidate.host == retained.host && candidate.source_manifest == retained.source_manifest,
        "candidate_runtime_changed")?;
    capability_from_kit(&retained.kit, &r.registration, "maximum_bridge_frames")
}
fn capability_from_kit(kit: &Artifact, r: &Registration, capability: &str) -> Result<Option<u32>> {
    kit.verify()?;
    let request = serde_json::json!({"kit":kit.path,"class_id":r.metadata.class_id,
        "module_sha256":r.module.sha256,"native_sha256":r.native.sha256,"host_sha256":r.host.sha256,"capability":capability});
    let mut child = Command::new("python3")
        .args(["-I", "-c", include_str!("../../../tools/mf3/prebuilt_info.py")])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    child.stdin.take().ok_or("prebuilt_info_stdin")?.write_all(&serde_json::to_vec(&request)?)?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(status) = child.try_wait()? { break status; }
        if Instant::now() >= deadline {
            child.kill()?; child.wait()?;
            return Err("prebuilt_info_deadline".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut bytes = Vec::new();
    child.stdout.take().ok_or("prebuilt_info_stdout")?.take(33).read_to_end(&mut bytes)?;
    require(status.success() && bytes.len() <= 32, "prebuilt_info_invalid")?;
    let maximum: Option<u32> = serde_json::from_slice(&bytes)?;
    require(maximum.is_none_or(|n| if matches!(capability,
        "audio_completion_contract" | "loaded_engine_admission_contract") { n == 1 }
        else { matches!(n, 512 | 1024) }), "prebuilt_info_envelope")?;
    Ok(maximum)
}
pub fn runtime_declares_loaded_engine(runtime: &Runtime) -> Result<bool> {
    runtime.kit.verify()?;
    let request = serde_json::json!({"kit":runtime.kit.path,
        "capability":"loaded_engine_admission_contract"});
    let mut child = Command::new("python3")
        .args(["-I", "-c", include_str!("../../../tools/mf3/prebuilt_info.py")])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    child.stdin.take().ok_or("prebuilt_info_stdin")?
        .write_all(&serde_json::to_vec(&request)?)?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(status) = child.try_wait()? { break status; }
        if Instant::now() >= deadline {
            child.kill()?; child.wait()?;
            return Err("prebuilt_info_deadline".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut bytes = Vec::new();
    child.stdout.take().ok_or("prebuilt_info_stdout")?.take(33).read_to_end(&mut bytes)?;
    require(status.success() && bytes.len() <= 32, "prebuilt_info_invalid")?;
    Ok(serde_json::from_slice::<Option<u32>>(&bytes)? == Some(1))
}
/// Both executables must belong to the same recipe declaring this contract.
pub fn supports_audio_completion(m: &Manager, r: &Registration) -> Result<bool> {
    r.native.verify()?;
    r.host.verify()?;
    let software: crate::catalogue::Software = read_json(&m.root.join("software.json"))?;
    if software.preparation_kit.is_some()
        && capability_from_kit(&recipe(m)?, r, "audio_completion_contract")? == Some(1) { return Ok(true); }
    let registry = m.registry()?;
    let Some(entry) = registry.classes.get(&r.metadata.class_id)
        .filter(|entry| entry.registration == *r) else { return Ok(false); };
    let Some(reference) = &entry.managed_revision else { return Ok(false); };
    revision_supports_audio_completion(m, &m.load_revision(&r.metadata.class_id, reference)?)
}
pub fn supports_loaded_engine_admission(m: &Manager, c: &Candidate) -> Result<bool> {
    c.native.artifact.verify()?;
    let Some(descriptor) = c.native.descriptor.as_ref() else { return Ok(false); };
    if !valid_hex(&c.recipe_sha256, 64) { return Ok(false); }
    crate::verify_native_descriptor(&c.native.artifact, descriptor,
        &c.native.class, &c.native.module_sha256)?;
    let retained = existing_runtime(m, &c.recipe_sha256)?;
    let registration = super::configuration::registration(c)?;
    Ok(capability_from_kit(&retained.kit, &registration,
        "loaded_engine_admission_contract")? == Some(1)
        && supports_loaded_engine_admission_record(m, c)?)
}
/// Status evidence from the immutable output of a completed schema-4 build.
/// Mutation and launch still use supports_loaded_engine_admission and hash all
/// executable/runtime bytes.
pub fn supports_loaded_engine_admission_record(m: &Manager, c: &Candidate) -> Result<bool> {
    let Some(descriptor) = c.native.descriptor.as_ref() else {
        return Ok(false);
    };
    if !valid_hex(&c.recipe_sha256, 64) {
        return Ok(false);
    }
    require(
        descriptor.sha256 == c.native.descriptor_sha256
            && descriptor.path
                == c
                    .native
                    .artifact
                    .path
                    .with_file_name(lvb_plugin_descriptor::FILE_NAME),
        "native_descriptor_binding",
    )?;
    c.native.artifact.validate_record()?;
    descriptor.validate_record()?;
    let runtime_path = m
        .root
        .join("software/preparation-kits")
        .join(&c.recipe_sha256)
        .join("runtime.json");
    if !runtime_path.try_exists()? {
        return Ok(false);
    }
    let runtime = existing_runtime_record(m, &c.recipe_sha256)?;
    require(
        c.host == runtime.host && c.source_manifest == runtime.source_manifest,
        "candidate_runtime_changed",
    )?;
    let (Some(builder), Some(generator)) = (&runtime.builder, &runtime.generator) else {
        return Ok(false);
    };
    let path = c.native.artifact.path.with_file_name("build.json");
    if !path.try_exists()? {
        return Ok(false);
    }
    require(
        path.canonicalize()? == path && file(&path)?.metadata()?.mode() & 0o222 == 0,
        "retained_build_record_mutability",
    )?;
    let row: Value = bounded(&path)?;
    require(
        row["schema"] == 1
            && row["dropped_bytes"] == 0
            && row["kit_sha256"] == c.recipe_sha256
            && row["native_sha256"] == c.native.artifact.sha256
            && row["descriptor_sha256"] == descriptor.sha256
            && row["descriptor_sha256"] == c.native.descriptor_sha256
            && row["source_commit"] == c.native.source_commit
            && row["builder_sha256"] == builder.sha256
            && row["generator_sha256"] == generator.sha256
            && row["prebuilt_index_sha256"]
                .as_str()
                .is_some_and(|digest| valid_hex(digest, 64))
            && matches!(row["delivery"].as_str(), Some("prebuilt" | "reusable_engine")),
        "retained_build_identity",
    )?;
    if row.get("loaded_engine_admission_contract").is_none() {
        return Ok(false);
    }
    require(row["loaded_engine_admission_contract"] == 1,
        "retained_build_identity")?;
    Ok(row["delivery"] == "reusable_engine")
}
pub fn revision_supports_loaded_engine_admission(m: &Manager, r: &Revision) -> Result<bool> {
    let candidate = super::publication_candidate(m, &r.profile, &r.registration)?;
    supports_loaded_engine_admission(m, &candidate)
}
pub(crate) fn publication_supports_loaded_engine_admission(m: &Manager,
    p: &Profile, r: &Registration) -> Result<bool> {
    let candidate = super::publication_candidate(m, p, r)?;
    supports_loaded_engine_admission(m, &candidate)
}
pub fn revision_supports_audio_completion(m: &Manager, r: &Revision) -> Result<bool> {
    publication_supports_audio_completion(m, &r.profile, &r.registration)
}
pub(crate) fn publication_supports_audio_completion(m: &Manager, p: &Profile, r: &Registration) -> Result<bool> {
    r.native.verify()?;
    r.host.verify()?;
    let software: crate::catalogue::Software = read_json(&m.root.join("software.json"))?;
    if software.preparation_kit.is_some()
        && capability_from_kit(&recipe(m)?, r, "audio_completion_contract")? == Some(1) { return Ok(true); }
    let candidate = super::publication_candidate(m, p, r)?;
    if !valid_hex(&candidate.recipe_sha256, 64) { return Ok(false); }
    let retained = existing_runtime(m, &candidate.recipe_sha256)?;
    require(candidate.host == retained.host && candidate.source_manifest == retained.source_manifest,
        "candidate_runtime_changed")?;
    Ok(capability_from_kit(&retained.kit, r, "audio_completion_contract")? == Some(1))
}
pub fn construct(
    m: &Manager,
    s: Selection,
    i: Inspection,
    host: Artifact,
    manifest: Artifact,
    operation: &str,
    event_output: Option<EventOutputPolicy>,
) -> Result<Candidate> {
    let runtime = stage_runtime(m)?;
    require(runtime.host == host && runtime.source_manifest == manifest,
        "inspection_generation_refresh_required")?;
    construct_with_runtime(m, s, i, runtime, operation, event_output)
}
pub fn construct_with_runtime(
    m: &Manager,
    s: Selection,
    i: Inspection,
    runtime: Runtime,
    operation: &str,
    event_output: Option<EventOutputPolicy>,
) -> Result<Candidate> {
    require(valid_hex(operation, 32), "preparation_operation")?;
    require(
        !matches!(i.controller, ControllerAssociation::Unavailable { .. }),
        "Inspection did not retain the exact controller association; updated inspection required",
    )?;
    let host = runtime.host.clone();
    let manifest = runtime.source_manifest.clone();
    require(i.host == host && i.source_manifest == manifest,
        "inspection_generation_refresh_required")?;
    require(
        runtime.builder.is_some() && runtime.generator.is_some(),
        "build_recipe_generator_identity_missing",
    )?;
    let kit = runtime.kit.clone();
    let parent = m.root.join("preparation/work");
    private_dir(&parent)?;
    let dir = parent.join(operation);
    require(!dir.exists(), "preparation_operation_already_started")?;
    let request = serde_json::json!({"directory":dir,"kit":kit.path,"kit_sha256":kit.sha256,"inspection":i.report.path,"class_id":s.class.id,"module_sha256":s.module.sha256,"event_output":event_output});
    let script = include_str!("../../../tools/mf3/kit_entry.py");
    let mut child = Command::new("python3")
        .args(["-I", "-c", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("build_stdin")?
        .write_all(&serde_json::to_vec(&request)?)?;
    let mut pipe = child.stdout.take().ok_or("build_stdout")?;
    let reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut data = vec![];
        pipe.by_ref().take(65537).read_to_end(&mut data)?;
        std::io::copy(&mut pipe, &mut std::io::sink())?;
        Ok(data)
    });
    let deadline = Instant::now() + Duration::from_secs(1250);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            let _ = child.wait();
            return Err("native_build_deadline".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let bytes = reader.join().map_err(|_| "build_reader")??;
    require(bytes.len() <= 65536, "build_output_bound")?;
    let reply: Value = serde_json::from_slice(&bytes)?;
    require(
        status.success(),
        reply["error"].as_str().unwrap_or("native_build_failed"),
    )?;
    require(reply.get("event_output").unwrap_or(&Value::Null) == &serde_json::to_value(&event_output)?,
        "native_build_event_output_policy_changed")?;
    let descriptor = if reply["delivery"] == "reusable_engine" {
        require(reply["loaded_engine_admission_contract"] == 1,
            "loaded_engine_admission_contract_missing")?;
        let artifact = Artifact { path: dir.join(lvb_plugin_descriptor::FILE_NAME),
            sha256: reply["descriptor_sha256"].as_str().ok_or("build_descriptor")?.into() };
        artifact.verify()?;
        fs::set_permissions(&artifact.path, fs::Permissions::from_mode(0o400))?;
        file(&artifact.path)?.sync_all()?;
        Some(artifact)
    } else { None };
    let native = NativeArtifact {
        descriptor,
        class: i.census()?.selected,
        module_sha256: s.module.sha256.clone(),
        artifact: Artifact {
            path: dir.join("native.so"),
            sha256: reply["native_sha256"]
                .as_str()
                .ok_or("build_identity")?
                .into(),
        },
        source_commit: reply["source_commit"]
            .as_str()
            .ok_or("build_source")?
            .into(),
        descriptor_sha256: reply["descriptor_sha256"]
            .as_str()
            .ok_or("build_descriptor")?
            .into(),
        external_ids: external_ids(&s.class.id)?,
    };
    require(
        reply["kit_sha256"] == kit.sha256
            && valid_hex(reply["builder_sha256"].as_str().unwrap_or(""), 64)
            && valid_hex(reply["generator_sha256"].as_str().unwrap_or(""), 64),
        "build_recipe_identity",
    )?;
    native.artifact.verify()?;
    if let Some(descriptor) = &native.descriptor {
        crate::verify_native_descriptor(&native.artifact, descriptor,
            &native.class, &native.module_sha256)?;
    }
    fs::set_permissions(&native.artifact.path, fs::Permissions::from_mode(0o500))?;
    file(&native.artifact.path)?.sync_all()?;
    immutable(&dir.join("build.json"), &reply)?;
    prepared(s, i, native, host, manifest, kit.sha256)
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub kit: Artifact,
    pub host: Artifact,
    pub source_manifest: Artifact,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builder: Option<Artifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<Artifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub direct_audio_helpers: Vec<Artifact>,
}
fn runtime_dir(m: &Manager, sha: &str) -> Result<PathBuf> {
    require(valid_hex(sha, 64), "preparation_kit_identity")?;
    Ok(m.root.join("software/preparation-kits").join(sha))
}
pub fn existing_runtime(m: &Manager, sha: &str) -> Result<Runtime> {
    let r = existing_runtime_record(m, sha)?;
    for a in [&r.kit, &r.host, &r.source_manifest].into_iter()
        .chain(r.builder.iter()).chain(r.generator.iter()).chain(r.direct_audio_helpers.iter()) { a.verify()?; }
    Ok(r)
}
pub fn existing_runtime_record(m: &Manager, sha: &str) -> Result<Runtime> {
    let dir = runtime_dir(m, sha)?;
    let r: Runtime = bounded(&dir.join("runtime.json"))?;
    require(
        r.kit.sha256 == sha
            && r.host.path == dir.join("host.exe")
            && r.source_manifest.path == dir.join("host-source-manifest.json")
            && r.kit.path.starts_with(m.root.join("software")),
        "preparation_runtime_identity",
    )?;
    for a in [&r.kit, &r.host, &r.source_manifest]
        .into_iter()
        .chain(r.builder.iter())
        .chain(r.generator.iter()).chain(r.direct_audio_helpers.iter())
    {
        require(
            a.path.canonicalize()? == a.path && file(&a.path)?.metadata()?.mode() & 0o222 == 0,
            "preparation_runtime_mutability",
        )?;
        a.validate_record()?;
    }
    const HELPERS:[&str;4]=["lvb-direct-wait.dll","x86_64-windows/lvb-direct-wait.dll",
        "x86_64-unix/lvb-direct-wait.so","direct-audio-helper.json"];
    require(r.direct_audio_helpers.is_empty() || r.direct_audio_helpers.len()==HELPERS.len()
        && r.direct_audio_helpers.iter().zip(HELPERS).all(|(a,name)|a.path==dir.join(name)),
        "direct_audio_helper_identity")?;
    Ok(r)
}
pub fn verify_runtime(m: &Manager, c: &Candidate) -> Result<()> {
    let r = existing_runtime(m, &c.recipe_sha256)?;
    require(
        c.host == r.host && c.source_manifest == r.source_manifest,
        "candidate_runtime_changed",
    )
}
/// A separate installed preparation host adds inspection metadata without
/// replacing the default host or invalidating concurrent SV1 product bytes.
pub fn stage_runtime(m: &Manager) -> Result<Runtime> {
    let kit = recipe(m)?;
    stage_runtime_with_kit(m, kit)
}
pub fn stage_runtime_for_software(m: &Manager,
    software: &crate::catalogue::Software) -> Result<Runtime> {
    stage_runtime_with_kit(m, recipe_for_software(m, software)?)
}
fn stage_runtime_with_kit(m: &Manager, kit: Artifact) -> Result<Runtime> {
    let dir = runtime_dir(m, &kit.sha256)?;
    if dir.join("runtime.json").exists() {
        return existing_runtime(m, &kit.sha256);
    }
    let _lock = m.lock("preparation-kit.lock")?;
    private_dir(dir.parent().ok_or("runtime_parent")?)?;
    let stage = dir.with_file_name(format!(".runtime-{}", random_id()?));
    private_dir(&stage)?;
    let unpacked=(||->Result<()> {
        package_host::unpack(&kit,Some(&stage),"preparation")?;
        kit.verify()
    })();
    if let Err(error)=unpacked {
        fs::remove_dir_all(&stage)?;
        return Err(error);
    }
    let artifact = |name: &str| -> Result<Artifact> {
        let p = stage.join(name);
        let sha = digest(&p)?;
        fs::set_permissions(&p, fs::Permissions::from_mode(0o400))?;
        file(&p)?.sync_all()?;
        Ok(Artifact {
            path: dir.join(name),
            sha256: sha,
        })
    };
    let direct_audio_helpers=if stage.join("direct-audio-helper.json").exists() {
        ["lvb-direct-wait.dll","x86_64-windows/lvb-direct-wait.dll","x86_64-unix/lvb-direct-wait.so","direct-audio-helper.json"]
            .into_iter().map(artifact).collect::<Result<Vec<_>>>()?
    } else {vec![]};
    if stage.join("installer-launch.exe").exists() {
        artifact("installer-launch.exe")?;artifact("installer-launch-source.json")?;
    }
    let r = Runtime {
        direct_audio_helpers,
        kit,
        host: artifact("host.exe")?,
        source_manifest: artifact("host-source-manifest.json")?,
        builder: if stage.join("native_builder.py").exists() {
            Some(artifact("native_builder.py")?)
        } else {
            None
        },
        generator: if stage.join("ap8_descriptor.py").exists() {
            Some(artifact("ap8_descriptor.py")?)
        } else {
            None
        },
    };
    immutable(&stage.join("runtime.json"), &r)?;
    crate::publication::rename_link(&stage, &dir, false)?;
    File::open(dir.parent().ok_or("runtime_parent")?)?.sync_all()?;
    existing_runtime(m, &r.kit.sha256)
}

/// ExecStopPost invokes this after the worker cgroup has retired. Keep committed
/// immutable native bytes, remove only this exact operation's scratch files.
pub fn cleanup_work(m: &Manager, operation: &str) -> Result<()> {
    require(valid_hex(operation, 32), "preparation_operation")?;
    let dir = m.root.join("preparation/work").join(operation);
    if !dir.exists() {
        return Ok(());
    }
    require(dir.canonicalize()? == dir, "preparation_work_alias")?;
    let mut retained = false;
    for path in list(&root(m).join("candidates"))? {
        let c: Candidate = bounded(&path.join("candidate.json"))?;
        retained |= c.native.artifact.path == dir.join("native.so");
    }
    // A guided check can be interrupted after its immutable candidate
    // checkpoint but before candidate.json is committed. The original worker
    // must retire, yet its verified native bytes are needed by the offered
    // continuation. Do not preserve unrelated or unbound work output.
    if !retained {
        if let Some(stage) = guided_check_stage(m, operation, "candidate")? {
            let intent = guided_check_stage(m, operation, "intent")?
                .ok_or("guided_check_intent_missing")?;
            require(stage["schema"] == 1 && stage["operation"] == operation
                && stage["action"] == intent["action"],
                "guided_check_candidate_stage_binding")?;
            let c: Candidate = serde_json::from_value(stage["value"]["candidate"].clone())?;
            require(stage["value"]["id"] == c.id()?,
                "guided_check_candidate_stage_identity")?;
            if c.native.artifact.path == dir.join("native.so") {
                c.native.artifact.verify()?;
                retained = true;
            }
        }
    }
    if !retained {
        for name in ["failure.json", "build.log"] {
            let path = dir.join(name);
            if path.try_exists()? {
                let mut bytes = vec![];
                file(&path)?.take(32769).read_to_end(&mut bytes)?;
                require(bytes.len() <= 32768, "build_failure_bound")?;
                let dest = m.root.join("preparation/failures").join(operation);
                private_dir(&dest)?;
                immutable(
                    &dest.join(format!("{name}.json")),
                    &String::from_utf8_lossy(&bytes),
                )?;
            }
        }
        fs::remove_dir_all(dir)?;
        return Ok(());
    }
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if matches!(
            path.file_name().and_then(|n| n.to_str()),
            Some("native.so" | "build.json" | "plugin-descriptor.json")
        ) {
            continue;
        }
        let md = fs::symlink_metadata(&path)?;
        if md.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Reuse only the complete requested generation and its verified retained output.
/// Multiple matching inputs with different output artifacts are not guessed.
pub fn reusable(
    m: &Manager,
    s: &Selection,
    i: &Inspection,
    kit: &str,
    event_output: Option<EventOutputPolicy>,
) -> Result<Option<Candidate>> {
    let requested_policy = serde_json::to_value(event_output)?;
    let mut found = vec![];
    for c in retained_candidates(m)? {
        if c.origin != Origin::ManagedPreparation
            || c.selection != *s
            || c.inspection != *i
            || c.host != i.host
            || c.source_manifest != i.source_manifest
            || c.recipe_sha256 != kit
        {
            continue;
        }
        let path = c.native.artifact.path.with_file_name("build.json");
        let row: Value = bounded(&path)?;
        if row.get("event_output").unwrap_or(&Value::Null) != &requested_policy {
            continue;
        }
        let runtime = existing_runtime(m, kit)?;
        require(
            row["builder_sha256"].as_str() == runtime.builder.as_ref().map(|a| a.sha256.as_str())
                && runtime.builder.is_some()
                && row["generator_sha256"].as_str()
                    == runtime.generator.as_ref().map(|a| a.sha256.as_str()),
            "retained_build_generator_identity",
        )?;
        require(
            row["kit_sha256"] == kit
                && row["native_sha256"] == c.native.artifact.sha256
                && row["descriptor_sha256"] == c.native.descriptor_sha256
                && row["source_commit"] == c.native.source_commit,
            "retained_build_identity",
        )?;
        verify_candidate(m, &c, &s.scanner, &s.scanner_source)?;
        let technical = prepared(s.clone(), i.clone(), c.native.clone(),
            i.host.clone(), i.source_manifest.clone(), kit.into())?;
        // Event interpretation is a build input, matched above. Other launch
        // settings do not change these native bytes. Resolve today's defaults
        // before the caller carries its exact retained configuration.
        if !found.contains(&technical) {
            found.push(technical);
        }
    }
    require(found.len() <= 1, "preparation_outputs_ambiguous")?;
    Ok(found.pop())
}

#[cfg(test)] mod launch_component_tests {
    use super::*;
    #[test]
    fn selected_kit_component_is_exact_and_old_runtime_shape_stays_readable() {
        let (f,_)=crate::test_fixture::prepared_candidate(&"13".repeat(16));
        let selected=crate::catalogue::Software {manager:f.r.host.clone(),supervisor:f.r.host.clone(),
            ownership:f.r.host.clone(),host:f.r.host.clone(),source_manifest:Artifact {
                path:f.r.host.path.with_file_name("host-source-manifest.json"),sha256:f.r.host_source_sha256.clone()},
            source_sha256:f.r.host_source_sha256.clone(),installer_launch:None,preparation_kit:None,
            operator_frontend:None,native_catalogue:None};
        let kit=f.m.root.join("software/launch-kit.zip");
        crate::test_fixture::reusable_kit(&kit,&selected.host,&selected.source_manifest,"test engine");
        let mut original=selected.clone();original.preparation_kit=Some(Artifact {path:kit.clone(),sha256:digest(&kit).unwrap()});
        atomic_json(&f.m.root.join("software.json"),&original).unwrap();
        assert!(launch_software_record(&f.m).is_err());
        assert!(!f.m.root.join("software/preparation-kits").join(&original.preparation_kit.as_ref().unwrap().sha256).exists());
        assert!(launch_software(&f.m).err().unwrap().to_string().contains("launch component is missing"));
        let script=r#"import zipfile,json,hashlib,sys
p=sys.argv[1]
with zipfile.ZipFile(p) as z:values={n:z.read(n) for n in z.namelist()}
r=json.loads(values['recipe.json']);values['runtime/installer-launch.exe']=b'MZfirst-party launch component'
values['runtime/installer-launch-source.json']=json.dumps({'schema':1,'component':'installer_launch','artifact_sha256':hashlib.sha256(values['runtime/installer-launch.exe']).hexdigest()}).encode()
for n in ['runtime/installer-launch.exe','runtime/installer-launch-source.json']:r['files'][n]=hashlib.sha256(values[n]).hexdigest()
values['recipe.json']=json.dumps(r).encode()
with zipfile.ZipFile(p,'w') as z:
 for n,b in values.items():z.writestr(n,b)
"#;
        fs::set_permissions(&kit,fs::Permissions::from_mode(0o600)).unwrap();
        assert!(Command::new("python3").args(["-I","-c",script]).arg(&kit).status().unwrap().success());
        original.preparation_kit.as_mut().unwrap().sha256=digest(&kit).unwrap();
        fs::set_permissions(&kit,fs::Permissions::from_mode(0o400)).unwrap();
        atomic_json(&f.m.root.join("software.json"),&original).unwrap();let selected_bytes=fs::read(f.m.root.join("software.json")).unwrap();
        let effective=launch_software(&f.m).unwrap();let adapter=effective.installer_launch.clone().unwrap();adapter.verify().unwrap();
        assert!(launch_software_record(&f.m).unwrap()==effective);
        assert_eq!(file(&adapter.path).unwrap().metadata().unwrap().mode()&0o222,0);
        assert_eq!(fs::read(f.m.root.join("software.json")).unwrap(),selected_bytes);
        let runtime=existing_runtime(&f.m,&original.preparation_kit.as_ref().unwrap().sha256).unwrap();
        let raw:Value=read_json(&runtime.host.path.with_file_name("runtime.json")).unwrap();
        assert!(raw.get("installer_launch").is_none());
        let runtime_path=runtime.host.path.with_file_name("runtime.json");let runtime_bytes=fs::read(&runtime_path).unwrap();
        // A prior reader's cache has the same exact owned runtime.json, but
        // does not know these optional members. Readback must not install them.
        fs::remove_file(&adapter.path).unwrap();
        fs::remove_file(adapter.path.with_file_name("installer-launch-source.json")).unwrap();
        assert!(launch_software_record(&f.m).is_err());assert!(!adapter.path.exists());
        assert_eq!(fs::read(&runtime_path).unwrap(),runtime_bytes);
        assert!(launch_software(&f.m).unwrap()==effective);
        assert!(launch_software_record(&f.m).unwrap()==effective);
        assert_eq!(fs::read(&runtime_path).unwrap(),runtime_bytes);
        assert_eq!(fs::read(f.m.root.join("software.json")).unwrap(),selected_bytes);
        fs::set_permissions(&adapter.path,fs::Permissions::from_mode(0o600)).unwrap();fs::write(&adapter.path,b"changed").unwrap();
        fs::set_permissions(&adapter.path,fs::Permissions::from_mode(0o400)).unwrap();
        assert!(launch_software(&f.m).is_err());
    }
}
