//! Current Home/Setup authority. Historical revisions, incidents and operation
//! bodies belong to the separate Diagnostics snapshot.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::MetadataExt;
use linux_vst_bridge::{catalogue, profiles, publication};

type CurrentRevisions = BTreeMap<String,
    std::result::Result<Option<publication::Revision>, &'static str>>;

#[derive(Clone, Copy, PartialEq, Eq)]
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
    paths.insert(m.root.join("registry.json"));
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
        if let Some(reference) = &entry.managed_revision {
            require(valid_hex(&reference.id,32),"operator_current_revision_identity")?;
            let base = m.root.join("publications").join(class).join("revisions").join(&reference.id);
            paths.insert(base.join("revision.json"));
            paths.insert(base.join(format!("LVB_{class}.vst3/bridge-provenance.json")));
        }
    }
    require(paths.len() <= 4096,"operator_current_watch_bound")?;
    paths.into_iter().map(|path| Ok((path.clone(),stamp(&path)?))).collect()
}

pub(super) struct CurrentOverviewContext {
    pub snapshot: ui::Snapshot,
    pub profiles: Vec<profiles::Profile>,
    pub revisions: CurrentRevisions,
    pub current_generation: String,
    owners: Vec<capacity::Owner>,
    watched: BTreeMap<PathBuf,Option<FileStamp>>,
    #[cfg(feature = "pb0-c0-audit")]
    captured_at: Instant,
}
impl CurrentOverviewContext {
    pub(super) fn recheck(&self, m: &Manager) -> Result<()> {
        let started = Instant::now();
        let _guard = acquire_readback(m, ui::OperatorLock::Registry, None,
            OPERATOR_WAIT, &mut vec![])?;
        require(self.snapshot.state_token == token(m)?
            && self.current_generation == pulse_generation(m)?
            && self.owners == capacity::owners(m)?, "operator_state_changed_refresh")?;
        for (path, before) in &self.watched {
            require(stamp(path)? == *before,"operator_current_artifact_changed_refresh")?;
        }
        #[cfg(feature = "pb0-c0-audit")]
        eprintln!("PB0_PHASE {}",json!({"token_after_ms":started.elapsed().as_millis(),
            "total_manager_ms":self.captured_at.elapsed().as_millis()}));
        #[cfg(not(feature = "pb0-c0-audit"))]
        let _ = started;
        Ok(())
    }
}

#[derive(Default)]
struct VerificationCache {
    artifacts: BTreeMap<String, bool>,
    runners: BTreeMap<String, bool>,
    environments: BTreeMap<String, bool>,
}
impl VerificationCache {
    fn artifact(&mut self, artifact: &Artifact) -> bool {
        let key = format!("{}:{}", artifact.path.display(), artifact.sha256);
        *self.artifacts.entry(key).or_insert_with(|| artifact.verify().is_ok())
    }
    fn runner(&mut self, runner: &Runner) -> Result<bool> {
        let key = catalogue::runner_key(runner)?;
        Ok(*self.runners.entry(key).or_insert_with(|| runner.verify().is_ok()))
    }
    fn environment(&mut self, environment: &Environment) -> bool {
        *self.environments.entry(environment.id.clone()).or_insert_with(||
            read_json::<Environment>(&environment.root.join("environment.json"))
                .is_ok_and(|actual| actual == *environment))
    }
}

fn physical(m: &Manager, class: &str) -> Result<Option<PathBuf>> {
    let link = m.link(class);
    match fs::symlink_metadata(&link) {
        Ok(meta) => {
            require(meta.file_type().is_symlink(), "operator_publication_not_symlink")?;
            Ok(Some(fs::read_link(link)?))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn current_products(m: &Manager, db: &Registry, sw: &Software,
    profiles: &[profiles::Profile]) -> Result<(Vec<ui::Product>, CurrentRevisions)> {
    let mut cache = VerificationCache::default();
    let mut products = Vec::new();
    let mut revisions = BTreeMap::new();
    for (class, entry) in &db.classes {
        let r = &entry.registration;
        let revision = entry.managed_revision.as_ref().map(|reference|
            m.load_revision(class, reference).map_err(|_| "READINESS_REVISION_UNAVAILABLE"))
            .transpose();
        let loaded = revision.as_ref().ok().and_then(Option::as_ref);
        let module_valid = cache.artifact(&r.module);
        let environment_valid = cache.environment(&r.environment);
        let runner_valid = cache.runner(&r.environment.runner)?;
        let native_valid = loaded.is_some() || (entry.managed_revision.is_none() && cache.artifact(&r.native));
        let host_source = Artifact { path:r.host.path.with_file_name("host-source-manifest.json"),
            sha256:r.host_source_sha256.clone() };
        let host_bytes_valid = cache.artifact(&r.host) && cache.artifact(&host_source);
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
        let actual = physical(m, class);
        let publication_valid = match (&entry.publication, &expected, &actual) {
            (Publication::Published, Some(expected), Ok(Some(actual))) => expected == actual,
            (Publication::Removed, _, Ok(None)) => true,
            _ => false,
        } && !m.publication_pending(class)?;
        let publication_selected = entry.publication == Publication::Published;
        let current_valid = module_valid && environment_valid && runner_valid
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
                "module_valid":module_valid,"environment_valid":environment_valid,
                "runner_valid":runner_valid,"native_valid":native_valid,
                "host_valid":host_valid,"publication_valid":publication_valid,
                "publication_selected":publication_selected,
                "host_sha256":r.host.sha256,"host_source_sha256":r.host_source_sha256,
                "native_sha256":r.native.sha256,"qualification":qualification,
                "publication":entry.managed_revision,
                "refusal":if current_valid {Value::Null}
                    else if !publication_selected {json!({"code":"publication_not_selected"})}
                    else {json!({"code":"current_artifact_or_publication_changed"})}}),
        });
        revisions.insert(class.clone(), revision);
    }
    Ok((products, revisions))
}

fn append_discovered(m: &Manager, sw: &Software, records: &[onboarding::Record],
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
            let stale = inventory::stale_reason(&module, &scan.environment, &scan.host,
                &scan.host_source_sha256, &record.environment, &sw.host, &sw.source_sha256);
            if module.quarantine_reason.is_some() {
                products.push(quarantined_product(&scan,index,module,stale,busy,None)?);
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
                    limitations:vec![stale.unwrap_or("Installed but not published to Bitwig").into()],
                    history:vec![],actions:vec![],compatibility:None,
                    details:json!({"scan":scan.id,"current":stale.is_none(),"observed_at":scan.completed_at}) });
            }
        }
    }
    Ok(())
}

pub(super) fn capture(m: &Manager) -> Result<CurrentOverviewContext> {
    let started = Instant::now();
    let mut phases = Vec::new();
    let _serialization = acquire_readback(m, ui::OperatorLock::Canonical, None,
        OPERATOR_WAIT, &mut vec![])?;
    let _registry_guard = acquire_readback(m, ui::OperatorLock::Registry, None,
        OPERATOR_WAIT, &mut vec![])?;
    let owners = capacity::owners(m)?;
    let db = m.registry()?;
    drop(_registry_guard);
    std::thread::scope(|scope| {
    let capacity_task = scope.spawn(|| {
        let started = Instant::now();
        (live_capacity(m).ok(), started.elapsed().as_millis())
    });
    let before = token_with_registry(m, &db)?;
    let token_at = Instant::now(); phases.push(("token_before",token_at.duration_since(started).as_millis()));
    let sw = software(m)?;
    let mut watched = watch_paths(m, &sw, &db)?;
    let catalogue = operator_catalogue(m, &sw, &db)?;
    let profiles = profiles::installed_profiles()?;
    let authority_at = Instant::now(); phases.push(("software_catalogue_profiles",authority_at.duration_since(token_at).as_millis()));
    let (mut products, revisions) = current_products(m, &db, &sw, &profiles)?;
    let product_at = Instant::now(); phases.push(("current_products",product_at.duration_since(authority_at).as_millis()));
    let pending = pending_transactions(m)?;
    let vendor = vendor_retired(m)?;
    let records = onboarding::history_records(m)?;
    for record in &records {
        let path = m.root.join("inventory").join(format!("{}.json",record.environment.id));
        watched.insert(path.clone(),stamp(&path)?);
    }
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
    append_discovered(m, &sw, &records, &mut products, None)?;
    let discovery_at = Instant::now(); phases.push(("inventory_discovery",discovery_at.duration_since(record_at).as_millis()));
    let mut onboarding = onboarding::projection_current(m, None,
        onboarding::CurrentProjectionInputs {sw:&sw,catalogue:catalogue.as_ref(),
            registry:&db,records:&records,installers:&installers}, onboarding::live)?;
    let setup_at = Instant::now(); phases.push(("current_setup",setup_at.duration_since(discovery_at).as_millis()));
    let (cap, capacity_ms) = capacity_task.join().unwrap_or((None, 0));
    let joined_at = Instant::now(); phases.push(("capacity_wait",joined_at.duration_since(setup_at).as_millis()));
    let cap = cap.filter(|read| read.owners == owners);
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
    let (workspaces, workspace_installers) = daw_workspace::current_projection(m)?;
    onboarding.retain(|row| !workspace_installers.contains(&row.installer));
    let runners = catalogue.as_ref().map(|c| &c.environments);
    let default = runners.and_then(|environments| catalogue::OnboardingRuntimePolicy::from_environments(environments).ok().flatten())
        .and_then(|policy| environments_runner(catalogue.as_ref(), &policy.default_runner_key));
    let installer_setups = onboarding::setup_projection_current(m, &onboarding, &products,
        &workspace_installers, &records, &installers, default.as_ref())?;
    let workspace_at = Instant::now(); phases.push(("workspace_and_cards",workspace_at.duration_since(joined_at).as_millis()));
    let current_generation = pulse_generation(m)?;
    #[cfg(feature = "pb0-c0-audit")]
    eprintln!("PB0_PHASE {}", serde_json::to_string(&phases)?);
    #[cfg(not(feature = "pb0-c0-audit"))]
    let _ = phases;
    let system = system_from_capacity(cap.as_ref(),pending,stale_transports(cap.as_ref())?);
    let snapshot = ui::Snapshot {schema:ui::OPERATOR_SCHEMA,state_token:before,system,
        onboarding,installer_setups,environments:vec![],vendor_applications:vec![],products,
        workspaces,active_sessions:vec![],capture:Value::Null,recent_incidents:vec![],
        actions:vec![action("Create sanitized support export",ui::Action::SupportExport {},None),
            action("Reconcile interrupted transaction",ui::Action::TransactionReconcile {},
                inactive_reason(cap.as_ref(),retired,pending,true))],
        operation:optional(&m.root.join("operator/latest.json"))?.as_object()
            .map(|v|Value::Object(v.clone()))};
    Ok(CurrentOverviewContext {snapshot,profiles,revisions,owners,watched,current_generation,
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
        let context = CurrentOverviewContext {snapshot,profiles:vec![],
            revisions:BTreeMap::new(),current_generation:pulse_generation(&fixture.m).unwrap(),
            owners:vec![],watched,
            #[cfg(feature = "pb0-c0-audit")]
            captured_at:Instant::now()};
        context.recheck(&fixture.m).unwrap();
        fs::write(&revision,b"changed revision").unwrap();
        assert!(context.recheck(&fixture.m).is_err());
        fs::write(&revision,b"retained revision").unwrap();
        fs::remove_file(&publication).unwrap();
        std::os::unix::fs::symlink("other",&publication).unwrap();
        assert!(context.recheck(&fixture.m).is_err());
    }
}
