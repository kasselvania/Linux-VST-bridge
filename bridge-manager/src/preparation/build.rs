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
    let runtime = stage_runtime(m)?;
    require(
        runtime.host == host && runtime.source_manifest == manifest,
        "inspection_generation_refresh_required",
    )?;
    require(
        runtime.builder.is_some() && runtime.generator.is_some(),
        "build_recipe_generator_identity_missing",
    )?;
    let kit = recipe(m)?;
    let parent = m.root.join("preparation/work");
    private_dir(&parent)?;
    let dir = parent.join(operation);
    require(!dir.exists(), "preparation_operation_already_started")?;
    let request = serde_json::json!({"directory":dir,"kit":kit.path,"kit_sha256":kit.sha256,"inspection":i.report.path,"class_id":s.class.id,"module_sha256":s.module.sha256});
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
    require(
        reply["kit_sha256"] == kit.sha256
            && valid_hex(reply["builder_sha256"].as_str().unwrap_or(""), 64)
            && valid_hex(reply["generator_sha256"].as_str().unwrap_or(""), 64),
        "build_recipe_identity",
    )?;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builder: Option<Artifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<Artifact>,
}
fn runtime_dir(m: &Manager, sha: &str) -> Result<PathBuf> {
    require(valid_hex(sha, 64), "preparation_kit_identity")?;
    Ok(m.root.join("software/preparation-kits").join(sha))
}
pub fn existing_runtime(m: &Manager, sha: &str) -> Result<Runtime> {
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
        .chain(r.generator.iter())
    {
        require(
            a.path.canonicalize()? == a.path && file(&a.path)?.metadata()?.mode() & 0o222 == 0,
            "preparation_runtime_mutability",
        )?;
        a.verify()?;
    }
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
    let dir = runtime_dir(m, &kit.sha256)?;
    if dir.join("runtime.json").exists() {
        return existing_runtime(m, &kit.sha256);
    }
    let _lock = m.lock("preparation-kit.lock")?;
    private_dir(dir.parent().ok_or("runtime_parent")?)?;
    let stage = dir.with_file_name(format!(".runtime-{}", random_id()?));
    private_dir(&stage)?;
    let script = r#"import sys,json,zipfile,pathlib,hashlib
kit,out=sys.argv[1:];out=pathlib.Path(out)
with zipfile.ZipFile(kit) as z:
 assert z.getinfo('recipe.json').file_size<=65536
 recipe=json.loads(z.read('recipe.json'));assert recipe['schema'] in (1,2)
 names=[('runtime/host.exe','host.exe'),('runtime/host-source-manifest.json','host-source-manifest.json')]
 if recipe['schema']==2:names += [('tools/mf3/native_builder.py','native_builder.py'),('tools/ap8_descriptor.py','ap8_descriptor.py')]
 for key,name in names:
  i=z.getinfo(key);assert not i.is_dir() and i.file_size<=64*1024*1024
  b=z.read(i);assert hashlib.sha256(b).hexdigest()==recipe['files'][key]
  (out/name).write_bytes(b)
"#;
    let launch = Command::new("python3")
        .args(["-I", "-c", script])
        .arg(&kit.path)
        .arg(&stage)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match launch {
        Ok(c) => c,
        Err(e) => {
            fs::remove_dir_all(&stage)?;
            return Err(e.into());
        }
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            fs::remove_dir_all(&stage)?;
            return Err("preparation_runtime_unpack_deadline".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if !status.success() {
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

/// Reuse only the complete requested generation and its verified retained output.
/// Multiple matching inputs with different output artifacts are not guessed.
pub fn reusable(
    m: &Manager,
    s: &Selection,
    i: &Inspection,
    kit: &str,
) -> Result<Option<Candidate>> {
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
        require(
            bind_preparation_basis(
                prepared(
                    s.clone(),
                    i.clone(),
                    c.native.clone(),
                    i.host.clone(),
                    i.source_manifest.clone(),
                    kit.into(),
                )?,
                c.preparation_basis.clone(),
            )? == c,
            "retained_build_policy_changed",
        )?;
        let technical = bind_preparation_basis(c, None)?;
        if !found.contains(&technical) {
            found.push(technical);
        }
    }
    require(found.len() <= 1, "preparation_outputs_ambiguous")?;
    Ok(found.pop())
}
