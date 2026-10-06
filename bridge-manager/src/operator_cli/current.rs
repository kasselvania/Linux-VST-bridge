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
    paths.insert(m.root.join("software.json"));
    paths.insert(linux_vst_bridge::runtime_delivery::record_path(m));
    paths.insert(m.root.join("registry.json"));
    paths.insert(m.root.join("operator/latest.json"));
    paths.insert(m.root.join("daw-workspaces/fl-studio/workspace.json"));
    paths.insert(m.root.join("vendor-applications").join(ASC).join("operation-result.json"));
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
            }
            paths.insert(m.root.join("inventory").join(format!("{}.json",record.environment.id)));
        }
    }
    let inventory = m.root.join("inventory");
    paths.insert(inventory);
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
    paths.into_iter().map(|path| Ok((path.clone(),stamp(&path)?))).collect()
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
    vendor_retired: bool,
    all_vendor_retired: bool,
    cleanup_seen: Option<bool>,
    #[cfg(feature = "pb0-c0-audit")]
    captured_at: Instant,
}
impl CurrentOverviewContext {
    pub(super) fn capture_preparation_watches(&mut self) -> Result<()> {
        for path in self.preparation.watched_paths()? {
            if !self.watched.contains_key(&path) { self.watched.insert(path.clone(), stamp(&path)?); }
        }
        require(self.watched.len() <= 4096, "operator_current_watch_bound")
    }
    pub(super) fn class_busy(&self, class: &str) -> Option<&'static str> {
        class_inactive_reason(&self.snapshot.system, &self.owners, self.all_vendor_retired, class)
    }

    fn live_installer_progress_paths(&self, m: &Manager) -> BTreeMap<PathBuf,PathBuf> {
        self.snapshot.onboarding.iter().filter_map(|row| {
            let environment = row.environment.as_ref()?;
            let operation = row.details["installation"]["operation"].as_str()?;
            if self.installer_live.get(operation) != Some(&true) { return None; }
            let directory = m.root.join("onboarding").join(environment);
            let report = directory.join(format!("{operation}-result.json"));
            Some((directory,report))
        }).collect()
    }
    fn recheck_external(&self, m: &Manager) -> Result<()> {
        // These fixed, bounded service observations are independent. Run at
        // most four installer cohorts alongside vendor, workspace and cleanup
        // so a larger retained setup history does not create one thread per
        // attempt. None of these probes holds registry.lock.
        std::thread::scope(|scope| -> Result<()> {
            let vendor = scope.spawn(|| {
                let at = Instant::now();
                (vendor_retired(m), at.elapsed().as_millis())
            });
            let workspace = scope.spawn(|| {
                let at = Instant::now();
                (daw_workspace::current_projection(m), at.elapsed().as_millis())
            });
            let cleanup = self.cleanup_seen.map(|_| scope.spawn(|| {
                let at = Instant::now();
                (pulse_cleanup(m), at.elapsed().as_millis())
            }));
            let installer = scope.spawn(|| {
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
            require(self.vendor_retired == vendor_result?,
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
            let progress = self.live_installer_progress_paths(m);
            for (path, before) in &self.watched {
                // An active supervisor atomically replaces its progress report.
                // Those bytes cannot authorize installation or retirement:
                // the row offers only exact Focus/Stop, and both external
                // rechecks require the same operation to remain live. Keep
                // record.json and every other authority input fully watched.
                if progress.values().any(|report| report == path) { continue; }
                let after = stamp(path)?;
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
    pub(super) fn recheck(&self, m: &Manager) -> Result<()> {
        self.recheck_with(m, || self.recheck_external(m))
    }
}

fn installer_liveness(operations: &[String]) -> Result<BTreeMap<String, bool>> {
    // A fixed four-cohort limit keeps the number of supervised systemctl
    // helpers bounded while preserving exact per-operation identity.
    let chunk_size = operations.len().div_ceil(4).max(1);
    std::thread::scope(|scope| -> Result<BTreeMap<String, bool>> {
        let checks: Vec<_> = operations.chunks(chunk_size).map(|chunk| scope.spawn(move || {
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
    environments: BTreeMap<String, (Environment,bool)>,
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
        if let Some((seen, verified)) = self.environments.get(&environment.id) {
            require(seen == environment,"operator_current_environment_conflict")?;
            return Ok(*verified);
        }
        let verified = read_json::<Environment>(&environment.root.join("environment.json"))
            .is_ok_and(|actual| actual == *environment);
        self.environments.insert(environment.id.clone(),(environment.clone(),verified));
        Ok(verified)
    }
}

fn current_products(m: &Manager, db: &Registry, sw: &Software,
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
            disposition:if current_valid {"ready"} else {"needs_attention"}.into(),
            active_revision:profile.map(|p| p.revision),
            recommended_revision:recommended.map(|p| p.revision),
            environment:r.environment.id.clone(), runner:r.environment.runner.id.clone(),
            module_sha256:r.module.sha256.clone(), limitations, history:vec![], actions:vec![],
            compatibility:None,
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
        revisions.insert(class.clone(), revision);
    }
    Ok((products, revisions))
}

fn append_discovered(m: &Manager, sw: &Software, records: &[onboarding::Record],
    bindings: &[linux_vst_bridge::catalogue::EnvironmentBinding], db: &Registry,
    products: &mut Vec<ui::Product>, busy: Option<&str>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for record in records {
        if !seen.insert(&record.environment.id) { continue; }
        let path = m.root.join("inventory").join(format!("{}.json", record.environment.id));
        if !path.try_exists()? { continue; }
        let scan: inventory::Scan = read_json(&path)?;
        require(scan.schema == 1 && scan.environment.id == record.environment.id,
            "operator_inventory_environment_changed")?;
        for (index, module) in scan.modules.iter().cloned().enumerate() {
            let stale = inventory::record_stale_reason(&module, &scan.environment, &scan.host,
                &scan.host_source_sha256, &record.environment, &sw.host, &sw.source_sha256);
            if module.quarantine_reason.is_some() {
                let retry_disabled = quarantined_retry_disabled(m, bindings, db,
                    &record.environment)?;
                products.push(quarantined_product(&scan,index,module,stale,busy,
                    retry_disabled)?);
                continue;
            }
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
                    details:json!({"scan":scan.id,"current":stale.is_none(),"observed_at":scan.completed_at}) });
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
    let started = Instant::now();
    let mut phases = Vec::new();
    // Status is an observation, not a serialized user operation. Capture the
    // registry/owners together, then perform expensive and external probes
    // without action ownership. The caller rechecks exact tokens, file stamps,
    // owners and external state before exposing any offered action.
    let _registry_guard = timing::measure(Stage::CaptureRegistry, ||
        acquire_readback(m, ui::OperatorLock::Registry, None, OPERATOR_WAIT, &mut vec![]))?;
    let owners = capacity::owners(m)?;
    let db = m.registry()?;
    drop(_registry_guard);
    let current_generation = pulse_generation(m)?;
    std::thread::scope(|scope| {
    let capacity_task = scope.spawn(|| {
        let started = Instant::now();
        (live_capacity(m).ok(), started.elapsed().as_millis())
    });
    let cleanup_task = scope.spawn(|| pulse_cleanup(m));
    let workspace_task = scope.spawn(|| daw_workspace::current_projection(m));
    let before = token_with_registry(m, &db)?;
    let token_at = Instant::now(); phases.push(("token_before",token_at.duration_since(started).as_millis()));
    let authority_observed = timing::span(Stage::CaptureSoftwareCatalogue);
    let sw = software_record(m)?;
    let preparation = preparation::RecordReadback::capture(m)?;
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
    let retired = vendor && installers_retired;
    let owner_at = Instant::now(); phases.push(("current_owners",owner_at.duration_since(product_at).as_millis()));
    let installers = installer_import::list_records(m)?;
    let record_at = Instant::now(); phases.push(("setup_records",record_at.duration_since(owner_at).as_millis()));
    let managed_bindings = managed_environment_bindings(m, catalogue.as_ref(), &db)?;
    append_discovered(m, &sw, &records, &managed_bindings, &db, &mut products, None)?;
    let discovery_at = Instant::now(); phases.push(("inventory_discovery",discovery_at.duration_since(record_at).as_millis()));
    let mut installer_live = BTreeMap::new();
    let runners = catalogue.as_ref().map(|c| &c.environments);
    let legacy_default = runners.and_then(|environments| catalogue::OnboardingRuntimePolicy::from_environments(environments).ok().flatten())
        .and_then(|policy| environments_runner(catalogue.as_ref(), &policy.default_runner_key));
    let delivered = linux_vst_bridge::runtime_delivery::installed_record(m)?;
    let default = delivered.as_ref().map(|runner| -> Result<(String, Runner)> {
        Ok((catalogue::runner_key(runner)?,runner.clone())) }).transpose()?.or(legacy_default);
    let mut onboarding = onboarding::projection_current(m, None,
        onboarding::CurrentProjectionInputs {sw:&sw,default_runner:default.as_ref(),
            registry:&db,records:&records,installers:&installers}, |operation| {
                let live=onboarding::live(operation)?;
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
    let installer_setups = onboarding::setup_projection_current(m, &onboarding, &products,
        &workspace_installers, &records, &installers, default.as_ref())?;
    let workspace_at = Instant::now(); phases.push(("workspace_and_cards",workspace_at.duration_since(joined_at).as_millis()));
    #[cfg(feature = "pb0-c0-audit")]
    eprintln!("PB0_PHASE {}", serde_json::to_string(&phases)?);
    #[cfg(not(feature = "pb0-c0-audit"))]
    let _ = phases;
    let system = system_from_capacity(cap.as_ref(),pending,stale_transports(cap.as_ref())?);
    let snapshot = ui::Snapshot {schema:ui::OPERATOR_SCHEMA,state_token:before,system,
        onboarding,installer_setups,environments:vec![],vendor_applications:vec![],products,
        workspaces,active_sessions:vec![],capture:Value::Null,recent_incidents:vec![],
        actions:vec![action("Install compatibility runtime (728 MB download)",ui::Action::RuntimeInstall {},
            if delivered.is_some() {Some("The selected compatibility runtime is already installed")} else {busy}),
            action("Create sanitized support export",ui::Action::SupportExport {},None),
            action("Reconcile interrupted transaction",ui::Action::TransactionReconcile {},
                inactive_reason(cap.as_ref(),retired,pending,true))],
        operation:optional(&m.root.join("operator/latest.json"))?.as_object()
            .map(|v|Value::Object(v.clone()))};
    Ok(CurrentOverviewContext {snapshot,busy,profiles,revisions,owners,watched,current_generation,
        software:sw,registry:db,preparation,bindings:managed_bindings,
        installer_live,vendor_retired:vendor,all_vendor_retired:retired,cleanup_seen,
        #[cfg(feature = "pb0-c0-audit")]
        captured_at:started})
    })
}

fn environments_runner(catalogue: Option<&catalogue::Catalogue>, key: &str) -> Option<(String, Runner)> {
    catalogue?.environments.iter().find_map(|binding| {
        let runner = &binding.environment.runner;
        (catalogue::runner_key(runner).ok().as_deref() == Some(key))
            .then(||(key.into(),runner.clone()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
            published:false,previous_attempt:None};
        let db=m.registry().unwrap();
        let bindings=managed_environment_bindings(m,None,&db).unwrap();
        assert!(bindings.is_empty());
        let mut products=vec![];
        append_discovered(m,&sw,&[record],&bindings,&db,&mut products,None).unwrap();
        assert_eq!(products.len(),1);
        assert_eq!(products[0].actions[0].disabled_reason.as_deref(),
            Some("This environment is not currently managed"));
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
                system:system_from_capacity(None,pending,0),onboarding:vec![],
                installer_setups:vec![],environments:vec![],vendor_applications:vec![],
                products:vec![],workspaces,active_sessions:vec![],capture:Value::Null,
                recent_incidents:vec![],actions:vec![],operation:None},
            busy:None,profiles:vec![],revisions:BTreeMap::new(),
            current_generation:pulse_generation(m).unwrap(),
            software,registry:m.registry().unwrap(),preparation:preparation::RecordReadback::capture(m).unwrap(),
            bindings:vec![],
            owners:capacity::owners(m).unwrap(),watched,installer_live:BTreeMap::new(),
            vendor_retired:vendor_retired(m).unwrap(),all_vendor_retired:true,cleanup_seen:None,
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
        assert!(cache.environment(&changed).is_err());
        changed=exact.clone();
        changed.root=fixture.outer.join("other-root");
        assert!(cache.environment(&changed).is_err());
        changed=exact.clone();
        changed.runner.id.push_str("-other");
        assert!(cache.environment(&changed).is_err());
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
            installation_operation:Some(operation.clone()),published:false,previous_attempt:None}).unwrap();
        let result=onboarding_dir.join(format!("{operation}-result.json"));
        fs::write(&result,b"old result").unwrap();
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
        let vendor=m.root.join("vendor-applications").join(ASC).join("operation-result.json");
        let context=context_for_watched(m,watch_paths(m,&sw,&db).unwrap());
        let before_mode=stamp(&installer_path).unwrap();
        fs::set_permissions(&installer_path,fs::Permissions::from_mode(0o600)).unwrap();
        assert_ne!(stamp(&installer_path).unwrap(),before_mode);
        assert!(context.recheck(m).is_err());
        for path in [&vendor,&installer_record,&installer_path,&presentation,&onboarding_record,
            &result,&inventory,&pending,&performance,&workspace] {
            assert!(before.contains_key(path),"unwatched current input: {}",path.display());
            let context=context_for_watched(m,watch_paths(m,&sw,&db).unwrap());
            context.recheck(m).unwrap();
            let original=fs::read(path).ok();
            private_dir(path.parent().unwrap()).unwrap();
            fs::write(path,b"a changed current record").unwrap();
            assert_ne!(stamp(path).unwrap(),before[path]);
            assert!(context.recheck(m).is_err(),"accepted changed current input: {}",path.display());
            if let Some(bytes)=original {fs::write(path,bytes).unwrap();}
            else {
                fs::remove_file(path).unwrap();
                if path == &vendor {fs::remove_dir(path.parent().unwrap()).unwrap();}
            }
        }
        // A live installer needs recovery controls while reports and private
        // diagnostics change. Its stable record and directory custody still
        // bind the exact operation; a retired report keeps the full byte watch.
        let mut live=context_for_watched(m,watch_paths(m,&sw,&db).unwrap());
        live.installer_live.insert(operation.clone(),true);
        live.snapshot.onboarding.push(ui::Onboarding {failure:None,
            installer:installer.id.clone(),name:"Fixture".into(),byte_size:9,
            format:"pe_executable".into(),environment:Some(environment_id.clone()),
            state:"running".into(),required_human_action:"Use exact Stop".into(),
            details:json!({"installation":{"operation":operation}}),actions:vec![]});
        atomic_json(&result,&json!({"schema":2,"operation":operation,
            "state":"running","owned_live":2,"cleanup_confirmed":false})).unwrap();
        live.recheck_with(m,||Ok(())).unwrap();
        atomic_json(&result,&json!({"schema":2,"operation":operation,
            "state":"running","owned_live":7,"cleanup_confirmed":false})).unwrap();
        let private_report=onboarding_dir.join("diagnostic.private.json");
        atomic_json(&private_report,&json!({"progress":1})).unwrap();
        live.recheck_with(m,||Ok(())).unwrap();
        // The real external recheck cannot accept our simulated live unit.
        assert!(live.recheck(m).is_err());
        live.installer_live.insert(operation.clone(),false);
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
    fn current_recheck_refuses_revision_and_publication_drift_before_ready() {
        let fixture = test_fixture::Fixture::new();
        let revision = fixture.outer.join("selected-revision.json");
        let publication = fixture.outer.join("selected-publication");
        fs::write(&revision,b"retained revision").unwrap();
        std::os::unix::fs::symlink("first",&publication).unwrap();
        let watched = [revision.clone(),publication.clone()].into_iter()
            .map(|path| (path.clone(),stamp(&path).unwrap())).collect();
        let snapshot = ui::Snapshot {schema:ui::OPERATOR_SCHEMA,
            state_token:token(&fixture.m).unwrap(),
            system:system_from_capacity(None,0,0),onboarding:vec![],
            installer_setups:vec![],environments:vec![],vendor_applications:vec![],
            products:vec![],workspaces:vec![],active_sessions:vec![],
            capture:Value::Null,recent_incidents:vec![],actions:vec![],operation:None};
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
}
