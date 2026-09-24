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
    #[serde(default)]
    predecessor_candidate: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TreeIdentity {
    pub(super) schema: u32,
    pub(super) entries: u64,
    pub(super) regular_bytes: u64,
    pub(super) sha256: String,
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

const TOUCH_ENVIRONMENT: &str = "4db060b14388e41103834fc4dfdd023a";
const TOUCH_CLASS: &str = "56534558667350736572756D20320000";
const TOUCH_BASE_RUNNER: &str = "proton-11.0-2c-25118279-slr4-4.0.20260805.254769";
const TOUCH_RUNNER: &str = "proton-11.0-2c-x11-touch-release-v1";
const TOUCH_PROTON_SOURCE: &str = "5b89db940e0ebe3a137a6009a3589232fe084c09";
const TOUCH_WINE_SOURCE: &str = "dc26e61847081a1b5cb0733dc30feba6ee575482";
const TOUCH_WINE_TREE: &str = "da4b1eb3b7f209eb4a971d4b5929fcda08ff6b4e";
const TOUCH_PATCH_SHA256: &str = "3e8fa75ddb3f1dcfa2b7f73a82d97eeaf8bed0a0d3d51eed6afd514ab3b0723a";
const TOUCH_PATCHED_MOUSE_SHA256: &str = "d7a76c5d9769f2abb4eefed56293d1ce83fbbce87eab2ca9d4fef4316cc43131";
const TOUCH_HEADER_SHA256: &str = "ff549851137e2ec4eaacbdb1960c5b25771bafe519cbe008da702b9b261f73a9";
const TOUCH_SDK_SHA256: &str = "97526b794ce1a9bed5f891084462260b3a02399569f7438a3a57b5a253001db9";
const TOUCH_BASE_PROTON_SHA256: &str =
    "787504a79bacf6b303984a9a846cf47463f36599248e8306b26d5faf78267aad";
const TOUCH_BASE_ENTRY_SHA256: &str =
    "caa39b5cde8ea955288b574b49b3416fd16e7be3f53a17249d673f5ecfdce2c1";
const TOUCH_TREE_SHA256: &str = "d095f1f052ecebb67c66d685dbd88e373b633b0c3000343a7607c4f1bc4d1920";
const TOUCH_ROUTING_RUNNER: &str = "proton-11.0-2c-x11-touch-routing-v2";
const TOUCH_ROUTING_PATCH_SHA256: &str = "44c3c11806fe48ed0358fd3469d60c5572c0f78773057ef4c335198c1c497203";
const TOUCH_ROUTING_MOUSE_SHA256: &str = "8e79383dd969aeb7c48533dcfab945a42c27ac6072ff96a79d9091f7f5a7a2aa";
const TOUCH_ROUTING_HEADER_SHA256: &str = "fb91d7b0c1bb51b183aadb71e3505687c42827b1381dbc700a4b8fddefa6c57d";
const TOUCH_ROUTING_BASE_DRIVER_SHA256: &str = "c939ed62a6226a2e88827428281f2e934ebb5546fad101bd354a8a64164d00dc";
const TOUCH_ROUTING_TREE_SHA256: &str = "de6c55c2a8c82abf2b1b0b47a97ee97a00657fa04582b2112327fa3d0095a698";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TouchSourceIdentity {
    proton_distribution_source_commit: String,
    wine_commit: String,
    wine_tree: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TouchCandidateManifest {
    schema: u32,
    kind: String,
    environment: String,
    base_runner_id: String,
    base_runner_sha256: String,
    source: TouchSourceIdentity,
    patch: Artifact,
    build_receipt: Artifact,
    root: PathBuf,
    tree: TreeIdentity,
    runner: Runner,
    changed_artifacts: BTreeMap<String, Artifact>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TouchBuildReceipt {
    schema: u32,
    proton_distribution_source_commit: String,
    wine_commit: String,
    wine_tree: String,
    patch_sha256: String,
    patched_mouse_sha256: String,
    touch_header_sha256: String,
    sdk_image_sha256: String,
    configure: Vec<String>,
    install_prefix: PathBuf,
    configure_log: Artifact,
    build_log: Artifact,
    install_log: Artifact,
    changed_artifacts: BTreeMap<String, String>,
    candidate_tree_sha256: String,
    mapping_test_passed: bool,
    build_passed: bool,
    unlicensed_smoke_passed: bool,
    unlicensed_smoke_initialize_exit: i32,
    unlicensed_smoke_cmd_exit: i32,
    unlicensed_smoke_cleanup_confirmed: bool,
    wow64_no_i386_unix_driver: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateKind {
    Dcomp,
    X11TouchRelease,
    X11TouchRouting,
}

pub(super) fn verify_tree(root: &Path, expected: &TreeIdentity) -> Result<()> {
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

fn dcomp_candidate(manifest: &Artifact) -> Result<(Runner, String, String)> {
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

fn touch_candidate(manifest: &Artifact, routing: bool) -> Result<(Runner, String, String)> {
    manifest.verify()?;
    let c: TouchCandidateManifest = read_json(&manifest.path)?;
    let kind = if routing { "x11_touch_routing_reference_runner" } else { "x11_touch_release_reference_runner" };
    let base_runner = if routing { TOUCH_RUNNER } else { TOUCH_BASE_RUNNER };
    let runner_id = if routing { TOUCH_ROUTING_RUNNER } else { TOUCH_RUNNER };
    let patch_sha = if routing { TOUCH_ROUTING_PATCH_SHA256 } else { TOUCH_PATCH_SHA256 };
    let mouse_sha = if routing { TOUCH_ROUTING_MOUSE_SHA256 } else { TOUCH_PATCHED_MOUSE_SHA256 };
    let header_sha = if routing { TOUCH_ROUTING_HEADER_SHA256 } else { TOUCH_HEADER_SHA256 };
    let tree_sha = if routing { TOUCH_ROUTING_TREE_SHA256 } else { TOUCH_TREE_SHA256 };
    require(
        c.schema == 1
            && c.kind == kind
            && c.environment == TOUCH_ENVIRONMENT
            && c.base_runner_id == base_runner
            && valid_hex(&c.base_runner_sha256, 64)
            && c.source.proton_distribution_source_commit == TOUCH_PROTON_SOURCE
            && c.source.wine_commit == TOUCH_WINE_SOURCE
            && c.source.wine_tree == TOUCH_WINE_TREE
            && c.patch.sha256 == patch_sha
            && c.tree.schema == 1
            && c.tree.entries == 8_262
            && c.tree.regular_bytes == if routing { 1_447_962_774 } else { 1_447_967_926 }
            && c.tree.sha256 == tree_sha
            && c.runner.id == runner_id
            && c.runner.version == if routing {
                "1788504981 proton-11.0-2c-x86_64+x11-touch-routing-v2; SLR 4.0.20260805.254769"
            } else {
                "1788504981 proton-11.0-2c-x86_64+x11-touch-release-v1; SLR 4.0.20260805.254769"
            }
            && c.runner.files.len() == 32
            && c.runner.policy.is_none()
            && c.root.is_absolute()
            && c.root.canonicalize()? == c.root,
        "experimental_runner_touch_manifest",
    )?;
    c.patch.verify()?;
    verify_tree(&c.root, &c.tree)?;
    let expected_paths: std::collections::BTreeSet<_> = [
        "version",
        "files/lib/wine/x86_64-unix/winex11.so",
    ]
    .into_iter()
    .collect();
    require(
        c.changed_artifacts
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            == expected_paths,
        "experimental_runner_touch_artifacts",
    )?;
    for (relative, artifact) in &c.changed_artifacts {
        artifact.verify()?;
        require(
            artifact.path == c.root.join(relative) && c.runner.files.contains(artifact),
            "experimental_runner_touch_artifacts",
        )?;
    }
    c.build_receipt.verify()?;
    let receipt: TouchBuildReceipt = read_json(&c.build_receipt.path)?;
    receipt.configure_log.verify()?;
    receipt.build_log.verify()?;
    receipt.install_log.verify()?;
    require(
        receipt.schema == 1
            && receipt.proton_distribution_source_commit == TOUCH_PROTON_SOURCE
            && receipt.wine_commit == TOUCH_WINE_SOURCE
            && receipt.wine_tree == TOUCH_WINE_TREE
            && receipt.patch_sha256 == patch_sha
            && receipt.patched_mouse_sha256 == mouse_sha
            && receipt.touch_header_sha256 == header_sha
            && receipt.sdk_image_sha256 == TOUCH_SDK_SHA256
            && receipt.configure == ["--enable-archs=i386,x86_64"]
            && receipt.install_prefix == c.build_receipt.path.parent().ok_or("experimental_runner_touch_build_receipt")?.join("stage")
            && receipt.configure_log.path == c.build_receipt.path.parent().ok_or("experimental_runner_touch_build_receipt")?.join("configure.private.log")
            && receipt.build_log.path == c.build_receipt.path.parent().ok_or("experimental_runner_touch_build_receipt")?.join("build.private.log")
            && receipt.install_log.path == c.build_receipt.path.parent().ok_or("experimental_runner_touch_build_receipt")?.join("install.private.log")
            && receipt.candidate_tree_sha256 == c.tree.sha256
            && receipt.mapping_test_passed
            && receipt.build_passed
            && receipt.unlicensed_smoke_passed
            && receipt.unlicensed_smoke_initialize_exit == 0
            && receipt.unlicensed_smoke_cmd_exit == 0
            && receipt.unlicensed_smoke_cleanup_confirmed
            && receipt.wow64_no_i386_unix_driver
            && receipt.changed_artifacts
                == c.changed_artifacts
                    .iter()
                    .map(|(path, artifact)| (path.clone(), artifact.sha256.clone()))
                    .collect(),
        "experimental_runner_touch_build_receipt",
    )?;
    require(
        c.runner.proton.canonicalize()?.starts_with(&c.root),
        "experimental_runner_proton_root",
    )?;
    let mut runner = c.runner;
    runner.policy = Some(if routing { RunnerPolicy::X11TouchRoutingV2 } else { RunnerPolicy::X11TouchReleaseV1 });
    runner.verify()?;
    Ok((runner, c.tree.sha256, c.base_runner_sha256))
}

fn candidate(manifest: &Artifact) -> Result<(CandidateKind, Runner, String, String)> {
    manifest.verify()?;
    let value: Value = read_json(&manifest.path)?;
    match value.get("kind").and_then(Value::as_str) {
        Some("blackhole_dcomp_reference_runner") => {
            let (runner, tree, base) = dcomp_candidate(manifest)?;
            Ok((CandidateKind::Dcomp, runner, tree, base))
        }
        Some("x11_touch_release_reference_runner") => {
            let (runner, tree, base) = touch_candidate(manifest, false)?;
            Ok((CandidateKind::X11TouchRelease, runner, tree, base))
        }
        Some("x11_touch_routing_reference_runner") => {
            let (runner, tree, base) = touch_candidate(manifest, true)?;
            Ok((CandidateKind::X11TouchRouting, runner, tree, base))
        }
        _ => Err("experimental_runner_manifest_kind".into()),
    }
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

fn require_touch_runner_mapping(before: &Runner, after: &Runner) -> Result<()> {
    require(
        before.id == TOUCH_BASE_RUNNER
            && before.policy.is_none()
            && after.id == TOUCH_RUNNER
            && after.policy == Some(RunnerPolicy::X11TouchReleaseV1)
            && before
                .files
                .iter()
                .any(|file| file.path == before.proton && file.sha256 == TOUCH_BASE_PROTON_SHA256)
            && before.files.iter().any(|file| {
                file.path == before.entry_point && file.sha256 == TOUCH_BASE_ENTRY_SHA256
            }),
        "experimental_runner_touch_base",
    )?;
    let before_root = before
        .proton
        .parent()
        .ok_or("experimental_runner_touch_base")?;
    let after_root = after
        .proton
        .parent()
        .ok_or("experimental_runner_touch_base")?;
    let actual: BTreeMap<_, _> = after
        .files
        .iter()
        .map(|file| (file.path.clone(), file.sha256.clone()))
        .collect();
    require(
        actual.len() == after.files.len(),
        "experimental_runner_touch_roster",
    )?;
    let mut expected = BTreeMap::new();
    for file in &before.files {
        let path = match file.path.strip_prefix(before_root) {
            Ok(relative) => after_root.join(relative),
            Err(_) => file.path.clone(),
        };
        let hash = if path == after_root.join("version") {
            let value = actual
                .get(&path)
                .ok_or("experimental_runner_touch_roster")?;
            require(value != &file.sha256, "experimental_runner_touch_version")?;
            value.clone()
        } else {
            file.sha256.clone()
        };
        require(
            expected.insert(path, hash).is_none(),
            "experimental_runner_touch_roster",
        )?;
    }
    let path = after_root.join("files/lib/wine/x86_64-unix/winex11.so");
    let hash = actual
        .get(&path)
        .ok_or("experimental_runner_touch_roster")?;
    require(
        expected.insert(path, hash.clone()).is_none(),
        "experimental_runner_touch_roster",
    )?;
    require(expected == actual, "experimental_runner_touch_roster")
}

fn require_touch_routing_runner_mapping(before: &Runner, after: &Runner) -> Result<()> {
    require(
        before.id == TOUCH_RUNNER
            && before.policy == Some(RunnerPolicy::X11TouchReleaseV1)
            && after.id == TOUCH_ROUTING_RUNNER
            && after.policy == Some(RunnerPolicy::X11TouchRoutingV2),
        "experimental_runner_routing_base",
    )?;
    let before_root = before.proton.parent().ok_or("experimental_runner_routing_base")?;
    let after_root = after.proton.parent().ok_or("experimental_runner_routing_base")?;
    let old_driver = before_root.join("files/lib/wine/x86_64-unix/winex11.so");
    require(
        before.files.iter().any(|file| file.path == old_driver && file.sha256 == TOUCH_ROUTING_BASE_DRIVER_SHA256),
        "experimental_runner_routing_base_driver",
    )?;
    let actual: BTreeMap<_, _> = after.files.iter().map(|file| (file.path.clone(), file.sha256.clone())).collect();
    require(actual.len() == after.files.len() && after.files.len() == before.files.len(), "experimental_runner_routing_roster")?;
    let mut expected = BTreeMap::new();
    for file in &before.files {
        let path = match file.path.strip_prefix(before_root) {
            Ok(relative) => after_root.join(relative),
            Err(_) => file.path.clone(),
        };
        let hash = if path == after_root.join("version") || path == after_root.join("files/lib/wine/x86_64-unix/winex11.so") {
            let changed = actual.get(&path).ok_or("experimental_runner_routing_roster")?;
            require(changed != &file.sha256, "experimental_runner_routing_artifact_unchanged")?;
            changed.clone()
        } else {
            file.sha256.clone()
        };
        require(expected.insert(path, hash).is_none(), "experimental_runner_routing_roster")?;
    }
    require(expected == actual, "experimental_runner_routing_roster")
}

fn advance(
    before: &Environment,
    request: &Request,
    runner: Runner,
    policy: RunnerPolicy,
) -> Result<Environment> {
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
        runner.policy == Some(policy) && runner != before.runner,
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

pub(super) fn restore(path: &Path, bytes: &[u8]) -> Result<()> {
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
    let (kind, runner, tree_sha256, base_runner_sha256) = candidate(&request.candidate_manifest)?;
    let policy = match kind {
        CandidateKind::Dcomp => RunnerPolicy::DcompWineBuiltinsReferenceV1,
        CandidateKind::X11TouchRelease => RunnerPolicy::X11TouchReleaseV1,
        CandidateKind::X11TouchRouting => RunnerPolicy::X11TouchRoutingV2,
    };
    if matches!(kind, CandidateKind::X11TouchRelease | CandidateKind::X11TouchRouting) {
        let predecessor = if kind == CandidateKind::X11TouchRouting {
            preparation::SERUM_TOUCH_ROUTING_PREDECESSOR
        } else {
            preparation::SERUM_TOUCH_PREDECESSOR
        };
        require(
            request.environment == TOUCH_ENVIRONMENT
                && request.retired_class == TOUCH_CLASS
                && request.predecessor_candidate.as_deref() == Some(predecessor),
            "experimental_runner_touch_selection",
        )?;
    } else {
        require(request.predecessor_candidate.is_none(), "experimental_runner_dcomp_request")?;
    }

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
    match kind {
        CandidateKind::X11TouchRelease => require_touch_runner_mapping(&before.runner, &runner)?,
        CandidateKind::X11TouchRouting => require_touch_routing_runner_mapping(&before.runner, &runner)?,
        CandidateKind::Dcomp => {},
    }
    let after = advance(&before, &request, runner, policy)?;
    let predecessor_id = match kind {
        CandidateKind::X11TouchRelease => Some(preparation::SERUM_TOUCH_PREDECESSOR),
        CandidateKind::X11TouchRouting => Some(preparation::SERUM_TOUCH_ROUTING_PREDECESSOR),
        CandidateKind::Dcomp => None,
    };
    let predecessor = if let Some(predecessor_id) = predecessor_id {
        let prior = preparation::retained_candidates(m)?
            .into_iter()
            .find(|candidate| candidate.id().is_ok_and(|id| id == predecessor_id))
            .ok_or("experimental_runner_touch_predecessor_absent")?;
        require(
            prior.selection.environment == before
                && prior.selection.class.id == TOUCH_CLASS
                && prior.origin == if kind == CandidateKind::X11TouchRouting {
                    preparation::Origin::X11TouchReleaseV1
                } else {
                    preparation::Origin::ManagedPreparation
                },
            "experimental_runner_touch_predecessor",
        )?;
        preparation::verify_candidate(m, &prior, &prior.selection.scanner, &prior.selection.scanner_source)?;
        Some(prior)
    } else { None };
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
    let backup = m.root.join("private-rollback").join(match kind {
        CandidateKind::Dcomp => format!("experimental-runner-{}", random_id()?),
        CandidateKind::X11TouchRelease => format!("experimental-runner-touch-{}", before.id),
        CandidateKind::X11TouchRouting => format!("experimental-runner-routing-{}", before.id),
    });
    require(
        fs::symlink_metadata(&backup).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "experimental_runner_rollback_exists",
    )?;
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
    let complete = (|| -> Result<Option<String>> {
        let result = backup.join("result.json");
        let mut record = json!({
            "schema": 1,
            "state": "completed",
            "environment": after.id,
            "revision": after.revision,
            "runner_sha256": onboarding::runner_key(&after.runner)?,
            "candidate_manifest_sha256": request.candidate_manifest.sha256
        });
        if predecessor.is_some() {
            record["carried_predecessor"] = json!(predecessor_id.unwrap());
        }
        if predecessor.is_some() {
            // A kill between environment advance and candidate materialization
            // must not leave a falsely completed transition receipt.
            let mut pending = record.clone();
            pending["state"] = json!("candidate_pending");
            atomic_json(&result, &pending)?;
        } else {
            atomic_json(&result, &record)?;
        }
        if let Some(prior) = predecessor {
            let mut expected_result_bytes = serde_json::to_vec(&record)?;
            expected_result_bytes.push(b'\n');
            let provenance = preparation::TouchCarryForward {
                predecessor: predecessor_id.unwrap().into(),
                transition: Artifact { path: result.clone(), sha256: hex(&sha2::Sha256::digest(&expected_result_bytes)) },
                runner_manifest: request.candidate_manifest.clone(),
            };
            let candidate = if kind == CandidateKind::X11TouchRouting {
                preparation::touch_routing_successor(&prior, &after, provenance)?
            } else {
                preparation::touch_successor(&prior, &after, provenance)?
            };
            let id = preparation::record_candidate_with_predecessor(m, &candidate, predecessor_id)?;
            atomic_json(&result, &record)?;
            preparation::verify_candidate(m, &candidate, &candidate.host, &candidate.source_manifest.sha256)?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
    })();
    let successor = match complete {
        Ok(value) => value,
        Err(error) => {
            for (path, bytes) in &originals { restore(path, bytes)?; }
            atomic_json(&backup.join("result.json"), &json!({"schema":1,"state":"rolled_back","error":error.to_string()}))?;
            return Err(error);
        }
    };
    let mut output = json!({
        "environment": after.id,
        "revision": after.revision,
        "runner": after.runner.id,
        "runner_policy": after.runner.policy,
        "rollback": backup
    });
    if let Some(id) = successor {
        output[if kind == CandidateKind::X11TouchRouting { "candidate_d" } else { "candidate_c" }] = json!(id);
    }
    println!("{output}");
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
            predecessor_candidate: None,
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
        let after = advance(
            &before,
            &request(&before),
            next,
            RunnerPolicy::DcompWineBuiltinsReferenceV1,
        )
        .unwrap();
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
            advance(
                &before,
                &stale,
                after.runner.clone(),
                RunnerPolicy::DcompWineBuiltinsReferenceV1,
            )
            .unwrap_err()
            .to_string(),
            "experimental_runner_stale_request"
        );
        let mut untyped = after.runner;
        untyped.policy = None;
        assert_eq!(
            advance(
                &before,
                &request(&before),
                untyped,
                RunnerPolicy::DcompWineBuiltinsReferenceV1,
            )
            .unwrap_err()
            .to_string(),
            "experimental_runner_unchanged_or_untyped"
        );
    }

    #[test]
    fn touch_successor_preserves_base_roster_and_changes_only_declared_files() {
        let fixture = test_fixture::Fixture::new();
        let mut before = fixture.r.environment.runner.clone();
        before.id = TOUCH_BASE_RUNNER.into();
        before.proton = "/old/proton".into();
        before.entry_point = "/runtime/entry".into();
        before.files = vec![
            Artifact {
                path: before.proton.clone(),
                sha256: TOUCH_BASE_PROTON_SHA256.into(),
            },
            Artifact {
                path: before.entry_point.clone(),
                sha256: TOUCH_BASE_ENTRY_SHA256.into(),
            },
            Artifact {
                path: "/old/version".into(),
                sha256: "aa".repeat(32),
            },
            Artifact {
                path: "/old/files/bin/wine".into(),
                sha256: "bb".repeat(32),
            },
        ];
        let mut after = before.clone();
        after.id = TOUCH_RUNNER.into();
        after.policy = Some(RunnerPolicy::X11TouchReleaseV1);
        after.proton = "/new/proton".into();
        after.files = vec![
            Artifact {
                path: after.proton.clone(),
                sha256: TOUCH_BASE_PROTON_SHA256.into(),
            },
            before.files[1].clone(),
            Artifact {
                path: "/new/version".into(),
                sha256: "cc".repeat(32),
            },
            Artifact {
                path: "/new/files/bin/wine".into(),
                sha256: "bb".repeat(32),
            },
            Artifact {
                path: "/new/files/lib/wine/x86_64-unix/winex11.so".into(),
                sha256: "ee".repeat(32),
            },
        ];
        require_touch_runner_mapping(&before, &after).unwrap();
        after.files[3].sha256 = "ff".repeat(32);
        assert_eq!(
            require_touch_runner_mapping(&before, &after)
                .unwrap_err()
                .to_string(),
            "experimental_runner_touch_roster"
        );
        after.files[3].sha256 = "bb".repeat(32);
        after.files.pop();
        assert_eq!(
            require_touch_runner_mapping(&before, &after)
                .unwrap_err()
                .to_string(),
            "experimental_runner_touch_roster"
        );
    }

    #[test]
    fn touch_routing_successor_preserves_candidate_c_roster() {
        let fixture = test_fixture::Fixture::new();
        let mut before = fixture.r.environment.runner.clone();
        before.id = TOUCH_RUNNER.into();
        before.policy = Some(RunnerPolicy::X11TouchReleaseV1);
        before.proton = "/candidate-c/proton".into();
        before.entry_point = "/runtime/entry".into();
        before.files = vec![
            Artifact { path: before.proton.clone(), sha256: TOUCH_BASE_PROTON_SHA256.into() },
            Artifact { path: before.entry_point.clone(), sha256: TOUCH_BASE_ENTRY_SHA256.into() },
            Artifact { path: "/candidate-c/version".into(), sha256: "aa".repeat(32) },
            Artifact { path: "/candidate-c/files/bin/wine".into(), sha256: "bb".repeat(32) },
            Artifact { path: "/candidate-c/files/lib/wine/x86_64-unix/winex11.so".into(), sha256: TOUCH_ROUTING_BASE_DRIVER_SHA256.into() },
        ];
        let mut after = before.clone();
        after.id = TOUCH_ROUTING_RUNNER.into();
        after.policy = Some(RunnerPolicy::X11TouchRoutingV2);
        after.proton = "/candidate-d/proton".into();
        after.files = vec![
            Artifact { path: after.proton.clone(), sha256: TOUCH_BASE_PROTON_SHA256.into() },
            before.files[1].clone(),
            Artifact { path: "/candidate-d/version".into(), sha256: "cc".repeat(32) },
            Artifact { path: "/candidate-d/files/bin/wine".into(), sha256: "bb".repeat(32) },
            Artifact { path: "/candidate-d/files/lib/wine/x86_64-unix/winex11.so".into(), sha256: "dd".repeat(32) },
        ];
        require_touch_routing_runner_mapping(&before, &after).unwrap();
        after.files[3].sha256 = "ff".repeat(32);
        assert_eq!(require_touch_routing_runner_mapping(&before, &after).unwrap_err().to_string(),
                   "experimental_runner_routing_roster");
        after.files[3].sha256 = "bb".repeat(32);
        after.files.pop();
        assert_eq!(require_touch_routing_runner_mapping(&before, &after).unwrap_err().to_string(),
                   "experimental_runner_routing_roster");
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
