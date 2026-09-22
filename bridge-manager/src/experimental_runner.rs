//! Explicit inactive transition to one sealed experimental reference runner.
//! It changes environment authority only; prefix provisioning is a separately
//! retained operator transaction and no publication is carried forward.
use super::*;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: u32,
    environment: String,
    expected_environment_sha256: String,
    expected_revision: u64,
    expected_runner_sha256: String,
    retired_class: String,
    expected_removed_publication: publication::RevisionRef,
    candidate_manifest: Artifact,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TreeIdentity {
    schema: u32,
    entries: u64,
    regular_bytes: u64,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateManifest {
    schema: u32,
    kind: String,
    source: SourceIdentity,
    build_recipe: Artifact,
    base_runner_sha256: String,
    root: PathBuf,
    tree: TreeIdentity,
    runner: Runner,
    graphics: BTreeMap<String, Artifact>,
    dll_overrides: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceIdentity {
    proton_distribution_source_commit: String,
    wine_commit: String,
    wine_tree: String,
    wine_configure: Vec<String>,
    wine_inf_sha256: String,
}

fn verify_tree(root: &Path, expected: &TreeIdentity) -> Result<()> {
    let root_metadata = fs::symlink_metadata(root)?;
    require(
        root.is_absolute()
            && root.canonicalize()? == root
            && root_metadata.is_dir()
            && root_metadata.uid() == unsafe { libc::getuid() }
            && root_metadata.mode() & 0o077 == 0,
        "experimental_runner_tree_root",
    )?;
    let mut pending = vec![root.to_owned()];
    let mut rows: Vec<(String, Value)> = Vec::new();
    let mut regular_bytes = 0_u64;
    while let Some(directory) = pending.pop() {
        for item in fs::read_dir(directory)? {
            let path = item?.path();
            let metadata = fs::symlink_metadata(&path)?;
            require(
                metadata.uid() == unsafe { libc::getuid() } && rows.len() < 14_000,
                "experimental_runner_tree_owner_or_capacity",
            )?;
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .ok_or("experimental_runner_tree_name")?
                .replace('\\', "/");
            let mode = metadata.mode() & 0o7777;
            let row = if metadata.is_dir() {
                pending.push(path);
                json!(["d", relative, mode])
            } else if metadata.is_file() {
                regular_bytes = regular_bytes
                    .checked_add(metadata.len())
                    .ok_or("experimental_runner_tree_extent")?;
                require(
                    regular_bytes <= 4 * 1024 * 1024 * 1024,
                    "experimental_runner_tree_extent",
                )?;
                json!(["f", relative, mode, metadata.len(), digest(&path)?])
            } else if metadata.file_type().is_symlink() {
                require(
                    path.canonicalize()?.starts_with(root),
                    "experimental_runner_tree_link",
                )?;
                let target = fs::read_link(&path)?
                    .to_str()
                    .ok_or("experimental_runner_tree_link")?
                    .to_owned();
                json!(["l", relative, mode, target])
            } else {
                return Err("experimental_runner_tree_type".into());
            };
            rows.push((relative, row));
        }
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hash = sha2::Sha256::new();
    for (_, row) in &rows {
        hash.update(serde_json::to_vec(row)?);
        hash.update(b"\n");
    }
    require(
        rows.len() as u64 == expected.entries
            && regular_bytes == expected.regular_bytes
            && hex(&hash.finalize()) == expected.sha256,
        "experimental_runner_tree_changed",
    )
}

fn candidate(manifest: &Artifact) -> Result<(Runner, String, String)> {
    manifest.verify()?;
    let c: CandidateManifest = read_json(&manifest.path)?;
    require(
        c.schema == 1
            && c.kind == "blackhole_dcomp_reference_runner"
            && c.dll_overrides == "d2d1,d3d11,dxgi,dcomp=b"
            && c.root.is_absolute()
            && c.root.canonicalize()? == c.root
            && c.tree.schema == 1
            && c.tree.entries == 12_278
            && c.tree.regular_bytes == 2_750_371_829
            && c.tree.sha256 == "5fde7d74c099818ea79cce78f329421425baecde5ed12da2ee427d7f4a0de1db"
            && valid_hex(&c.base_runner_sha256, 64)
            && c.source.proton_distribution_source_commit
                == "5b89db940e0ebe3a137a6009a3589232fe084c09"
            && c.source.wine_commit == "c27f058814b402a5709e073adccd42baa66810b9"
            && c.source.wine_tree == "622801bdffacb23188c29958c620cc12bf658843"
            && c.source.wine_configure == ["--enable-archs=i386,x86_64"]
            && c.source.wine_inf_sha256
                == "3b3e5c088988ae66da5b8908471751c7dee3255122ac6b66011840ded5e27d77"
            && c.runner.id == "proton-11.0-2c-dcomp-c27f058-reference"
            && c.runner.version == "11.0-2c+dcomp-c27f058-reference"
            && c.runner.files.len() == 12
            && c.graphics.len() == 10,
        "experimental_runner_manifest",
    )?;
    verify_tree(&c.root, &c.tree)?;
    c.build_recipe.verify()?;
    require(
        c.runner.policy.is_none(),
        "experimental_runner_manifest_policy",
    )?;
    require(
        c.runner.proton.canonicalize()?.starts_with(&c.root),
        "experimental_runner_proton_root",
    )?;
    let expected_graphics: std::collections::BTreeSet<_> = [
        "i386-windows/d2d1.dll",
        "i386-windows/d3d11.dll",
        "i386-windows/dcomp.dll",
        "i386-windows/dxgi.dll",
        "i386-windows/wined3d.dll",
        "x86_64-windows/d2d1.dll",
        "x86_64-windows/d3d11.dll",
        "x86_64-windows/dcomp.dll",
        "x86_64-windows/dxgi.dll",
        "x86_64-windows/wined3d.dll",
    ]
    .into_iter()
    .collect();
    require(
        c.graphics
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            == expected_graphics,
        "experimental_runner_graphics_binding",
    )?;
    for (relative, artifact) in &c.graphics {
        artifact.verify()?;
        require(
            artifact.path == c.root.join("files/lib/wine").join(relative)
                && c.runner.files.contains(artifact),
            "experimental_runner_graphics_binding",
        )?;
    }
    let mut runner = c.runner;
    runner.policy = Some(RunnerPolicy::DcompWineBuiltinsReferenceV1);
    runner.verify()?;
    Ok((runner, c.tree.sha256, c.base_runner_sha256))
}

fn canonical_runner_key(runner: &Runner) -> Result<String> {
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(
        &serde_json::to_value(runner)?,
    )?)))
}

fn require_base_runner(before: &Runner, candidate: &Runner) -> Result<()> {
    fn artifact<'a>(runner: &'a Runner, path: &Path) -> Result<&'a Artifact> {
        Ok(runner
            .files
            .iter()
            .find(|artifact| artifact.path == path)
            .ok_or("experimental_runner_base_artifact")?)
    }
    let before_entry = artifact(before, &before.entry_point)?;
    let candidate_entry = artifact(candidate, &candidate.entry_point)?;
    let before_proton = artifact(before, &before.proton)?;
    let candidate_proton = artifact(candidate, &candidate.proton)?;
    require(
        candidate.entry_point == before.entry_point
            && candidate_entry == before_entry
            && candidate_proton.sha256 == before_proton.sha256,
        "experimental_runner_candidate_base_changed",
    )
}

fn advance(before: &Environment, request: &Request, runner: Runner) -> Result<Environment> {
    require(
        request.schema == 1
            && request.environment == before.id
            && valid_hex(&request.expected_environment_sha256, 64)
            && request.expected_revision == before.revision
            && valid_hex(&request.expected_runner_sha256, 64)
            && request.expected_runner_sha256 == onboarding::runner_key(&before.runner)?,
        "experimental_runner_stale_request",
    )?;
    require(
        runner.policy == Some(RunnerPolicy::DcompWineBuiltinsReferenceV1)
            && runner != before.runner,
        "experimental_runner_unchanged_or_untyped",
    )?;
    let mut after = before.clone();
    after.runner = runner;
    after.revision = before
        .revision
        .checked_add(1)
        .ok_or("environment_revision_overflow")?;
    require(
        after.id == before.id && after.root == before.root,
        "experimental_runner_environment_changed",
    )?;
    Ok(after)
}

fn retire_removed(
    registry: &mut Registry,
    environment: &Environment,
    class: &str,
    expected: &publication::RevisionRef,
) -> Result<()> {
    let retired = registry
        .classes
        .get(class)
        .ok_or("experimental_runner_removed_publication_absent")?;
    require(
        registry.classes.iter().all(|(other, entry)| {
            other == class || entry.registration.environment.id != environment.id
        }) && retired.publication == Publication::Removed
            && retired.registration.environment == *environment
            && retired.managed_revision.as_ref() == Some(expected),
        "experimental_runner_removed_publication_changed",
    )?;
    registry.classes.remove(class);
    registry.revision = registry
        .revision
        .checked_add(1)
        .ok_or("registry revision overflow")?;
    Ok(())
}

fn restore(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension(format!("restore-{}", random_id()?));
    let result = (|| -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        File::open(path.parent().ok_or("experimental_runner_parent")?)?.sync_all()?;
        Ok(())
    })();
    let _ = fs::remove_file(temp);
    result
}

pub fn update(m: &Manager, input: &Path) -> Result<()> {
    let request: Request = read_json(input)?;
    require(
        valid_hex(&request.environment, 32),
        "experimental_runner_environment",
    )?;
    let (runner, tree_sha256, base_runner_sha256) = candidate(&request.candidate_manifest)?;

    let _service = m.lock("service.lock")?;
    let _canonical = m.lock("operator-canonical.lock")?;
    let _registry = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(
        renderer_cli::all_retired(m)?
            && dependency_cli::all_retired(m)?
            && onboarding::all_retired(m)?,
        "experimental_runner_session_active",
    )?;
    let service = Command::new("systemctl")
        .args([
            "--user",
            "show",
            "linux-vst-bridge.service",
            "--property=ActiveState",
            "--value",
        ])
        .output()?;
    require(
        service.status.success() && service.stdout == b"inactive\n",
        "experimental_runner_stop_bridge_first",
    )?;
    require(
        valid_hex(&request.retired_class, 32)
            && request.retired_class == request.retired_class.to_uppercase(),
        "experimental_runner_class",
    )?;
    let mut history: Vec<_> = onboarding::history_records(m)?
        .into_iter()
        .filter(|record| record.id == request.environment)
        .collect();
    require(history.len() == 1, "experimental_runner_onboarding_history")?;
    let mut onboarding = history.pop().unwrap();
    require(
        !onboarding.published,
        "experimental_runner_onboarding_published",
    )?;
    let marker = onboarding.environment.root.join("environment.json");
    let operation = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(onboarding.environment.root.join("operation.lock"))?;
    require(
        unsafe { libc::flock(operation.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "experimental_runner_environment_busy",
    )?;
    let record = onboarding::directory(m, &request.environment)?.join("record.json");
    require(
        digest(&marker)? == request.expected_environment_sha256
            && read_json::<Environment>(&marker)? == onboarding.environment
            && serde_json::to_value(read_json::<onboarding::Record>(&record)?)?
                == serde_json::to_value(&onboarding)?,
        "experimental_runner_environment_changed",
    )?;
    let before = onboarding.environment.clone();
    require(
        base_runner_sha256 == canonical_runner_key(&before.runner)?,
        "experimental_runner_candidate_base_changed",
    )?;
    require_base_runner(&before.runner, &runner)?;
    let after = advance(&before, &request, runner)?;
    let registry_path = m.root.join("registry.json");
    let mut registry = m.registry()?;
    require(
        !m.publication_pending(&request.retired_class)?
            && fs::symlink_metadata(m.link(&request.retired_class))
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "experimental_runner_removed_publication_changed",
    )?;
    retire_removed(
        &mut registry,
        &before,
        &request.retired_class,
        &request.expected_removed_publication,
    )?;
    let originals = [
        (marker.clone(), fs::read(&marker)?),
        (record.clone(), fs::read(&record)?),
        (registry_path.clone(), fs::read(&registry_path)?),
    ];
    let backup = m
        .root
        .join("private-rollback")
        .join(format!("experimental-runner-{}", random_id()?));
    private_dir(&backup)?;
    for (path, bytes) in &originals {
        let name = path.file_name().ok_or("experimental_runner_backup_name")?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(backup.join(name))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    atomic_json(
        &backup.join("transition.json"),
        &json!({
            "schema": 1,
            "state": "prepared",
            "environment": before.id,
            "before_revision": before.revision,
            "before_runner_sha256": onboarding::runner_key(&before.runner)?,
            "after_revision": after.revision,
            "after_runner_sha256": onboarding::runner_key(&after.runner)?,
            "candidate_manifest_sha256": request.candidate_manifest.sha256,
            "candidate_tree_sha256": tree_sha256,
            "retired_class": request.retired_class,
            "removed_publication": request.expected_removed_publication,
            "prefix_recreated": false,
            "installation_changed": false,
            "historical_results_rewritten": false,
            "publication_carried_forward": false
        }),
    )?;

    // This is a coordinated three-file transition, not crash-atomic storage.
    // Normal failures restore exact bytes. An interruption leaves the prepared
    // receipt and incompatible authority refuses later admission until the
    // retained rollback set is applied under the same locks.
    onboarding.environment = after.clone();
    let changed = (|| -> Result<()> {
        atomic_json(&marker, &after)?;
        atomic_json(&record, &onboarding)?;
        atomic_json(&registry_path, &registry)?;
        require(
            read_json::<Environment>(&marker)? == after,
            "experimental_runner_readback",
        )?;
        require(
            onboarding::load(m, &request.environment)?.environment == after,
            "experimental_runner_onboarding_readback",
        )?;
        require(
            !m.registry()?.classes.contains_key(&request.retired_class),
            "experimental_runner_registry_readback",
        )?;
        Ok(())
    })();
    if let Err(error) = changed {
        for (path, bytes) in &originals {
            restore(path, bytes)?;
        }
        atomic_json(
            &backup.join("result.json"),
            &json!({"schema":1,"state":"rolled_back","error":error.to_string()}),
        )?;
        return Err(error);
    }
    atomic_json(
        &backup.join("result.json"),
        &json!({
            "schema": 1,
            "state": "completed",
            "environment": after.id,
            "revision": after.revision,
            "runner_sha256": onboarding::runner_key(&after.runner)?,
            "candidate_manifest_sha256": request.candidate_manifest.sha256
        }),
    )?;
    println!(
        "{}",
        json!({
            "environment": after.id,
            "revision": after.revision,
            "runner": after.runner.id,
            "runner_policy": after.runner.policy,
            "rollback": backup
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(environment: &Environment) -> Request {
        Request {
            schema: 1,
            environment: environment.id.clone(),
            expected_environment_sha256: "ab".repeat(32),
            expected_revision: environment.revision,
            expected_runner_sha256: onboarding::runner_key(&environment.runner).unwrap(),
            retired_class: "EF".repeat(16),
            expected_removed_publication: publication::RevisionRef {
                id: "12".repeat(16),
                sha256: "34".repeat(32),
            },
            candidate_manifest: Artifact {
                path: "/unused".into(),
                sha256: "cd".repeat(32),
            },
        }
    }

    #[test]
    fn transition_is_revisioned_typed_and_compare_and_swap_bound() {
        let fixture = test_fixture::Fixture::new();
        let before = fixture.r.environment.clone();
        assert!(serde_json::to_value(&before.runner)
            .unwrap()
            .get("policy")
            .is_none());
        let mut next = before.runner.clone();
        next.id = "dcomp-reference".into();
        next.policy = Some(RunnerPolicy::DcompWineBuiltinsReferenceV1);
        let after = advance(&before, &request(&before), next).unwrap();
        assert_eq!(after.id, before.id);
        assert_eq!(after.root, before.root);
        assert_eq!(after.revision, before.revision + 1);
        assert_eq!(
            after.runner.policy,
            Some(RunnerPolicy::DcompWineBuiltinsReferenceV1)
        );
        let mut stale = request(&before);
        stale.expected_revision += 1;
        assert_eq!(
            advance(&before, &stale, after.runner.clone())
                .unwrap_err()
                .to_string(),
            "experimental_runner_stale_request"
        );
        let mut untyped = after.runner;
        untyped.policy = None;
        assert_eq!(
            advance(&before, &request(&before), untyped)
                .unwrap_err()
                .to_string(),
            "experimental_runner_unchanged_or_untyped"
        );
    }

    #[test]
    fn rollback_restore_preserves_exact_authority_bytes() {
        let fixture = test_fixture::Fixture::new();
        let path = fixture.outer.join("rollback.json");
        fs::write(&path, b"{\n  \"exact\": true\n}\n").unwrap();
        let before = fs::read(&path).unwrap();
        fs::write(&path, b"changed").unwrap();
        restore(&path, &before).unwrap();
        assert_eq!(fs::read(path).unwrap(), before);
    }

    #[test]
    fn only_exact_removed_publication_can_be_retired() {
        let fixture = test_fixture::Fixture::new();
        let class = fixture.r.key();
        let reference = publication::RevisionRef {
            id: "12".repeat(16),
            sha256: "34".repeat(32),
        };
        let entry = Entry {
            registration: fixture.r.clone(),
            publication: Publication::Removed,
            managed_revision: Some(reference.clone()),
        };
        let mut registry = Registry::default();
        registry.classes.insert(class.clone(), entry.clone());
        retire_removed(&mut registry, &fixture.r.environment, &class, &reference).unwrap();
        assert!(!registry.classes.contains_key(&class));

        let mut published = Registry::default();
        published.classes.insert(
            class.clone(),
            Entry {
                publication: Publication::Published,
                ..entry.clone()
            },
        );
        assert_eq!(
            retire_removed(&mut published, &fixture.r.environment, &class, &reference)
                .unwrap_err()
                .to_string(),
            "experimental_runner_removed_publication_changed"
        );
        let mut sibling = fixture.r.clone();
        sibling.metadata.class_id = "AB".repeat(16);
        let mut shared = Registry::default();
        shared.classes.insert(class.clone(), entry.clone());
        shared.classes.insert(
            sibling.key(),
            Entry {
                registration: sibling,
                ..entry
            },
        );
        assert_eq!(
            retire_removed(&mut shared, &fixture.r.environment, &class, &reference)
                .unwrap_err()
                .to_string(),
            "experimental_runner_removed_publication_changed"
        );
    }

    #[test]
    fn sealed_tree_hash_matches_build_manifest_algorithm() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let fixture = test_fixture::Fixture::new();
        let root = fixture.outer.join("sealed-tree");
        private_dir(&root).unwrap();
        let file = root.join("file");
        fs::write(&file, b"x").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        symlink("file", root.join("link")).unwrap();
        let expected = if cfg!(target_os = "macos") {
            "14ef9d8bc823c5d4d8199a6af7e18d2a9d52aa36913be5807f0cbaff02f01bf7"
        } else {
            "02026e602bb7e73cad6aad4977af30593ef5b21d93cbfe6a7cb1cdb16ce11d71"
        };
        verify_tree(
            &root,
            &TreeIdentity {
                schema: 1,
                entries: 2,
                regular_bytes: 1,
                sha256: expected.into(),
            },
        )
        .unwrap();
        fs::write(file, b"y").unwrap();
        assert_eq!(
            verify_tree(
                &root,
                &TreeIdentity {
                    schema: 1,
                    entries: 2,
                    regular_bytes: 1,
                    sha256: expected.into(),
                },
            )
            .unwrap_err()
            .to_string(),
            "experimental_runner_tree_changed"
        );
    }
}
