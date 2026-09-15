//! Fixed offline recipe. Compilation is non-RT and never owns registry.lock.
use super::*;
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};
pub fn recipe_available(m: &Manager) -> Result<Artifact> {
    let sw: crate::catalogue::Software = read_json(&m.root.join("software.json"))?;
    let kit=sw.preparation_kit.ok_or("Preparation tools are not installed. Install the manager package with its pinned build tools")?;
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
pub fn construct(
    m: &Manager,
    s: Selection,
    i: Inspection,
    host: Artifact,
    manifest: Artifact,
    operation: &str,
) -> Result<Candidate> {
    require(valid_hex(operation, 32), "preparation_operation")?;
    require(
        !matches!(i.controller, ControllerAssociation::Unavailable { .. }),
        "Inspection did not retain the exact controller association; updated inspection required",
    )?;
    let kit = recipe(m)?;
    let parent = m.root.join("preparation/work");
    private_dir(&parent)?;
    let dir = parent.join(operation);
    require(!dir.exists(), "preparation_operation_already_started")?;
    let request = serde_json::json!({"directory":dir,"kit":kit.path,"kit_sha256":kit.sha256,"inspection":i.report.path,"class_id":s.class.id,"module_sha256":s.module.sha256});
    let script=format!("{}\n{}\ntry:\n print(json.dumps(build(json.load(sys.stdin), generate)))\nexcept Exception as e:\n print(json.dumps({{'error':str(e)[:256]}}))\n raise SystemExit(1)\n",include_str!("../../../tools/mf3/native_builder.py"),include_str!("../../../tools/ap8_descriptor.py").split("def main():").next().ok_or("descriptor_recipe")?);
    let mut child = Command::new("python3")
        .args(["-I", "-c", &script])
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
    let native = NativeArtifact {
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
    native.artifact.verify()?;
    fs::set_permissions(&native.artifact.path, fs::Permissions::from_mode(0o500))?;
    file(&native.artifact.path)?.sync_all()?;
    immutable(&dir.join("build.json"), &reply)?;
    prepared(s, i, native, host, manifest, kit.sha256)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub kit: Artifact,
    pub host: Artifact,
    pub source_manifest: Artifact,
}
fn runtime_dir(m: &Manager, sha: &str) -> Result<PathBuf> {
    require(valid_hex(sha, 64), "preparation_kit_identity")?;
    Ok(m.root.join("preparation/kits").join(sha))
}
fn read_runtime(m: &Manager, sha: &str) -> Result<Runtime> {
    let dir = runtime_dir(m, sha)?;
    let r: Runtime = bounded(&dir.join("runtime.json"))?;
    require(
        r.kit.sha256 == sha
            && r.host.path == dir.join("host.exe")
            && r.source_manifest.path == dir.join("host-source-manifest.json")
            && r.kit.path.starts_with(m.root.join("software")),
        "preparation_runtime_identity",
    )?;
    for a in [&r.kit, &r.host, &r.source_manifest] {
        require(
            a.path.canonicalize()? == a.path && file(&a.path)?.metadata()?.mode() & 0o222 == 0,
            "preparation_runtime_mutability",
        )?;
        a.verify()?;
    }
    Ok(r)
}
pub fn verify_runtime(m: &Manager, c: &Candidate) -> Result<()> {
    let r = read_runtime(m, &c.recipe_sha256)?;
    require(
        c.host == r.host && c.source_manifest == r.source_manifest,
        "candidate_runtime_changed",
    )
}
/// A separate installed preparation host adds inspection metadata without
/// replacing the default host or invalidating concurrent SV1 product bytes.
pub fn stage_runtime(m: &Manager) -> Result<Runtime> {
    let kit = recipe(m)?;
    let dir = runtime_dir(m, &kit.sha256)?;
    if dir.join("runtime.json").exists() {
        return read_runtime(m, &kit.sha256);
    }
    let _lock = m.lock("preparation-kit.lock")?;
    private_dir(dir.parent().ok_or("runtime_parent")?)?;
    let stage = dir.with_file_name(format!(".runtime-{}", random_id()?));
    private_dir(&stage)?;
    let script = r#"import sys,json,zipfile,pathlib,hashlib
kit,out=sys.argv[1:];out=pathlib.Path(out)
with zipfile.ZipFile(kit) as z:
 assert z.getinfo('recipe.json').file_size<=65536
 recipe=json.loads(z.read('recipe.json'));assert recipe['schema']==1
 for name in ['host.exe','host-source-manifest.json']:
  key='runtime/'+name;i=z.getinfo(key);assert not i.is_dir() and i.file_size<=64*1024*1024
  b=z.read(i);assert hashlib.sha256(b).hexdigest()==recipe['files'][key]
  (out/name).write_bytes(b)
"#;
    let result = Command::new("python3")
        .args(["-I", "-c", script])
        .arg(&kit.path)
        .arg(&stage)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if !result?.success() {
        fs::remove_dir_all(&stage)?;
        return Err("preparation_runtime_package_incomplete".into());
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
    let r = Runtime {
        kit,
        host: artifact("host.exe")?,
        source_manifest: artifact("host-source-manifest.json")?,
    };
    immutable(&stage.join("runtime.json"), &r)?;
    crate::publication::rename_link(&stage, &dir, false)?;
    File::open(dir.parent().ok_or("runtime_parent")?)?.sync_all()?;
    read_runtime(m, &r.kit.sha256)
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
    if !retained {
        fs::remove_dir_all(dir)?;
        return Ok(());
    }
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if matches!(
            path.file_name().and_then(|n| n.to_str()),
            Some("native.so" | "build.json")
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
