//! Current Home/Setup authority. Historical revisions, incidents and operation
//! bodies belong to the separate Diagnostics snapshot.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::MetadataExt;
use linux_vst_bridge::{catalogue, profiles, publication};

type CurrentRevisions = BTreeMap<String,
    std::result::Result<Option<publication::Revision>, &'static str>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileStamp { device:u64, inode:u64, extent:u64, mode:u32,
    modified:i64, modified_ns:i64, changed:i64, changed_ns:i64 }
fn stamp(path: &Path) -> Result<Option<FileStamp>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(FileStamp {device:meta.dev(),inode:meta.ino(),
            extent:meta.len(),mode:meta.mode(),modified:meta.mtime(),
            modified_ns:meta.mtime_nsec(),changed:meta.ctime(),changed_ns:meta.ctime_nsec()})),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
fn watch_paths(m:&Manager, sw:&Software, db:&Registry) -> Result<BTreeMap<PathBuf,Option<FileStamp>>> {
    let mut paths = BTreeSet::new();
    let runtime_records: BTreeSet<_>=linux_vst_bridge::runtime_delivery::record_paths(m).into_iter().collect();
    paths.insert(m.root.join("software.json"));
    paths.insert(m.root.join("runners"));
    paths.extend(runtime_records.iter().cloned());
    paths.insert(m.root.join("registry.json"));
    paths.insert(m.root.join("operator/latest.json"));
    paths.insert(m.root.join("operator/resume.json"));
    paths.extend(renderer_continuation::watch_paths(m)?);
    for refresh in environment_refresh::records(m)? {
        let directory=job_dir(m,&refresh.intent.owner)?;
        paths.insert(directory.join("refresh.json"));
        paths.insert(directory.join("request.json"));
        paths.insert(refresh.environment.root.join("environment.json"));
        paths.insert(m.root.join("inventory").join(format!("{}.json",refresh.environment.id)));
    }
    for binding in environment_revision::renderer_records(m)? {
        let directory=job_dir(m,&binding.operation)?;
        paths.insert(directory.join("renderer-maintenance.json"));
        paths.insert(directory.join("request.json"));
        paths.insert(binding.selected.path);
        paths.insert(binding.spec.path);
        paths.insert(binding.result.path);
    }
    paths.insert(m.root.join("daw-workspaces/fl-studio/workspace.json"));
    let deleted = linux_vst_bridge::deleted_setup::directory(m);
    paths.insert(deleted.clone());
    let deleted_scan = linux_vst_bridge::deleted_setup::scan(m)?;
    for environment in deleted_scan.records.iter().map(|record| &record.environment).chain(&deleted_scan.unreadable) {
        paths.insert(linux_vst_bridge::deleted_setup::path(m, environment)?);
    }
    let vendor_root = m.root.join("vendor-applications");
    paths.insert(vendor_root.clone());
    if vendor_root.is_dir() {
        for (n, entry) in fs::read_dir(&vendor_root)?.enumerate() {
            require(n < 128, "vendor_application_count_bound")?;
            let entry = entry?;
            if vendor_application::ApplicationId::parse(&entry.file_name().to_string_lossy()).is_ok() {
                paths.insert(entry.path());
                paths.insert(entry.path().join("application.json"));
            }
        }
    }
    for selector in vendor_cli::selectors(m)? {
        let vendor = vendor_cli::directory(m, &selector)?;
    for path in [&vendor,&vendor.join("application.json"),&vendor.join("operation.json"),
        &vendor.join("operation-result.json"),
        ] { paths.insert(path.clone()); }
    let app = vendor_cli::application(m, &selector)?;
        require(
            matches!(app.schema, 1..=3)
                && app.environment.root == m.root.join("environments").join(&app.environment.id),
            "operator_current_vendor_binding",
        )?;
        paths.insert(app.environment.root.join("environment.json"));
        if app.user_selection.is_some() { paths.insert(app.executable.path); }
    }
    for artifact in [&sw.manager,&sw.supervisor,&sw.ownership,&sw.host,&sw.source_manifest] {
        paths.insert(artifact.path.clone());
    }
    for artifact in [&sw.installer_launch,&sw.preparation_kit,&sw.operator_frontend,&sw.native_catalogue]
        .into_iter().flatten() { paths.insert(artifact.path.clone()); }
    for (class, entry) in &db.classes {
        require(valid_hex(class,32),"operator_current_class_identity")?;
        let registration = &entry.registration;
        for artifact in [&registration.module,&registration.host,&registration.native] {
            paths.insert(artifact.path.clone());
        }
        paths.insert(registration.host.path.with_file_name("host-source-manifest.json"));
        paths.insert(registration.environment.root.join("environment.json"));
        paths.insert(registration.environment.runner.proton.clone());
        paths.insert(registration.environment.runner.entry_point.clone());
        for artifact in &registration.environment.runner.files { paths.insert(artifact.path.clone()); }
        paths.insert(m.link(class));
        paths.insert(m.root.join("performance").join(format!("{}.json",class.to_uppercase())));
        if let Some(reference) = &entry.managed_revision {
            require(valid_hex(&reference.id,32),"operator_current_revision_identity")?;
            let base = m.root.join("publications").join(class).join("revisions").join(&reference.id);
            paths.insert(base.join("revision.json"));
            paths.insert(base.join(format!("LVB_{class}.vst3/bridge-provenance.json")));
            if let Ok(revision) = m.load_revision_record(class, reference) {
                paths.insert(m.root.join("transactions").join(format!("{}.json",revision.transaction)));
                paths.insert(m.root.join("transactions").join(format!("{}.result.json",revision.transaction)));
            }
        }
    }
    let installers = m.root.join("installers");
    paths.insert(installers.clone());
    paths.insert(installers.join("presentation"));
    if installers.is_dir() {
        for (n, entry) in fs::read_dir(&installers)?.enumerate() {
            require(n < 512,"operator_current_installer_watch_bound")?;
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                paths.insert(path.clone());
                let installer: installer_import::Installer = read_json(&path)?;
                let extension = match installer.format.as_str() {
                    "pe_executable"=>"exe","msi_compound"=>"msi",
                    _=>return Err("operator_current_installer_format".into()),
                };
                require(valid_hex(&installer.id,64)
                    && installer.artifact.sha256 == installer.id
                    && path == installers.join(format!("{}.json",installer.id))
                    && installer.artifact.path == installers.join(format!("{}.{}",installer.id,extension)),
                    "operator_current_installer_binding")?;
                paths.insert(installer.artifact.path);
            }
        }
    }
    let presentations = installers.join("presentation");
    if presentations.is_dir() {
        for (n, entry) in fs::read_dir(presentations)?.enumerate() {
            require(n < 512,"operator_current_presentation_watch_bound")?;
            paths.insert(entry?.path());
        }
    }
    let onboarding = m.root.join("onboarding");
    paths.insert(onboarding.clone());
    if onboarding.is_dir() {
        for (n, entry) in fs::read_dir(&onboarding)?.enumerate() {
            require(n < 128,"operator_current_onboarding_watch_bound")?;
            let dir = entry?.path();
            let Some(id) = dir.file_name().and_then(|name|name.to_str()) else {continue};
            if !valid_hex(id,32) {continue;}
            paths.insert(dir.clone());
            let record_path = dir.join("record.json");
            paths.insert(record_path.clone());
            let record: onboarding::Record = read_json(&record_path)?;
            require(record.schema == 1 && record.id == id
                && record.environment.id == id
                && record.environment.root == m.root.join("environments").join(id)
                && record.installation_operation.as_ref().is_none_or(|op|valid_hex(op,32)),
                "operator_current_onboarding_binding")?;
            if let Some(op) = record.installation_operation {
                paths.insert(dir.join(format!("{op}-result.json")));
                paths.insert(dir.join(format!("{op}-transaction-private.json")));
            }
            paths.insert(m.root.join("inventory").join(format!("{}.json",record.environment.id)));
        }
    }
    let inventory = m.root.join("inventory");
    paths.insert(inventory);
    paths.insert(m.root.join("operator"));
    for binding in automatic_install::current_vendor_bindings(m)? {
        let dir=job_dir(m,&binding.operation)?;
        paths.insert(dir.clone());
        paths.insert(dir.join("automatic.json"));
        paths.insert(dir.join("vendor-maintenance.json"));
        paths.insert(binding.environment.root.join("environment.json"));
        paths.insert(m.root.join("inventory").join(format!("{}.json",binding.environment.id)));
    }
    for installation in environment_install::records(m)? {
        let dir = job_dir(m,&installation.operation)?;
        paths.insert(dir.clone());
        paths.insert(dir.join("installation.json"));
        paths.insert(dir.join("installation-result.json"));
        paths.insert(installation.environment.root.join("environment.json"));
        paths.insert(m.root.join("inventory").join(format!("{}.json",installation.environment.id)));
    }
    for transition in environment_runtime::records(m)? {
        paths.insert(job_dir(m,&transition.operation)?.join("runtime-transition.json"));
        paths.insert(transition.environment.root.join("environment.json"));
        paths.insert(m.root.join("inventory").join(format!("{}.json",transition.environment.id)));
        for runner in [&transition.before.runner,&transition.environment.runner] {
            for artifact in &runner.files {paths.insert(artifact.path.clone());}
        }
    }
    let transactions = m.root.join("transactions");
    paths.insert(transactions.clone());
    if transactions.is_dir() {
        for (n, entry) in fs::read_dir(transactions)?.enumerate() {
            require(n < 4096,"operator_current_transaction_watch_bound")?;
            let path=entry?.path();
            if path.file_name().is_some_and(|name|name.to_string_lossy().ends_with(".pending.json")) {
                paths.insert(path);
            }
        }
    }
    require(paths.len() <= 4096,"operator_current_watch_bound")?;
    // Declared-runtime read failures are bound by the token's per-record
    // failure observation. This watch cannot grant runtime admission.
    paths.into_iter().map(|path| {
        let observed=stamp(&path).or_else(|error|
            if runtime_records.contains(&path) {Ok(None)} else {Err(error)})?;
        Ok((path,observed))
    }).collect()
}

pub(super) struct CurrentOverviewContext {
    pub snapshot: ui::Snapshot,
    pub busy: Option<&'static str>,
    pub profiles: Vec<profiles::Profile>,
    pub revisions: CurrentRevisions,
    pub current_generation: String,
    pub software: Software,
    pub registry: Registry,
    pub preparation: preparation::RecordReadback,
    pub bindings: Vec<catalogue::EnvironmentBinding>,
    owners: Vec<capacity::Owner>,
    watched: BTreeMap<PathBuf,Option<FileStamp>>,
    installer_live: BTreeMap<String,bool>,
    vendor_progress: Option<vendor_application::Operation>,
    vendor_retired: bool,
    all_vendor_retired: bool,
    cleanup_seen: Option<bool>,
    #[cfg(feature = "pb0-c0-audit")]
    captured_at: Instant,
}
impl CurrentOverviewContext {
    /// Stamp the preparation records a projection read. The view's own files
    /// are bounded at capture; each product's history, and all of them
    /// together, by the readback, which leaves out what it cannot watch.
    pub(super) fn capture_preparation_watches(&mut self) -> Result<()> {
        require(self.preparation.watching(), "preparation_record_watch_absent")?;
        for path in self.preparation.watched_paths()? {
            if !self.watched.contains_key(&path) { self.watched.insert(path.clone(), stamp(&path)?); }
        }
        Ok(())
    }
    pub(super) fn class_busy(&self, class: &str) -> Option<&'static str> {
        class_inactive_reason(&self.snapshot.system, &self.owners, self.all_vendor_retired, class)
    }

    fn live_installer_progress_paths(&self, m: &Manager) -> Result<BTreeMap<PathBuf,Vec<PathBuf>>> {
        let mut paths = BTreeMap::new();
        let in_progress = |receipt: &Value| matches!(receipt["state"].as_str(),
            Some("queued" | "starting" | "running" | "unknown"))
            && receipt["cleanup_confirmed"] != true;
        for row in &self.snapshot.onboarding {
            let Some(environment) = row.environment.as_ref() else {continue};
            let installation = &row.details["installation"];
            let Some(operation) = installation["operation"].as_str() else {continue};
            if self.installer_live.get(operation) != Some(&true)
                || !in_progress(installation) { continue; }
            let directory = m.root.join("onboarding").join(environment);
            let report = directory.join(format!("{operation}-result.json"));
            require(file(&report)?.metadata()?.len() <= 16384,"installer_result_bound")?;
            let current: Value = read_json(&report)?;
            // A terminal receipt can precede service exit. Its observations can
            // already authorize application selection, so require a refresh.
            require(current["schema"] == 2 && current["operation"] == operation
                && in_progress(&current),"operator_installer_state_changed_refresh")?;
            let observation = directory.join(format!("{operation}-transaction-private.json"));
            paths.insert(directory,vec![report,observation]);
        }
        for row in &self.snapshot.environment_installers {
            if let Some(op) = row.operation.as_ref().filter(|op|self.installer_live.get(*op) == Some(&true)) {
                let dir = m.root.join("operator").join(op);
                paths.insert(dir.clone(),vec![dir.join("installation-result.json")]);
            }
        }
        Ok(paths)
    }
    fn recheck_external(&self, m: &Manager) -> Result<()> {
        // These fixed, bounded service observations are independent. Run at
        // most four installer cohorts alongside vendor, workspace and cleanup
        // so a larger retained setup history does not create one thread per
        // attempt. None of these probes holds registry.lock.
        std::thread::scope(|scope| -> Result<()> {
            let vendor = linux_vst_bridge::spawn_recorded(scope, || {
                let at = Instant::now();
                ((vendor_retired(m), live_vendor_progress(m)), at.elapsed().as_millis())
            });
            let workspace = linux_vst_bridge::spawn_recorded(scope, || {
                let at = Instant::now();
                (daw_workspace::current_projection(m), at.elapsed().as_millis())
            });
            let cleanup = self.cleanup_seen.map(|_| linux_vst_bridge::spawn_recorded(scope, || {
                let at = Instant::now();
                (pulse_cleanup(m), at.elapsed().as_millis())
            }));
            let installer = linux_vst_bridge::spawn_recorded(scope, || {
                let at = Instant::now();
                let operations = self.installer_live.keys().cloned().collect::<Vec<_>>();
                (installer_liveness(&operations), at.elapsed().as_millis())
            });
            let (vendor_result, vendor_ms) = vendor.join()
                .map_err(|_| "operator_vendor_probe_failed")?;
            let (workspace_result, workspace_ms) = workspace.join()
                .map_err(|_| "operator_workspace_probe_failed")?;
            let (cleanup_result, cleanup_ms) = match cleanup {
                Some(check) => {
                    let (result, elapsed) = check.join()
                        .map_err(|_| "operator_cleanup_probe_failed")?;
                    (result, elapsed)
                }
                None => (None, 0),
            };
            let (installer_result, installer_ms) = installer.join()
                .map_err(|_| "operator_installer_probe_failed")?;
            require(self.vendor_retired == vendor_result.0?
                && self.vendor_progress == vendor_result.1?,
                "operator_vendor_state_changed_refresh")?;
            if let Some(expected) = self.cleanup_seen {
                require(cleanup_result == Some(expected),
                    "operator_cleanup_state_changed_refresh")?;
            }
            require(installer_result? == self.installer_live,
                "operator_installer_state_changed_refresh")?;
            let (workspaces, _) = workspace_result?;
            #[cfg(feature = "pb0-c0-audit")]
            eprintln!("PB0_PHASE {}",json!({"external_vendor_ms":vendor_ms,
                "external_cleanup_ms":cleanup_ms,"external_installer_ms":installer_ms,
                "external_workspace_ms":workspace_ms,"installer_cohorts":self.installer_live.len()}));
            #[cfg(not(feature = "pb0-c0-audit"))]
            let _ = (vendor_ms, cleanup_ms, installer_ms, workspace_ms);
            require(serde_json::to_value(&workspaces)? == serde_json::to_value(&self.snapshot.workspaces)?,
                "operator_workspace_state_changed_refresh")
        })
    }
    fn recheck_with(&self, m: &Manager, mut external: impl FnMut() -> Result<()>) -> Result<()> {
        let observed = timing::span(Stage::CurrentRecheck);
        let started = Instant::now();
        // Both external observations occur without registry.lock. Manager-owned
        // bytes and owner records are compared between them under the guard.
        external()?;
        let installer_progress = self.live_installer_progress_paths(m)?;
        #[cfg(feature = "pb0-c0-audit")]
        let external_before_ms = started.elapsed().as_millis();
        #[cfg(feature = "pb0-c0-audit")]
        let registry_at = Instant::now();
        {
            let _guard = acquire_readback(m, ui::OperatorLock::Registry, None,
                OPERATOR_WAIT, &mut vec![])?;
            require(self.snapshot.state_token == token(m)?
                && self.current_generation == pulse_generation(m)?
                && self.owners == capacity::owners(m)?
                && self.snapshot.system.pending_transactions == pending_transactions(m)?,
                "operator_state_changed_refresh")?;
            let mut progress = installer_progress.clone();
            if let Some(vendor) = &self.vendor_progress {
                require_live_vendor_report(vendor)?;
                progress.insert(vendor.report.parent().ok_or("operator_vendor_report_directory")?.to_owned(),vec![vendor.report.clone()]);
            }
            let runtime_records: BTreeSet<_>=linux_vst_bridge::runtime_delivery::record_paths(m).into_iter().collect();
            for (path, before) in &self.watched {
                // An active supervisor atomically replaces its observations.
                // While live, those bytes cannot authorize launch or retirement:
                // both external rechecks require the same operation to remain live.
                // Keep stable application/job/environment/record inputs watched.
                if progress.values().any(|reports| reports.contains(path)) { continue; }
                let after = stamp(path).or_else(|error|
                    if runtime_records.contains(path) {Ok(None)} else {Err(error)})?;
                let unchanged = if progress.contains_key(path) {
                    // Report replacement also changes the containing directory.
                    // Preserve its identity/type/mode, not progress-write times.
                    before.zip(after).is_some_and(|(a,b)|
                        (a.device,a.inode,a.mode) == (b.device,b.inode,b.mode))
                } else { after == *before };
                require(unchanged,"operator_current_artifact_changed_refresh")?;
            }
        }
        #[cfg(feature = "pb0-c0-audit")]
        let registry_ms = registry_at.elapsed().as_millis();
        #[cfg(feature = "pb0-c0-audit")]
        let external_after_at = Instant::now();
        external()?;
        require(self.live_installer_progress_paths(m)? == installer_progress,
            "operator_installer_state_changed_refresh")?;
        #[cfg(feature = "pb0-c0-audit")]
        eprintln!("PB0_PHASE {}",json!({"token_after_ms":started.elapsed().as_millis(),
            "external_before_ms":external_before_ms,"registry_ms":registry_ms,
            "external_after_ms":external_after_at.elapsed().as_millis(),
            "total_manager_ms":self.captured_at.elapsed().as_millis()}));
        #[cfg(not(feature = "pb0-c0-audit"))]
        let _ = started;
        observed.end(true);
        Ok(())
    }
    /// A second observation of what the capture read: every part of it
    /// derives its facts afresh, never from what the capture derived.
    pub(super) fn recheck(&self, m: &Manager) -> Result<()> {
        self.recheck_with(&m.composing(), || self.recheck_external(&m.composing()))
    }
    /// What the service needs to answer this view again (`service::view`).
    pub(super) fn reuse(&self) -> Reuse {
        let operation_live = self.snapshot.operation.as_ref().and_then(|operation| operation["state"].as_str())
            .is_some_and(|state| WORKER_OWNED_STATES.contains(&state));
        let quiescent = self.all_vendor_retired && self.vendor_progress.is_none()
            && !self.installer_live.values().any(|live| *live) && !operation_live
            && !self.snapshot.workspaces.iter().any(|workspace| workspace.state == "running")
            && self.snapshot.system.capacity_available() && !self.snapshot.system.cleanup_unconfirmed;
        let mut directories = BTreeSet::new();
        for (path, stamp) in &self.watched {
            if let Some(parent) = path.parent() { directories.insert(parent.to_owned()); }
            if stamp.is_some_and(|stamp| stamp.mode & libc::S_IFMT == libc::S_IFDIR) {
                directories.insert(path.clone());
            }
        }
        Reuse { quiescent, directories: directories.into_iter().collect() }
    }
}
/// Whether nothing a view shows can change without a record write, and the
/// directories of the files it stamped. A live operation, installer,
/// application or workspace session changes what the view shows without
/// writing a record (its unit can end), and so can a service census that was
/// unreadable or awaits cleanup.
pub(super) struct Reuse {
    pub quiescent: bool,
    pub directories: Vec<PathBuf>,
}

fn require_live_vendor_report(operation: &vendor_application::Operation) -> Result<()> {
    use vendor_application::OperationState;
    let result: vendor_application::OperationResult = read_json(&operation.report)?;
    require(result.schema == 3
        && result.operation_id.as_deref() == Some(operation.operation_id.as_str())
        && matches!(result.state,OperationState::Running | OperationState::Unknown)
        && !result.cleanup_confirmed
        && (result.owned_live.is_some_and(|count|count > 0)
            || (result.state == OperationState::Unknown && result.owned_live.is_none()
                && result.launcher_exit.is_none())),
        "operator_vendor_state_changed_refresh")
}
fn live_vendor_progress(m: &Manager) -> Result<Option<vendor_application::Operation>> {
    let mut live = None;
    for selector in vendor_cli::selectors(m)? {
        if vendor_cli::live(&selector)? {
            require(live.is_none(), "operator_multiple_vendor_owners")?;
            live = live_vendor_progress_selected(m, &selector, || Ok(true))?;
        }
    }
    Ok(live)
}
#[cfg(test)]
pub(super) fn live_vendor_progress_with(m: &Manager, is_live: impl FnOnce() -> Result<bool>,
)
    -> Result<Option<vendor_application::Operation>> {
    live_vendor_progress_selected(m, ASC, is_live)
}
pub(super) fn live_vendor_progress_selected(
    m: &Manager,
    selector: &str,
    is_live: impl FnOnce() -> Result<bool>,
) -> Result<Option<vendor_application::Operation>> {
    let directory = vendor_cli::directory(m, selector)?;
    if !directory.exists() || !is_live()? { return Ok(None); }
    let mut app: vendor_application::Application = read_json(&directory.join("application.json"))?;
    app.environment=environment_install::current_environment(m,&app.environment)?;
    let operation: vendor_application::Operation = read_json(&directory.join("operation.json"))?;
    require(
        matches!(app.schema, 1..=3)
            && operation.application == app
        && valid_hex(&operation.operation_id,32)
        && operation.report == directory.join("operation-result.json")
        && app.environment.root == m.root.join("environments").join(&app.environment.id)
        && read_json::<Environment>(&app.environment.root.join("environment.json"))? == app.environment,
        "operator_current_vendor_binding",
    )?;
    require_live_vendor_report(&operation)?;
    Ok(Some(operation))
}

fn installer_liveness(operations: &[String]) -> Result<BTreeMap<String, bool>> {
    // A fixed four-cohort limit keeps the number of supervised systemctl
    // helpers bounded while preserving exact per-operation identity.
    let chunk_size = operations.len().div_ceil(4).max(1);
    std::thread::scope(|scope| -> Result<BTreeMap<String, bool>> {
        let checks: Vec<_> = operations.chunks(chunk_size).map(|chunk| linux_vst_bridge::spawn_recorded(scope, move || {
            chunk.iter().map(|operation| Ok((operation.clone(), onboarding::live(operation)?)))
                .collect::<Result<Vec<_>>>()
        })).collect();
        let mut states = BTreeMap::new();
        for check in checks {
            for (operation, live) in check.join()
                .map_err(|_| "operator_installer_probe_failed")?? {
                require(states.insert(operation, live).is_none(),
                    "operator_installer_probe_duplicate")?;
            }
        }
        Ok(states)
    })
}

#[derive(Default)]
struct RecordBindings {
    artifacts: BTreeMap<String, bool>,
    runners: BTreeMap<String, bool>,
    environments: BTreeMap<String, bool>,
}
impl RecordBindings {
    fn artifact(&mut self, artifact: &Artifact) -> bool {
        let key = format!("{}:{}", artifact.path.display(), artifact.sha256);
        *self.artifacts.entry(key).or_insert_with(|| artifact.validate_record().is_ok())
    }
    fn runner(&mut self, runner: &Runner) -> Result<bool> {
        let key = catalogue::runner_key(runner)?;
        Ok(*self.runners.entry(key).or_insert_with(|| runner.validate_record().is_ok()))
    }
    fn environment(&mut self, environment: &Environment) -> Result<bool> {
        let key = hex(&sha2::Sha256::digest(serde_json::to_vec(environment)?));
        if let Some(verified) = self.environments.get(&key) {
            return Ok(*verified);
        }
        let verified = read_json::<Environment>(&environment.root.join("environment.json"))
            .is_ok_and(|actual| actual == *environment);
        self.environments.insert(key,verified);
        Ok(verified)
    }
}

pub(super) fn current_products(m: &Manager, db: &Registry, sw: &Software,
    profiles: &[profiles::Profile]) -> Result<(Vec<ui::Product>, CurrentRevisions)> {
    let mut cache = RecordBindings::default();
    let mut products = Vec::new();
    let mut revisions = BTreeMap::new();
    for (class, entry) in &db.classes {
        let r = &entry.registration;
        let revision = entry.managed_revision.as_ref().map(|reference|
            m.load_revision_record(class, reference).map_err(|_| "READINESS_REVISION_UNAVAILABLE"))
            .transpose();
        let loaded = revision.as_ref().ok().and_then(Option::as_ref);
        let completed_publication = loaded.zip(entry.managed_revision.as_ref())
            .is_some_and(|(revision, reference)|
                m.verify_completed_publication(revision, reference).is_ok());
        let performance = m.performance(class);
        let performance_valid = performance.is_ok();
        let added_frames = performance.as_ref().ok().map(|record| record.effective_frames());
        let buffered_frames = performance.as_ref().ok().map(|record|record.added_frames);
        let delivery_mode = performance.as_ref().ok().map(|record|record.delivery_mode);
        let registration_valid = r.validate_record(&m.root).is_ok();
        let module_valid = registration_valid && cache.artifact(&r.module);
        let environment_valid = cache.environment(&r.environment)? && registration_valid;
        let runner_valid = cache.runner(&r.environment.runner)? && registration_valid;
        let native_valid = registration_valid
            && (loaded.is_some() || (entry.managed_revision.is_none() && cache.artifact(&r.native)));
        let host_source = Artifact { path:r.host.path.with_file_name("host-source-manifest.json"),
            sha256:r.host_source_sha256.clone() };
        let host_bytes_valid = registration_valid
            && cache.artifact(&r.host) && cache.artifact(&host_source);
        let host_valid = if let Some(revision) = loaded {
            host_bytes_valid && revision.registration == *r
                && revision.profile.requirements.host_sha256 == r.host.sha256
                && revision.profile.requirements.host_source_sha256 == r.host_source_sha256
        } else if entry.managed_revision.is_none() {
            host_bytes_valid && sw.host.sha256 == r.host.sha256
                && sw.source_sha256 == r.host_source_sha256
        } else { false };
        let expected = loaded.map(|rev| rev.target.clone())
            .or_else(|| (entry.managed_revision.is_none()).then(|| m.target(r)));
        let actual = publication::physical(&m.link(class));
        let publication_valid = match (&entry.publication, &expected, &actual) {
            (Publication::Published, Some(expected), Ok(Some(actual))) => expected == actual,
            (Publication::Removed, _, Ok(None)) => true,
            _ => false,
        } && !m.publication_pending(class)?
            && (entry.managed_revision.is_none() || completed_publication);
        let publication_selected = entry.publication == Publication::Published;
        // A complete retained publication with no DAW link was removed on purpose.
        let removed_by_choice = entry.publication == Publication::Removed && publication_valid
            && entry.managed_revision.as_ref().is_some_and(|publication|
                setup_removal::removed_by_musician(m, class, publication));
        let uninstalled = removed_by_choice && setup_removal::gone(&r.module.path);
        let current_valid = module_valid && environment_valid && runner_valid && performance_valid
            && native_valid && host_valid && publication_valid && publication_selected;
        let profile = loaded.map(|revision| &revision.profile);
        let qualification = loaded.and_then(|revision| revision.qualification);
        let recommended = profiles.iter().find(|profile| profile.class.class_id == *class);
        let limitations = profile.map(|profile| serde_json::to_value(&profile.limitations))
            .transpose()?.and_then(|v| v.as_array().cloned())
            .map(|values| values.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
            .unwrap_or_default();
        products.push(ui::Product {
            class_id: class.clone(), name:r.metadata.name.clone(), vendor:r.metadata.vendor.clone(),
            role:serde_json::to_value(profiles::role(&r.metadata)?)?.as_str().unwrap_or("unknown").into(),
            version:r.metadata.version.clone(),
            disposition:if current_valid {"ready"} else if uninstalled {"uninstalled"}
                else if removed_by_choice {"removed_from_daw"} else {"needs_attention"}.into(),
            active_revision:profile.map(|p| p.revision),
            recommended_revision:recommended.map(|p| p.revision),
            environment:r.environment.id.clone(), runner:r.environment.runner.id.clone(),
            module_sha256:r.module.sha256.clone(), limitations, history:vec![], actions:vec![],
            compatibility:None,availability:Some(ui::NativeAvailability {available:current_valid,
                reason:if current_valid {None} else if uninstalled {Some(setup_removal::UNINSTALLED.into())}
                    else if removed_by_choice {Some(setup_removal::REMOVED_FROM_DAW.into())}
                    else {Some("The native DAW connection needs attention.".into())}}),
            details:json!({"profile":profile.map(|p|json!({"id":p.id,"revision":p.revision,"claim":p.claim})),
                "environment_revision":r.environment.revision,
                "verification_scope":"control_records",
                "execution_verification":"deferred_to_worker_or_launch",
                "module_binding_valid":module_valid,"environment_valid":environment_valid,
                "runner_binding_valid":runner_valid,"native_binding_valid":native_valid,
                "module_valid":Value::Null,"runner_valid":Value::Null,
                "native_valid":Value::Null,"host_valid":Value::Null,
                "runner_policy":r.environment.runner.policy,
                "requested_graphics":linux_vst_bridge::graphics::requested_backend(r.environment.runner.policy.as_ref()),
                "performance_valid":performance_valid,"added_frames":added_frames,
                "buffered_frames":buffered_frames,"delivery_mode":delivery_mode,
                "buffering_capacity":"deferred_to_mutation_or_launch",
                "publication_complete":completed_publication,
                "host_binding_valid":host_valid,"publication_valid":publication_valid,
                "publication_selected":publication_selected,
                "host_sha256":r.host.sha256,"host_source_sha256":r.host_source_sha256,
                "native_sha256":r.native.sha256,"qualification":qualification,
                "publication":entry.managed_revision,
                "refusal":if current_valid {Value::Null}
                    else if !publication_selected {json!({"code":"publication_not_selected"})}
                    else if entry.managed_revision.is_some() && !completed_publication {
                        json!({"code":"publication_transaction_incomplete"})
                    } else if !performance_valid {json!({"code":"performance_authority_unavailable"})}
                    else {json!({"code":"current_artifact_or_publication_changed"})}}),
        });
        if current_valid {
            if let Some(product) = products.last_mut() { project_last_load(m, product); }
        }
        revisions.insert(class.clone(), revision);
    }
    Ok((products, revisions))
}

/// A plug-in the DAW can load whose last load was refused because its Windows
/// side had stopped says so on its card, until a later load is accepted (the
/// service rewrites the same per-class record on every load). It is kept under
/// `details.last_load`, apart from `details.problem`, which discovery and
/// compatibility projections own and replace. An absent or unreadable record
/// explains nothing; it never fails the overview.
pub(super) fn project_last_load(m: &Manager, product: &mut ui::Product) {
    if !valid_hex(&product.class_id, 32) { return; }
    let name = format!("{}.json", product.class_id.to_ascii_lowercase());
    let startup = read_json::<Value>(&m.root.join("runtime/native-startup").join(&name)).ok();
    // The bridge plug-in in the DAW keeps why the DAW's last attempt to start
    // this plug-in's audio was refused (#403). It explains the card while no
    // later load was recorded, and only for this exact plug-in and bridge.
    if let Ok(activation) = read_json::<Value>(&m.root.join("runtime/native-activation").join(&name)) {
        let observed = activation["observed_at"].as_u64();
        let later_load = startup.as_ref().and_then(|record| record["observed_at"].as_u64())
            .zip(observed).is_some_and(|(load, refused)| load > refused);
        let exact = activation["class_id"].as_str()
                .is_some_and(|class| class.eq_ignore_ascii_case(&product.class_id))
            && activation["module_sha256"].as_str() == Some(product.module_sha256.as_str())
            && activation["engine_sha256"].is_string()
            && activation["engine_sha256"] == product.details["native_sha256"];
        if let (Some(observed), false, true) = (observed, later_load, exact) {
            if let Some(problem) = linux_vst_bridge::problem::from_activation_record(
                activation["stage"].as_str().unwrap_or(""), activation["reason"].as_str().unwrap_or(""),
                activation["requested"].as_u64().unwrap_or(0)) {
                product.details["last_load"] = json!({"outcome":"activation_refused",
                    "observed_at":observed,"problem":problem.value()});
                return;
            }
        }
    }
    let Some(record) = startup else { return };
    let Some(outcome) = record["outcome"].as_str() else { return };
    if !record["observed_at"].is_u64() { return; }
    // Only a record made against this exact publication explains it; one made
    // against another (an update, reinstall or rollback since), or never tied
    // to one, says nothing about this plug-in now.
    let binding = &record["binding"];
    if !binding.is_object() || binding["environment"] != product.environment.as_str()
        || binding["environment_revision"] != product.details["environment_revision"]
        || binding["native_sha256"] != product.details["native_sha256"] { return; }
    let phase = record["phases"].as_array().and_then(|phases| phases.last())
        .and_then(|phase| phase[0].as_str());
    let Some(problem) = linux_vst_bridge::problem::from_load_record(outcome, phase,
        record["refusal"].as_str()) else { return };
    product.details["last_load"] = json!({"outcome":outcome,
        "observed_at":record["observed_at"],"problem":problem.value()});
}

fn append_discovered(m: &Manager, sw: &Software, environments: &[Environment],
    bindings: &[linux_vst_bridge::catalogue::EnvironmentBinding], db: &Registry,
    products: &mut Vec<ui::Product>, busy: Option<&str>) -> Result<()> {
    for environment in environments {
        let path = m.root.join("inventory").join(format!("{}.json", environment.id));
        if !path.try_exists()? { continue; }
        let scan: inventory::Scan = read_json(&path)?;
        require(scan.schema == 1 && scan.environment.id == environment.id,
            "operator_inventory_environment_changed")?;
        for (index, module) in scan.modules.iter().cloned().enumerate() {
            let stale = inventory::record_stale_reason(&module, &scan.environment, &scan.host,
                &scan.host_source_sha256, environment, &sw.host, &sw.source_sha256);
            if module.quarantine_reason.is_some() {
                let retry_disabled = quarantined_retry_disabled(m, bindings, db,
                    environment)?;
                products.push(quarantined_product(&scan,index,module,stale,busy,
                    retry_disabled)?);
                continue;
            }
            let problem = if stale.is_none() { inventory::problem(&module) } else { None };
            for class in module.classes {
                if class.category != "Audio Module Class" || products.iter().any(|p|
                    p.class_id == class.id && p.module_sha256 == module.artifact.sha256
                        && p.environment == scan.environment.id) { continue; }
                products.push(ui::Product { class_id:class.id,name:class.name,vendor:class.vendor,
                    role:class.role,version:class.version,
                    disposition:if stale.is_none() {"installed_unqualified"} else {"needs_attention"}.into(),
                    active_revision:None,recommended_revision:None,environment:scan.environment.id.clone(),
                    runner:scan.environment.runner.id.clone(),module_sha256:module.artifact.sha256.clone(),
                    limitations:vec![stale.unwrap_or("Installed but not published").into()],
                    history:vec![],actions:vec![],compatibility:None,
                    availability:discovered_availability(problem.as_ref()),
                    details:json!({"scan":scan.id,"current":stale.is_none(),"observed_at":scan.completed_at,
                        "problem":problem}) });
            }
        }
    }
    Ok(())
}

pub(super) fn capture(m: &Manager) -> Result<CurrentOverviewContext> {
    timing::measure(Stage::CurrentCapture, || {
        linux_vst_bridge::with_readback_digests(|| capture_readonly(m))
    })
}
fn capture_readonly(m: &Manager) -> Result<CurrentOverviewContext> {
    capture_readonly_with(m,onboarding::live)
}
#[cfg(test)]
pub(super) fn capture_with_installer_live(m: &Manager, is_live: impl FnMut(&str)->Result<bool>)
    -> Result<CurrentOverviewContext> {
    capture_readonly_with(m,is_live)
}
fn capture_readonly_with(m: &Manager, mut installer_is_live: impl FnMut(&str)->Result<bool>)
    -> Result<CurrentOverviewContext> {
    let started = Instant::now();
    let mut phases = Vec::new();
    // Status is an observation, not a serialized user operation. Capture the
    // registry/owners together, then perform expensive and external probes
    // without action ownership. The caller rechecks exact tokens, file stamps,
    // owners and external state before exposing any offered action.
    let _registry_guard = timing::measure(Stage::CaptureRegistry, ||
        acquire_readback(m, ui::OperatorLock::Registry, None, OPERATOR_WAIT, &mut vec![]))?;
    let owners = capacity::owners(m)?;
    // The composition's registry read: the page and its token both come
    // from it, and every later composed read is this one.
    let db = (*m.composed_registry()?).clone();
    drop(_registry_guard);
    let current_generation = pulse_generation(m)?;
    std::thread::scope(|scope| {
    let capacity_task = linux_vst_bridge::spawn_recorded(scope, || {
        let started = Instant::now();
        (live_capacity(m).ok(), started.elapsed().as_millis())
    });
    let cleanup_task = linux_vst_bridge::spawn_recorded(scope, || pulse_cleanup(m));
    let workspace_task = linux_vst_bridge::spawn_recorded(scope, || daw_workspace::current_projection(m));
    let before = token_with_registry(m, &db)?;
    let token_at = Instant::now(); phases.push(("token_before",token_at.duration_since(started).as_millis()));
    let authority_observed = timing::span(Stage::CaptureSoftwareCatalogue);
    let sw = software_record(m)?;
    let preparation = preparation::RecordReadback::capture_watched(m)?;
    let mut watched = watch_paths(m, &sw, &db)?;
    let catalogue = operator_catalogue_records(m, &sw, &db, &preparation)?;
    for path in preparation.watched_paths()? { watched.insert(path.clone(), stamp(&path)?); }
    require(watched.len() <= 4096, "operator_current_watch_bound")?;
    let profiles = profiles::installed_profiles()?;
    authority_observed.end(true);
    let authority_at = Instant::now(); phases.push(("software_catalogue_profiles",authority_at.duration_since(token_at).as_millis()));
    let (mut products, revisions) = timing::measure(Stage::CaptureProducts, ||
        current_products(m, &db, &sw, &profiles))?;
    let product_at = Instant::now(); phases.push(("current_products",product_at.duration_since(authority_at).as_millis()));
    let pending = pending_transactions(m)?;
    let vendor = vendor_retired(m)?;
    let vendor_progress = live_vendor_progress(m)?;
    let records = onboarding::history_records(m)?;
    let managed_environments: BTreeSet<_> = db.classes.values()
        .map(|entry| entry.registration.environment.id.as_str()).collect();
    let installers_retired = records.iter()
        .filter(|record| !managed_environments.contains(record.environment.id.as_str()))
        .try_fold(true, |retired, record| -> Result<bool> {
            if !retired { return Ok(false); }
            Ok(record.installation_operation.is_none()
                || onboarding::retired(&onboarding::result(m, record)?))
        })?;
    let retired = vendor && installers_retired && environment_install::all_retired(m)? && uninstall::all_retired(m)?;
    let owner_at = Instant::now(); phases.push(("current_owners",owner_at.duration_since(product_at).as_millis()));
    let installers = installer_import::list_identity_records(m)?;
    let record_at = Instant::now(); phases.push(("setup_records",record_at.duration_since(owner_at).as_millis()));
    let managed_bindings = managed_environment_bindings(m, catalogue.as_ref(), &db)?;
    let inventory_environments = discovery_environments(&managed_bindings,&records)?;
    for environment in &inventory_environments {
        for path in [m.root.join("inventory").join(format!("{}.json",environment.id)),
            environment.root.join("environment.json")] {
            // Preserve any earlier observation. Capture each exact inventory
            // before reading it; directory stamps do not detect in-place writes.
            if let std::collections::btree_map::Entry::Vacant(entry)=watched.entry(path) {
                let before=stamp(entry.key())?;
                entry.insert(before);
            }
        }
    }
    require(watched.len() <= 4096,"operator_current_watch_bound")?;
    append_discovered(m, &sw, &inventory_environments, &managed_bindings, &db, &mut products, None)?;
    // This readback has no compatibility workflow. A retained candidate for
    // the exact plug-in file means a later check passed, which supersedes
    // the discovery-time explanation; the product detail applies the full rule.
    for product in products.iter_mut() {
        let Ok(found) = serde_json::from_value::<linux_vst_bridge::problem::Explanation>(
            product.details["problem"].clone()) else { continue };
        if preparation.candidates.iter().any(|c| c.selection.class.id == product.class_id
            && c.selection.module.sha256 == product.module_sha256
            && c.selection.environment.id == product.environment) {
            retire_discovered_problem(product, &found);
        }
    }
    let discovery_at = Instant::now(); phases.push(("inventory_discovery",discovery_at.duration_since(record_at).as_millis()));
    let mut installer_live = BTreeMap::new();
    let delivered = linux_vst_bridge::runtime_delivery::recommended_record_state(m)?;
    let default = if let Some(state)=&delivered {
        if let Some(runner)=state.runner.as_ref().filter(|_|state.failure.is_none()) {
            Some((catalogue::runner_key(runner)?,runner.clone()))
        } else {None}
    } else {None};
    let mut onboarding = onboarding::projection_current(m, None,
        onboarding::CurrentProjectionInputs {sw:&sw,default_runner:default.as_ref(),
            registry:&db,records:&records,installers:&installers}, |operation| {
                let live=installer_is_live(operation)?;
                installer_live.insert(operation.to_owned(),live);
                Ok(live)
            })?;
    let setup_at = Instant::now(); phases.push(("current_setup",setup_at.duration_since(discovery_at).as_millis()));
    let (cap, capacity_ms) = capacity_task.join().unwrap_or((None, 0));
    let joined_at = Instant::now(); phases.push(("capacity_wait",joined_at.duration_since(setup_at).as_millis()));
    let cap = cap.filter(|read| read.owners == owners);
    // Historical selected services predate LVP1. Their staged readback still
    // uses the full LVC1 capacity result; an installed paired service also
    // supplies the cheap blocked-state recheck.
    let cleanup_seen = cleanup_task.join()
        .map_err(|_| "operator_cleanup_probe_failed")?;
    if let (Some(cap),Some(signal)) = (cap.as_ref(),cleanup_seen) {
        require(cap.cleanup_unconfirmed == signal,"operator_cleanup_state_changed_refresh")?;
    }
    phases.push(("capacity_parallel", capacity_ms));
    let busy = inactive_reason(cap.as_ref(), retired, pending, false);
    if let Some(reason) = busy {
        for offer in products.iter_mut().flat_map(|product| &mut product.actions)
            .chain(onboarding.iter_mut().flat_map(|row| &mut row.actions)) {
            if !matches!(offer.action, ui::Action::InstallerFocus {..} | ui::Action::InstallerStop {..}) {
                offer.disabled_reason = Some(reason.into());
            }
        }
    }
    let (workspaces, workspace_installers) = workspace_task.join()
        .map_err(|_| "operator_workspace_probe_failed")??;
    onboarding.retain(|row| !workspace_installers.contains(&row.installer));
    preparation_cli::project_required_recovery(m,&mut products,&preparation,&|class|
        class_inactive_reason(&system_from_capacity(cap.as_ref(),pending,0),&owners,retired,class))?;
    let mut installer_setups = onboarding::setup_projection_current(m, &onboarding, &products,
        &workspace_installers, &records, &installers, default.as_ref())?;
    automatic_install::project_from(m, &mut installer_setups, busy, &records)?;
    automatic_install::project_revalidation(m,&mut products,&records)?;
    for product in &mut products {
        if product.availability.is_none() {
            product.availability=Some(ui::NativeAvailability {available:false,
                reason:Some(NO_NATIVE_CONNECTION.into())});
        }
    }
    let environment_installers = environment_install::projection(m,&products,busy,|operation| {
        let live = installer_is_live(operation)?;
        installer_live.insert(operation.to_owned(),live);Ok(live)
    })?;
    let mut vendor_applications = vendor_projections(m,busy)?;
    let environments=environment_projection_from(m,&sw,&managed_bindings,&db,busy)?;
    environment_refresh::project(m,&mut installer_setups,&environments,&products,busy)?;
    plugin_setup::project(m,&mut products,&mut installer_setups,busy)?;
    let deleted_setups = {
        let system = system_from_capacity(cap.as_ref(),pending,0);
        setup_removal::project(m,&mut installer_setups,&records,&db,&managed_bindings,
            &|class| class_inactive_reason(&system,&owners,retired,class), &mut |operation| {
                if let Some(live) = installer_live.get(operation) { return Ok(*live); }
                let live = installer_is_live(operation)?;
                installer_live.insert(operation.to_owned(),live);Ok(live)
            })?
    };
    if let Some(app)=renderer_projection(m,busy,vendor.then_some(true),Some(&mut installer_setups))? {
        vendor_applications.push(app);
    }
    automatic_install::settle_busy_offers(&mut installer_setups);
    let workspace_at = Instant::now(); phases.push(("workspace_and_cards",workspace_at.duration_since(joined_at).as_millis()));
    let uninstallers=uninstall::project(m,&installer_setups,&records,&db,&managed_bindings,busy)?;
    phases.push(("uninstallers",workspace_at.elapsed().as_millis()));
    #[cfg(feature = "pb0-c0-audit")]
    eprintln!("PB0_PHASE {}", serde_json::to_string(&phases)?);
    #[cfg(not(feature = "pb0-c0-audit"))]
    let _ = phases;
    let system = system_from_capacity(cap.as_ref(),pending,stale_transports(cap.as_ref())?);
    let runtime_reason=linux_vst_bridge::runtime_delivery::installation_disabled_reason(m)?;
    let operation=automatic_install::project_current_operation(m,automatic_install::current_operation(m)?,
        &installer_setups,&records)?;
    let snapshot = ui::Snapshot {schema:ui::OPERATOR_SCHEMA,state_token:before,system,
        managed_runtime_records:linux_vst_bridge::runtime_delivery::readback_records(m)?,
        onboarding,installer_setups,environment_installers,environments,vendor_applications,products,
        workspaces,active_sessions:vec![],capture:Value::Null,recent_incidents:vec![],
        actions:vec![action(&linux_vst_bridge::runtime_delivery::installation_label(),ui::Action::RuntimeInstall {},
            runtime_reason.as_deref().or(busy)),
            action("Send a problem report",ui::Action::SupportExport {},None),
            action("Reconcile interrupted transaction",ui::Action::TransactionReconcile {},
                inactive_reason(cap.as_ref(),retired,pending,true))],
        operation,deleted_setups,uninstallers,
        bridge:crate::package_authority::bridge_build(m,&sw)};
    Ok(CurrentOverviewContext {snapshot,busy,profiles,revisions,owners,watched,current_generation,
        software:sw,registry:db,preparation,bindings:managed_bindings,
        installer_live,vendor_progress,vendor_retired:vendor,all_vendor_retired:retired,cleanup_seen,
        #[cfg(feature = "pb0-c0-audit")]
        captured_at:started})
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::operator_cli::tests::{capacity_json, overview_service_reply, overview_service_reply_with_counts};
    fn selected_vendor_fixture() -> (test_fixture::Fixture, vendor_application::Application) {
        let (f, _) = preparation_cli::tests::projection_fixture();
        let app=selected_vendor_application(&f);
        (f,app)
    }
    pub(crate) fn selected_vendor_application(f: &test_fixture::Fixture) -> vendor_application::Application {
        let environment = f.r.environment.clone();
        let executable = environment.root.join("compatdata/pfx/drive_c/Hub.exe");
        fs::write(&executable, b"MZ owned source hub").unwrap();
        let installers = f.m.root.join("installers");
        private_dir(&installers).unwrap();
        let bytes = b"MZ owned source installer";
        let installer = hex(&sha2::Sha256::digest(bytes));
        let path = installers.join(format!("{installer}.exe"));
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        atomic_json(&installers.join(format!("{installer}.json")), &installer_import::Installer {
            schema:1,id:installer.clone(),artifact:Artifact {path,sha256:installer.clone()},
            byte_size:bytes.len() as u64,format:"pe_executable".into(),name_hint:"Source hub".into(),
            source_class:"operator_selected_file".into(),import_result:"imported".into(),
            created_at:1,vendor:None,product:None,
        }).unwrap();
        let operation = random_id().unwrap();
        let directory = onboarding::directory(&f.m,&environment.id).unwrap();
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("record.json"),&onboarding::Record {
            schema:1,id:environment.id.clone(),installer:installer.clone(),
            environment:environment.clone(),created_at:1,creation_operation:operation.clone(),
            installation_operation:Some(operation.clone()),published:false,automatic:None,
            previous_attempt:None,
        }).unwrap();
        let witness = directory.join(format!("{operation}-result.json"));
        atomic_json(&witness,&json!({"schema":2,"operation":operation,"state":"completed",
            "raw_exit":0,"cleanup_confirmed":true,"owned_live":0,"error":null})).unwrap();
        let app = vendor_application::Application {schema:2,id:vendor_application::ApplicationId::Selected,
            environment,executable:Artifact {sha256:digest(&executable).unwrap(),path:executable},
            helpers:vec![],installer_sha256:Some(installer),
            installer_result:Some(Artifact {sha256:digest(&witness).unwrap(),path:witness}),
            observed_installer_version:String::new(),user_selection:None,
        };
        app.verify(&f.m.root).unwrap();
        let directory = vendor_cli::directory(&f.m,&app.selector().unwrap()).unwrap();
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("application.json"),&app).unwrap();
        app
    }
    #[test]
    fn registered_vendor_application_reaches_fast_setup_and_normal_action_authority() {
        let (f,app) = selected_vendor_fixture();
        let record:onboarding::Record=read_json(&app.installer_result.as_ref().unwrap().path
            .parent().unwrap().join("record.json")).unwrap();
        let units=[vendor_cli::unit(&app.selector().unwrap()).unwrap(),
            onboarding::unit(record.installation_operation.as_deref().unwrap()).unwrap()];
        readiness::with_unit_state_fixture(&units,"inactive",|| {
            let watched=[f.m.root.join("registry.json"),f.m.root.join("software.json"),
                vendor_cli::directory(&f.m,&app.selector().unwrap()).unwrap().join("application.json"),
                app.environment.root.join("environment.json"),
                app.installer_result.as_ref().unwrap().path.parent().unwrap().join("record.json"),
                app.installer_result.as_ref().unwrap().path.clone(),app.executable.path.clone(),
                f.m.root.join("installers").join(format!("{}.json",app.installer_sha256.as_ref().unwrap())),
                f.m.root.join("installers").join(format!("{}.exe",app.installer_sha256.as_ref().unwrap()))];
            let before:Vec<_>=watched.iter().map(|path|fs::read(path).unwrap()).collect();
            let server = overview_service_reply(&f.m,capacity_json(false,0,0));
            let current = current_overview(&f.m).unwrap();
            server.join().unwrap();
            assert_eq!(current.current.vendor_applications.len(),1,
                "Normal Setup must receive the registered application's current card");
            let application = &current.current.vendor_applications[0];
            assert_eq!(application.id,app.selector().unwrap());
            assert_eq!(application.name,"Hub");
            assert_eq!(application.state,"closed");
            let offer = application.actions.iter().find(|offer|
                matches!(offer.action,ui::Action::VendorSoftwareInstall {..})).unwrap();
            assert!(offer.disabled_reason.is_none());
            let request = ui::Request {schema:ui::OPERATOR_SCHEMA,
                state_token:current.current.state_token.clone(),
                action:ui::Action::VendorSoftwareInstall {application:application.id.clone(),
                    daw:ui::DawChoice::Bitwig}};
            let server = overview_service_reply(&f.m,capacity_json(false,0,0));
            validate_current_request(&f.m,&request).unwrap();
            server.join().unwrap();
            assert!(watched.iter().zip(&before).all(|(path,bytes)|fs::read(path).unwrap()==*bytes),
                "Current cards and action admission must not launch or alter installation records");
            for (index,changed) in [ui::Request {state_token:"stale".into(),..request.clone()},
                ui::Request {action:ui::Action::VendorSoftwareInstall {
                    application:"de".repeat(32),daw:ui::DawChoice::Bitwig},..request.clone()}].into_iter().enumerate() {
                // An unknown exact app now refuses before external final rechecks.
                let counts=if index == 1 {(1,1)} else {(1,3)};
                let server = overview_service_reply_with_counts(&f.m,capacity_json(false,0,0),counts);
                let refusal=validate_current_request(&f.m,&changed).unwrap_err().to_string();
                assert_eq!(refusal,if index == 1 {"vendor_application_choice_unavailable"}
                    else {"operator_stale_request_refresh"});
                server.join().unwrap();
            }
            fs::write(&app.executable.path,b"changed source hub").unwrap();
            let server = overview_service_reply(&f.m,capacity_json(false,0,0));
            let current = current_overview(&f.m).unwrap();
            server.join().unwrap();
            assert!(current.current.vendor_applications[0].actions[0].disabled_reason.is_none(),
                "Readback observes records; the execution owner verifies payload bytes");
            let server = overview_service_reply(&f.m,capacity_json(false,0,0));
            validate_current_request(&f.m,&request).unwrap();
            server.join().unwrap();
            let owner=random_id().unwrap();
            let job=job_dir(&f.m,&owner).unwrap();private_dir(&job).unwrap();
            let changed_before:Vec<_>=watched.iter().map(|path|fs::read(path).unwrap()).collect();
            let refusal=execute_with_receipt_policy(&f.m,&request.action,Some(&owner),
                &idle_capacity_test,OPERATOR_WAIT,&mut vec![]).unwrap_err().to_string();
            assert_eq!(refusal,"artifact missing or changed");
            assert!(watched.iter().zip(&changed_before).all(|(path,bytes)|fs::read(path).unwrap()==*bytes));
            assert!(!f.m.root.join("operator/resume.json").exists());
            assert!(!job.join("vendor-maintenance.json").exists());
            assert!(!job.join("automatic.json").exists());
            fs::write(&app.executable.path,&before[6]).unwrap();
            let mut receipt:Value=read_json(&app.installer_result.as_ref().unwrap().path).unwrap();
            receipt["raw_exit"]=json!(1);
            atomic_json(&app.installer_result.as_ref().unwrap().path,&receipt).unwrap();
            let server = overview_service_reply(&f.m,capacity_json(false,0,0));
            let current = current_overview(&f.m).unwrap();
            server.join().unwrap();
            assert_eq!(current.current.vendor_applications[0].actions[0].disabled_reason.as_deref(),
                Some("The selected application or installation changed"),"Changed receipt bindings are observed");
            let request=ui::Request {state_token:current.current.state_token, ..request};
            let server = overview_service_reply(&f.m,capacity_json(false,0,0));
            assert_eq!(validate_current_request(&f.m,&request).unwrap_err().to_string(),
                "The selected application or installation changed");
            server.join().unwrap();
        });
    }
    #[test]
    fn starting_vendor_keeps_current_status_and_exact_controls_readable() {
        let (f,app)=selected_vendor_fixture();
        let mut other=app.environment.clone();other.id=random_id().unwrap();
        other.root=f.m.root.join("environments").join(&other.id);
        let lease=vendor_cli::selection::tests::dsp_lease(&f,Some(&other));
        let selector=app.selector().unwrap();
        let directory=vendor_cli::directory(&f.m,&selector).unwrap();
        let operation=random_id().unwrap();
        let report=directory.join("operation-result.json");
        atomic_json(&directory.join("operation.json"),&vendor_application::Operation {
            application:app.clone(),report:report.clone(),mode:vendor_application::LaunchMode::Normal,
            operation_id:operation.clone(),launcher:None,managed:None,capability_notice:None}).unwrap();
        atomic_json(&report,&json!({"schema":3,"operation_id":operation,"state":"unknown",
            "launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        let record=onboarding::retained_environment_record(&f.m,&app.environment.id).unwrap().unwrap();
        let installer=onboarding::unit(record.installation_operation.as_deref().unwrap()).unwrap();
        readiness::with_unit_state_fixture(&[installer],"inactive",||
            readiness::with_unit_state_fixture(&[vendor_cli::unit(&selector).unwrap()],"active",|| {
                for (state,count,exit,label) in [("unknown",None,None,"starting"),
                    ("running",Some(3),None,"running"),("unknown",Some(3),Some(0),"running"),
                    ("unknown",Some(3),Some(42),"running")] {
                    atomic_json(&report,&json!({"schema":3,"operation_id":operation,"state":state,
                        "launcher_exit":exit,"owned_live":count,"cleanup_confirmed":false,
                        "discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
                    let current=current_overview(&f.m).unwrap();
                    let row=current.current.vendor_applications.iter().find(|row|row.id==selector).unwrap();
                    assert_eq!(row.state,label);
                    for action in [ui::Action::VendorApplicationFocus {application:selector.clone()},
                        ui::Action::VendorApplicationStop {application:selector.clone()}] {
                        assert!(row.actions.iter().any(|offer|offer.action==action
                            && offer.disabled_reason.is_none()));
                        validate_current_request(&f.m,&ui::Request {schema:ui::OPERATOR_SCHEMA,
                            state_token:current.current.state_token.clone(),action}).unwrap();
                    }
                    assert!(lease.exists());
                }
                let original:Value=read_json(&report).unwrap();
                for (field,value) in [("operation_id",json!(random_id().unwrap())),("schema",json!(4)),
                    ("owned_live",json!(0)),("state",json!("completed")),
                    ("cleanup_confirmed",json!(true)),("unexpected",json!(true))] {
                    let mut changed=original.clone();changed[field]=value;
                    atomic_json(&report,&changed).unwrap();
                    assert!(current_overview(&f.m).is_err(),"accepted {field}");
                }
                let mut contradictory=original;
                contradictory["owned_live"]=Value::Null;
                contradictory["launcher_exit"]=0.into();
                atomic_json(&report,&contradictory).unwrap();
                assert!(current_overview(&f.m).is_err(),"an exited launcher with no observed cohort is not starting");
            }));
    }
    #[test]
    fn vendor_checkpoint_change_invalidates_captured_watch_and_action_token() {
        let (f,app)=selected_vendor_fixture();let selector=app.selector().unwrap();
        let owner=random_id().unwrap();let dir=job_dir(&f.m,&owner).unwrap();private_dir(&dir).unwrap();
        let selected=control_artifact(vendor_cli::directory(&f.m,&selector).unwrap().join("application.json")).unwrap();
        let vendor_operation=random_id().unwrap();
        atomic_json(&dir.join("request.json"),&ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:"test".into(),
            action:ui::Action::VendorSoftwareInstall {application:selector.clone(),daw:ui::DawChoice::Bitwig}}).unwrap();
        let mut after=app.environment.clone();after.revision+=1;
        atomic_json(&dir.join("vendor-maintenance.json"),&environment_revision::VendorBinding {
            schema:1,operation:owner.clone(),application:selector.clone(),selected:selected.clone(),
            vendor_operation:vendor_operation.clone(),before:app.environment.clone(),environment:after.clone()}).unwrap();
        atomic_json(&after.root.join("environment.json"),&after).unwrap();
        let mut intent=automatic_install::Intent {installer_policy:None,owner:owner.clone(),active_operation:owner,
            vendor:Some(automatic_install::VendorWork {application:selector.clone(),selected,operation:vendor_operation,result:None}),
            journey:ui::InstallationJourney {daw:ui::DawChoice::Bitwig,phase:ui::InstallationPhase::NeedsAttention,
                plugins:vec![],failure:Some(ui::InstallationFailure {stage:ui::InstallationFailureStage::Preparation,
                    code:"interrupted".into(),reason:"Interrupted connection".into(),retryable:true})},
            scan:None,prior_selections:vec![],prior_modules:vec![],failed_modules:vec![],work:vec![],predecessors:vec![],previous:None};
        let checkpoint=dir.join("automatic.json");atomic_json(&checkpoint,&intent).unwrap();
        readiness::with_unit_state_fixture(&[vendor_cli::unit(&selector).unwrap()],"inactive",|| {
        let before=token(&f.m).unwrap();let sw=software_record(&f.m).unwrap();
        let watched=watch_paths(&f.m,&sw,&f.m.registry().unwrap()).unwrap();
        assert!(watched.contains_key(&checkpoint));
        let context=context_for_watched(&f.m,watched);
        let request=ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:before.clone(),
            action:ui::Action::VendorSoftwareInstall {application:selector,daw:ui::DawChoice::Bitwig}};
        intent.scan=Some(random_id().unwrap());intent.prior_modules.push("de".repeat(32));
        atomic_json(&checkpoint,&intent).unwrap();
        assert_ne!(token(&f.m).unwrap(),before);
        assert_eq!(context.recheck_with(&f.m,||Ok(())).unwrap_err().to_string(),"operator_state_changed_refresh");
        let mut offered=context.snapshot.clone();offered.state_token=token(&f.m).unwrap();
        assert_eq!(validate(&request,&offered).unwrap_err().to_string(),"operator_stale_request_refresh");
        });
    }
    fn managed_discovery_fixture() -> (test_fixture::Fixture, preparation::Candidate, ui::Action) {
        let (f, candidate, _, check) = preparation_cli::tests::guided_fixture_with_role(false);
        assert!(onboarding::history_records(&f.m).unwrap().is_empty());
        let mut sibling = f.r.clone();
        sibling.metadata.class_id = "03".repeat(16);
        sibling.metadata.name = "Registered sibling".into();
        sibling.module.path = f.r.module.path.with_file_name("sibling.vst3");
        fs::write(&sibling.module.path, b"retained sibling module").unwrap();
        sibling.module.sha256 = digest(&sibling.module.path).unwrap();
        f.m.register(sibling.clone()).unwrap();
        let inventory = f.m.root.join("inventory").join(format!("{}.json",f.r.environment.id));
        let mut scan: inventory::Scan = read_json(&inventory).unwrap();
        let mut existing = scan.modules[0].clone();
        existing.artifact = sibling.module;
        existing.classes.retain(|class| class.category == "Audio Module Class");
        existing.classes[0].id = sibling.metadata.class_id;
        existing.classes[0].name = sibling.metadata.name;
        scan.modules.push(existing);
        atomic_json(&inventory,&scan).unwrap();
        (f,candidate,check)
    }
    #[test]
    fn managed_inventory_without_onboarding_reaches_current_product_and_preparation() {
        let (f,candidate,check) = managed_discovery_fixture();
        let registry = fs::read(f.m.root.join("registry.json")).unwrap();
        let server = overview_service_reply(&f.m,capacity_json(false,0,0));
        let overview = overview(&f.m).unwrap();
        server.join().unwrap();
        let selection = &candidate.selection;
        let matching: Vec<_> = overview.current.products.iter().filter(|product|
            product.class_id == selection.class.id && product.module_sha256 == selection.module.sha256
                && product.environment == selection.environment.id).collect();
        assert_eq!(matching.len(),1,"current library must include the healthy managed inventory class");
        assert_eq!(matching[0].role,"instrument");
        assert_eq!(matching[0].disposition,"installed_unqualified");
        assert_eq!(overview.current.products.len(),2,
            "registered sibling is not duplicated and metadata-only factory class is not a product");
        let server = overview_service_reply(&f.m,capacity_json(false,0,0));
        let detail = product_detail_readonly(&f.m,&selection.environment.id,
            &selection.module.sha256,&selection.class.id).unwrap();
        server.join().unwrap();
        let primary = detail.product.compatibility.as_ref().unwrap().primary.as_ref().unwrap();
        assert_eq!(primary.action,check);
        assert!(primary.disabled_reason.is_none());
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(),registry);
    }
    #[test]
    fn managed_inventory_environment_union_requires_exact_shared_identity() {
        let f=test_fixture::Fixture::new();
        let environment=f.r.environment.clone();
        let binding=catalogue::EnvironmentBinding {family:profiles::Family::ManagedInstallerV1,
            environment:environment.clone()};
        let record=onboarding::Record {schema:1,id:environment.id.clone(),installer:"ab".repeat(32),
            environment:environment.clone(),created_at:1,creation_operation:"cd".repeat(16),
            installation_operation:None,published:false,automatic:None,previous_attempt:None};
        assert_eq!(discovery_environments(&[binding.clone(),binding.clone()],
            &[record.clone(),record.clone()]).unwrap(),vec![environment]);
        for mismatch in ["revision","root","runner"] {
            let mut changed=record.clone();
            match mismatch {
                "revision"=>changed.environment.revision+=1,
                "root"=>changed.environment.root=f.outer.join("another-space"),
                "runner"=>changed.environment.runner.id.push_str("-changed"),
                _=>unreachable!(),
            }
            assert_eq!(discovery_environments(std::slice::from_ref(&binding),&[changed])
                .unwrap_err().to_string(),"operator_managed_environment_binding_conflict");
        }
    }
    #[test]
    fn managed_inventory_capture_refuses_changed_or_new_scan() {
        let (f,_,_)=managed_discovery_fixture();
        let path=f.m.root.join("inventory").join(format!("{}.json",f.r.environment.id));
        let original=fs::read(&path).unwrap();
        for mutation in ["in_place","replacement","removed","created"] {
            fs::write(&path,&original).unwrap();
            if mutation=="created" {fs::remove_file(&path).unwrap();}
            let server=overview_service_reply(&f.m,capacity_json(false,0,0));
            let captured=capture(&f.m).unwrap();
            captured.recheck(&f.m).unwrap();
            server.join().unwrap();
            assert!(captured.watched.contains_key(&path));
            let mut changed:inventory::Scan=serde_json::from_slice(&original).unwrap();
            changed.completed_at+=1;
            match mutation {
                "in_place"=>{
                    let parent=stamp(path.parent().unwrap()).unwrap();
                    fs::write(&path,serde_json::to_vec(&changed).unwrap()).unwrap();
                    assert_eq!(stamp(path.parent().unwrap()).unwrap(),parent,
                        "directory-only watching cannot observe this change");
                }
                "replacement"=>atomic_json(&path,&changed).unwrap(),
                "removed"=>fs::remove_file(&path).unwrap(),
                "created"=>fs::write(&path,&original).unwrap(),
                _=>unreachable!(),
            }
            let refusal=captured.recheck_with(&f.m,||Ok(())).unwrap_err().to_string();
            if mutation=="in_place" {
                assert_eq!(refusal,"operator_current_artifact_changed_refresh");
            } else {
                assert!(matches!(refusal.as_str(),"operator_state_changed_refresh"
                    | "operator_current_artifact_changed_refresh"),"{mutation}: {refusal}");
            }
        }
    }
    #[test]
    fn managed_inventory_projection_retains_stale_and_quarantine_posture() {
        let (f,candidate,_)=managed_discovery_fixture();
        let path=f.m.root.join("inventory").join(format!("{}.json",f.r.environment.id));
        let original:inventory::Scan=read_json(&path).unwrap();
        for posture in ["stale","quarantined"] {
            let mut scan=original.clone();
            if posture=="stale" {scan.host_source_sha256="ef".repeat(32);}
            else {scan.modules[0].quarantine_reason=Some("inventory_factory_absent".into());}
            atomic_json(&path,&scan).unwrap();
            let server=overview_service_reply(&f.m,capacity_json(false,0,0));
            let current=overview(&f.m).unwrap();
            server.join().unwrap();
            let product=current.current.products.iter().find(|product|
                product.module_sha256==candidate.selection.module.sha256).unwrap();
            assert_eq!(product.disposition,if posture=="stale" {"needs_attention"} else {"quarantined"});
            if posture=="quarantined" {
                assert!(product.class_id.is_empty());
                assert!(matches!(product.actions[0].action,ui::Action::QuarantinedModuleRetry {..}));
            } else {
                assert_eq!(product.class_id,candidate.selection.class.id);
                assert!(product.limitations.iter().any(|reason|reason.contains("Scanner binding changed")));
            }
            let server=overview_service_reply(&f.m,capacity_json(false,0,0));
            let detail=product_detail_readonly(&f.m,&product.environment,
                &product.module_sha256,&product.class_id).unwrap();
            server.join().unwrap();
            assert!(!detail.product.actions.iter().chain(detail.product.compatibility.iter()
                .flat_map(|workflow|workflow.primary.iter().chain(workflow.alternatives.iter())))
                .any(|offer|matches!(offer.action,ui::Action::CompatibilityCheck {..})
                    && offer.disabled_reason.is_none()));
        }
    }
    #[test]
    fn status_readback_does_not_wait_for_user_action_serialization() {
        let fixture = test_fixture::Fixture::new();
        let m = &fixture.m;
        let source = fixture.r.host.path.with_file_name("host-source-manifest.json");
        fs::write(&source, b"fixture source").unwrap();
        let source_sha256 = digest(&source).unwrap();
        let sw = Software {
            installer_launch: None, preparation_kit: None, operator_frontend: None,
            manager: fixture.r.host.clone(), supervisor: fixture.r.host.clone(),
            ownership: fixture.r.host.clone(), host: fixture.r.host.clone(),
            source_manifest: Artifact { path: source.clone(), sha256: source_sha256.clone() },
            source_sha256, native_catalogue: None,
        };
        atomic_json(&m.root.join("software.json"), &sw).unwrap();
        // A user operation owns serialization while status captures and
        // rechecks its independent registry and artifact observations.
        let _action = canonical_lock(m).unwrap();
        let captured = capture(m).unwrap();
        captured.recheck(m).unwrap();
        fs::write(&source, b"changed source").unwrap();
        assert!(captured.recheck(m).is_err());
        // A new observation may still display the retained software identity;
        // the executable owner separately refuses changed source bytes.
        capture(m).unwrap();
        assert!(software(m).is_err());
    }
    #[test]
    fn onboarding_only_quarantine_cannot_offer_managed_retry() {
        let fixture=test_fixture::Fixture::new();
        let m=&fixture.m;
        let env=fixture.r.environment.clone();
        let sw=Software {installer_launch:None,preparation_kit:None,
            operator_frontend:None,manager:fixture.r.host.clone(),
            supervisor:fixture.r.host.clone(),ownership:fixture.r.host.clone(),
            host:fixture.r.host.clone(),source_manifest:Artifact {
                path:fixture.r.host.path.with_file_name("host-source-manifest.json"),
                sha256:fixture.r.host_source_sha256.clone()},
            source_sha256:fixture.r.host_source_sha256.clone(),native_catalogue:None};
        let scan=inventory::Scan {schema:1,id:"aa".repeat(16),environment:env.clone(),
            host:sw.host.clone(),host_source_sha256:sw.source_sha256.clone(),
            completed_at:1,changes:inventory::Changes::default(),
            modules:vec![inventory::Module {artifact:fixture.r.module.clone(),
                classes:vec![],report:fixture.r.host.clone(),inspection_error:None,
                quarantine_reason:Some("inventory_factory_absent".into())}]};
        let inventory=m.root.join("inventory");
        private_dir(&inventory).unwrap();
        atomic_json(&inventory.join(format!("{}.json",env.id)),&scan).unwrap();
        let record=onboarding::Record {schema:1,id:env.id.clone(),
            installer:"bb".repeat(16),environment:env,created_at:1,
            creation_operation:"cc".repeat(16),installation_operation:None,
            published:false,automatic:None,previous_attempt:None};
        let db=m.registry().unwrap();
        let bindings=managed_environment_bindings(m,None,&db).unwrap();
        assert!(bindings.is_empty());
        let mut products=vec![];
        let environments=discovery_environments(&bindings,&[record]).unwrap();
        append_discovered(m,&sw,&environments,&bindings,&db,&mut products,None).unwrap();
        assert_eq!(products.len(),1);
        assert_eq!(products[0].actions[0].disabled_reason.as_deref(),
            Some("No completed setup is recorded for this plug-in. Choose its installer in Setup."));
    }
    #[test]
    fn every_helper_thread_of_the_current_view_records_what_it_opens() {
        // The service names the directories of every file a held page opens.
        // A helper thread started without the composition's recorder would
        // open files no watch covers.
        let source = include_str!("current.rs");
        let production = &source[..source.find("#[cfg(test)]\npub(super) mod tests").unwrap()];
        assert!(!production.contains("scope.spawn("), "start helper threads with linux_vst_bridge::spawn_recorded");
    }
    #[test]
    fn a_view_is_reused_only_while_nothing_live_can_change_it_unwritten() {
        let fixture = test_fixture::Fixture::new();
        let module = fixture.r.module.path.clone();
        let quiet = || {
            let mut context = context_for_watched(&fixture.m,
                BTreeMap::from([(module.clone(), stamp(&module).unwrap())]));
            context.snapshot.system.service = "active".into();
            context.snapshot.system.cleanup_unconfirmed = false;
            context
        };
        let reuse = quiet().reuse();
        assert!(reuse.quiescent);
        assert_eq!(reuse.directories, [module.parent().unwrap().to_owned()], "a stamped file's directory");
        let live: [fn(&mut CurrentOverviewContext); 7] = [
            |c| c.snapshot.operation = Some(json!({"state":"running"})),
            |c| c.snapshot.operation = Some(json!({"state":"submission_uncertain"})),
            |c| { c.installer_live.insert("ab".repeat(16), true); },
            |c| c.all_vendor_retired = false,
            |c| c.snapshot.workspaces.push(serde_json::from_value(json!({"id":"fl-studio","name":"FL Studio",
                "state":"running","selected_installer":"","selected_release":"","installed_advertised_release":null,
                "observed_file_version":null,"installed_image_sha256":null,"active_installation_operation":null,
                "cleanup":"running","first_useful_failure":null,"actions":[],"installer_choices":[],"details":{}}))
                .unwrap()),
            |c| c.snapshot.system.service = "capacity unavailable".into(),
            |c| c.snapshot.system.cleanup_unconfirmed = true,
        ];
        for (index, make_live) in live.iter().enumerate() {
            let mut context = quiet();
            make_live(&mut context);
            assert!(!context.reuse().quiescent, "live condition {index}");
        }
        let mut finished = quiet();
        finished.snapshot.operation = Some(json!({"state":"completed"}));
        finished.installer_live.insert("cd".repeat(16), false);
        assert!(finished.reuse().quiescent, "finished work changes only by writing");
    }
    fn context_for_watched(m:&Manager, watched:BTreeMap<PathBuf,Option<FileStamp>>)
        -> CurrentOverviewContext {
        let pending=pending_transactions(m).unwrap();
        let (workspaces,_)=daw_workspace::current_projection(m).unwrap();
        let artifact = Artifact {path:m.root.join("test-context-only"),sha256:"00".repeat(32)};
        let software = Software {manager:artifact.clone(),supervisor:artifact.clone(),
            ownership:artifact.clone(),host:artifact.clone(),source_manifest:artifact,
            source_sha256:"00".repeat(32),installer_launch:None,preparation_kit:None,
            operator_frontend:None,native_catalogue:None};
        CurrentOverviewContext {
            snapshot:ui::Snapshot {schema:ui::OPERATOR_SCHEMA,state_token:token(m).unwrap(),
                managed_runtime_records:vec![],
                system:system_from_capacity(None,pending,0),onboarding:vec![],
                installer_setups:vec![],environment_installers:vec![],environments:vec![],vendor_applications:vec![],
                products:vec![],workspaces,active_sessions:vec![],capture:Value::Null,
                recent_incidents:vec![],actions:vec![],operation:None,deleted_setups:vec![],uninstallers:vec![],
                bridge:Default::default()},
            busy:None,profiles:vec![],revisions:BTreeMap::new(),
            current_generation:pulse_generation(m).unwrap(),
            software,registry:m.registry().unwrap(),preparation:preparation::RecordReadback::capture_watched(m).unwrap(),
            bindings:vec![],
            owners:capacity::owners(m).unwrap(),watched,installer_live:BTreeMap::new(),
            vendor_progress:None,
            vendor_retired:vendor_retired_with(m,||Ok(false)).unwrap(),all_vendor_retired:true,cleanup_seen:None,
            #[cfg(feature = "pb0-c0-audit")]
            captured_at:Instant::now(),
        }
    }
    #[test]
    fn environment_cache_requires_complete_identity_for_shared_id() {
        let fixture=test_fixture::Fixture::new();
        let exact=fixture.r.environment.clone();
        let mut cache=RecordBindings::default();
        assert!(cache.environment(&exact).unwrap());
        assert!(cache.environment(&exact).unwrap());
        let mut changed=exact.clone();
        changed.revision+=1;
        assert!(!cache.environment(&changed).unwrap());
        changed=exact.clone();
        changed.root=fixture.outer.join("other-root");
        assert!(!cache.environment(&changed).unwrap());
        changed=exact.clone();
        changed.runner.id.push_str("-other");
        assert!(!cache.environment(&changed).unwrap());
    }
    #[test]
    fn partially_refreshed_siblings_and_unrelated_product_remain_readable() {
        let (f,_,_,_)=test_fixture::prepared();
        let mut sibling=f.r.clone();sibling.metadata.class_id="02".repeat(16);
        sibling.metadata.name="Sibling".into();f.m.register(sibling.clone()).unwrap();
        let mut unrelated=f.r.clone();unrelated.metadata.class_id="03".repeat(16);
        unrelated.metadata.name="Unrelated".into();
        unrelated.environment.id="44".repeat(16);
        unrelated.environment.root=f.m.root.join("environments").join(&unrelated.environment.id);
        let module=unrelated.environment.root.join("compatdata/pfx/drive_c/a.vst3");
        private_dir(module.parent().unwrap()).unwrap();fs::copy(&unrelated.module.path,&module).unwrap();
        unrelated.module.path=module;
        atomic_json(&unrelated.environment.root.join("environment.json"),&unrelated.environment).unwrap();
        f.m.register(unrelated.clone()).unwrap();
        let mut current=f.r.environment.clone();current.revision+=1;
        atomic_json(&current.root.join("environment.json"),&current).unwrap();
        let mut db=f.m.registry().unwrap();db.classes.remove(&sibling.metadata.class_id);
        atomic_json(&f.m.root.join("registry.json"),&db).unwrap();
        sibling.environment=current;f.m.register(sibling.clone()).unwrap();
        let a=f.r.host.clone();
        let sw=Software {manager:a.clone(),supervisor:a.clone(),ownership:a.clone(),host:a,
            source_manifest:Artifact {path:f.r.host.path.with_file_name("host-source-manifest.json"),
                sha256:f.r.host_source_sha256.clone()},source_sha256:f.r.host_source_sha256.clone(),
            operator_frontend:None,native_catalogue:None,preparation_kit:None,installer_launch:None};
        let (products,_)=current_products(&f.m,&f.m.registry().unwrap(),&sw,&[]).unwrap();
        assert_eq!(products.len(),3);
        assert_eq!(products.iter().find(|p|p.class_id==f.r.metadata.class_id).unwrap().disposition,"needs_attention");
        assert_eq!(products.iter().find(|p|p.class_id==sibling.metadata.class_id).unwrap().disposition,"ready");
        assert_eq!(products.iter().find(|p|p.class_id==unrelated.metadata.class_id).unwrap().disposition,"ready");
    }
    #[test]
    fn available_plug_in_explains_a_load_refused_because_its_windows_side_stopped() {
        use linux_vst_bridge::preparation as prep;
        // A normally published plug-in has a retained preparation candidate;
        // the overview withdraws discovery-time details.problem for it.
        let (f,base)=preparation_cli::tests::projection_fixture();
        prep::record_candidate(&f.m,&base).unwrap();
        prep::enable(&f.m,&base,false).unwrap();
        let sw=preparation_cli::tests::projection_software(&base);
        atomic_json(&f.m.root.join("software.json"),&sw).unwrap();
        let class=base.selection.class.id.clone();
        let startup=f.m.root.join("runtime/native-startup");
        private_dir(&startup).unwrap();
        let product=|| {
            let captured=capture(&f.m).unwrap();
            captured.snapshot.products.iter().find(|p|p.class_id==class).unwrap().clone()
        };
        let before=product();
        let binding=json!({"environment":before.environment,
            "environment_revision":before.details["environment_revision"],
            "native_sha256":before.details["native_sha256"]});
        let record=|outcome:&str,last:&str| atomic_json(&startup.join(format!("{}.json",class.to_ascii_lowercase())),
            &json!({"schema":1,"class_id":class,"request":"07".repeat(16),
                "observed_at":1792100498u64,"outcome":outcome,"binding":binding,
                "phases":[["request_received",0],["binding_verified",31],[last,267]],"total_ms":267})).unwrap();
        assert!(before.details["last_load"].is_null());
        let available=before.availability.clone();
        // Before: the musician saw an available plug-in and a blank editor.
        record("refused","keeper_failed");
        let shown=product();
        assert_eq!(shown.details["last_load"]["problem"]["code"],"windows_side_stopped");
        assert_eq!(shown.details["last_load"]["observed_at"],1792100498u64);
        let advice=shown.details["last_load"]["problem"]["what_to_try"].as_str().unwrap();
        assert!(advice.contains("Load the plug-in again") && advice.contains("wait a minute"));
        assert!(!advice.contains("first found"));
        // The explanation never changes the published plug-in's availability.
        assert_eq!(serde_json::to_value(&shown.availability).unwrap(),
            serde_json::to_value(&available).unwrap());
        // A later accepted load withdraws it; any other refusal explains
        // itself in general terms; a stale DAW module says to restart it.
        record("accepted","keeper_ready");
        assert!(product().details["last_load"].is_null());
        record("refused","bridge_update_required");
        assert_eq!(product().details["last_load"]["problem"]["code"],"bridge_updated_daw_old");
        record("refused","publication_changed_during_startup");
        assert_eq!(product().details["last_load"]["problem"]["code"],"load_refused");
        // Capacity refusals name the limit rather than the generic advice.
        atomic_json(&startup.join(format!("{}.json",class.to_ascii_lowercase())),
            &json!({"schema":1,"class_id":class,"request":"07".repeat(16),
                "observed_at":1792100498u64,"outcome":"refused","binding":binding,
                "refusal":"admission_global_capacity",
                "phases":[["request_received",0],["binding_verified",31]],"total_ms":31})).unwrap();
        assert_eq!(product().details["last_load"]["problem"]["code"],"bridge_plugin_limit");
        // A record made against another publication does not linger.
        let shown=shown.details["native_sha256"].clone();
        atomic_json(&startup.join(format!("{}.json",class.to_ascii_lowercase())),
            &json!({"schema":1,"class_id":class,"request":"07".repeat(16),
                "observed_at":1792100498u64,"outcome":"refused","phases":[["keeper_failed",267]],
                "binding":{"environment":base.selection.environment.id,
                    "environment_revision":base.selection.environment.revision,
                    "native_sha256":"00".repeat(32)},"total_ms":267})).unwrap();
        assert!(product().details["last_load"].is_null());
        atomic_json(&startup.join(format!("{}.json",class.to_ascii_lowercase())),
            &json!({"schema":1,"class_id":class,"request":"07".repeat(16),
                "observed_at":1792100498u64,"outcome":"refused","phases":[["keeper_failed",267]],
                "binding":{"environment":base.selection.environment.id,
                    "environment_revision":base.selection.environment.revision,
                    "native_sha256":shown},"total_ms":267})).unwrap();
        assert_eq!(product().details["last_load"]["problem"]["code"],"windows_side_stopped");
        fs::write(startup.join(format!("{}.json",class.to_ascii_lowercase())),b"not json").unwrap();
        assert!(product().details["last_load"].is_null());
        // A record never tied to a publication does not explain this one.
        atomic_json(&startup.join(format!("{}.json",class.to_ascii_lowercase())),
            &json!({"schema":1,"class_id":class,"request":"07".repeat(16),
                "observed_at":1792100498u64,"outcome":"refused","phases":[["keeper_failed",267]],
                "total_ms":267})).unwrap();
        assert!(product().details["last_load"].is_null());
        // The DAW's refused audio start (#403), kept by the bridge plug-in in
        // the DAW, explains the card until a later load is recorded.
        let activation=f.m.root.join("runtime/native-activation");
        private_dir(&activation).unwrap();
        let refused=|observed:u64,reason:&str,engine:&Value| atomic_json(
            &activation.join(format!("{}.json",class.to_ascii_lowercase())),
            &json!({"schema":1,"class_id":class,"module_sha256":before.module_sha256,
                "engine_sha256":engine,"observed_at":observed,"stage":"setup",
                "reason":reason,"requested":65536})).unwrap();
        let engine=before.details["native_sha256"].clone();
        refused(1792100499,"daw_block_size",&engine);
        let shown=product();
        assert_eq!(shown.details["last_load"]["outcome"],"activation_refused");
        assert_eq!(shown.details["last_load"]["observed_at"],1792100499u64);
        assert_eq!(shown.details["last_load"]["problem"]["code"],"daw_block_size_unsupported");
        assert!(shown.details["last_load"]["problem"]["likely_cause"].as_str().unwrap().contains("65536 samples"));
        assert_eq!(serde_json::to_value(&shown.availability).unwrap(),
            serde_json::to_value(&available).unwrap());
        refused(1792100499,"windows_setup_refused",&engine);
        assert_eq!(product().details["last_load"]["problem"]["code"],"windows_side_refused_start");
        // A load recorded after it withdraws it; so do another bridge build
        // and a reason this manager does not know.
        refused(1792100497,"daw_block_size",&engine);
        assert!(product().details["last_load"].is_null());
        refused(1792100499,"daw_block_size",&json!("00".repeat(32)));
        assert!(product().details["last_load"].is_null());
        refused(1792100499,"something_new",&engine);
        assert!(product().details["last_load"].is_null());
    }
    #[test]
    fn current_product_requires_committed_publication_and_reads_selected_delay() {
        let (fixture,profile,census,_native)=test_fixture::prepared();
        let m=&fixture.m;
        let reference=m.managed_publish(&profile,&census,fixture.r.clone(),
            &fixture.r.host,&fixture.r.host_source_sha256,None).unwrap();
        let db=m.registry().unwrap();
        let a=fixture.r.host.clone();
        let sw=Software {installer_launch:None,preparation_kit:None,operator_frontend:None,
            manager:a.clone(),supervisor:a.clone(),ownership:a.clone(),host:a.clone(),
            source_manifest:Artifact {path:a.path.with_file_name("host-source-manifest.json"),
                sha256:fixture.r.host_source_sha256.clone()},source_sha256:fixture.r.host_source_sha256.clone(),
            native_catalogue:None};
        let class=&profile.class.class_id;
        let (products,_)=current_products(m,&db,&sw,std::slice::from_ref(&profile)).unwrap();
        assert_eq!(products[0].details["publication_complete"],true);
        assert_eq!(products[0].details["added_frames"],512);
        assert_eq!(products[0].disposition,"ready");
        let performance=m.root.join("performance").join(format!("{class}.json"));
        private_dir(performance.parent().unwrap()).unwrap();
        atomic_json(&performance,&Performance {schema:1,added_frames:512, delivery_mode:DeliveryMode::Buffered }).unwrap();
        let (products,_)=current_products(m,&db,&sw,std::slice::from_ref(&profile)).unwrap();
        assert_eq!(products[0].details["performance_valid"],true);
        assert_eq!(products[0].details["added_frames"],512);
        atomic_json(&performance,&Performance {schema:1,added_frames:256, delivery_mode:DeliveryMode::Buffered }).unwrap();
        let (products,_)=current_products(m,&db,&sw,std::slice::from_ref(&profile)).unwrap();
        assert_eq!(products[0].details["performance_valid"],true);
        assert_eq!(products[0].details["added_frames"],256);
        atomic_json(&performance,&json!({"schema":9,"added_frames":512})).unwrap();
        let (products,_)=current_products(m,&db,&sw,std::slice::from_ref(&profile)).unwrap();
        assert_eq!(products[0].details["performance_valid"],false);
        assert_eq!(products[0].disposition,"needs_attention");
        fs::remove_file(&performance).unwrap();
        let revision=m.load_revision(class,&reference).unwrap();
        let result=m.root.join("transactions").join(format!("{}.result.json",revision.transaction));
        let intent=m.root.join("transactions").join(format!("{}.json",revision.transaction));
        let watched=watch_paths(m,&sw,&db).unwrap();
        assert!(watched.contains_key(&intent) && watched.contains_key(&result));
        let original_intent=fs::read(&intent).unwrap();
        let original_result=fs::read(&result).unwrap();
        let context=context_for_watched(m,watched.clone());
        context.recheck(m).unwrap();
        fs::remove_file(&result).unwrap();
        assert_ne!(stamp(&result).unwrap(),watched[&result]);
        assert!(context.recheck(m).is_err());
        let (products,_)=current_products(m,&db,&sw,std::slice::from_ref(&profile)).unwrap();
        assert_eq!(products[0].details["publication_complete"],false);
        assert_eq!(products[0].details["publication_valid"],false);
        assert_eq!(products[0].disposition,"needs_attention");
        atomic_json(&result,&serde_json::from_slice::<Value>(&original_result).unwrap()).unwrap();
        fs::remove_file(&intent).unwrap();
        assert_eq!(current_products(m,&db,&sw,std::slice::from_ref(&profile)).unwrap().0[0]
            .details["publication_complete"],false);
        atomic_json(&intent,&serde_json::from_slice::<Value>(&original_intent).unwrap()).unwrap();
        let mut aborted:Value=serde_json::from_slice(&original_result).unwrap();
        aborted["outcome"]="aborted".into();
        atomic_json(&result,&aborted).unwrap();
        assert_eq!(current_products(m,&db,&sw,std::slice::from_ref(&profile)).unwrap().0[0]
            .details["publication_complete"],false);
        atomic_json(&result,&serde_json::from_slice::<Value>(&original_result).unwrap()).unwrap();
        let mut changed:Value=serde_json::from_slice(&original_intent).unwrap();
        changed["candidate"]["id"]="00".repeat(16).into();
        atomic_json(&intent,&changed).unwrap();
        assert_eq!(current_products(m,&db,&sw,std::slice::from_ref(&profile)).unwrap().0[0]
            .details["publication_complete"],false);
        atomic_json(&intent,&serde_json::from_slice::<Value>(&original_intent).unwrap()).unwrap();
        let mut changed:Value=serde_json::from_slice(&original_intent).unwrap();
        changed["prior"]=json!({"entry":db.classes[class],
            "revision":{"id":"11".repeat(16),"sha256":"22".repeat(32)},"target":null});
        atomic_json(&intent,&changed).unwrap();
        assert_eq!(current_products(m,&db,&sw,std::slice::from_ref(&profile)).unwrap().0[0]
            .details["publication_complete"],false);
    }
    #[test]
    fn current_publication_reader_uses_canonical_symlink_custody() {
        let fixture=test_fixture::Fixture::new();
        let link=fixture.outer.join("published.vst3");
        assert_eq!(publication::physical(&link).unwrap(),None);
        std::os::unix::fs::symlink("first",&link).unwrap();
        assert_eq!(publication::physical(&link).unwrap(),Some(PathBuf::from("first")));
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("changed",&link).unwrap();
        assert_ne!(publication::physical(&link).unwrap(),Some(PathBuf::from("first")));
        fs::remove_file(&link).unwrap();
        fs::write(&link,b"ordinary file").unwrap();
        assert!(publication::physical(&link).is_err());
        fs::remove_file(&link).unwrap();
        fs::create_dir(&link).unwrap();
        assert!(publication::physical(&link).is_err());
        fs::remove_dir(&link).unwrap();
        #[cfg(target_os="linux")]
        if unsafe {libc::geteuid()} == 0 {
            use std::os::unix::ffi::OsStrExt;
            std::os::unix::fs::symlink("first",&link).unwrap();
            let path=std::ffi::CString::new(link.as_os_str().as_bytes()).unwrap();
            assert_eq!(unsafe {libc::lchown(path.as_ptr(),65534,65534)},0);
            assert!(publication::physical(&link).is_err());
        }
    }
    #[test]
    fn current_watch_binds_setup_performance_workspace_and_pending_inputs() {
        let fixture=test_fixture::Fixture::new();
        let m=&fixture.m;
        let a=fixture.r.host.clone();
        let sw=Software {installer_launch:None,preparation_kit:None,operator_frontend:None,
            manager:a.clone(),supervisor:a.clone(),ownership:a.clone(),host:a.clone(),
            source_manifest:a,source_sha256:"ab".repeat(32),native_catalogue:None};
        let installer_id="ab".repeat(32);
        let installer_path=m.root.join("installers").join(format!("{installer_id}.exe"));
        private_dir(installer_path.parent().unwrap()).unwrap();
        fs::write(&installer_path,b"installer").unwrap();
        let installer=installer_import::Installer {schema:1,id:installer_id.clone(),
            artifact:Artifact {path:installer_path.clone(),sha256:installer_id.clone()},
            byte_size:9,format:"pe_executable".into(),name_hint:"Fixture".into(),
            source_class:"operator_selected_file".into(),import_result:"imported".into(),
            created_at:1,vendor:None,product:None};
        let installer_record=m.root.join("installers").join(format!("{installer_id}.json"));
        atomic_json(&installer_record,&installer).unwrap();
        let presentation=m.root.join("installers/presentation").join(format!("{installer_id}.json"));
        private_dir(presentation.parent().unwrap()).unwrap();
        atomic_json(&presentation,&installer_import::Presentation {schema:1,
            installer_sha256:installer_id.clone(),display_label:"Fixture".into(),
            label_source:"operator_named".into(),updated_at:1}).unwrap();
        let environment_id="ef".repeat(16);
        let mut environment=fixture.r.environment.clone();
        environment.id=environment_id.clone();
        environment.root=m.root.join("environments").join(&environment_id);
        private_dir(&environment.root).unwrap();
        atomic_json(&environment.root.join("environment.json"),&environment).unwrap();
        let operation="12".repeat(16);
        let onboarding_dir=m.root.join("onboarding").join(&environment_id);
        private_dir(&onboarding_dir).unwrap();
        let onboarding_record=onboarding_dir.join("record.json");
        atomic_json(&onboarding_record,&onboarding::Record {schema:1,id:environment_id.clone(),
            installer:installer_id,environment,created_at:1,creation_operation:"34".repeat(16),
            installation_operation:Some(operation.clone()),published:false,automatic:None,previous_attempt:None}).unwrap();
        let result=onboarding_dir.join(format!("{operation}-result.json"));
        fs::write(&result,b"old result").unwrap();
        let private_report=onboarding_dir.join(format!("{operation}-transaction-private.json"));
        let mut observation=json!({"schema":1,"operation":operation,
            "durable_installation":{"application_registrations":[]},"diagnostics":{"progress":0}});
        atomic_json(&private_report,&observation).unwrap();
        let inventory=m.root.join("inventory").join(format!("{environment_id}.json"));
        private_dir(inventory.parent().unwrap()).unwrap();
        fs::write(&inventory,b"old scan").unwrap();
        let workspace=m.root.join("daw-workspaces/fl-studio/workspace.json");
        let pending=m.root.join("transactions").join(format!("{}.pending.json","56".repeat(16)));
        private_dir(pending.parent().unwrap()).unwrap();
        fs::write(&pending,b"old pending").unwrap();
        let class="78".repeat(16);
        let performance=m.root.join("performance").join(format!("{class}.json"));
        private_dir(performance.parent().unwrap()).unwrap();
        fs::write(&performance,b"old performance").unwrap();
        let mut db=Registry::default();
        let mut registration=fixture.r.clone();
        registration.metadata.class_id=class;
        db.classes.insert(registration.key(),linux_vst_bridge::Entry {
            registration,publication:Publication::Removed,managed_revision:None});
        let before=watch_paths(m,&sw,&db).unwrap();
        let vendor=m.root.join("vendor-applications");
        let context=context_for_watched(m,watch_paths(m,&sw,&db).unwrap());
        let before_mode=stamp(&installer_path).unwrap();
        fs::set_permissions(&installer_path,fs::Permissions::from_mode(0o600)).unwrap();
        assert_ne!(stamp(&installer_path).unwrap(),before_mode);
        assert!(context.recheck(m).is_err());
        for path in [&vendor,&installer_record,&installer_path,&presentation,&onboarding_record,
            &result,&private_report,&inventory,&pending,&performance,&workspace] {
            assert!(before.contains_key(path),"unwatched current input: {}",path.display());
            let context=context_for_watched(m,watch_paths(m,&sw,&db).unwrap());
            context.recheck(m).unwrap();
            let original=fs::read(path).ok();
            if path == &vendor {private_dir(path).unwrap();}
            else {
                private_dir(path.parent().unwrap()).unwrap();
                fs::write(path,b"a changed current record").unwrap();
            }
            assert_ne!(stamp(path).unwrap(),before[path]);
            assert!(context.recheck(m).is_err(),"accepted changed current input: {}",path.display());
            if let Some(bytes)=original {fs::write(path,bytes).unwrap();}
            else {
                if path == &vendor {fs::remove_dir(path).unwrap();}
                else {fs::remove_file(path).unwrap();}
            }
        }
        // Registration can finish inside an already-created selector directory.
        // Neither a root-only watch nor only registered applications covers it.
        let unregistered=vendor.join("ab".repeat(32));
        private_dir(&unregistered).unwrap();
        let context=context_for_watched(m,watch_paths(m,&sw,&db).unwrap());
        let application=unregistered.join("application.json");
        assert!(context.watched.contains_key(&application));
        fs::write(&application,b"new selected application").unwrap();
        assert!(context.recheck(m).is_err());
        fs::remove_file(application).unwrap();
        fs::remove_dir(unregistered).unwrap();
        // A live installer needs recovery controls while its observations change.
        // The same file retains installation evidence after retirement, so that
        // file remains covered by the retired-input checks above.
        let mut live=context_for_watched(m,watch_paths(m,&sw,&db).unwrap());
        live.installer_live.insert(operation.clone(),true);
        live.snapshot.onboarding.push(ui::Onboarding {failure:None,
            installer:installer.id.clone(),name:"Fixture".into(),byte_size:9,
            format:"pe_executable".into(),environment:Some(environment_id.clone()),
            state:"running".into(),required_human_action:"Use exact Stop".into(),
            details:json!({"installation":{"operation":operation,"state":"running"}}),actions:vec![]});
        atomic_json(&result,&json!({"schema":2,"operation":operation,
            "state":"running","owned_live":2,"cleanup_confirmed":false})).unwrap();
        live.recheck_with(m,||Ok(())).unwrap();
        atomic_json(&result,&json!({"schema":2,"operation":operation,
            "state":"running","owned_live":7,"cleanup_confirmed":false})).unwrap();
        observation["diagnostics"]["progress"]=1.into();
        atomic_json(&private_report,&observation).unwrap();
        live.recheck_with(m,||Ok(())).unwrap();
        for state in ["queued","starting","running","unknown"] {
            live.snapshot.onboarding[0].details["installation"]["state"]=state.into();
            atomic_json(&result,&json!({"schema":2,"operation":operation,
                "state":state,"owned_live":7,"cleanup_confirmed":false})).unwrap();
            observation["diagnostics"]["state"]=state.into();
            atomic_json(&private_report,&observation).unwrap();
            live.recheck_with(m,||Ok(())).unwrap();
        }
        live.snapshot.onboarding[0].details["installation"]["state"]="running".into();
        let running=fs::read(&result).unwrap();
        let terminal=json!({"schema":2,"operation":operation,"state":"completed",
            "raw_exit":0,"owned_live":0,"cleanup_confirmed":true});
        // Retirement may be published before the same live unit exits. Once
        // terminal, durable installation observations are authoritative.
        atomic_json(&result,&terminal).unwrap();
        live.snapshot.onboarding[0].details["installation"]["state"]="completed".into();
        live.watched=watch_paths(m,&sw,&db).unwrap();
        observation["durable_installation"]["application_registrations"]=json!([{"changed":true}]);
        atomic_json(&private_report,&observation).unwrap();
        assert!(live.recheck_with(m,||Ok(())).is_err());
        // A running snapshot cannot exempt a receipt that retired during
        // capture, even if its new file stamp was captured afterward.
        live.snapshot.onboarding[0].details["installation"]["state"]="running".into();
        live.watched=watch_paths(m,&sw,&db).unwrap();
        assert!(live.recheck_with(m,||Ok(())).is_err());
        fs::write(&result,&running).unwrap();
        live.watched=watch_paths(m,&sw,&db).unwrap();
        let mut checks=0;
        assert!(live.recheck_with(m,|| {
            checks+=1;
            if checks==2 {atomic_json(&result,&terminal)?;}
            Ok(())
        }).is_err());
        assert_eq!(checks,2,"receipt transition must also be checked after the watch");
        fs::write(&result,&running).unwrap();
        live.watched=watch_paths(m,&sw,&db).unwrap();
        // The real external recheck cannot accept our simulated live unit.
        assert!(live.recheck(m).is_err());
        live.installer_live.insert(operation.clone(),false);
        observation["diagnostics"]["progress"]=2.into();
        atomic_json(&private_report,&observation).unwrap();
        assert!(live.recheck_with(m,||Ok(())).is_err());
        live.installer_live.insert(operation.clone(),true);
        let original_record=fs::read(&onboarding_record).unwrap();
        let mut changed:Value=serde_json::from_slice(&original_record).unwrap();
        changed["installation_operation"]="90".repeat(16).into();
        atomic_json(&onboarding_record,&changed).unwrap();
        assert!(live.recheck_with(m,||Ok(())).is_err());
        fs::write(&onboarding_record,&original_record).unwrap();
        // Recapture after restoring the record so only directory replacement
        // changes custody, even when its record/result files keep their inodes.
        live.watched=watch_paths(m,&sw,&db).unwrap();
        let moved=onboarding_dir.with_extension("retained");
        fs::rename(&onboarding_dir,&moved).unwrap();
        private_dir(&onboarding_dir).unwrap();
        for name in ["record.json".into(),format!("{operation}-result.json")] {
            fs::rename(moved.join(&name),onboarding_dir.join(&name)).unwrap();
        }
        assert!(live.recheck_with(m,||Ok(())).is_err());
    }
    #[test]
    fn user_selected_running_program_has_current_show_and_close_controls() {
        let (f, legacy) = selected_vendor_fixture();
        fs::remove_dir_all(vendor_cli::directory(&f.m,&legacy.selector().unwrap()).unwrap()).unwrap();
        let folder=f.outer.join("Portable program");private_dir(&folder).unwrap();
        let path=folder.join("Program.exe");
        let mut bytes=vec![0_u8;256];bytes[..2].copy_from_slice(b"MZ");bytes[60]=128;
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664_u16.to_le_bytes());bytes[150]=2;
        fs::write(&path,bytes).unwrap();
        let app=vendor_application::Application {schema:3,id:vendor_application::ApplicationId::Selected,
            environment:legacy.environment,executable:vendor_cli::selection::program_artifact(&path).unwrap(),
            helpers:vec![],installer_sha256:None,installer_result:None,observed_installer_version:String::new(),
            user_selection:Some(vendor_application::ProgramSelection {working_directory:folder,
                arguments:vec![],previous:None})};
        app.verify(&f.m.root).unwrap();
        let selector=app.selector().unwrap();let directory=vendor_cli::directory(&f.m,&selector).unwrap();
        private_dir(&directory).unwrap();atomic_json(&directory.join("application.json"),&app).unwrap();
        let operation=random_id().unwrap();let report_path=directory.join("operation-result.json");
        let job=vendor_application::Operation {application:app.clone(),report:report_path.clone(),
            mode:vendor_application::LaunchMode::Normal,operation_id:operation.clone(),launcher:Some(f.r.host.clone()), managed:None,capability_notice:None};
        atomic_json(&directory.join("operation.json"),&job).unwrap();
        let mut report=json!({"schema":3,"operation_id":operation,"state":"running",
            "launcher_exit":null,"owned_live":2,"cleanup_confirmed":false,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown"});
        atomic_json(&report_path,&report).unwrap();
        let record=onboarding::retained_environment_record(&f.m,&app.environment.id).unwrap().unwrap();
        let installer_unit=onboarding::unit(record.installation_operation.as_deref().unwrap()).unwrap();
        readiness::with_unit_state_fixture(&[installer_unit],"inactive",||
            readiness::with_unit_state_fixture(&[vendor_cli::unit(&selector).unwrap()],"active",|| {
                // A vendor session suspends the bridge service. Current program
                // controls must remain readable without an available DSP service.
                let current=current_overview(&f.m).unwrap();
                assert!(!current.current.system.capacity_available());
                let row=current.current.vendor_applications.iter().find(|row|row.id==selector).unwrap();
                assert_eq!(row.state,"running");assert_eq!(row.details["user_selected"],true);
                for action in [ui::Action::VendorApplicationFocus {application:selector.clone()},
                    ui::Action::VendorApplicationClose {application:selector.clone()}] {
                    let offer=row.actions.iter().find(|offer|offer.action==action).unwrap();
                    assert!(offer.disabled_reason.is_none());
                    validate_current_request(&f.m,&ui::Request {schema:ui::OPERATOR_SCHEMA,
                        state_token:current.current.state_token.clone(),action}).unwrap();
                }
                let captured=capture(&f.m).unwrap();
                report["owned_live"]=7.into();atomic_json(&report_path,&report).unwrap();
                captured.recheck(&f.m).unwrap();
                // Accepting this origin does not relax exact operation identity.
                let mut changed=job.clone();changed.operation_id=random_id().unwrap();
                atomic_json(&directory.join("operation.json"),&changed).unwrap();
                assert!(captured.recheck(&f.m).is_err());
                atomic_json(&directory.join("operation.json"),&job).unwrap();
                let mut unknown=app.clone();unknown.schema=4;
                atomic_json(&directory.join("application.json"),&unknown).unwrap();
                let mut unknown_job=job.clone();unknown_job.application=unknown;
                atomic_json(&directory.join("operation.json"),&unknown_job).unwrap();
                assert_eq!(live_vendor_progress_selected(&f.m,&selector,||Ok(true)).unwrap_err().to_string(),
                    "operator_current_vendor_binding");
            }));
    }
    #[test]
    fn live_vendor_progress_preserves_exact_current_readback() {
        readiness::with_unit_state_fixture(&[vendor_cli::unit(ASC).unwrap()],"inactive",|| {
        let f = test_fixture::Fixture::new();
        let m = &f.m;
        let directory = app_directory(m);
        private_dir(&directory).unwrap();
        let app = vendor_application::Application {schema:1,
            id:vendor_application::ApplicationId::ArturiaSoftwareCenter,
            environment:f.r.environment.clone(),executable:f.r.module.clone(),helpers:vec![],
            installer_sha256:Some("00".repeat(32)),installer_result:Some(f.r.host.clone()),
            observed_installer_version:"fixture".into(),user_selection:None};
        let operation = "ab".repeat(16);
        let report_path = directory.join("operation-result.json");
        atomic_json(&directory.join("application.json"),&app).unwrap();
        atomic_json(&directory.join("operation.json"),&json!({"application":app,
            "report":report_path,"mode":"normal","operation_id":operation})).unwrap();
        let mut report = json!({"schema":3,"operation_id":operation,"state":"running",
            "launcher_exit":null,"owned_live":2,"cleanup_confirmed":false,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown"});
        atomic_json(&report_path,&report).unwrap();
        let sw = context_for_watched(m,BTreeMap::new()).software;
        let mut context = context_for_watched(m,watch_paths(m,&sw,&m.registry().unwrap()).unwrap());
        context.vendor_progress = live_vendor_progress_with(m,||Ok(true)).unwrap();
        report["owned_live"] = 7.into();
        report["discarded_diagnostic_bytes"] = 1024.into();
        atomic_json(&report_path,&report).unwrap();
        context.recheck_with(m,||Ok(())).unwrap();
        // Progress is never retirement or permission to adopt a later launch.
        for (field,value) in [("operation_id",json!("cd".repeat(16))),
            ("state",json!("completed")),("cleanup_confirmed",json!(true)),
            ("owned_live",json!(0)),("schema",json!(4)),("token",json!("private"))] {
            let mut changed = report.clone();
            changed[field] = value;
            atomic_json(&report_path,&changed).unwrap();
            assert!(context.recheck_with(m,||Ok(())).is_err(),"accepted {field}");
        }
        atomic_json(&report_path,&report).unwrap();
        for path in [directory.join("operation.json"),directory.join("application.json"),
            app.environment.root.join("environment.json")] {
            assert!(context.watched.contains_key(&path));
            let original = fs::read(&path).unwrap();
            let mut changed:Value = serde_json::from_slice(&original).unwrap();
            if path.ends_with("operation.json") {changed["operation_id"] = "cd".repeat(16).into();}
            else if path.ends_with("application.json") {changed["executable"]["sha256"] = "cd".repeat(32).into();}
            else {changed["revision"] = 2.into();}
            atomic_json(&path,&changed).unwrap();
            assert!(context.recheck_with(m,||Ok(())).is_err(),"accepted {}",path.display());
            fs::write(&path,&original).unwrap();
            context.watched = watch_paths(m,&sw,&m.registry().unwrap()).unwrap();
        }
        assert!(live_vendor_progress_with(m,||Ok(false)).unwrap().is_none());
        assert!(context.recheck_with(m,||require(
            live_vendor_progress_with(m,||Ok(false))? == context.vendor_progress,
            "operator_vendor_state_changed_refresh")).is_err());
        // A closed report retains the full byte watch, including diagnostics.
        let mut terminal = report.clone();
        terminal["state"] = "completed".into();
        terminal["owned_live"] = 0.into();
        terminal["cleanup_confirmed"] = true.into();
        atomic_json(&report_path,&terminal).unwrap();
        let closed = context_for_watched(m,watch_paths(m,&sw,&m.registry().unwrap()).unwrap());
        terminal["discarded_diagnostic_bytes"] = 2048.into();
        atomic_json(&report_path,&terminal).unwrap();
        assert!(closed.recheck_with(m,||Ok(())).is_err());
        atomic_json(&report_path,&report).unwrap();
        context.watched = watch_paths(m,&sw,&m.registry().unwrap()).unwrap();
        let moved = directory.with_extension("retained");
        fs::rename(&directory,&moved).unwrap();
        private_dir(&directory).unwrap();
        for name in ["application.json","operation.json","operation-result.json"] {
            fs::rename(moved.join(name),directory.join(name)).unwrap();
        }
        assert!(context.recheck_with(m,||Ok(())).is_err());
        });
    }
    #[test]
    fn current_recheck_refuses_revision_and_publication_drift_before_ready() {
        let fixture = test_fixture::Fixture::new();
        let revision = fixture.outer.join("selected-revision.json");
        let publication = fixture.outer.join("selected-publication");
        fs::write(&revision,b"retained revision").unwrap();
        std::os::unix::fs::symlink("first",&publication).unwrap();
        let watched = [revision.clone(),publication.clone()].into_iter()
            .map(|path| (path.clone(),stamp(&path).unwrap())).collect();
        let snapshot = ui::Snapshot {schema:ui::OPERATOR_SCHEMA,managed_runtime_records:vec![],
            state_token:token(&fixture.m).unwrap(),
            system:system_from_capacity(None,0,0),onboarding:vec![],
            installer_setups:vec![],environment_installers:vec![],environments:vec![],vendor_applications:vec![],
            products:vec![],workspaces:vec![],active_sessions:vec![],
            capture:Value::Null,recent_incidents:vec![],actions:vec![],operation:None,deleted_setups:vec![],uninstallers:vec![],
            bridge:Default::default()};
        let mut context = context_for_watched(&fixture.m, watched);
        context.snapshot = snapshot;
        context.recheck(&fixture.m).unwrap();
        fs::write(&revision,b"changed revision").unwrap();
        assert!(context.recheck(&fixture.m).is_err());
        fs::write(&revision,b"retained revision").unwrap();
        fs::remove_file(&publication).unwrap();
        std::os::unix::fs::symlink("other",&publication).unwrap();
        assert!(context.recheck(&fixture.m).is_err());
    }
    #[test]
    fn delayed_external_rechecks_do_not_hold_registry_authority() {
        let fixture=test_fixture::Fixture::new();
        let context=context_for_watched(&fixture.m,BTreeMap::new());
        std::thread::scope(|scope| {
            let (entered_tx,entered_rx)=std::sync::mpsc::channel();
            let (release_tx,release_rx)=std::sync::mpsc::channel();
            let manager=&fixture.m;
            let context=&context;
            let worker=scope.spawn(move || {
                let mut calls=0;
                context.recheck_with(manager,|| {
                    calls+=1;
                    entered_tx.send(calls).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(2))
                        .map_err(|_| "external_probe_release_timeout")?;
                    Ok(())
                })
            });
            for expected in 1..=2 {
                assert_eq!(entered_rx.recv_timeout(Duration::from_secs(2)).unwrap(),expected);
                let guard=acquire_readback(&fixture.m,ui::OperatorLock::Registry,None,
                    Duration::from_millis(200),&mut vec![]).unwrap();
                drop(guard);
                release_tx.send(()).unwrap();
            }
            worker.join().unwrap().unwrap();
        });
    }
    const SETUPS: usize = 15;
    /// A Deck-shaped history (#504): three published plug-ins beside fifteen
    /// setups, each reached through eight retained installer runs, and 380
    /// other finished operations. Returns the installer units, all closed.
    fn retained_history() -> (test_fixture::Fixture, Vec<linux_vst_bridge::preparation::Candidate>, Vec<String>) {
        use linux_vst_bridge::preparation as prep;
        let (f, candidates) = test_fixture::projection_candidates_in(test_fixture::Fixture::new(), false, 3);
        for candidate in &candidates {
            prep::record_candidate(&f.m, candidate).unwrap();
            prep::enable(&f.m, candidate, false).unwrap();
        }
        let units = test_fixture::retained_history(&f, test_fixture::Changes::Installers, SETUPS, 8, 380)
            .iter().map(|operation| onboarding::unit(operation).unwrap()).collect();
        (f, candidates, units)
    }
    const PAGES: [&str; 3] = ["current overview", "plug-in page", "snapshot"];
    /// The current overview, the first plug-in's page, or the snapshot, as written.
    fn page(m: &Manager, candidates: &[linux_vst_bridge::preparation::Candidate], name: &str) -> Result<Vec<u8>> {
        let s = &candidates[0].selection;
        Ok(match name {
            "current overview" => serde_json::to_vec(&current_overview(m)?)?,
            "plug-in page" => serde_json::to_vec(&product_detail(m, &s.environment.id, &s.module.sha256, &s.class.id)?)?,
            _ => serde_json::to_vec(&snapshot_for_operation(m, None, OPERATOR_WAIT, &mut vec![],
                &|| crate::operator_cli::tests::capacity_fixture(m))?)?,
        })
    }
    #[test]
    fn a_composition_reads_each_retained_record_a_bounded_number_of_times() {
        // Before #504 each setup's revision chain read every operation
        // record again: on this history each was opened up to 210 times per
        // page, and more with every setup.
        let (f, candidates, units) = retained_history();
        readiness::with_unit_state_fixture(&units, "inactive", || {
            for name in PAGES {
                // The snapshot is given the service's census directly.
                let server = (name != "snapshot").then(|| overview_service_reply(&f.m, capacity_json(false,0,0)));
                let (composed, opened) = linux_vst_bridge::with_opened_counts(|| page(&f.m, &candidates, name));
                if let Some(server) = server { server.join().unwrap(); }
                composed.unwrap();
                let opened = opened.unwrap();
                let records: Vec<_> = opened.iter().filter(|(path, _)|
                    ["operator", "onboarding"].iter().any(|kind| path.starts_with(f.m.root.join(kind)))).collect();
                assert!(records.len() > 500, "{name}: {} records", records.len());
                // The capture and its recheck each read a record a few times.
                // Reading it again for each setup, as the revision chains did,
                // opens it at least as often as there are setups.
                let (path, most) = records.iter().max_by_key(|(_, opens)| **opens).unwrap();
                assert!((**most as usize) < SETUPS, "{name} opened {path:?} {most} times");
                // The snapshot read the registry once for every retained
                // preparation it showed (#503), and each setup's card read it
                // twice: 36 times per current overview, 42 per plug-in page.
                // A page now reads it a few times, however many setups.
                let registry = opened.get(&f.m.root.join("registry.json")).copied().unwrap_or(0);
                let most = match name { "current overview" => 6, "plug-in page" => 11, _ => 5 * (SETUPS + 1) };
                assert!((registry as usize) <= most, "{name} opened the registry {registry} times");
            }
        });
    }
    #[test]
    fn deriving_each_fact_once_changes_nothing_a_page_shows() {
        let (f, candidates, units) = retained_history();
        readiness::with_unit_state_fixture(&units, "inactive", || {
            let server = overview_service_reply_with_counts(&f.m, capacity_json(false,0,0), (2,6));
            let every_use = capture(&f.m).unwrap();
            let once = capture(&f.m.composing()).unwrap();
            every_use.recheck(&f.m).unwrap();
            once.recheck(&f.m).unwrap();
            server.join().unwrap();
            assert_eq!(serde_json::to_string(&once.snapshot).unwrap(),
                serde_json::to_string(&every_use.snapshot).unwrap());
            assert_eq!(once.current_generation, every_use.current_generation);
            let s = &candidates[0].selection;
            let server = overview_service_reply_with_counts(&f.m, capacity_json(false,0,0), (2,6));
            let every_use = product_detail_readonly(&f.m, &s.environment.id, &s.module.sha256, &s.class.id);
            let once = product_detail(&f.m, &s.environment.id, &s.module.sha256, &s.class.id);
            server.join().unwrap();
            assert_eq!(serde_json::to_string(&once.unwrap()).unwrap(),
                serde_json::to_string(&every_use.unwrap()).unwrap());
        });
    }
    /// Not a check: what a page reads on a Deck-shaped history (#504), by
    /// kind of record: opens, this process's read calls and bytes, and CPU.
    #[test]
    #[ignore = "measurement; run with --ignored --nocapture --test-threads=1"]
    fn composition_reads_on_a_retained_history() {
        fn io() -> (u64, u64) {
            let text = fs::read_to_string("/proc/self/io").unwrap_or_default();
            let field = |name: &str| text.lines().find_map(|line| line.strip_prefix(name))
                .and_then(|value| value.trim().parse().ok()).unwrap_or(0);
            (field("syscr:"), field("rchar:"))
        }
        fn cpu() -> f64 {
            let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
            unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
            let ms = |t: libc::timeval| t.tv_sec as f64 * 1000.0 + t.tv_usec as f64 / 1000.0;
            ms(usage.ru_utime) + ms(usage.ru_stime)
        }
        let (f, candidates, units) = retained_history();
        // Records of one kind differ only in their identities.
        let kind = |path: &Path| path.strip_prefix(&f.m.root).unwrap_or(path).iter().map(|part| {
            let part = part.to_string_lossy();
            let identity = part.chars().take_while(|c| c.is_ascii_hexdigit() || *c == '-').count();
            if identity >= 32 { format!("*{}", &part[identity..]) } else { part.into_owned() }
        }).collect::<Vec<_>>().join("/");
        readiness::with_unit_state_fixture(&units, "inactive", || {
            for name in PAGES {
                let server = overview_service_reply(&f.m, capacity_json(false,0,0));
                let ((calls, bytes), cpu_before, started) = (io(), cpu(), Instant::now());
                let (composed, opened) = linux_vst_bridge::with_opened_counts(|| page(&f.m, &candidates, name));
                let (wall, cpu_ms, (calls_after, bytes_after)) = (started.elapsed(), cpu() - cpu_before, io());
                server.join().unwrap();
                let (reply, opened) = (composed.unwrap(), opened.unwrap());
                let mut kinds: BTreeMap<String, (u32, u32, u32)> = BTreeMap::new();
                for (path, opens) in &opened {
                    let entry = kinds.entry(kind(path)).or_default();
                    entry.0 += 1;
                    entry.1 += opens;
                    entry.2 = entry.2.max(*opens);
                }
                eprintln!("READS {name}: {} bytes; {} files opened {} times; {} read calls, {:.1} MB; \
                    CPU {cpu_ms:.0} ms, wall {} ms", reply.len(), opened.len(), opened.values().sum::<u32>(),
                    calls_after - calls, (bytes_after - bytes) as f64 / 1e6, wall.as_millis());
                let mut rows: Vec<_> = kinds.into_iter().collect();
                rows.sort_by_key(|(_, (_, opens, _))| std::cmp::Reverse(*opens));
                for (kind, (files, opens, most)) in rows.iter().take(12) {
                    eprintln!("  {opens:>7} opens of {files:>4} files, at most {most:>4} each: {kind}");
                }
            }
        });
    }
}
