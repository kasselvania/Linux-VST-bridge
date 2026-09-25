//! WD0: one typed Windows DAW workspace adjacent to, never inside, DSP admission.
//! The immutable executable/runtime closure and mutable FL data have distinct owners.
use super::*;
use linux_vst_bridge::operator_model as ui;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::{FileTypeExt, MetadataExt};

const APP: &str = "fl-studio";
const STANDARD_RUNNER: &str = "proton-11.0-2c-25118279-slr4-4.0.20260805.254769";
const FL_CRYPT32_RUNNER: &str = "proton-11.0-2c-fl-crypt32-order-v1";
const FL_CRYPT32_TREE: &str = "ef6e2412c74a018831393226f2c3b9816f561d897be508dceda8f2dd9b03f92e";
const FL_CRYPT32_PATCH: &str = "e3cf1069975463c78506926b30f3f7f4d94169ef178d68a4eeb4b0a41f69e806";
const FL_CRYPT32_SDK: &str = "97526b794ce1a9bed5f891084462260b3a02399569f7438a3a57b5a253001db9";
const FL_CRYPT32_VERSION_SHA: &str =
    "2592ccc90077658b3359a0b85e9fd4c319c8f724968811ebd955879dcae03646";
const FL_CRYPT32_I386_SHA: &str =
    "d57b16f7149d583fd9b8b0a2a8851a3fa9578599fe3acfadcefd372253bdf1e5";
const FL_CRYPT32_X64_SHA: &str = "bc5af588ddd0571c42bf6fce3bfa513362a15549d340ec49f8d4acf7952b8ffd";
const FL_CRYPT32_SOURCE: &[(&str, &str)] = &[
    (
        "dlls/crypt32/crypt32_private.h",
        "4161501bbfea841c08bcdcf7ac8012da6544f1b1c527efe79c2b1165dfecbd19",
    ),
    (
        "dlls/crypt32/encode.c",
        "b736b05d134e236260a1a9e58b7e1a05c7d5006d6ecf5c3eae2353d1d832a77c",
    ),
    (
        "dlls/crypt32/msg.c",
        "89c24b49dc458cae7610b195ec36f0f2796a462a04284975368227a2f83572ca",
    ),
    (
        "dlls/crypt32/tests/msg.c",
        "ff5ee9df8ebd3e22a814cc2b549197d605fcb23079fac70f477ac3d8f5be05a9",
    ),
];
const SESSION_SECONDS: u64 = 30;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ApplicationId {
    FlStudio,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum State {
    Imported,
    Installing,
    NeedsUserAction,
    Installed,
    Ready,
    Starting,
    Running,
    Stopping,
    Uninstalling,
    Uninstalled,
    Failed,
    CleanupUnconfirmed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InstalledApplication {
    executable: Artifact,
    installer_advertised_release: String,
    observed_file_version: Option<String>,
    resource_root: PathBuf,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AudioBackend {
    PulseAudio,
    WineAsio,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct AudioObservation {
    backend: AudioBackend,
    sample_rate: u32,
    buffer_frames: u32,
    linux_endpoint: String,
    operator_audible: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InstallerSelection {
    sha256: String,
    release: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum InstallationOutcome {
    Pending,
    Installed,
    NeedsUserAction,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InstallationRecord {
    operation: String,
    installer: InstallerSelection,
    outcome: InstallationOutcome,
    installed_image: Option<Artifact>,
    observed_version: Option<String>,
    failure: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum UninstallOutcome {
    Pending,
    Completed,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UninstallRecord {
    operation: String,
    installation_operation: Option<String>,
    installed_image: Option<Artifact>,
    outcome: UninstallOutcome,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Workspace {
    schema: u32,
    id: String,
    revision: u64,
    application: ApplicationId,
    selected_installer: InstallerSelection,
    environment: Environment,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runner_manifest: Option<Artifact>,
    projects: PathBuf,
    exports: PathBuf,
    preferences: PathBuf,
    state: State,
    active_installation_operation: Option<String>,
    installations: Vec<InstallationRecord>,
    current_installation_operation: Option<String>,
    installed: Option<InstalledApplication>,
    audio: Option<AudioObservation>,
    session_operation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_uninstall_operation: Option<String>,
    uninstalls: Vec<UninstallRecord>,
    history_recovery_required: bool,
    first_failure: Option<String>,
}

// Exact on-disk WD0 shape from before repeatable installation. Do not edit it
// in place or treat its one retained operation as a lifetime install gate.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyWorkspace {
    schema: u32,
    id: String,
    revision: u64,
    application: ApplicationId,
    installer: String,
    installer_release: String,
    environment: Environment,
    #[serde(default)]
    runner_manifest: Option<Artifact>,
    projects: PathBuf,
    exports: PathBuf,
    preferences: PathBuf,
    state: State,
    installation_operation: Option<String>,
    installed: Option<InstalledApplication>,
    audio: Option<AudioObservation>,
    session_operation: Option<String>,
    #[serde(default)]
    uninstall_operation: Option<String>,
    first_failure: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FlRunnerSource {
    proton: String,
    wine: String,
    wine_tree: String,
    upstream_merge_request: String,
    upstream_patch: Artifact,
    patched_files: BTreeMap<String, String>,
    sdk_image_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FlRunnerManifest {
    schema: u32,
    kind: String,
    workspace: String,
    base_runner: Runner,
    source: FlRunnerSource,
    root: PathBuf,
    tree: experimental_runner::TreeIdentity,
    runner: Runner,
    changed_artifacts: BTreeMap<String, Artifact>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Tooling {
    schema: u32,
    generation: String,
    source_head: String,
    source_tree: String,
    manager: Artifact,
    supervisor: Artifact,
    ownership: Artifact,
    importer: Artifact,
}

fn base(m: &Manager) -> PathBuf {
    m.root.join("daw-workspaces").join(APP)
}

fn record(m: &Manager) -> PathBuf {
    base(m).join("workspace.json")
}

fn fl_runner_root(m: &Manager) -> PathBuf {
    base(m).join("runner-closures").join(FL_CRYPT32_RUNNER)
}

fn fl_runner_manifest(m: &Manager) -> PathBuf {
    base(m).join("runner-closures/fl-runner-manifest.private.json")
}

fn fl_runner_transition(m: &Manager) -> PathBuf {
    base(m).join("runner-closures/transition.private.json")
}

fn transition_settled(m: &Manager) -> Result<()> {
    let path = fl_runner_transition(m);
    if path.try_exists()? {
        let receipt: Value = read_json(&path)?;
        require(
            receipt["schema"] == 1
                && matches!(receipt["state"].as_str(), Some("committed" | "rolled_back")),
            "daw_workspace_runner_recovery_required",
        )?;
    }
    Ok(())
}

fn fl_runner_candidate(
    m: &Manager,
    manifest: &Artifact,
    verify_full_tree: bool,
) -> Result<FlRunnerManifest> {
    require(
        manifest.path == fl_runner_manifest(m),
        "daw_workspace_runner_manifest_path",
    )?;
    manifest.verify()?;
    let c: FlRunnerManifest = read_json(&manifest.path)?;
    let root = fl_runner_root(m);
    require(
        c.schema == 1
            && c.kind == "fl_crypt32_order_reference_runner"
            && c.root == root
            && c.workspace
                == read_json::<Value>(&record(m))?["id"]
                    .as_str()
                    .ok_or("daw_workspace_id_absent")?
            && c.base_runner.id == STANDARD_RUNNER
            && c.base_runner.policy.is_none()
            && c.runner.id == FL_CRYPT32_RUNNER
            && c.runner.policy.is_none()
            && c.runner.proton == root.join("proton")
            && c.runner.entry_point == c.base_runner.entry_point
            && c.runner.version
                == "1788504981 proton-11.0-2c-x86_64+fl-crypt32-order-v1; SLR 4.0.20260805.254769"
            && c.tree.schema == 1
            && c.tree.sha256 == FL_CRYPT32_TREE
            && c.source.proton == "5b89db940e0ebe3a137a6009a3589232fe084c09"
            && c.source.wine == "dc26e61847081a1b5cb0733dc30feba6ee575482"
            && c.source.wine_tree == "da4b1eb3b7f209eb4a971d4b5929fcda08ff6b4e"
            && c.source.upstream_merge_request == "wine/wine!11824"
            && c.source.sdk_image_sha256 == FL_CRYPT32_SDK
            && c.source.upstream_patch.path
                == root
                    .parent()
                    .ok_or("daw_workspace_runner_root")?
                    .join("upstream-11824.patch")
            && c.source.upstream_patch.sha256 == FL_CRYPT32_PATCH,
        "daw_workspace_runner_candidate_identity",
    )?;
    c.source.upstream_patch.verify()?;
    let expected_source: BTreeMap<String, String> = FL_CRYPT32_SOURCE
        .iter()
        .map(|(path, sha)| ((*path).to_owned(), (*sha).to_owned()))
        .collect();
    require(
        c.source.patched_files == expected_source,
        "daw_workspace_runner_source",
    )?;
    let expected_changed = [
        ("version", FL_CRYPT32_VERSION_SHA),
        (
            "files/lib/wine/i386-windows/crypt32.dll",
            FL_CRYPT32_I386_SHA,
        ),
        (
            "files/lib/wine/x86_64-windows/crypt32.dll",
            FL_CRYPT32_X64_SHA,
        ),
    ];
    require(
        c.changed_artifacts.len() == expected_changed.len(),
        "daw_workspace_runner_delta",
    )?;
    for (relative, sha) in expected_changed {
        let artifact = c
            .changed_artifacts
            .get(relative)
            .ok_or("daw_workspace_runner_delta")?;
        require(
            artifact.path == root.join(relative) && artifact.sha256 == sha,
            "daw_workspace_runner_delta",
        )?;
        artifact.verify()?;
    }
    let old_root = c
        .base_runner
        .proton
        .parent()
        .ok_or("daw_workspace_base_runner_root")?;
    let mut expected_files = BTreeMap::new();
    for item in &c.base_runner.files {
        let path = if item.path.starts_with(old_root) {
            root.join(item.path.strip_prefix(old_root)?)
        } else {
            item.path.clone()
        };
        let relative = path.strip_prefix(&root).ok().and_then(|v| v.to_str());
        let sha = if let Some(relative) = relative {
            c.changed_artifacts
                .get(relative)
                .map_or(item.sha256.as_str(), |a| a.sha256.as_str())
        } else {
            item.sha256.as_str()
        };
        require(
            expected_files.insert(path, sha.to_owned()).is_none(),
            "daw_workspace_runner_roster",
        )?;
    }
    for relative in [
        "files/lib/wine/i386-windows/crypt32.dll",
        "files/lib/wine/x86_64-windows/crypt32.dll",
    ] {
        expected_files.insert(
            root.join(relative),
            c.changed_artifacts[relative].sha256.clone(),
        );
    }
    let actual_files: BTreeMap<_, _> = c
        .runner
        .files
        .iter()
        .map(|a| (a.path.clone(), a.sha256.clone()))
        .collect();
    require(
        actual_files == expected_files && c.runner.files.len() == expected_files.len(),
        "daw_workspace_runner_roster",
    )?;
    c.runner.verify()?;
    if verify_full_tree {
        experimental_runner::verify_tree(&root, &c.tree)?;
    }
    Ok(c)
}

fn private_exact(path: &Path) -> Result<()> {
    private_dir(path)?;
    verify_private_exact(path)
}

fn verify_private_exact(path: &Path) -> Result<()> {
    let md = fs::symlink_metadata(path)?;
    require(
        md.is_dir()
            && !md.file_type().is_symlink()
            && md.uid() == unsafe { libc::getuid() }
            && md.mode() & 0o077 == 0
            && md.nlink() >= 2,
        "workspace_directory_type",
    )?;
    require(path.canonicalize()? == path, "workspace_directory_alias")
}

fn owned_vendor_dir(path: &Path) -> Result<()> {
    let md = fs::symlink_metadata(path)?;
    require(
        md.is_dir()
            && !md.file_type().is_symlink()
            && md.uid() == unsafe { libc::getuid() }
            && path.canonicalize()? == path,
        "daw_workspace_vendor_directory",
    )
}

pub(super) fn release_syntax(s: &str) -> bool {
    if !(3..=32).contains(&s.len()) {
        return false;
    }
    let parts: Vec<_> = s.split('.').collect();
    (2..=5).contains(&parts.len())
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn audio_syntax(a: &AudioObservation) -> bool {
    (8_000..=192_000).contains(&a.sample_rate)
        && (16..=8_192).contains(&a.buffer_frames)
        && !a.linux_endpoint.is_empty()
        && a.linux_endpoint.len() <= 128
        && a.linux_endpoint
            .bytes()
            .all(|b| b.is_ascii_graphic() || b == b' ')
}

fn legacy_install_bound(m: &Manager, w: &LegacyWorkspace, op: &str) -> bool {
    let Ok(spec) = read_json::<Value>(&result_path(m, op, true).with_file_name("spec.json")) else {
        return false;
    };
    spec["operation"] == op
        && spec["installer"]["sha256"] == w.installer
        && spec["report"] == result_path(m, op, true).to_string_lossy().as_ref()
        && spec["environment"]["id"] == w.id
        && spec["environment"]["root"] == w.environment.root.to_string_lossy().as_ref()
}

fn legacy_uninstall_image(m: &Manager, w: &LegacyWorkspace, op: &str) -> Option<Artifact> {
    let spec: Value = read_json(&result_path(m, op, false).with_file_name("spec.json")).ok()?;
    if spec["kind"] != "daw_workspace_uninstall"
        || spec["operation_id"] != op
        || spec["report"] != result_path(m, op, false).to_string_lossy().as_ref()
        || spec["application"]["id"] != "fl_studio"
        || spec["application"]["environment"]["id"] != w.id
        || spec["application"]["environment"]["root"]
            != w.environment.root.to_string_lossy().as_ref()
        || spec["application"]["projects"] != w.projects.to_string_lossy().as_ref()
        || spec["application"]["preferences"] != w.preferences.to_string_lossy().as_ref()
        || spec["application"]["exports"] != w.exports.to_string_lossy().as_ref()
    {
        return None;
    }
    let image: Artifact =
        serde_json::from_value(spec["application"]["installed_image"].clone()).ok()?;
    image
        .path
        .starts_with(
            w.environment
                .root
                .join("compatdata/pfx/drive_c/Program Files/Image-Line"),
        )
        .then_some(image)
}

fn migrate_legacy(m: &Manager, old: LegacyWorkspace) -> Result<Workspace> {
    require(
        old.schema == 1
            && valid_hex(&old.id, 32)
            && valid_hex(&old.installer, 64)
            && release_syntax(&old.installer_release)
            && old
                .installation_operation
                .as_deref()
                .is_none_or(|op| valid_hex(op, 32))
            && old
                .session_operation
                .as_deref()
                .is_none_or(|op| valid_hex(op, 32))
            && old
                .uninstall_operation
                .as_deref()
                .is_none_or(|op| valid_hex(op, 32)),
        "daw_workspace_legacy_identity",
    )?;
    let selection = InstallerSelection {
        sha256: old.installer.clone(),
        release: old.installer_release.clone(),
    };
    let uninstall_image = old
        .uninstall_operation
        .as_deref()
        .and_then(|op| legacy_uninstall_image(m, &old, op));
    let installed_image = old
        .installed
        .as_ref()
        .map(|app| app.executable.clone())
        .or_else(|| uninstall_image.clone());
    let mut installations = Vec::new();
    let mut active_installation_operation = None;
    if let Some(op) = &old.installation_operation {
        let receipt = result(m, op, true).ok().flatten();
        let bound = legacy_install_bound(m, &old, op);
        let outcome = if !bound {
            InstallationOutcome::Unknown
        } else if receipt.as_ref().is_some_and(retired) {
            if installed_image.is_some() {
                if receipt.as_ref().is_some_and(|v| v["state"] == "completed") {
                    InstallationOutcome::Installed
                } else {
                    InstallationOutcome::NeedsUserAction
                }
            } else if old.state == State::Uninstalled {
                InstallationOutcome::Unknown
            } else {
                InstallationOutcome::Failed
            }
        } else if old.state == State::Installing {
            InstallationOutcome::Pending
        } else {
            InstallationOutcome::Unknown
        };
        if outcome == InstallationOutcome::Pending {
            active_installation_operation = Some(op.clone());
        }
        installations.push(InstallationRecord {
            operation: op.clone(),
            installer: selection.clone(),
            outcome,
            installed_image: installed_image.clone(),
            observed_version: old
                .installed
                .as_ref()
                .and_then(|app| app.observed_file_version.clone()),
            failure: (outcome == InstallationOutcome::NeedsUserAction
                || outcome == InstallationOutcome::Failed)
                .then(|| {
                    old.first_failure.clone().unwrap_or_else(|| {
                        receipt
                            .as_ref()
                            .and_then(|v| v["error"].as_str())
                            .unwrap_or("prior_installation_failed")
                            .to_owned()
                    })
                }),
        });
    }
    let mut uninstalls = Vec::new();
    let mut active_uninstall_operation = None;
    if let Some(op) = &old.uninstall_operation {
        let receipt = result(m, op, false).ok().flatten();
        let outcome = if uninstall_image.is_none() {
            UninstallOutcome::Unknown
        } else if old.state == State::Uninstalled
            && receipt
                .as_ref()
                .is_some_and(|v| retired(v) && v["state"] == "completed")
        {
            UninstallOutcome::Completed
        } else if old.state == State::Uninstalling {
            UninstallOutcome::Pending
        } else {
            UninstallOutcome::Unknown
        };
        if outcome == UninstallOutcome::Pending {
            active_uninstall_operation = Some(op.clone());
        }
        uninstalls.push(UninstallRecord {
            operation: op.clone(),
            installation_operation: old.installation_operation.clone(),
            installed_image: uninstall_image,
            outcome,
        });
    }
    let history_recovery_required = installations
        .iter()
        .any(|record| record.outcome == InstallationOutcome::Unknown)
        || uninstalls
            .iter()
            .any(|record| record.outcome == UninstallOutcome::Unknown)
        || (old.state == State::Uninstalled && (installations.is_empty() || uninstalls.is_empty()));
    Ok(Workspace {
        schema: 2,
        id: old.id,
        revision: old.revision,
        application: old.application,
        selected_installer: selection,
        environment: old.environment,
        runner_manifest: old.runner_manifest,
        projects: old.projects,
        exports: old.exports,
        preferences: old.preferences,
        state: old.state,
        active_installation_operation,
        installations,
        current_installation_operation: old.installed.as_ref().and(old.installation_operation),
        installed: old.installed,
        audio: old.audio,
        session_operation: old.session_operation,
        active_uninstall_operation,
        uninstalls,
        history_recovery_required,
        first_failure: old.first_failure,
    })
}

fn validate_history(w: &Workspace) -> Result<()> {
    require(
        w.installations.len() <= 128 && w.uninstalls.len() <= 128,
        "daw_workspace_history_bound",
    )?;
    let image_line = w
        .environment
        .root
        .join("compatdata/pfx/drive_c/Program Files/Image-Line");
    let mut operations = BTreeSet::new();
    for record in &w.installations {
        require(
            valid_hex(&record.operation, 32)
                && operations.insert(record.operation.as_str())
                && valid_hex(&record.installer.sha256, 64)
                && release_syntax(&record.installer.release)
                && record
                    .observed_version
                    .as_deref()
                    .is_none_or(release_syntax)
                && record.installed_image.as_ref().is_none_or(|image| {
                    valid_hex(&image.sha256, 64) && image.path.starts_with(&image_line)
                })
                && (!matches!(
                    record.outcome,
                    InstallationOutcome::Installed | InstallationOutcome::NeedsUserAction
                ) || record.installed_image.is_some())
                && (record.outcome != InstallationOutcome::Pending
                    || w.active_installation_operation.as_deref()
                        == Some(record.operation.as_str())),
            "daw_workspace_installation_history_identity",
        )?;
    }
    for record in &w.uninstalls {
        require(
            valid_hex(&record.operation, 32)
                && operations.insert(record.operation.as_str())
                && record
                    .installation_operation
                    .as_deref()
                    .is_none_or(|op| w.installations.iter().any(|i| i.operation == op))
                && record.installed_image.as_ref().is_none_or(|image| {
                    valid_hex(&image.sha256, 64) && image.path.starts_with(&image_line)
                })
                && (record.outcome != UninstallOutcome::Completed
                    || (record.installation_operation.is_some()
                        && record.installed_image.is_some()))
                && (record.outcome != UninstallOutcome::Pending
                    || w.active_uninstall_operation.as_deref() == Some(record.operation.as_str())),
            "daw_workspace_uninstall_history_identity",
        )?;
    }
    require(
        w.active_installation_operation.as_deref().is_none_or(|op| {
            w.installations
                .iter()
                .any(|i| i.operation == op && i.outcome == InstallationOutcome::Pending)
        }) && w.active_uninstall_operation.as_deref().is_none_or(|op| {
            w.uninstalls
                .iter()
                .any(|i| i.operation == op && i.outcome == UninstallOutcome::Pending)
        }) && w
            .current_installation_operation
            .as_deref()
            .is_none_or(|op| {
                w.installations.iter().any(|i| {
                    i.operation == op
                        && matches!(
                            i.outcome,
                            InstallationOutcome::Installed | InstallationOutcome::NeedsUserAction
                        )
                        && w.installed
                            .as_ref()
                            .is_some_and(|app| i.installed_image.as_ref() == Some(&app.executable))
                })
            })
            && (w.installed.is_some() == w.current_installation_operation.is_some())
            && (w.state != State::Uninstalled
                || (w.installed.is_none()
                    && w.active_installation_operation.is_none()
                    && w.active_uninstall_operation.is_none()))
            && (w.history_recovery_required
                || (!w
                    .installations
                    .iter()
                    .any(|record| record.outcome == InstallationOutcome::Unknown)
                    && !w
                        .uninstalls
                        .iter()
                        .any(|record| record.outcome == UninstallOutcome::Unknown))),
        "daw_workspace_history_state",
    )
}

fn load(m: &Manager) -> Result<Workspace> {
    transition_settled(m)?;
    let raw: Value = read_json(&record(m))?;
    let w: Workspace = match raw["schema"].as_u64() {
        Some(1) => migrate_legacy(m, serde_json::from_value(raw)?)?,
        Some(2) => serde_json::from_value(raw)?,
        _ => return Err("daw_workspace_schema".into()),
    };
    let root = base(m);
    require(
        w.schema == 2
            && valid_hex(&w.id, 32)
            && w.revision >= 1
            && w.application == ApplicationId::FlStudio
            && valid_hex(&w.selected_installer.sha256, 64)
            && release_syntax(&w.selected_installer.release)
            && w.environment.id == w.id
            && w.environment.root == root.join("environment")
            && ((w.environment.revision == 1
                && w.environment.runner.id == STANDARD_RUNNER
                && w.environment.runner.policy.is_none()
                && w.runner_manifest.is_none())
                || (w.environment.revision == 2
                    && w.environment.runner.id == FL_CRYPT32_RUNNER
                    && w.environment.runner.policy.is_none()
                    && w.runner_manifest.is_some()))
            && w.projects == root.join("projects")
            && w.exports == root.join("exports")
            && w.preferences == root.join("preferences")
            && w.active_installation_operation
                .as_ref()
                .is_none_or(|v| valid_hex(v, 32))
            && w.current_installation_operation
                .as_ref()
                .is_none_or(|v| valid_hex(v, 32))
            && w.session_operation
                .as_ref()
                .is_none_or(|v| valid_hex(v, 32))
            && w.active_uninstall_operation
                .as_ref()
                .is_none_or(|v| valid_hex(v, 32)),
        "daw_workspace_identity",
    )?;
    validate_history(&w)?;
    for dir in [
        &m.root.join("daw-workspaces"),
        &root,
        &w.environment.root,
        &w.projects,
        &w.exports,
        &w.preferences,
    ] {
        verify_private_exact(dir)?;
    }
    require(
        read_json::<Environment>(&w.environment.root.join("environment.json"))? == w.environment,
        "daw_workspace_environment_changed",
    )?;
    if let Some(manifest) = &w.runner_manifest {
        let candidate = fl_runner_candidate(m, manifest, false)?;
        require(
            candidate.runner == w.environment.runner,
            "daw_workspace_selected_runner_changed",
        )?;
    }
    if let Some(app) = &w.installed {
        let image_line = w
            .environment
            .root
            .join("compatdata/pfx/drive_c/Program Files/Image-Line");
        require(
            app.resource_root.starts_with(&image_line)
                && app.executable.path.parent() == Some(app.resource_root.as_path())
                && release_syntax(&app.installer_advertised_release)
                && app
                    .observed_file_version
                    .as_deref()
                    .is_none_or(release_syntax),
            "daw_workspace_application_binding",
        )?;
        match fs::symlink_metadata(&app.resource_root) {
            Ok(_) => owned_vendor_dir(&app.resource_root)?,
            Err(e)
                if e.kind() == std::io::ErrorKind::NotFound && w.state == State::Uninstalling => {}
            Err(e) => return Err(e.into()),
        }
    }
    if let Some(a) = &w.audio {
        require(audio_syntax(a), "daw_workspace_audio_observation")?;
    }
    Ok(w)
}

fn save(m: &Manager, w: &mut Workspace) -> Result<()> {
    w.revision = w
        .revision
        .checked_add(1)
        .ok_or("daw_workspace_revision_overflow")?;
    atomic_json(&record(m), w)
}

fn unit(op: &str, install: bool) -> Result<String> {
    require(valid_hex(op, 32), "daw_workspace_operation_identity")?;
    Ok(format!(
        "linux-vst-bridge-{}-{op}",
        if install { "installer" } else { "daw-fl" }
    ))
}

fn unit_active(name: &str) -> Result<bool> {
    let output = Command::new("systemctl")
        .args(["--user", "show", name, "-p", "ActiveState", "--value"])
        .output()?;
    require(output.status.success(), "daw_workspace_unit_readback")?;
    Ok(std::str::from_utf8(&output.stdout)?.trim() == "active")
}

fn result_path(m: &Manager, op: &str, install: bool) -> PathBuf {
    base(m)
        .join(if install { "installations" } else { "sessions" })
        .join(op)
        .join("result.json")
}

fn result(m: &Manager, op: &str, install: bool) -> Result<Option<Value>> {
    let path = result_path(m, op, install);
    if !path.try_exists()? {
        return Ok(None);
    }
    let v: Value = read_json(&path)?;
    require(
        v["operation"] == op || (!install && v["operation_id"] == op),
        "daw_workspace_result_operation",
    )?;
    Ok(Some(v))
}

fn retired(v: &Value) -> bool {
    v["cleanup_confirmed"] == true
        && v["owned_live"] == 0
        && matches!(
            v["state"].as_str(),
            Some("completed" | "failed" | "cancelled")
        )
}

fn selected_runner(m: &Manager) -> Result<Runner> {
    let mut selected = None;
    for (_, runner) in onboarding::runners(m)? {
        if runner.id == STANDARD_RUNNER && runner.policy.is_none() {
            require(selected.is_none(), "daw_workspace_runner_ambiguous")?;
            selected = Some(runner);
        }
    }
    selected.ok_or("daw_workspace_standard_runner_absent".into())
}

fn select_fl_crypt32_runner(m: &Manager) -> Result<()> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    require(
        w.environment.revision == 1
            && w.environment.runner.id == STANDARD_RUNNER
            && w.runner_manifest.is_none()
            && w.installed.is_some()
            && session_ready(m, &w)?,
        "daw_workspace_runner_prestate",
    )?;
    let install_op = w
        .current_installation_operation
        .as_deref()
        .ok_or("daw_workspace_installer_absent")?;
    require(
        !unit_active(&unit(install_op, true)?)?
            && result(m, install_op, true)?.as_ref().is_some_and(retired),
        "daw_workspace_installer_owner_uncertain",
    )?;
    w.environment.runner.verify()?;
    w.installed
        .as_ref()
        .ok_or("daw_workspace_not_installed")?
        .executable
        .verify()?;
    let operation = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(w.environment.root.join("operation.lock"))?;
    require(
        operation.metadata()?.is_file()
            && operation.metadata()?.uid() == unsafe { libc::getuid() }
            && unsafe { libc::flock(operation.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "daw_workspace_environment_busy",
    )?;
    let manifest_path = fl_runner_manifest(m);
    let manifest = Artifact {
        path: manifest_path.clone(),
        sha256: digest(&manifest_path)?,
    };
    let candidate = fl_runner_candidate(m, &manifest, true)?;
    require(
        candidate.workspace == w.id && candidate.base_runner == w.environment.runner,
        "daw_workspace_runner_base_changed",
    )?;
    let smoke_path = base(m).join("runner-closures/fl-runner-smoke.private.json");
    let smoke: Value = read_json(&smoke_path)?;
    require(
        smoke["schema"] == 1
            && smoke["kind"] == "fl_crypt32_order_unlicensed_smoke"
            && smoke["runner_tree"] == FL_CRYPT32_TREE
            && smoke["initialize_exit"] == 0
            && smoke["cmd_exit"] == 0
            && smoke["crypt32_msg_exit"] == 0
            && smoke["cleanup_confirmed"] == true,
        "daw_workspace_runner_smoke_required",
    )?;
    let marker = w.environment.root.join("environment.json");
    let prior_marker = fs::read(&marker)?;
    let prior_record = fs::read(record(m))?;
    let prior_env = w.environment.clone();
    require(
        read_json::<Environment>(&marker)? == prior_env,
        "daw_workspace_environment_changed",
    )?;
    let receipt = fl_runner_transition(m);
    require(
        !receipt.try_exists()?,
        "daw_workspace_runner_already_selected",
    )?;
    let backup = base(m).join("runner-closures/transition-backup");
    require(!backup.try_exists()?, "daw_workspace_runner_backup_exists")?;
    private_exact(&backup)?;
    for (name, bytes) in [
        ("environment.json", &prior_marker),
        ("workspace.json", &prior_record),
    ] {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(backup.join(name))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    atomic_json(
        &receipt,
        &json!({"schema":1,"state":"prepared","workspace":w.id,
        "before_revision":prior_env.revision,"after_revision":2,
        "before_environment_sha256":digest(&marker)?,"runner_manifest_sha256":manifest.sha256,
        "runner_tree_sha256":FL_CRYPT32_TREE,"prefix_changed":false,
        "native_bridge_changed":false}),
    )?;
    w.environment.revision = 2;
    w.environment.runner = candidate.runner;
    w.runner_manifest = Some(manifest);
    let transition = (|| -> Result<()> {
        atomic_json(&marker, &w.environment)?;
        save(m, &mut w)?;
        require(
            read_json::<Environment>(&marker)? == w.environment
                && read_json::<Workspace>(&record(m))? == w,
            "daw_workspace_runner_transition_readback",
        )?;
        atomic_json(
            &receipt,
            &json!({"schema":1,"state":"committed","workspace":w.id,
            "before_revision":prior_env.revision,"after_revision":2,
            "runner_manifest_sha256":w.runner_manifest.as_ref().ok_or("daw_workspace_runner_manifest")?.sha256,
            "runner_tree_sha256":FL_CRYPT32_TREE,"prefix_changed":false,
            "native_bridge_changed":false}),
        )
    })();
    if let Err(error) = transition {
        if experimental_runner::restore(&marker, &prior_marker).is_err()
            || experimental_runner::restore(&record(m), &prior_record).is_err()
        {
            return Err("daw_workspace_runner_recovery_required".into());
        }
        atomic_json(
            &receipt,
            &json!({"schema":1,"state":"rolled_back","workspace":w.id,
            "reason":error.to_string()}),
        )?;
        return Err(error);
    }
    println!(
        "{}",
        json!({"workspace":w.id,"environment_revision":2,
        "runner":FL_CRYPT32_RUNNER,"runner_tree":FL_CRYPT32_TREE,
        "manifest_sha256":w.runner_manifest.ok_or("daw_workspace_runner_manifest")?.sha256})
    );
    Ok(())
}

fn create(m: &Manager, installer: &str, release: &str) -> Result<()> {
    require(
        valid_hex(installer, 64) && release_syntax(release),
        "daw_workspace_import_identity",
    )?;
    let imported = installer_import::load(m, installer)?;
    require(
        imported.format == "pe_executable",
        "daw_workspace_requires_pe_installer",
    )?;
    let runner = selected_runner(m)?;
    let w = create_exact(m, installer, release, runner)?;
    println!("{}", serde_json::to_string(&w)?);
    Ok(())
}

fn create_exact(m: &Manager, installer: &str, release: &str, runner: Runner) -> Result<Workspace> {
    require(
        valid_hex(installer, 64)
            && release_syntax(release)
            && runner.id == STANDARD_RUNNER
            && runner.policy.is_none(),
        "daw_workspace_creation_authority",
    )?;
    runner.verify()?;
    let _guard = m.lock("daw-workspace.lock")?;
    require(!record(m).try_exists()?, "daw_workspace_already_exists")?;
    let root = base(m);
    for dir in [
        m.root.join("daw-workspaces"),
        root.clone(),
        root.join("environment"),
        root.join("projects"),
        root.join("exports"),
        root.join("preferences"),
        root.join("installations"),
        root.join("sessions"),
    ] {
        private_exact(&dir)?;
    }
    let env_root = root.join("environment");
    for name in [
        "compatdata",
        "runtime-var",
        "host-cache",
        "host-config",
        "host-data",
        "host-tmp",
        "client",
        "home",
    ] {
        private_exact(&env_root.join(name))?;
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(env_root.join("operation.lock"))?
        .sync_all()?;
    let id = random_id()?;
    let environment = Environment {
        id: id.clone(),
        root: env_root,
        runner,
        revision: 1,
    };
    atomic_json(&environment.root.join("environment.json"), &environment)?;
    let w = Workspace {
        schema: 2,
        id,
        revision: 1,
        application: ApplicationId::FlStudio,
        selected_installer: InstallerSelection {
            sha256: installer.into(),
            release: release.into(),
        },
        environment,
        runner_manifest: None,
        projects: root.join("projects"),
        exports: root.join("exports"),
        preferences: root.join("preferences"),
        state: State::Imported,
        active_installation_operation: None,
        installations: Vec::new(),
        current_installation_operation: None,
        installed: None,
        session_operation: None,
        active_uninstall_operation: None,
        uninstalls: Vec::new(),
        history_recovery_required: false,
        audio: None,
        first_failure: None,
    };
    atomic_json(&record(m), &w)?;
    Ok(w)
}

fn supervisor(m: &Manager) -> Result<PathBuf> {
    // A workspace package is separate from the installed native-bridge service.
    // The executable and supervisor must be siblings in one selected generation.
    let tooling_root = m.root.join("daw-workspaces/tooling");
    let t: Tooling = read_json(&tooling_root.join("selection.json"))?;
    require(
        t.schema == 1
            && valid_hex(&t.generation, 64)
            && valid_hex(&t.source_head, 40)
            && valid_hex(&t.source_tree, 40),
        "daw_workspace_tooling_identity",
    )?;
    let dir = tooling_root.join("generations").join(&t.generation);
    verify_private_exact(&dir)?;
    let current = std::env::current_exe()?;
    let exact_canonical_manager = if current == t.manager.path {
        false
    } else {
        let native_manager = software(m)?.manager;
        current == native_manager.path
            && native_manager.sha256 == t.manager.sha256
            && native_manager.verify().is_ok()
    };
    require(
        t.manager.path == dir.join("linux-vst-bridge")
            && t.supervisor.path == dir.join("session.py")
            && t.ownership.path == dir.join("ownership.py")
            && t.importer.path == dir.join("import_installer.py")
            && (current == t.manager.path || exact_canonical_manager),
        "daw_workspace_unmanaged_tooling",
    )?;
    t.manager.verify()?;
    t.supervisor.verify()?;
    t.ownership.verify()?;
    t.importer.verify()?;
    Ok(t.supervisor.path)
}

fn reserve_install_record(w: &mut Workspace, op: &str) -> Result<()> {
    require(
        !w.history_recovery_required
            && matches!(
                w.state,
                State::Imported | State::Uninstalled | State::Failed
            )
            && w.installed.is_none()
            && w.current_installation_operation.is_none()
            && w.active_installation_operation.is_none()
            && w.active_uninstall_operation.is_none()
            && w.installations.len() < 128
            && valid_hex(op, 32)
            && !w.installations.iter().any(|record| record.operation == op)
            && !w.uninstalls.iter().any(|record| record.operation == op),
        "daw_workspace_install_not_admitted",
    )?;
    w.installations.push(InstallationRecord {
        operation: op.into(),
        installer: w.selected_installer.clone(),
        outcome: InstallationOutcome::Pending,
        installed_image: None,
        observed_version: None,
        failure: None,
    });
    w.active_installation_operation = Some(op.into());
    w.state = State::Installing;
    Ok(())
}

fn retire_install_record(
    w: &mut Workspace,
    op: &str,
    result_state: &str,
    installed: Option<InstalledApplication>,
    absent_failure: &str,
) -> Result<()> {
    require(
        w.active_installation_operation.as_deref() == Some(op)
            && matches!(result_state, "completed" | "failed" | "cancelled"),
        "daw_workspace_install_retirement_identity",
    )?;
    let attempt = w
        .installations
        .iter_mut()
        .find(|record| record.operation == op)
        .ok_or("daw_workspace_install_record_absent")?;
    require(
        attempt.outcome == InstallationOutcome::Pending,
        "daw_workspace_install_record_already_retired",
    )?;
    w.active_installation_operation = None;
    if let Some(installed) = installed {
        require(
            installed.installer_advertised_release == attempt.installer.release,
            "daw_workspace_installed_release_binding",
        )?;
        attempt.installed_image = Some(installed.executable.clone());
        attempt.observed_version = installed.observed_file_version.clone();
        attempt.outcome = if result_state == "completed" {
            InstallationOutcome::Installed
        } else {
            InstallationOutcome::NeedsUserAction
        };
        if result_state != "completed" {
            let failure = "installer_exit_nonzero_application_present".to_owned();
            attempt.failure = Some(failure.clone());
            w.first_failure.get_or_insert(failure);
        }
        w.installed = Some(installed);
        w.current_installation_operation = Some(op.into());
        w.state = if result_state == "completed" {
            State::Installed
        } else {
            State::NeedsUserAction
        };
    } else {
        attempt.outcome = InstallationOutcome::Failed;
        attempt.failure = Some(absent_failure.into());
        w.first_failure.get_or_insert_with(|| absent_failure.into());
        w.state = State::Failed;
    }
    Ok(())
}

fn reserve_uninstall_record(w: &mut Workspace, op: &str) -> Result<()> {
    let app = w.installed.as_ref().ok_or("daw_workspace_not_installed")?;
    let install_op = w
        .current_installation_operation
        .as_ref()
        .ok_or("daw_workspace_install_record_absent")?;
    require(
        !w.history_recovery_required
            && w.active_installation_operation.is_none()
            && w.active_uninstall_operation.is_none()
            && w.uninstalls.len() < 128
            && valid_hex(op, 32)
            && !w.installations.iter().any(|record| record.operation == op)
            && !w.uninstalls.iter().any(|record| record.operation == op),
        "daw_workspace_uninstall_not_admitted",
    )?;
    w.uninstalls.push(UninstallRecord {
        operation: op.into(),
        installation_operation: Some(install_op.clone()),
        installed_image: Some(app.executable.clone()),
        outcome: UninstallOutcome::Pending,
    });
    w.active_uninstall_operation = Some(op.into());
    w.session_operation = Some(op.into());
    w.state = State::Uninstalling;
    Ok(())
}

fn retire_uninstall_record(w: &mut Workspace, op: &str) -> Result<()> {
    require(
        w.active_uninstall_operation.as_deref() == Some(op) && w.state == State::Uninstalling,
        "daw_workspace_uninstall_retirement_identity",
    )?;
    let attempt = w
        .uninstalls
        .iter_mut()
        .find(|record| record.operation == op)
        .ok_or("daw_workspace_uninstall_record_absent")?;
    require(
        attempt.outcome == UninstallOutcome::Pending,
        "daw_workspace_uninstall_record_already_retired",
    )?;
    attempt.outcome = UninstallOutcome::Completed;
    w.active_uninstall_operation = None;
    w.current_installation_operation = None;
    w.installed = None;
    w.state = State::Uninstalled;
    Ok(())
}

fn selection_admission(
    w: &Workspace,
    session_ready: bool,
    prior_install_active: bool,
) -> Result<()> {
    require(
        !w.history_recovery_required && w.state != State::CleanupUnconfirmed,
        "daw_workspace_history_recovery_required",
    )?;
    require(
        w.active_installation_operation.is_none() && w.active_uninstall_operation.is_none(),
        "daw_workspace_operation_active",
    )?;
    require(session_ready, "daw_workspace_session_active")?;
    require(
        !prior_install_active,
        "daw_workspace_prior_installer_active",
    )
}

fn install_admission(
    w: &Workspace,
    session_ready: bool,
    prior_install_active: bool,
    image_present: bool,
) -> Result<()> {
    selection_admission(w, session_ready, prior_install_active)?;
    require(
        matches!(
            w.state,
            State::Imported | State::Uninstalled | State::Failed
        ) && w.installed.is_none()
            && w.current_installation_operation.is_none(),
        "daw_workspace_uninstall_required_before_install",
    )?;
    require(
        w.installations.len() < 128,
        "daw_workspace_install_history_full",
    )?;
    require(
        !image_present,
        "daw_workspace_unmanaged_or_incomplete_image_present",
    )
}

fn apply_installer_selection(
    w: &mut Workspace,
    imported: &installer_import::Installer,
    release: &str,
) -> Result<bool> {
    require(
        release_syntax(release)
            && imported.format == "pe_executable"
            && imported.id == imported.artifact.sha256,
        "daw_workspace_installer_selection_identity",
    )?;
    let selected = InstallerSelection {
        sha256: imported.id.clone(),
        release: release.into(),
    };
    if w.selected_installer == selected {
        return Ok(false);
    }
    w.selected_installer = selected;
    Ok(true)
}

fn select_installer(m: &Manager, sha256: &str, release: &str) -> Result<Value> {
    require(
        valid_hex(sha256, 64) && release_syntax(release),
        "daw_workspace_installer_selection_syntax",
    )?;
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    let prior_active = w
        .installations
        .last()
        .map(|prior| unit_active(&unit(&prior.operation, true)?))
        .transpose()?
        .unwrap_or(false);
    selection_admission(&w, session_ready(m, &w)?, prior_active)?;
    let imported = installer_import::load(m, sha256)?;
    if apply_installer_selection(&mut w, &imported, release)? {
        save(m, &mut w)?;
    }
    Ok(json!({"workspace":w.id,"selected_installer":w.selected_installer,"revision":w.revision}))
}

fn install(m: &Manager, requested_operation: Option<&str>) -> Result<Value> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    let prior_active = w
        .installations
        .last()
        .map(|prior| unit_active(&unit(&prior.operation, true)?))
        .transpose()?
        .unwrap_or(false);
    install_admission(
        &w,
        session_ready(m, &w)?,
        prior_active,
        !image_roster(&w)?.is_empty(),
    )?;
    let imported = installer_import::load(m, &w.selected_installer.sha256)?;
    require(
        imported.format == "pe_executable",
        "daw_workspace_requires_pe_installer",
    )?;
    w.environment.runner.verify()?;
    let sw = software(m)?;
    let owner = supervisor(m)?;
    let op = requested_operation
        .map(str::to_owned)
        .map_or_else(random_id, Ok)?;
    require(
        valid_hex(&op, 32)
            && !w.installations.iter().any(|record| record.operation == op)
            && !w.uninstalls.iter().any(|record| record.operation == op),
        "daw_workspace_operation_identity",
    )?;
    let dir = result_path(m, &op, true)
        .parent()
        .ok_or("daw_workspace_install_dir")?
        .to_path_buf();
    private_exact(&dir)?;
    let spec = dir.join("spec.json");
    atomic_json(
        &spec,
        &json!({"schema":2,"operation":op,"environment":w.environment,
        "installer":imported.artifact,"format":imported.format,
        "installer_launch":sw.installer_launch,"report":result_path(m,&op,true)}),
    )?;
    reserve_install_record(&mut w, &op)?;
    save(m, &mut w)?;
    let status = Command::new("systemd-run")
        .args([
            "--user",
            "--collect",
            "--property=UMask=0077",
            "--property=KillMode=control-group",
            "--property=TimeoutStopSec=30",
            "--property=StandardOutput=null",
            "--property=StandardError=null",
        ])
        .arg(format!("--unit={}", unit(&op, true)?))
        .arg("/usr/bin/python3")
        .arg(owner)
        .arg("--install")
        .arg(spec)
        .status();
    if !status.is_ok_and(|s| s.success()) {
        require(
            !unit_active(&unit(&op, true)?)?,
            "daw_workspace_installer_owner_uncertain",
        )?;
        retire_install_record(
            &mut w,
            &op,
            "failed",
            None,
            "installer_worker_launch_failed",
        )?;
        save(m, &mut w)?;
        return Err("daw_workspace_installer_launch_failed".into());
    }
    Ok(json!({"workspace":w.id,"operation":op,"state":"installing"}))
}

fn pe_machine(path: &Path) -> Result<u16> {
    use std::io::{Seek, SeekFrom};
    let mut f = file(path)?;
    let mut header = [0u8; 64];
    f.read_exact(&mut header)?;
    require(&header[..2] == b"MZ", "daw_workspace_installed_pe")?;
    let at = u32::from_le_bytes(header[60..64].try_into()?) as u64;
    require(
        at >= 64 && at + 24 <= f.metadata()?.len(),
        "daw_workspace_installed_pe",
    )?;
    f.seek(SeekFrom::Start(at))?;
    let mut coff = [0u8; 24];
    f.read_exact(&mut coff)?;
    require(&coff[..4] == b"PE\0\0", "daw_workspace_installed_pe")?;
    Ok(u16::from_le_bytes(coff[4..6].try_into()?))
}

fn image_roster(w: &Workspace) -> Result<Vec<PathBuf>> {
    let root = w
        .environment
        .root
        .join("compatdata/pfx/drive_c/Program Files/Image-Line");
    if !root.try_exists()? {
        return Ok(Vec::new());
    }
    owned_vendor_dir(&root)?;
    let mut images = Vec::new();
    for (i, entry) in fs::read_dir(&root)?.enumerate() {
        require(i < 64, "daw_workspace_installation_roster_bound")?;
        let path = entry?.path();
        if !fs::symlink_metadata(&path)?.is_dir() {
            continue;
        }
        owned_vendor_dir(&path)?;
        for (j, file) in fs::read_dir(&path)?.enumerate() {
            require(j < 1024, "daw_workspace_application_roster_bound")?;
            let file = file?.path();
            let name = file.file_name().and_then(|v| v.to_str()).unwrap_or("");
            if !matches!(name, "FL64.exe" | "FL Studio.exe") {
                continue;
            }
            if pe_machine(&file)? == 0x8664 {
                images.push(file);
            }
        }
    }
    Ok(images)
}

fn discover(w: &Workspace, release: &str) -> Result<InstalledApplication> {
    let mut images = image_roster(w)?;
    require(
        images.len() == 1,
        "daw_workspace_installed_image_ambiguous_or_absent",
    )?;
    let executable = Artifact {
        path: images.remove(0),
        sha256: String::new(),
    };
    let executable = Artifact {
        sha256: digest(&executable.path)?,
        ..executable
    };
    Ok(InstalledApplication {
        resource_root: executable
            .path
            .parent()
            .ok_or("daw_workspace_resource_root")?
            .into(),
        executable,
        installer_advertised_release: release.into(),
        observed_file_version: None,
    })
}

fn finalize_install(m: &Manager, w: &mut Workspace) -> Result<()> {
    if w.installed.is_some() {
        return Ok(());
    }
    let op = w
        .active_installation_operation
        .as_deref()
        .ok_or("daw_workspace_not_installed")?
        .to_owned();
    require(
        !unit_active(&unit(&op, true)?)?,
        "daw_workspace_installer_still_running",
    )?;
    let v = result(m, &op, true)?.ok_or("daw_workspace_install_result_absent")?;
    require(retired(&v), "daw_workspace_installer_cleanup_unconfirmed")?;
    let index = w
        .installations
        .iter()
        .position(|attempt| attempt.operation == op)
        .ok_or("daw_workspace_install_record_absent")?;
    let release = w.installations[index].installer.release.clone();
    let images = image_roster(w)?;
    let installed = if images.len() == 1 {
        Some(discover(w, &release)?)
    } else {
        None
    };
    if images.len() > 1 {
        w.installations[index].outcome = InstallationOutcome::Unknown;
        w.installations[index].failure = Some("installed_image_ambiguous".into());
        w.active_installation_operation = None;
        w.history_recovery_required = true;
        w.state = State::Failed;
        save(m, w)?;
        return Err("daw_workspace_installed_image_ambiguous".into());
    }
    if let Some(app) = &installed {
        app.executable.verify()?;
    }
    retire_install_record(
        w,
        &op,
        v["state"]
            .as_str()
            .ok_or("daw_workspace_install_result_state")?,
        installed,
        v["error"]
            .as_str()
            .unwrap_or("installed_image_absent_after_retired_installer"),
    )?;
    save(m, w)
}

fn finish_install(m: &Manager) -> Result<Value> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    finalize_install(m, &mut w)?;
    Ok(json!({"workspace":w.id,"state":w.state,
        "installation_operation":w.current_installation_operation,
        "installed":w.installed}))
}

fn session_ready(m: &Manager, w: &Workspace) -> Result<bool> {
    let Some(op) = &w.session_operation else {
        return Ok(true);
    };
    if unit_active(&unit(op, false)?)? {
        return Ok(false);
    }
    let v = result(m, op, false)?.ok_or("daw_workspace_session_result_absent")?;
    require(retired(&v), "daw_workspace_cleanup_unconfirmed")?;
    Ok(true)
}

fn check_prerequisites(runtime: &Path, graphical: &str, authority: &Path) -> Result<()> {
    require(
        graphical.starts_with(':') && graphical.len() <= 16,
        "daw_workspace_graphical_session_unavailable",
    )?;
    let md = fs::symlink_metadata(authority).map_err(|_| "daw_workspace_xauthority_unavailable")?;
    require(
        md.is_file()
            && !md.file_type().is_symlink()
            && md.uid() == unsafe { libc::getuid() }
            && md.mode() & 0o077 == 0,
        "daw_workspace_xauthority_unavailable",
    )?;
    let pulse = fs::symlink_metadata(runtime.join("pulse/native"))
        .map_err(|_| "daw_workspace_audio_endpoint_unavailable")?;
    require(
        pulse.file_type().is_socket() && pulse.uid() == unsafe { libc::getuid() },
        "daw_workspace_audio_endpoint_unavailable",
    )
}

fn desktop_prerequisites() -> Result<()> {
    let output = Command::new("systemctl")
        .args(["--user", "show-environment"])
        .output()?;
    require(
        output.status.success(),
        "daw_workspace_session_environment_unavailable",
    )?;
    let value = std::str::from_utf8(&output.stdout)?;
    let field = |name: &str| -> Result<String> {
        value
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .map(str::to_owned)
            .ok_or_else(|| "daw_workspace_session_environment_unavailable".into())
    };
    let runtime = PathBuf::from(field("XDG_RUNTIME_DIR")?);
    require(
        runtime == Path::new(&format!("/run/user/{}", unsafe { libc::getuid() })),
        "daw_workspace_runtime_identity",
    )?;
    check_prerequisites(
        &runtime,
        &field("DISPLAY")?,
        Path::new(&field("XAUTHORITY")?),
    )
}

fn launch(m: &Manager) -> Result<()> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    finalize_install(m, &mut w)?;
    require(
        !w.history_recovery_required
            && w.state != State::CleanupUnconfirmed
            && w.active_installation_operation.is_none()
            && w.active_uninstall_operation.is_none(),
        "daw_workspace_cleanup_unconfirmed",
    )?;
    require(
        session_ready(m, &w)?,
        "daw_workspace_already_running_use_focus",
    )?;
    desktop_prerequisites()?;
    w.environment.runner.verify()?;
    let app = w.installed.as_ref().ok_or("daw_workspace_not_installed")?;
    app.executable.verify()?;
    let owner = supervisor(m)?;
    let op = random_id()?;
    let dir = result_path(m, &op, false)
        .parent()
        .ok_or("daw_workspace_session_dir")?
        .to_path_buf();
    private_exact(&dir)?;
    let spec = dir.join("spec.json");
    atomic_json(
        &spec,
        &json!({"kind":"daw_workspace_application","application":{
        "id":"fl_studio","environment":w.environment,"executable":app.executable,"helpers":[],
        "preferences":w.preferences,"projects":w.projects,"exports":w.exports},
        "report":result_path(m,&op,false),"mode":"normal","operation_id":op}),
    )?;
    w.session_operation = Some(op.clone());
    w.state = State::Starting;
    save(m, &mut w)?;
    let status = Command::new("systemd-run")
        .args([
            "--user",
            "--collect",
            "--property=UMask=0077",
            "--property=KillMode=control-group",
            "--property=TimeoutStopSec=30",
            "--property=StandardOutput=null",
            "--property=StandardError=null",
        ])
        .arg(format!("--unit={}", unit(&op, false)?))
        .arg("/usr/bin/python3")
        .arg(owner)
        .arg("--vendor-application")
        .arg(spec)
        .status();
    if !status.is_ok_and(|s| s.success()) {
        require(
            !unit_active(&unit(&op, false)?)?,
            "daw_workspace_application_owner_uncertain",
        )?;
        w.state = State::Failed;
        w.first_failure
            .get_or_insert_with(|| "application_worker_launch_failed".into());
        save(m, &mut w)?;
        return Err("daw_workspace_application_launch_failed".into());
    }
    println!(
        "{}",
        json!({"workspace":w.id,"operation":op,"state":"starting"})
    );
    Ok(())
}

fn uninstall(m: &Manager, requested_operation: Option<&str>) -> Result<()> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    finalize_install(m, &mut w)?;
    require(
        !w.history_recovery_required
            && w.active_installation_operation.is_none()
            && w.active_uninstall_operation.is_none(),
        "daw_workspace_uninstall_already_started",
    )?;
    require(
        session_ready(m, &w)?,
        "daw_workspace_application_still_running",
    )?;
    let app = w.installed.as_ref().ok_or("daw_workspace_not_installed")?;
    app.executable.verify()?;
    w.environment.runner.verify()?;
    desktop_prerequisites()?;
    let uninstaller = app.resource_root.join("uninstall.exe");
    require(
        pe_machine(&uninstaller)? == 0x14c,
        "daw_workspace_uninstaller_machine",
    )?;
    let md = fs::symlink_metadata(&uninstaller)?;
    require(
        md.is_file()
            && !md.file_type().is_symlink()
            && md.uid() == unsafe { libc::getuid() }
            && md.nlink() == 1
            && md.mode() & 0o077 == 0,
        "daw_workspace_uninstaller_custody",
    )?;
    let artifact = Artifact {
        path: uninstaller,
        sha256: digest(&app.resource_root.join("uninstall.exe"))?,
    };
    artifact.verify()?;
    let owner = supervisor(m)?;
    let op = requested_operation
        .map(str::to_owned)
        .map_or_else(random_id, Ok)?;
    require(valid_hex(&op, 32), "daw_workspace_operation_identity")?;
    let dir = result_path(m, &op, false)
        .parent()
        .ok_or("daw_workspace_uninstall_dir")?
        .to_path_buf();
    private_exact(&dir)?;
    let spec = dir.join("spec.json");
    atomic_json(
        &spec,
        &json!({"kind":"daw_workspace_uninstall","application":{
        "id":"fl_studio","environment":w.environment,"executable":artifact,
        "installed_image":app.executable,"helpers":[],"preferences":w.preferences,
        "projects":w.projects,"exports":w.exports},
        "report":result_path(m,&op,false),"mode":"normal","operation_id":op}),
    )?;
    reserve_uninstall_record(&mut w, &op)?;
    save(m, &mut w)?;
    let status = Command::new("systemd-run")
        .args([
            "--user",
            "--collect",
            "--property=UMask=0077",
            "--property=KillMode=control-group",
            "--property=TimeoutStopSec=30",
            "--property=StandardOutput=null",
            "--property=StandardError=null",
        ])
        .arg(format!("--unit={}", unit(&op, false)?))
        .arg("/usr/bin/python3")
        .arg(owner)
        .arg("--vendor-application")
        .arg(spec)
        .status();
    require(
        status.is_ok_and(|s| s.success()) || unit_active(&unit(&op, false)?)?,
        "daw_workspace_uninstaller_launch_failed",
    )?;
    println!(
        "{}",
        json!({"workspace":w.id,"operation":op,"state":"uninstalling"})
    );
    Ok(())
}

fn finish_uninstall(m: &Manager) -> Result<()> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    require(
        w.state == State::Uninstalling,
        "daw_workspace_uninstall_not_pending",
    )?;
    let op = w
        .active_uninstall_operation
        .as_deref()
        .ok_or("daw_workspace_uninstall_operation_absent")?
        .to_owned();
    require(
        !unit_active(&unit(&op, false)?)?,
        "daw_workspace_uninstaller_still_running",
    )?;
    let v = result(m, &op, false)?.ok_or("daw_workspace_uninstall_result_absent")?;
    require(
        retired(&v) && v["state"] == "completed",
        "daw_workspace_uninstall_not_clean",
    )?;
    let app = w.installed.as_ref().ok_or("daw_workspace_not_installed")?;
    require(
        fs::symlink_metadata(&app.executable.path)
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
        "daw_workspace_application_still_installed",
    )?;
    retire_uninstall_record(&mut w, &op)?;
    save(m, &mut w)?;
    println!(
        "{}",
        json!({"workspace":w.id,"operation":op,"state":"uninstalled","cleanup_confirmed":true})
    );
    Ok(())
}

fn request(m: &Manager, action: &str) -> Result<()> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    let op = w
        .session_operation
        .as_deref()
        .ok_or("daw_workspace_no_session")?
        .to_owned();
    require(
        unit_active(&unit(&op, false)?)?,
        "daw_workspace_no_live_session",
    )?;
    let id = random_id()?;
    let file = result_path(m, &op, false)
        .parent()
        .ok_or("daw_workspace_session_dir")?
        .join(if action == "focus" {
            "focus.json"
        } else {
            "close.json"
        });
    require(!file.try_exists()?, "daw_workspace_control_pending")?;
    atomic_json(&file, &json!({"operation_id":op,"request":id}))?;
    if action == "stop" {
        w.state = State::Stopping;
        save(m, &mut w)?;
    }
    let deadline = Instant::now()
        + Duration::from_secs(if action == "focus" {
            4
        } else {
            SESSION_SECONDS
        });
    loop {
        if let Some(v) = result(m, &op, false)? {
            if action == "focus" && v["focus_result"]["request"] == id {
                require(
                    v["focus_result"]["result"] == "focused",
                    "daw_workspace_focus_refused",
                )?;
                println!("{}", v["focus_result"]);
                return Ok(());
            }
            if action == "stop" && retired(&v) && !unit_active(&unit(&op, false)?)? {
                println!(
                    "{}",
                    json!({"operation":op,"state":"retired","cleanup_confirmed":true})
                );
                return Ok(());
            }
        }
        require(
            Instant::now() < deadline,
            if action == "focus" {
                "daw_workspace_focus_timeout"
            } else {
                "daw_workspace_graceful_stop_pending"
            },
        )?;
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn current_failure(
    w: &Workspace,
    state: State,
    cleanup: &str,
    unmanaged_image_present: bool,
    session_result: Option<&Value>,
) -> Option<String> {
    if unmanaged_image_present {
        Some("FL files are present without a current manager-owned installation".to_owned())
    } else if cleanup == "cleanup_unconfirmed" {
        Some("Workspace ownership or history needs recovery".to_owned())
    } else if matches!(state, State::Failed | State::NeedsUserAction) {
        w.installations
            .last()
            .and_then(|record| record.failure.clone())
            .or_else(|| session_result.and_then(|v| v["error"].as_str().map(str::to_owned)))
    } else {
        None
    }
}

fn read_status(m: &Manager) -> Result<Value> {
    let w = load(m)?;
    let installation_history: Vec<Value> = w
        .installations
        .iter()
        .map(|record| Ok(json!({"record":record,"result":result(m,&record.operation,true)?})))
        .collect::<Result<_>>()?;
    let uninstall_history: Vec<Value> = w
        .uninstalls
        .iter()
        .map(|record| Ok(json!({"record":record,"result":result(m,&record.operation,false)?})))
        .collect::<Result<_>>()?;
    let install_result = w
        .installations
        .last()
        .map(|record| result(m, &record.operation, true))
        .transpose()?
        .flatten();
    let session_result = match &w.session_operation {
        Some(op) => result(m, op, false)?,
        None => None,
    };
    let owner_active = match &w.session_operation {
        Some(op) => unit_active(&unit(op, false)?)?,
        None => false,
    };
    let install_active = match &w.active_installation_operation {
        Some(op) => unit_active(&unit(op, true)?)?,
        None => false,
    };
    let uninstall_active = match &w.active_uninstall_operation {
        Some(op) => unit_active(&unit(op, false)?)?,
        None => false,
    };
    let detected_installed = if w.active_installation_operation.is_some()
        && w.installed.is_none()
        && !install_active
        && install_result.as_ref().is_some_and(retired)
    {
        w.installations
            .last()
            .and_then(|record| discover(&w, &record.installer.release).ok())
    } else {
        None
    };
    let unmanaged_image_present = w.installed.is_none()
        && w.active_installation_operation.is_none()
        && !image_roster(&w)?.is_empty();
    let active_install_result = w
        .active_installation_operation
        .as_ref()
        .map(|op| result(m, op, true))
        .transpose()?
        .flatten();
    let active_uninstall_result = w
        .active_uninstall_operation
        .as_ref()
        .map(|op| result(m, op, false))
        .transpose()?
        .flatten();
    let cleanup = if owner_active || install_active || uninstall_active {
        "running"
    } else if w.history_recovery_required
        || w.state == State::CleanupUnconfirmed
        || (w.session_operation.is_some() && !session_result.as_ref().is_some_and(retired))
        || (w.active_installation_operation.is_some()
            && !active_install_result.as_ref().is_some_and(retired))
        || (w.active_uninstall_operation.is_some()
            && !active_uninstall_result.as_ref().is_some_and(retired))
    {
        "cleanup_unconfirmed"
    } else {
        "confirmed"
    };
    let effective_state = if cleanup == "cleanup_unconfirmed" {
        State::CleanupUnconfirmed
    } else if install_active || w.active_installation_operation.is_some() {
        State::Installing
    } else if uninstall_active || w.active_uninstall_operation.is_some() {
        State::Uninstalling
    } else if owner_active && w.state == State::Stopping {
        State::Stopping
    } else if owner_active
        && session_result
            .as_ref()
            .is_some_and(|v| v["state"] == "running" || v["state"] == "unknown")
    {
        State::Running
    } else if owner_active {
        State::Starting
    } else if unmanaged_image_present
        || (w.state == State::Failed
            && session_result
                .as_ref()
                .is_some_and(|v| v["state"] == "failed"))
    {
        State::Failed
    } else if w.state == State::Uninstalled {
        State::Uninstalled
    } else if w.installed.is_some() && w.state == State::NeedsUserAction {
        State::NeedsUserAction
    } else if w.installed.is_some() {
        if desktop_prerequisites().is_ok() {
            State::Ready
        } else {
            State::Installed
        }
    } else {
        w.state
    };
    let current_failure = current_failure(
        &w,
        effective_state,
        cleanup,
        unmanaged_image_present,
        session_result.as_ref(),
    );
    Ok(
        json!({"schema":2,"workspace":w,"selected_installer":w.selected_installer,
        "current_installed_application":w.installed,
        "active_installation_operation":w.active_installation_operation,
        "active_uninstall_operation":w.active_uninstall_operation,
        "installer_result":install_result,"installation_history":installation_history,
        "uninstall_history":uninstall_history,"installer_active":install_active,
        "uninstall_active":uninstall_active,"session_result":session_result,
        "owner_active":owner_active,"cleanup":cleanup,"effective_state":effective_state,
        "detected_installed_not_committed":detected_installed,
        "unmanaged_image_present":unmanaged_image_present,
        "first_useful_failure":current_failure,
        "historical_first_failure":w.first_failure}),
    )
}

fn status(m: &Manager) -> Result<()> {
    println!("{}", read_status(m)?);
    Ok(())
}

fn record_audio(m: &Manager, args: &[String]) -> Result<()> {
    let [backend, rate, block, endpoint, audible] = args else {
        return Err(
            "Usage: workspace record-audio BACKEND RATE BUFFER ENDPOINT audible|not-audible".into(),
        );
    };
    let backend = match backend.as_str() {
        "pulseaudio" => AudioBackend::PulseAudio,
        "wineasio" => AudioBackend::WineAsio,
        _ => return Err("daw_workspace_audio_backend_unsupported".into()),
    };
    let observation = AudioObservation {
        backend,
        sample_rate: rate.parse()?,
        buffer_frames: block.parse()?,
        linux_endpoint: endpoint.clone(),
        operator_audible: match audible.as_str() {
            "audible" => true,
            "not-audible" => false,
            _ => return Err("daw_workspace_audio_witness_invalid".into()),
        },
    };
    require(
        audio_syntax(&observation),
        "daw_workspace_audio_observation",
    )?;
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    require(
        w.installed.is_some(),
        "daw_workspace_audio_before_installation",
    )?;
    w.audio = Some(observation);
    save(m, &mut w)?;
    println!("{}", serde_json::to_string(&w.audio)?);
    Ok(())
}

fn record_installed_version(m: &Manager, version: &str, image_sha: &str) -> Result<()> {
    require(
        release_syntax(version) && valid_hex(image_sha, 64),
        "daw_workspace_installed_version_syntax",
    )?;
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    let installed = w.installed.as_mut().ok_or("daw_workspace_not_installed")?;
    require(
        installed.executable.sha256 == image_sha,
        "daw_workspace_installed_version_image_changed",
    )?;
    installed.executable.verify()?;
    require(
        installed
            .observed_file_version
            .as_deref()
            .is_none_or(|v| v == version),
        "daw_workspace_installed_version_conflict",
    )?;
    installed.observed_file_version = Some(version.to_owned());
    let operation = w
        .current_installation_operation
        .as_deref()
        .ok_or("daw_workspace_install_record_absent")?;
    let history = w
        .installations
        .iter_mut()
        .find(|record| record.operation == operation)
        .ok_or("daw_workspace_install_record_absent")?;
    history.observed_version = Some(version.to_owned());
    save(m, &mut w)?;
    println!(
        "{}",
        json!({"file_version":version,"image_sha256":image_sha})
    );
    Ok(())
}

fn offer(label: &str, action: ui::Action, reason: Option<String>) -> ui::AvailableAction {
    ui::AvailableAction {
        label: label.into(),
        action,
        disabled_reason: reason,
    }
}

pub(super) fn projection(
    m: &Manager,
    imported: &[ui::Onboarding],
) -> Result<Vec<ui::DawWorkspace>> {
    if !record(m).try_exists()? {
        return Ok(Vec::new());
    }
    let w = load(m)?;
    let details = read_status(m)?;
    let cleanup = details["cleanup"].as_str().unwrap_or("cleanup_unconfirmed");
    let owner_active = details["owner_active"] == true;
    let installer_active = details["installer_active"] == true;
    let unmanaged_image = details["unmanaged_image_present"] == true;
    let active_uninstall = details["uninstall_active"] == true;
    let prior_active = w
        .installations
        .last()
        .map(|prior| unit_active(&unit(&prior.operation, true)?))
        .transpose()?
        .unwrap_or(false);
    let selection_reason = if cleanup == "confirmed" {
        selection_admission(&w, !owner_active, prior_active)
            .err()
            .map(|e| e.to_string())
    } else {
        Some("Workspace cleanup or history is not confirmed".into())
    };
    let mut choices = Vec::new();
    let mut seen = BTreeSet::new();
    for row in imported {
        if row.format != "pe_executable" || !seen.insert(row.installer.as_str()) {
            continue;
        }
        choices.push(offer(
            &format!(
                "Choose imported installer {}",
                row.installer.get(..12).unwrap_or(&row.installer)
            ),
            ui::Action::WorkspaceSelectInstaller {
                installer: row.installer.clone(),
                release: String::new(),
            },
            selection_reason.clone(),
        ));
    }
    let selected_in_custody = seen.contains(w.selected_installer.sha256.as_str());
    let install_reason = if cleanup != "confirmed" {
        Some("Workspace cleanup or history is not confirmed".into())
    } else if !selected_in_custody {
        Some("Selected installer is absent from canonical custody".into())
    } else {
        install_admission(&w, !owner_active, prior_active, unmanaged_image)
            .err()
            .map(|e| e.to_string())
    };
    let mut actions = Vec::new();
    if w.installed.is_none() && w.active_installation_operation.is_none() {
        actions.push(offer(
            "Install selected version",
            ui::Action::WorkspaceInstall {},
            install_reason,
        ));
    }
    if w.active_installation_operation.is_some() {
        let can_finish = !installer_active
            && details["installer_result"]
                .as_object()
                .is_some_and(|_| retired(&details["installer_result"]));
        actions.push(offer(
            "Complete installation readback",
            ui::Action::WorkspaceFinishInstall {},
            (!can_finish).then(|| "Installer is still active or cleanup is unconfirmed".into()),
        ));
    }
    if w.installed.is_some() && w.active_uninstall_operation.is_none() {
        let ready = cleanup == "confirmed"
            && !owner_active
            && !installer_active
            && desktop_prerequisites().is_ok();
        let launch_reason = (!ready)
            .then(|| "Close the FL session and confirm cleanup and desktop readiness first".into());
        actions.push(offer(
            "Launch FL Studio",
            ui::Action::WorkspaceLaunch {},
            launch_reason.clone(),
        ));
        actions.push(offer(
            "Uninstall FL Studio",
            ui::Action::WorkspaceUninstall {},
            if w.uninstalls.len() >= 128 {
                Some("Workspace uninstall history is full; recovery is required".into())
            } else {
                launch_reason
            },
        ));
    }
    if w.active_uninstall_operation.is_some() {
        let can_finish = !active_uninstall
            && details["uninstall_history"]
                .as_array()
                .and_then(|history| history.last())
                .is_some_and(|entry| {
                    retired(&entry["result"]) && entry["result"]["state"] == "completed"
                })
            && w.installed.as_ref().is_some_and(|app| {
                fs::symlink_metadata(&app.executable.path)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
            });
        actions.push(offer(
            "Complete uninstall readback",
            ui::Action::WorkspaceFinishUninstall {},
            (!can_finish).then(|| "Uninstaller is still active or removal is not confirmed".into()),
        ));
    }
    if owner_active {
        actions.push(offer(
            "Focus FL Studio",
            ui::Action::WorkspaceFocus {},
            None,
        ));
        actions.push(offer(
            "Close FL Studio gracefully",
            ui::Action::WorkspaceStop {},
            None,
        ));
    }
    let installed_advertised_release = w
        .installed
        .as_ref()
        .map(|app| app.installer_advertised_release.clone());
    let observed_file_version = w
        .installed
        .as_ref()
        .and_then(|app| app.observed_file_version.clone());
    Ok(vec![ui::DawWorkspace {
        id: w.id,
        name: "FL Studio".into(),
        state: details["effective_state"]
            .as_str()
            .unwrap_or("cleanup_unconfirmed")
            .into(),
        selected_installer: w.selected_installer.sha256,
        selected_release: w.selected_installer.release,
        installed_advertised_release,
        observed_file_version,
        installed_image_sha256: w
            .installed
            .as_ref()
            .map(|app| app.executable.sha256.clone()),
        active_installation_operation: w.active_installation_operation,
        cleanup: cleanup.into(),
        first_useful_failure: details["first_useful_failure"].as_str().map(str::to_owned),
        actions,
        installer_choices: choices,
        details,
    }])
}

pub(super) fn execute_action(m: &Manager, action: &ui::Action, operation: &str) -> Result<Value> {
    match action {
        ui::Action::WorkspaceSelectInstaller { installer, release } => {
            select_installer(m, installer, release)
        }
        ui::Action::WorkspaceInstall {} => install(m, Some(operation)),
        ui::Action::WorkspaceFinishInstall {} => finish_install(m),
        ui::Action::WorkspaceLaunch {} => {
            launch(m)?;
            read_status(m)
        }
        ui::Action::WorkspaceUninstall {} => {
            uninstall(m, Some(operation))?;
            read_status(m)
        }
        ui::Action::WorkspaceFinishUninstall {} => {
            finish_uninstall(m)?;
            read_status(m)
        }
        ui::Action::WorkspaceFocus {} => {
            request(m, "focus")?;
            read_status(m)
        }
        ui::Action::WorkspaceStop {} => {
            request(m, "stop")?;
            read_status(m)
        }
        _ => Err("daw_workspace_action_not_supported".into()),
    }
}

pub fn run(m: &Manager, args: &[String]) -> Result<()> {
    match args {
        [action] if action == "import" => {
            let source = fs::File::from(std::io::stdin().as_fd().try_clone_to_owned()?);
            println!("{}", serde_json::to_string(&installer_import::import(m, source)?)?);
            Ok(())
        }
        [action, id, release] if action == "create" => create(m, id, release),
        [action, sha256, release] if action == "select-installer" => {
            println!("{}", select_installer(m, sha256, release)?);
            Ok(())
        }
        [action] if action == "install" => {
            println!("{}", install(m, None)?);
            Ok(())
        }
        [action] if action == "finish-install" => {
            println!("{}", finish_install(m)?);
            Ok(())
        }
        [action] if action == "uninstall" => uninstall(m, None),
        [action] if action == "finish-uninstall" => finish_uninstall(m),
        [action] if action == "select-fl-crypt32-runner" => select_fl_crypt32_runner(m),
        [action] if action == "launch" => launch(m),
        [action] if action == "focus" => request(m, "focus"),
        [action] if action == "stop" => request(m, "stop"),
        [action] if action == "status" => status(m),
        [action, rest @ ..] if action == "record-audio" => record_audio(m, rest),
        [action, version, image_sha] if action == "record-installed-version" => record_installed_version(m, version, image_sha),
        _ => Err("Usage: workspace import | create INSTALLER_SHA RELEASE | select-installer INSTALLER_SHA RELEASE | install | finish-install | uninstall | finish-uninstall | select-fl-crypt32-runner | launch | focus | status | stop | record-audio BACKEND RATE BUFFER ENDPOINT audible|not-audible | record-installed-version VERSION IMAGE_SHA".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;

    fn fixture_workspace(f: &crate::test_fixture::Fixture, installer: &str) -> Workspace {
        let mut runner = f.r.environment.runner.clone();
        runner.id = STANDARD_RUNNER.into();
        create_exact(&f.m, installer, "26.1.6.0", runner).unwrap()
    }

    fn fixture_app(w: &Workspace, release: &str, seed: u8) -> InstalledApplication {
        let resource_root = w
            .environment
            .root
            .join("compatdata/pfx/drive_c/Program Files/Image-Line/FL Studio fixture");
        fs::create_dir_all(&resource_root).unwrap();
        let executable = resource_root.join("FL64.exe");
        let mut bytes = vec![seed; 1024];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        fs::write(&executable, bytes).unwrap();
        InstalledApplication {
            executable: Artifact {
                path: executable.clone(),
                sha256: digest(&executable).unwrap(),
            },
            installer_advertised_release: release.into(),
            observed_file_version: None,
            resource_root,
        }
    }

    fn imported_fixture(f: &crate::test_fixture::Fixture, seed: u8) -> installer_import::Installer {
        let path = f.outer.join(format!("fl-installer-{seed}.exe"));
        let mut bytes = vec![seed; 1024];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[148..150].copy_from_slice(&224u16.to_le_bytes());
        bytes[150..152].copy_from_slice(&2u16.to_le_bytes());
        bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        fs::write(&path, bytes).unwrap();
        installer_import::import(&f.m, fs::File::open(path).unwrap()).unwrap()
    }
    #[test]
    fn closed_workspace_and_release_grammar() {
        assert_eq!(APP, "fl-studio");
        assert!(release_syntax("26.1.6.0"));
        for value in ["", "latest", "26.1;run", "26/1", "26..1", ".26.1", "26.1."] {
            assert!(!release_syntax(value));
        }
        assert!(unit("a".repeat(32).as_str(), false)
            .unwrap()
            .starts_with("linux-vst-bridge-daw-fl-"));
        assert!(unit("other", false).is_err());
    }
    #[test]
    fn installer_and_application_are_distinct() {
        assert!(!retired(
            &json!({"state":"running","cleanup_confirmed":false,"owned_live":1})
        ));
        assert!(!retired(
            &json!({"state":"completed","cleanup_confirmed":false,"owned_live":0})
        ));
        assert!(retired(
            &json!({"state":"completed","cleanup_confirmed":true,"owned_live":0})
        ));
    }
    #[test]
    fn workspace_roots_are_private_and_native_registry_is_unchanged() {
        let f = crate::test_fixture::Fixture::new();
        let before = fs::read(f.m.root.join("registry.json")).ok();
        let mut runner = f.r.environment.runner.clone();
        runner.id = STANDARD_RUNNER.into();
        let w = create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner.clone()).unwrap();
        assert_eq!(w.state, State::Imported);
        assert_eq!(w.environment.runner, runner);
        assert_eq!(load(&f.m).unwrap(), w);
        for path in [&w.projects, &w.exports, &w.preferences, &w.environment.root] {
            let md = fs::symlink_metadata(path).unwrap();
            assert!(md.is_dir());
            assert_eq!(md.mode() & 0o077, 0);
        }
        assert_eq!(fs::read(f.m.root.join("registry.json")).ok(), before);
        assert!(create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner).is_err());
    }
    #[test]
    fn fl_runner_transition_requires_installed_retired_exact_prestate() {
        let f = crate::test_fixture::Fixture::new();
        let mut runner = f.r.environment.runner.clone();
        runner.id = STANDARD_RUNNER.into();
        let w = create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner).unwrap();
        let marker = fs::read(w.environment.root.join("environment.json")).unwrap();
        let record_bytes = fs::read(record(&f.m)).unwrap();
        assert!(select_fl_crypt32_runner(&f.m).is_err());
        assert_eq!(
            fs::read(w.environment.root.join("environment.json")).unwrap(),
            marker
        );
        assert_eq!(fs::read(record(&f.m)).unwrap(), record_bytes);
        assert!(!fl_runner_transition(&f.m).exists());
    }
    #[test]
    fn runner_change_and_interrupted_transition_refuse_at_readback() {
        let f = crate::test_fixture::Fixture::new();
        let mut runner = f.r.environment.runner.clone();
        runner.id = STANDARD_RUNNER.into();
        let mut w = create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner).unwrap();
        w.environment.revision = 2;
        w.environment.runner.id = FL_CRYPT32_RUNNER.into();
        atomic_json(&w.environment.root.join("environment.json"), &w.environment).unwrap();
        atomic_json(&record(&f.m), &w).unwrap();
        assert!(load(&f.m).is_err());
        private_exact(&base(&f.m).join("runner-closures")).unwrap();
        atomic_json(
            &fl_runner_transition(&f.m),
            &json!({"schema":1,"state":"prepared"}),
        )
        .unwrap();
        assert!(load(&f.m)
            .unwrap_err()
            .to_string()
            .contains("recovery_required"));
    }
    #[test]
    fn foreign_workspace_root_refuses_without_registry_mutation() {
        let f = crate::test_fixture::Fixture::new();
        let mut runner = f.r.environment.runner.clone();
        runner.id = STANDARD_RUNNER.into();
        private_exact(&f.m.root.join("daw-workspaces/fl-studio")).unwrap();
        symlink(&f.outer, base(&f.m).join("projects")).unwrap();
        let before = fs::read(f.m.root.join("registry.json")).ok();
        assert!(create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner).is_err());
        assert_eq!(fs::read(f.m.root.join("registry.json")).ok(), before);
        assert!(!record(&f.m).exists());
    }
    #[test]
    fn graphical_and_audio_prerequisites_are_separate() {
        let f = crate::test_fixture::Fixture::new();
        let runtime = f.outer.join("runtime");
        private_exact(&runtime).unwrap();
        let auth = runtime.join("xauthority");
        fs::write(&auth, b"fixture").unwrap();
        assert!(check_prerequisites(&runtime, "", &auth).is_err());
        assert!(check_prerequisites(&runtime, ":0", &auth).is_err());
        private_exact(&runtime.join("pulse")).unwrap();
        let _socket = UnixListener::bind(runtime.join("pulse/native")).unwrap();
        assert!(check_prerequisites(&runtime, ":0", &auth).is_ok());
        fs::remove_file(&auth).unwrap();
        symlink(&f.outer, &auth).unwrap();
        assert!(check_prerequisites(&runtime, ":0", &auth).is_err());
    }
    #[test]
    fn installed_image_requires_exact_x64_under_fl_resource_root() {
        let f = crate::test_fixture::Fixture::new();
        let mut runner = f.r.environment.runner.clone();
        runner.id = STANDARD_RUNNER.into();
        let mut w = create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner).unwrap();
        let parent = w
            .environment
            .root
            .join("compatdata/pfx/drive_c/Program Files/Image-Line/FL Studio 2025");
        fs::create_dir_all(&parent).unwrap();
        let pe = parent.join("FL64.exe");
        let mut bytes = vec![0u8; 192];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        fs::write(&pe, bytes).unwrap();
        let app = discover(&w, "26.1.6.0").unwrap();
        assert_eq!(app.executable.path, pe);
        assert_eq!(app.installer_advertised_release, "26.1.6.0");
        assert_eq!(app.observed_file_version, None);
        let image_sha = app.executable.sha256.clone();
        let op = "11".repeat(16);
        reserve_install_record(&mut w, &op).unwrap();
        retire_install_record(&mut w, &op, "completed", Some(app), "unused").unwrap();
        save(&f.m, &mut w).unwrap();
        assert!(record_installed_version(&f.m, "26.1.6.5639", &"cd".repeat(32)).is_err());
        record_installed_version(&f.m, "26.1.6.5639", &image_sha).unwrap();
        assert_eq!(
            load(&f.m)
                .unwrap()
                .installed
                .unwrap()
                .observed_file_version
                .as_deref(),
            Some("26.1.6.5639")
        );
    }

    #[test]
    fn clean_uninstall_allows_same_version_reinstall_with_new_operation() {
        let f = crate::test_fixture::Fixture::new();
        let a = imported_fixture(&f, 11);
        let mut w = fixture_workspace(&f, &a.id);
        let identity = (
            w.id.clone(),
            w.environment.clone(),
            w.projects.clone(),
            w.preferences.clone(),
            w.exports.clone(),
        );
        let native_before = fs::read(f.m.root.join("registry.json")).ok();
        let first = "11".repeat(16);
        let removal = "22".repeat(16);
        let second = "33".repeat(16);
        reserve_install_record(&mut w, &first).unwrap();
        let app = fixture_app(&w, "26.1.6.0", 1);
        retire_install_record(&mut w, &first, "completed", Some(app.clone()), "unused").unwrap();
        let first_record = w.installations[0].clone();
        reserve_uninstall_record(&mut w, &removal).unwrap();
        fs::remove_file(&app.executable.path).unwrap();
        retire_uninstall_record(&mut w, &removal).unwrap();
        assert_eq!(w.state, State::Uninstalled);
        assert!(w.installed.is_none());
        assert_eq!(w.installations[0], first_record);
        assert!(!apply_installer_selection(
            &mut w,
            &installer_import::load(&f.m, &a.id).unwrap(),
            "26.1.6.0"
        )
        .unwrap());
        install_admission(&w, true, false, !image_roster(&w).unwrap().is_empty()).unwrap();
        reserve_install_record(&mut w, &second).unwrap();
        let app = fixture_app(&w, "26.1.6.0", 2);
        retire_install_record(&mut w, &second, "completed", Some(app), "unused").unwrap();
        save(&f.m, &mut w).unwrap();
        let current = load(&f.m).unwrap();
        assert_ne!(first, second);
        assert_eq!(current.installations.len(), 2);
        assert_eq!(current.installations[0], first_record);
        assert_eq!(current.uninstalls[0].operation, removal);
        assert_eq!(
            current.current_installation_operation.as_deref(),
            Some(second.as_str())
        );
        assert_eq!(
            (
                current.id,
                current.environment,
                current.projects,
                current.preferences,
                current.exports
            ),
            identity
        );
        assert_eq!(fs::read(f.m.root.join("registry.json")).ok(), native_before);
    }

    #[test]
    fn admitted_version_change_keeps_a_history_and_workspace_roots() {
        let f = crate::test_fixture::Fixture::new();
        let a = imported_fixture(&f, 21);
        let b = imported_fixture(&f, 22);
        let mut w = fixture_workspace(&f, &a.id);
        let roots = (
            w.id.clone(),
            w.environment.clone(),
            w.projects.clone(),
            w.preferences.clone(),
            w.exports.clone(),
        );
        let first = "41".repeat(16);
        reserve_install_record(&mut w, &first).unwrap();
        let app_a = fixture_app(&w, "26.1.6.0", 3);
        retire_install_record(&mut w, &first, "completed", Some(app_a.clone()), "unused").unwrap();
        let old = w.installations[0].clone();
        let removal = "42".repeat(16);
        reserve_uninstall_record(&mut w, &removal).unwrap();
        fs::remove_file(&app_a.executable.path).unwrap();
        retire_uninstall_record(&mut w, &removal).unwrap();
        let verified_b = installer_import::load(&f.m, &b.id).unwrap();
        assert!(apply_installer_selection(&mut w, &verified_b, "27.0.0.0").unwrap());
        let second = "43".repeat(16);
        reserve_install_record(&mut w, &second).unwrap();
        let app_b = fixture_app(&w, "27.0.0.0", 4);
        retire_install_record(&mut w, &second, "completed", Some(app_b.clone()), "unused").unwrap();
        assert_eq!(w.selected_installer.sha256, b.id);
        assert_eq!(w.selected_installer.release, "27.0.0.0");
        assert_eq!(
            w.installed.as_ref().unwrap().installer_advertised_release,
            "27.0.0.0"
        );
        assert_eq!(w.installed.as_ref().unwrap().executable, app_b.executable);
        assert_eq!(w.installations[0], old);
        assert_eq!(w.installations[1].installer.sha256, b.id);
        assert_eq!(
            (w.id, w.environment, w.projects, w.preferences, w.exports),
            roots
        );
    }

    #[test]
    fn repeatable_workspace_lifecycle_leaves_native_authority_files_unchanged() {
        let f = crate::test_fixture::Fixture::new();
        let a = imported_fixture(&f, 23);
        let b = imported_fixture(&f, 24);
        let mut w = fixture_workspace(&f, &a.id);
        let publications = [
            "Pure LoFi",
            "Efx FRAGMENTS",
            "Pigments",
            "Serum 2",
            "Blackhole",
            "Kontakt",
        ];
        fs::create_dir_all(&f.m.publications).unwrap();
        fs::create_dir_all(f.m.root.join("policies")).unwrap();
        fs::create_dir_all(f.m.root.join("candidates")).unwrap();
        let mut native_paths = vec![
            f.m.root.join("registry.json"),
            f.m.root.join("software.json"),
            f.m.root.join("policies/x11_touch_routing_v2.json"),
            f.m.root.join("candidates/serum-candidate-d.json"),
        ];
        native_paths.extend(publications.iter().map(|name| f.m.publications.join(name)));
        for (index, path) in native_paths.iter().enumerate() {
            fs::write(path, format!("native authority canary {index}")).unwrap();
        }
        let before: Vec<_> = native_paths
            .iter()
            .map(|path| fs::read(path).unwrap())
            .collect();
        let first = "44".repeat(16);
        reserve_install_record(&mut w, &first).unwrap();
        let installed_a = fixture_app(&w, "26.1.6.0", 9);
        retire_install_record(
            &mut w,
            &first,
            "completed",
            Some(installed_a.clone()),
            "unused",
        )
        .unwrap();
        reserve_uninstall_record(&mut w, &"45".repeat(16)).unwrap();
        fs::remove_file(&installed_a.executable.path).unwrap();
        retire_uninstall_record(&mut w, &"45".repeat(16)).unwrap();
        apply_installer_selection(
            &mut w,
            &installer_import::load(&f.m, &b.id).unwrap(),
            "27.0.0.0",
        )
        .unwrap();
        let second = "46".repeat(16);
        reserve_install_record(&mut w, &second).unwrap();
        let installed_b = fixture_app(&w, "27.0.0.0", 10);
        retire_install_record(&mut w, &second, "completed", Some(installed_b), "unused").unwrap();
        save(&f.m, &mut w).unwrap();
        for (path, bytes) in native_paths.iter().zip(before) {
            assert_eq!(fs::read(path).unwrap(), bytes, "{} changed", path.display());
        }
    }

    #[test]
    fn two_install_and_uninstall_cycles_keep_both_exact_histories() {
        let f = crate::test_fixture::Fixture::new();
        let mut w = fixture_workspace(&f, &"ab".repeat(32));
        let mut expected_installs = Vec::new();
        let mut expected_uninstalls = Vec::new();
        for (index, install_op, uninstall_op) in [
            (1, "91".repeat(16), "92".repeat(16)),
            (2, "93".repeat(16), "94".repeat(16)),
        ] {
            reserve_install_record(&mut w, &install_op).unwrap();
            let app = fixture_app(&w, "26.1.6.0", index);
            retire_install_record(&mut w, &install_op, "completed", Some(app.clone()), "unused")
                .unwrap();
            expected_installs.push(w.installations.last().unwrap().clone());
            reserve_uninstall_record(&mut w, &uninstall_op).unwrap();
            fs::remove_file(&app.executable.path).unwrap();
            retire_uninstall_record(&mut w, &uninstall_op).unwrap();
            expected_uninstalls.push(w.uninstalls.last().unwrap().clone());
        }
        save(&f.m, &mut w).unwrap();
        let reloaded = load(&f.m).unwrap();
        assert_eq!(reloaded.state, State::Uninstalled);
        assert_eq!(reloaded.installations, expected_installs);
        assert_eq!(reloaded.uninstalls, expected_uninstalls);
        assert_eq!(reloaded.installations.len(), 2);
        assert_eq!(reloaded.uninstalls.len(), 2);
    }

    #[test]
    fn selection_and_install_refuse_live_or_uncertain_ownership() {
        let f = crate::test_fixture::Fixture::new();
        let mut w = fixture_workspace(&f, &"ab".repeat(32));
        assert!(selection_admission(&w, false, false).is_err());
        assert!(selection_admission(&w, true, true).is_err());
        assert!(install_admission(&w, true, false, true).is_err());
        let first = "51".repeat(16);
        reserve_install_record(&mut w, &first).unwrap();
        assert!(selection_admission(&w, true, false).is_err());
        assert!(install_admission(&w, true, false, false).is_err());
        let app = fixture_app(&w, "26.1.6.0", 5);
        retire_install_record(&mut w, &first, "completed", Some(app), "unused").unwrap();
        let removal = "52".repeat(16);
        reserve_uninstall_record(&mut w, &removal).unwrap();
        assert!(selection_admission(&w, true, false).is_err());
        assert!(install_admission(&w, true, false, false).is_err());
        retire_uninstall_record(&mut w, &removal).unwrap();
        w.state = State::CleanupUnconfirmed;
        assert!(selection_admission(&w, true, false).is_err());
        w.state = State::Uninstalled;
        w.history_recovery_required = true;
        assert!(selection_admission(&w, true, false).is_err());
        assert!(install_admission(&w, true, false, false).is_err());
    }

    #[test]
    fn retired_failed_install_can_retry_without_erasing_failure() {
        let f = crate::test_fixture::Fixture::new();
        let mut w = fixture_workspace(&f, &"ab".repeat(32));
        let first = "61".repeat(16);
        reserve_install_record(&mut w, &first).unwrap();
        retire_install_record(&mut w, &first, "failed", None, "first_failure_exact").unwrap();
        assert_eq!(w.state, State::Failed);
        assert_eq!(
            w.installations[0].failure.as_deref(),
            Some("first_failure_exact")
        );
        install_admission(&w, true, false, false).unwrap();
        let second = "62".repeat(16);
        reserve_install_record(&mut w, &second).unwrap();
        let app = fixture_app(&w, "26.1.6.0", 6);
        retire_install_record(&mut w, &second, "completed", Some(app), "unused").unwrap();
        assert_eq!(w.installations.len(), 2);
        assert_eq!(w.installations[0].outcome, InstallationOutcome::Failed);
        assert_eq!(
            w.installations[0].failure.as_deref(),
            Some("first_failure_exact")
        );
        assert_eq!(w.installations[1].outcome, InstallationOutcome::Installed);
        assert_eq!(w.first_failure.as_deref(), Some("first_failure_exact"));
        assert_eq!(
            current_failure(&w, State::Installed, "confirmed", false, None),
            None
        );
        assert_eq!(
            w.installations[0].failure.as_deref(),
            Some("first_failure_exact")
        );
    }

    #[test]
    fn schema_one_clean_uninstall_migrates_without_erasing_receipts() {
        let f = crate::test_fixture::Fixture::new();
        let a = imported_fixture(&f, 31);
        let w = fixture_workspace(&f, &a.id);
        let first = "71".repeat(16);
        let removal = "72".repeat(16);
        let app = fixture_app(&w, "26.1.6.0", 7);
        let install_dir = result_path(&f.m, &first, true)
            .parent()
            .unwrap()
            .to_path_buf();
        let uninstall_dir = result_path(&f.m, &removal, false)
            .parent()
            .unwrap()
            .to_path_buf();
        private_exact(&install_dir).unwrap();
        private_exact(&uninstall_dir).unwrap();
        atomic_json(
            &install_dir.join("spec.json"),
            &json!({"operation":first,
            "installer":a.artifact,"environment":w.environment,
            "report":result_path(&f.m,&first,true)}),
        )
        .unwrap();
        atomic_json(
            &result_path(&f.m, &first, true),
            &json!({"operation":first,
            "state":"failed","cleanup_confirmed":true,"owned_live":0,
            "error":"first_installer_exit"}),
        )
        .unwrap();
        atomic_json(
            &uninstall_dir.join("spec.json"),
            &json!({
            "kind":"daw_workspace_uninstall","operation_id":removal,
            "report":result_path(&f.m,&removal,false),
            "application":{"id":"fl_studio","environment":w.environment,
                "installed_image":app.executable,"projects":w.projects,
                "preferences":w.preferences,"exports":w.exports}}),
        )
        .unwrap();
        atomic_json(
            &result_path(&f.m, &removal, false),
            &json!({
            "operation_id":removal,"state":"completed","cleanup_confirmed":true,
            "owned_live":0}),
        )
        .unwrap();
        fs::remove_file(&app.executable.path).unwrap();
        atomic_json(
            &record(&f.m),
            &json!({"schema":1,"id":w.id,
            "revision":12,"application":"fl_studio","installer":a.id,
            "installer_release":"26.1.6.0","environment":w.environment,
            "runner_manifest":null,"projects":w.projects,"exports":w.exports,
            "preferences":w.preferences,"state":"uninstalled",
            "installation_operation":first,"installed":null,"audio":null,
            "session_operation":removal,"uninstall_operation":removal,
            "first_failure":"installer_exit_nonzero_application_present"}),
        )
        .unwrap();
        let old_record = fs::read(record(&f.m)).unwrap();
        let old_install_receipt = fs::read(result_path(&f.m, &first, true)).unwrap();
        let old_uninstall_receipt = fs::read(result_path(&f.m, &removal, false)).unwrap();
        let mut migrated = load(&f.m).unwrap();
        assert_eq!(migrated.schema, 2);
        assert_eq!(migrated.id, w.id);
        assert_eq!(migrated.environment, w.environment);
        assert_eq!(migrated.state, State::Uninstalled);
        assert!(!migrated.history_recovery_required);
        assert_eq!(migrated.installations[0].operation, first);
        assert_eq!(
            migrated.installations[0].outcome,
            InstallationOutcome::NeedsUserAction
        );
        assert_eq!(migrated.uninstalls[0].operation, removal);
        assert_eq!(migrated.uninstalls[0].outcome, UninstallOutcome::Completed);
        assert_eq!(fs::read(record(&f.m)).unwrap(), old_record);
        let verified_a = installer_import::load(&f.m, &a.id).unwrap();
        assert!(!apply_installer_selection(&mut migrated, &verified_a, "26.1.6.0").unwrap());
        reserve_install_record(&mut migrated, &"73".repeat(16)).unwrap();
        save(&f.m, &mut migrated).unwrap();
        let reloaded = load(&f.m).unwrap();
        assert_eq!(reloaded.installations.len(), 2);
        assert_eq!(reloaded.installations[0].operation, first);
        assert_eq!(
            fs::read(result_path(&f.m, &first, true)).unwrap(),
            old_install_receipt
        );
        assert_eq!(
            fs::read(result_path(&f.m, &removal, false)).unwrap(),
            old_uninstall_receipt
        );
    }

    #[test]
    fn missing_schema_one_receipt_is_unknown_and_blocks_reinstall() {
        let f = crate::test_fixture::Fixture::new();
        let w = fixture_workspace(&f, &"ab".repeat(32));
        atomic_json(
            &record(&f.m),
            &json!({"schema":1,"id":w.id,
            "revision":2,"application":"fl_studio","installer":w.selected_installer.sha256,
            "installer_release":"26.1.6.0","environment":w.environment,
            "runner_manifest":null,"projects":w.projects,"exports":w.exports,
            "preferences":w.preferences,"state":"uninstalled",
            "installation_operation":"81".repeat(16),"installed":null,"audio":null,
            "session_operation":"82".repeat(16),"uninstall_operation":"82".repeat(16),
            "first_failure":null}),
        )
        .unwrap();
        let migrated = load(&f.m).unwrap();
        assert!(migrated.history_recovery_required);
        assert_eq!(
            migrated.installations[0].outcome,
            InstallationOutcome::Unknown
        );
        assert_eq!(migrated.uninstalls[0].outcome, UninstallOutcome::Unknown);
        assert!(install_admission(&migrated, true, false, false).is_err());
    }

    #[test]
    fn discovered_unowned_image_cannot_be_adopted_as_current_install() {
        let f = crate::test_fixture::Fixture::new();
        let w = fixture_workspace(&f, &"ab".repeat(32));
        let app = fixture_app(&w, "26.1.6.0", 8);
        assert_eq!(image_roster(&w).unwrap(), vec![app.executable.path]);
        assert!(install_admission(&w, true, false, !image_roster(&w).unwrap().is_empty()).is_err());
        assert!(w.installed.is_none());
        assert!(w.installations.is_empty());
    }
}
