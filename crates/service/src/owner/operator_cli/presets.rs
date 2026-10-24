//! Preset operations use the existing operation journal. Backup payloads and
//! file names live only in the private preset store, outside support exports.
use super::*;
use linux_vst_bridge::preset_files as files;
use ui::{
    PresetCommand as Command, PresetFiles as View, PresetLocation as Location,
    PresetProduct as Product,
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Retained {
    source: files::Binding,
    backup: String,
    count: usize,
    bytes: u64,
    link: Option<PathBuf>,
    transition: Option<Transition>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Transition {
    operation: String,
    before: files::Binding,
    after: files::Binding,
    backup: String,
}
fn key(product: &Product) -> String {
    hex(&sha2::Sha256::digest(
        format!("{}:{}", product.environment, product.class_id).as_bytes(),
    ))
}
fn store(m: &Manager) -> PathBuf {
    m.root.join("preset-files/backups")
}
fn index(m: &Manager, product: &Product) -> PathBuf {
    m.root
        .join("preset-files/current")
        .join(format!("{}.json", key(product)))
}
fn retained(m: &Manager, product: &Product) -> Result<Option<Retained>> {
    let path = index(m, product);
    if !path.try_exists()? {
        return Ok(None);
    }
    let record: Retained = serde_json::from_slice(&crate::read_control_bytes(&path, 65536)?)?;
    require(
        record.source.product.environment == product.environment
            && record.source.product.class_id == product.class_id
            && valid_hex(&record.backup, 64)
            && record.count <= 512
            && record.bytes <= 256 * 1024 * 1024,
        "preset_retained_binding",
    )?;
    Ok(Some(record))
}
fn save(m: &Manager, record: &Retained) -> Result<()> {
    let path = index(m, &record.source.product);
    private_dir(path.parent().ok_or("preset_index_parent")?)?;
    atomic_json(&path, record)
}
fn product(p: &ui::Product) -> Product {
    Product {
        environment: p.environment.clone(),
        module_sha256: p.module_sha256.clone(),
        class_id: p.class_id.clone(),
    }
}
fn profile(product: &Product) -> Result<Option<files::Profile>> {
    let matching: Vec<_> = files::profiles()?
        .into_iter()
        .filter(|p| p.module_sha256 == product.module_sha256 && p.class_id == product.class_id)
        .collect();
    require(matching.len() <= 1, "preset_profile_ambiguous")?;
    Ok(matching.into_iter().next())
}
fn binding(m: &Manager, product: &Product) -> Result<Option<files::Binding>> {
    let Some(profile) = profile(product)? else {
        return Ok(None);
    };
    let env = environment_install::target_record(m, &product.environment)?;
    let candidates = installed(m, &env)?;
    let (_, module) = candidates
        .iter()
        .find(|(found, _)| found == product)
        .ok_or("preset_current_build_unavailable")?;
    module.verify()?;
    files::Binding::observe(product.clone(), &env.root, &profile).map(Some)
}
fn unknown() -> View {
    View {location:Location::Unknown,explanation:"User preset location is not known for this plug-in build. Use the plug-in's own preset controls.".into(),user_path:None,linux_path:None,files:None,bytes:None,backup:None,missing:0,conflicts:0,actions:vec![]}
}
fn recovery_allowed(m: &Manager, record: &Retained, current: &files::Binding) -> Result<bool> {
    if record.source == *current {
        return Ok(true);
    }
    let Some(t) = record.transition.as_ref() else {
        return Ok(false);
    };
    if t.before != record.source || t.after != *current || t.backup != record.backup {
        return Ok(false);
    }
    let retained: Transition = read_json(
        &job_dir(m, &t.operation)?
            .join(format!("preset-transition-{}.json", key(&current.product))),
    )?;
    require(retained == *t, "preset_transition_record_changed")?;
    let protection: Protection = read_json(&protection_path(
        m,
        &t.operation,
        &current.product.environment,
    )?)?;
    require(
        protection.schema == 1
            && protection.operation == t.operation
            && protection.backups.contains(&record.backup)
            && protection.environment.id == current.product.environment
            && protection.environment.root
                == m.root
                    .join("environments")
                    .join(&current.product.environment),
        "preset_transition_protection_changed",
    )?;
    Ok(true)
}
fn recovery_digest(recovery: &files::Recovery) -> Result<String> {
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(recovery)?)))
}
/// Overview reads at most one small retained record per product, never presets.
pub(super) fn summary(m: &Manager, p: &mut ui::Product) {
    let key = product(p);
    let mut view = unknown();
    if let Ok(Some(record)) = retained(m, &key) {
        view.explanation="A completed user preset backup is retained. Open Presets to inspect the current folder.".into();
        view.backup = Some(record.backup);
        view.files = Some(record.count);
        view.bytes = Some(record.bytes);
    }
    p.details["preset_files"] = serde_json::to_value(view).unwrap_or(Value::Null);
}
pub(super) fn project(m: &Manager, p: &mut ui::Product) -> Result<()> {
    let value=project_known(m,&product(p)).unwrap_or_else(|_|View {location:Location::Unavailable,
        explanation:"The qualified preset folder or its backup is unavailable or changed. Refresh after reconnecting it; no files were substituted.".into(),..unknown()});
    p.actions.extend(value.actions.clone());
    p.details["preset_files"] = serde_json::to_value(value)?;
    Ok(())
}
fn project_known(m: &Manager, key: &Product) -> Result<View> {
    let Some(binding) = binding(m, key)? else {
        return Ok(unknown());
    };
    let entries = files::inspect(&binding)?;
    let record = retained(m, key)?;
    let root = binding.token()?;
    let offer = |label: &str, command: Command| {
        action(
            label,
            ui::Action::PresetFiles {
                product: key.clone(),
                expected_root: root.clone(),
                command,
            },
            None,
        )
    };
    let mut view=View {location:Location::Known,
        explanation:if binding.external_to_setup {"User preset folder outside this setup. Backups are stored separately from the setup."}else{"User preset folder inside this setup. Only the qualified user preset formats are included."}.into(),
        user_path:Some(binding.root.clone()),linux_path:None,files:Some(entries.len()),bytes:Some(entries.iter().map(|f|f.bytes).sum()),backup:None,missing:0,conflicts:0,
        actions:vec![offer("Back up now",Command::Backup),offer("Import presets…",Command::Import)]};
    if let Some(record) = record {
        let backup = files::verify(&store(m), &record.backup)?;
        require(
            backup.source == record.source,
            "preset_backup_source_binding",
        )?;
        view.backup = Some(record.backup.clone());
        if let Some(link) = record
            .link
            .as_ref()
            .filter(|link| std::fs::read_link(link).ok().as_ref() == Some(&binding.root))
        {
            view.linux_path = Some(link.clone());
            view.actions.push(offer("Open folder", Command::Open));
        }
        if recovery_allowed(m, &record, &binding)? {
            let recovery = files::recovery(&binding, &backup)?;
            view.missing = recovery.missing.len();
            view.conflicts = recovery.conflicts;
            if !recovery.missing.is_empty() {
                view.actions.push(offer(
                    "Restore missing presets…",
                    Command::Restore {
                        backup: record.backup,
                        missing_digest: recovery_digest(&recovery)?,
                        confirmed: false,
                    },
                ));
            }
        }
    }
    if view.linux_path.is_none() {
        view.actions.push(offer("Link Linux folder", Command::Link));
    }
    Ok(view)
}
fn prepare_backup(m: &Manager, binding: &files::Binding, operation: &str) -> Result<Retained> {
    private_dir(&store(m))?;
    let backup = files::backup(&store(m), binding, operation)?;
    let prior = retained(m, &binding.product)?;
    let record = Retained {
        source: binding.clone(),
        backup: backup.digest()?,
        count: backup.files.len(),
        bytes: backup.total_bytes,
        link: prior.and_then(|p| p.link),
        transition: None,
    };
    Ok(record)
}
#[cfg(test)]
fn retain_backup(m: &Manager, binding: &files::Binding, operation: &str) -> Result<Retained> {
    let record = prepare_backup(m, binding, operation)?;
    save(m, &record)?;
    Ok(record)
}
fn normalized(action: &ui::Action) -> ui::Action {
    let mut action = action.clone();
    if let ui::Action::PresetFiles {
        command: Command::Restore { confirmed, .. },
        ..
    } = &mut action
    {
        *confirmed = false;
    }
    action
}
/// Synchronous bounded file work retains an ordinary operation. The selected
/// descriptor never crosses a pathname-based staging boundary or supervisor.
pub(crate) fn answer(
    m: &Manager,
    request: ui::Request,
    name: Option<String>,
    input: Option<fs::File>,
) -> Result<ui::PresetResult> {
    operator_wait(|| answer_waiting(m, request, name, input))
}
fn answer_waiting(
    m: &Manager,
    request: ui::Request,
    name: Option<String>,
    input: Option<fs::File>,
) -> Result<ui::PresetResult> {
    let mut folder = None;
    let receipt = dispatch_recorded(m, &request, |operation| {
        let _dispatch = m.lock("operator-dispatch.lock")?;
        let _canonical = m.lock("operator-canonical.lock")?;
        let ui::Action::PresetFiles {
            product: key,
            expected_root,
            command,
        } = &request.action
        else {
            return Err("preset_action_required".into());
        };
        require(request.schema == ui::OPERATOR_SCHEMA, "preset_schema")?;
        let detail = product_detail(m, &key.environment, &key.module_sha256, &key.class_id)?;
        require(
            detail.state_token == request.state_token,
            "operator_stale_request_refresh",
        )?;
        require(
            detail.product.actions.iter().any(|offer| {
                offer.disabled_reason.is_none() && offer.action == normalized(&request.action)
            }),
            "preset_offer_changed_refresh",
        )?;
        let _environment = maintenance::action_lock(m, &request.action)?;
        let registry = m.lock("registry.lock")?;
        let env = environment_install::target_record(m, &key.environment)?;
        require(
            !maintenance::blocks(m, &env)? && !vendor_maintenance::blocks(m, &env)?,
            "preset_setup_maintenance_active",
        )?;
        let scope = Scope::capture(m, &env)?;
        drop(registry);
        let bound = binding(m, key)?.ok_or("preset_location_unknown")?;
        require(
            bound.token()? == *expected_root,
            "preset_root_changed_refresh",
        )?;
        require(
            (name.is_some() && input.is_some()) == matches!(command, Command::Import),
            "preset_selected_file_required",
        )?;
        write_operation(
            m,
            operation,
            &json!({"schema":1,"operation":operation,"state":"running","stage":"preset_files","action":request.action}),
            true,
        )?;
        let mut failure_step = "copying preset files";
        let result: Result<()> = (|| {
            let mut pending = None;
            let rejoin = || {
                let registry = m.lock("registry.lock")?;
                scope.recheck(m)?;
                bound.revalidate()?;
                Ok(registry)
            };
            match command {
                Command::Backup => {
                    pending = Some(prepare_backup(m, &bound, operation)?);
                }
                Command::Import => files::import_with(
                    &bound,
                    name.as_deref().ok_or("preset_name_missing")?,
                    input.as_ref().ok_or("preset_descriptor_missing")?,
                    rejoin,
                )?,
                Command::Link => {
                    let mut record = prepare_backup(m, &bound, operation)?;
                    let home = m.root.ancestors().nth(4).ok_or("preset_home_unavailable")?;
                    require(
                        home.is_absolute()
                            && m.root == home.join(".local/share/linux-vst-bridge/managed"),
                        "preset_home_binding",
                    )?;
                    let parent = files::linux_parent(home)?;
                    // An exact identity suffix avoids collisions without replacing
                    // or automatically renaming any existing unrelated folder.
                    let label = if installer_import::valid_label(&detail.product.name) {
                        detail.product.name.as_str()
                    } else {
                        bound.profile.id.as_str()
                    };
                    let name = format!(
                        "{} ({}-{})",
                        label,
                        &key.environment[..8],
                        &key.class_id[..8]
                    );
                    record.link = Some(files::link_checked(&bound, &parent, &name, rejoin)?);
                    pending = Some(record);
                }
                Command::Open => {
                    let record = retained(m, key)?.ok_or("preset_link_missing")?;
                    let link = record.link.ok_or("preset_link_missing")?;
                    require(
                        std::fs::read_link(&link)? == bound.root,
                        "preset_link_changed",
                    )?;
                    // Return the qualified target, not an alias that could be
                    // redirected between validation and desktop dispatch.
                    folder = Some(bound.root.clone());
                }
                Command::Restore {
                    backup,
                    missing_digest,
                    confirmed,
                } => {
                    require(*confirmed, "preset_restore_confirmation_required")?;
                    let record = retained(m, key)?.ok_or("preset_backup_missing")?;
                    require(
                        record.backup == *backup && recovery_allowed(m, &record, &bound)?,
                        "preset_restore_transition_required",
                    )?;
                    let saved = files::verify(&store(m), backup)?;
                    require(
                        saved.source == record.source,
                        "preset_backup_source_binding",
                    )?;
                    let recovery = files::recovery(&bound, &saved)?;
                    require(
                        !recovery.missing.is_empty()
                            && recovery_digest(&recovery)? == *missing_digest,
                        "preset_missing_files_changed_refresh",
                    )?;
                    files::restore_missing_checked(&store(m), &bound, backup, &recovery, rejoin)?;
                }
            }
            failure_step = "reacquiring setup custody";
            let _registry = m.lock("registry.lock")?;
            failure_step = "checking the setup after copying";
            scope.recheck(m)?;
            failure_step = "checking the preset folder after copying";
            bound.revalidate()?;
            if let Some(record) = pending {
                failure_step = "recording the completed backup";
                save(m, &record)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                write_operation(
                    m,
                    operation,
                    &json!({"schema":1,"operation":operation,"state":"completed","stage":"preset_files","action":request.action,"result":{"preset_files":"completed"}}),
                    true,
                )?;
                Ok(ui::Receipt {
                    schema: ui::OPERATOR_SCHEMA,
                    accepted: true,
                    operation: Some(operation.into()),
                    refusal: None,
                })
            }
            Err(error) => {
                // Never export source filenames or OS errors containing them.
                let reason = if linux_vst_bridge::operator_lock::is_busy(error.as_ref()) {
                    linux_vst_bridge::operator_lock::BUSY_SENTENCE.to_owned()
                } else {
                    format!("Preset operation stopped while {failure_step}. Refresh Presets.")
                };
                write_operation(
                    m,
                    operation,
                    &json!({"schema":1,"operation":operation,"state":"refused","stage":"preset_files","action":request.action,"reason":reason}),
                    true,
                )?;
                Ok(ui::Receipt {schema:ui::OPERATOR_SCHEMA,accepted:false,operation:Some(operation.into()),refusal:Some(reason)})
            }
        }
    })?;
    Ok(ui::PresetResult {
        schema: ui::OPERATOR_SCHEMA,
        receipt,
        folder,
    })
}

/// Small owner facts captured under registry custody, then compared at rejoin.
/// No preset or module payload is read while holding that global lock.
#[derive(Debug)]
struct Scope {
    environment: Environment,
    publications: Value,
    stamps: Vec<(PathBuf, Option<crate::DigestFileIdentity>)>,
}
impl Scope {
    fn capture(m: &Manager, env: &Environment) -> Result<Self> {
        let environment = environment_install::current_environment(m, env)?;
        let entries: std::collections::BTreeMap<_, _> = m
            .registry()?
            .classes
            .into_iter()
            .filter(|(_, e)| e.registration.environment.id == env.id)
            .collect();
        for class in entries.keys() {
            require(!m.publication_pending(class)?, "preset_publication_pending")?;
        }
        let mut paths = std::collections::BTreeSet::new();
        paths.insert(env.root.join("environment.json"));
        paths.insert(m.root.join("inventory").join(format!("{}.json", env.id)));
        for (_, module) in installed(m, &environment)? {
            paths.insert(module.path);
        }
        let stamps = paths
            .into_iter()
            .map(|path| Ok((path.clone(), stamp(&path)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            environment,
            publications: serde_json::to_value(entries)?,
            stamps,
        })
    }
    fn recheck(&self, m: &Manager) -> Result<()> {
        let fresh = Manager::new(m.root.clone(), m.publications.clone());
        let current = Self::capture(&fresh, &self.environment)?;
        require(
            current.environment == self.environment
                && current.publications == self.publications
                && current.stamps == self.stamps,
            "preset_setup_or_publication_changed",
        )
    }
}
fn stamp(path: &std::path::Path) -> Result<Option<crate::DigestFileIdentity>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(crate::DigestFileIdentity::from(&meta))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
#[derive(Debug)]
pub(crate) struct ProtectionReady {
    scope: Scope,
    roots: Vec<files::Binding>,
}
impl ProtectionReady {
    /// Caller holds registry custody, and keeps the existing setup lock or
    /// reservation throughout preparation and this final authority check.
    pub(crate) fn recheck(&self, m: &Manager) -> Result<()> {
        self.scope.recheck(m)?;
        for root in &self.roots {
            root.revalidate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Protection {
    schema: u32,
    operation: String,
    environment: Environment,
    /// Each item is an independently verified backup, not a prefix snapshot.
    backups: Vec<String>,
    unknown_products: usize,
}
fn protection_path(m: &Manager, operation: &str, environment: &str) -> Result<PathBuf> {
    require(
        valid_product_environment(environment),
        "preset_protection_environment",
    )?;
    Ok(job_dir(m, operation)?.join(format!("preset-protection-{environment}.json")))
}
fn installed(m: &Manager, env: &Environment) -> Result<Vec<(Product, Artifact)>> {
    let mut products = std::collections::BTreeMap::new();
    for entry in m
        .registry()?
        .classes
        .values()
        .filter(|e| e.registration.environment == *env)
    {
        let r = &entry.registration;
        products.insert(
            r.metadata.class_id.clone(),
            (
                Product {
                    environment: env.id.clone(),
                    module_sha256: r.module.sha256.clone(),
                    class_id: r.metadata.class_id.clone(),
                },
                r.module.clone(),
            ),
        );
    }
    let scan = m.root.join("inventory").join(format!("{}.json", env.id));
    if scan.try_exists()? {
        let scan: inventory::Scan = read_json(&scan)?;
        if scan.environment == *env {
            // The current scanner's exact build supersedes an old registration.
            for module in scan.modules {
                for class in module.classes {
                    products.insert(
                        class.id.clone(),
                        (
                            Product {
                                environment: env.id.clone(),
                                module_sha256: module.artifact.sha256.clone(),
                                class_id: class.id,
                            },
                            module.artifact.clone(),
                        ),
                    );
                }
            }
        }
    }
    require(products.len() <= 128, "preset_product_bound")?;
    Ok(products.into_values().collect())
}
/// Called by the actual mutation owners after affected setup retirement and
/// before mutation. An unknown profile is recorded, never sold as protection.
pub(crate) fn protect_environment(
    m: &Manager,
    env: &Environment,
    operation: &str,
) -> Result<ProtectionReady> {
    let scope = {
        let _registry = m.lock("registry.lock")?;
        Scope::capture(m, env)?
    };
    let path = protection_path(m, operation, &env.id)?;
    let existing = path.try_exists()?;
    let protection = if existing {
        let retained: Protection =
            serde_json::from_slice(&crate::read_control_bytes(&path, 65536)?)?;
        require(retained.environment == *env, "preset_protection_changed")?;
        verify_protection(m, &retained, operation)?;
        retained
    } else {
        collect_environment(m, env, operation)?
    };
    let mut records = Vec::new();
    for digest in &protection.backups {
        let backup = files::verify(&store(m), digest)?;
        let prior = retained(m, &backup.source.product)?;
        records.push(Retained {
            source: backup.source,
            backup: digest.clone(),
            count: backup.files.len(),
            bytes: backup.total_bytes,
            link: prior.and_then(|p| p.link),
            transition: None,
        });
    }
    let ready = ProtectionReady {
        scope,
        roots: records.iter().map(|r| r.source.clone()).collect(),
    };
    let _registry = m.lock("registry.lock")?;
    ready.recheck(m)?;
    if !existing {
        require(!path.try_exists()?, "preset_protection_changed")?;
        for record in &records {
            save(m, record)?;
        }
        private_dir(path.parent().ok_or("preset_protection_parent")?)?;
        atomic_json(&path, &protection)?;
        let retained: Protection =
            serde_json::from_slice(&crate::read_control_bytes(&path, 65536)?)?;
        require(
            retained.backups == protection.backups
                && retained.environment == *env
                && retained.operation == operation,
            "preset_protection_not_retained",
        )?;
    }
    Ok(ready)
}
fn collect_environment(m: &Manager, env: &Environment, operation: &str) -> Result<Protection> {
    let mut protection = Protection {
        schema: 1,
        operation: operation.into(),
        environment: env.clone(),
        backups: vec![],
        unknown_products: 0,
    };
    for (product, module) in installed(m, env)? {
        let Some(profile) = profile(&product)? else {
            protection.unknown_products += 1;
            continue;
        };
        module.verify()?;
        let bound = files::Binding::observe(product, &env.root, &profile)?;
        // Independent identities permit multiple products in a shared setup.
        // Immutable copy first; only the mutation owner may publish a recovery
        // pointer after exact revalidation under its short registry admission.
        private_dir(&store(m))?;
        let digest = files::backup(&store(m), &bound, operation)?.digest()?;
        protection.backups.push(digest);
    }
    verify_protection(m, &protection, operation)?;
    Ok(protection)
}
pub(crate) fn verify_protection(
    m: &Manager,
    protection: &Protection,
    operation: &str,
) -> Result<()> {
    require(
        protection.schema == 1
            && protection.operation == operation
            && valid_product_environment(&protection.environment.id)
            && protection
                .backups
                .len()
                .saturating_add(protection.unknown_products)
                <= 128,
        "preset_protection_changed",
    )?;
    let mut products = std::collections::BTreeSet::new();
    for digest in &protection.backups {
        let backup = files::verify(&store(m), digest)?;
        require(
            backup.operation == operation
                && backup.source.product.environment == protection.environment.id
                && products.insert(backup.source.product.class_id),
            "preset_protection_source",
        )?;
    }
    Ok(())
}
/// Before forward package mutation, protection must still cover the observed
/// current roots and bytes. Recovery only verifies immutable backup custody.
pub(crate) fn verify_package_sources(m: &Manager, protection: &Protection) -> Result<()> {
    for digest in &protection.backups {
        let backup = files::verify(&store(m), digest)?;
        require(
            binding(m, &backup.source.product)?.as_ref() == Some(&backup.source)
                && files::inspect(&backup.source)? == backup.files,
            "preset_package_source_changed",
        )?;
    }
    Ok(())
}
/// Link a completed observed successor only to this operation's own backup.
/// A fresh scan alone cannot authorize arbitrary historical preset recovery.
pub(crate) fn after_maintenance(m: &Manager, env: &Environment, operation: &str) -> Result<()> {
    after_maintenance_with(m, env, operation, installed(m, env)?)
}
pub(crate) fn after_publication(
    m: &Manager,
    registration: &Registration,
    operation: &str,
) -> Result<()> {
    after_maintenance_with(
        m,
        &registration.environment,
        operation,
        vec![(
            Product {
                environment: registration.environment.id.clone(),
                module_sha256: registration.module.sha256.clone(),
                class_id: registration.metadata.class_id.clone(),
            },
            registration.module.clone(),
        )],
    )
}
fn after_maintenance_with(
    m: &Manager,
    env: &Environment,
    operation: &str,
    current: Vec<(Product, Artifact)>,
) -> Result<()> {
    let path = protection_path(m, operation, &env.id)?;
    if !path.try_exists()? {
        return Ok(());
    }
    let scope = {
        let _registry = m.lock("registry.lock")?;
        Scope::capture(m, env)?
    };
    let protection: Protection = serde_json::from_slice(&crate::read_control_bytes(&path, 65536)?)?;
    require(
        protection.schema == 1
            && protection.operation == operation
            && protection.environment.id == env.id
            && protection.environment.root == env.root,
        "preset_transition_operation",
    )?;
    for digest in protection.backups {
        let saved = files::verify(&store(m), &digest)?;
        let Some((product, module)) = current
            .iter()
            .find(|(p, _)| p.class_id == saved.source.product.class_id)
        else {
            continue;
        };
        let Some(profile) = profile(product)? else {
            continue;
        };
        module.verify()?;
        let Ok(after) = files::Binding::observe(product.clone(), &env.root, &profile) else {
            continue;
        };
        if after.root != saved.source.root
            || after.profile.compatibility != saved.source.profile.compatibility
        {
            continue;
        }
        let mut record = retained(m, product)?.ok_or("preset_transition_backup_missing")?;
        // Do not overwrite a newer backup from a separate operation.
        if record.backup != digest {
            continue;
        }
        let _registry = m.lock("registry.lock")?;
        scope.recheck(m)?;
        after.revalidate()?;
        require(
            retained(m, product)?.is_some_and(|current| current.backup == record.backup),
            "preset_recovery_pointer_changed",
        )?;
        let transition = Transition {
            operation: operation.into(),
            before: saved.source,
            after,
            backup: digest,
        };
        atomic_json(
            &job_dir(m, operation)?.join(format!("preset-transition-{}.json", key(product))),
            &transition,
        )?;
        record.transition = Some(transition);
        save(m, &record)?;
    }
    Ok(())
}
/// Package-only changes keep audio running. This is read/copy/verify work;
/// it does not acquire a setup maintenance reservation or stop another setup.
pub(crate) fn protect_package(m: &Manager, operation: &str) -> Result<Vec<Protection>> {
    let mut environments = std::collections::BTreeMap::new();
    for entry in m.registry()?.classes.values() {
        environments.insert(
            entry.registration.environment.id.clone(),
            entry.registration.environment.clone(),
        );
    }
    require(environments.len() <= 128, "preset_environment_bound")?;
    environments
        .values()
        .map(|env| collect_environment(m, env, operation))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (test_fixture::Fixture, Product, files::Profile) {
        let (f, candidate) =
            test_fixture::projection_candidate_in(test_fixture::Fixture::installed(), false);
        preparation::record_candidate(&f.m, &candidate).unwrap();
        preparation::enable(&f.m, &candidate, false).unwrap();
        let selected = &candidate.selection;
        let product = Product {
            environment: selected.environment.id.clone(),
            module_sha256: selected.module.sha256.clone(),
            class_id: selected.class.id.clone(),
        };
        let profile = files::Profile {
            id: "reference-user-presets".into(),
            module_sha256: product.module_sha256.clone(),
            class_id: product.class_id.clone(),
            root: files::Root::Setup {
                relative: "reference-user-presets".into(),
            },
            extensions: vec!["lvbpreset".into()],
            compatibility: "reference-v1".into(),
        };
        private_dir(&selected.environment.root.join("reference-user-presets")).unwrap();
        (f, product, profile)
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn preset_folder_changes_notify_an_open_product_and_refresh_restore_offers() {
        use linux_vst_bridge::service::{changes::Changes, view::Views};
        let (f, key, profile) = fixture();
        files::with_profiles(vec![profile], || {
            let views = Views::new();
            // No prefix tree watch: coverage must come from the actual product
            // read, including an empty preset folder and newly added subfolders.
            views.cover(Changes::new(vec![]));
            let request = contract::Read::Product(contract::ProductKey {
                environment: key.environment.clone(), module_sha256: key.module_sha256.clone(),
                class_id: key.class_id.clone(),
            });
            let read = || {
                let reply = views.answer(&request, 0, || super::super::view(&f.m, &request)).unwrap();
                serde_json::from_slice::<ui::CurrentProductDetail>(&reply).unwrap()
            };
            let presets = |detail: &ui::CurrentProductDetail| -> View {
                serde_json::from_value(detail.product.details["preset_files"].clone()).unwrap()
            };
            let root = f.r.environment.root.join("reference-user-presets");
            assert_eq!(presets(&read()).files, Some(0));
            let revision = views.records_revision().unwrap();
            read();
            assert_eq!(views.records_revision(), Some(revision), "quiet reads do not notify again");

            fs::write(root.join("fixture.lvbpreset"), b"first party fixture").unwrap();
            assert!(views.records_revision().unwrap() > revision, "external save must notify Console");
            assert_eq!(presets(&read()).files, Some(1));
            let bound = binding(&f.m, &key).unwrap().unwrap();
            retain_backup(&f.m, &bound, &"71".repeat(16)).unwrap();
            read();
            let revision = views.records_revision().unwrap();
            fs::remove_file(root.join("fixture.lvbpreset")).unwrap();
            assert!(views.records_revision().unwrap() > revision, "external removal must notify Console");
            let detail = read();
            let missing = presets(&detail);
            assert_eq!((missing.files, missing.missing), (Some(0), 1));
            let mut restore = missing.actions.iter().find(|offer| matches!(offer.action,
                ui::Action::PresetFiles {command: Command::Restore {..}, ..})).unwrap().action.clone();
            let ui::Action::PresetFiles {command: Command::Restore {confirmed, ..}, ..} = &mut restore else { unreachable!() };
            *confirmed = true;
            assert!(answer(&f.m, ui::Request {schema:ui::OPERATOR_SCHEMA,
                state_token:detail.state_token, action:restore}, None, None).unwrap().receipt.accepted);
            assert_eq!(fs::read(root.join("fixture.lvbpreset")).unwrap(), b"first party fixture");
            let restored = presets(&read());
            assert_eq!((restored.files, restored.missing), (Some(1), 0));

            let revision = views.records_revision().unwrap();
            fs::create_dir(root.join("Bass")).unwrap();
            assert!(views.records_revision().unwrap() > revision);
            read();
            let revision = views.records_revision().unwrap();
            fs::write(root.join("Bass/second.lvbpreset"), b"nested").unwrap();
            assert!(views.records_revision().unwrap() > revision, "empty nested folders need coverage too");
            assert_eq!(presets(&read()).files, Some(2));
            let revision = views.records_revision().unwrap();
            fs::write(root.join("Bass/second.lvbpreset"), b"changed").unwrap();
            assert!(views.records_revision().unwrap() > revision, "an in-place edit must notify");
            assert_eq!(presets(&read()).bytes, Some(26));

            let revision = views.records_revision().unwrap();
            fs::rename(&root, root.with_extension("retained")).unwrap();
            assert!(views.records_revision().unwrap() > revision);
            assert_eq!(presets(&read()).location, Location::Unavailable);
            let revision = views.records_revision().unwrap();
            private_dir(&root).unwrap();
            assert!(views.records_revision().unwrap() > revision, "a missing root returning must notify");
            let replaced = presets(&read());
            assert_eq!((replaced.location, replaced.files), (Location::Known, Some(0)));
            assert!(!replaced.actions.iter().any(|offer| matches!(offer.action,
                ui::Action::PresetFiles {command: Command::Restore {..}, ..})),
                "notification must not authorize restoring into a substituted root");
        });
    }
    #[test]
    fn gated_preset_copy_releases_registry_keeps_setup_custody_and_rechecks_owner() {
        for change in ["none", "reader_unwaited", "reader", "setup", "root", "publication"] {
            let (f, key, profile) = fixture();
            files::with_profiles(vec![profile], || {
                let root = f.r.environment.root.join("reference-user-presets");
                fs::write(root.join("fixture.lvbpreset"), b"first party fixture").unwrap();
                let detail =
                    product_detail(&f.m, &key.environment, &key.module_sha256, &key.class_id)
                        .unwrap();
                let action = detail
                    .product
                    .actions
                    .iter()
                    .find(|a| {
                        matches!(
                            a.action,
                            ui::Action::PresetFiles {
                                command: Command::Backup,
                                ..
                            }
                        )
                    })
                    .unwrap()
                    .action
                    .clone();
                let manager = Manager::new(f.m.root.clone(), f.m.publications.clone());
                let environment = f.r.environment.clone();
                let class = key.class_id.clone();
                let (began, copying) = std::sync::mpsc::channel();
                let (resume, resumed) = std::sync::mpsc::channel();
                files::COPY_CHECKPOINT.with(|hook| {
                    *hook.borrow_mut() = Some(Box::new(move || {
                        began.send(()).unwrap();
                        resumed.recv_timeout(Duration::from_secs(10)).unwrap();
                    }))
                });
                let observer = std::thread::spawn(move || {
                    copying.recv_timeout(Duration::from_secs(10)).unwrap();
                    // These are the actual status and admission registry readers,
                    // not a timing assertion or a longer timeout workaround.
                    assert!(capacity::status_observed(
                        &manager,
                        capacity::fixture_limits(),
                        0,
                        false
                    )
                    .is_ok());
                    let admission =
                        capacity::reserve(&manager, &capacity::fixture_limits(), None, false)
                            .unwrap();
                    assert!(manager
                        .lock(&format!("operator-environment-{}.lock", environment.id))
                        .is_err());
                    let unrelated = manager
                        .lock(&format!("operator-environment-{}.lock", "cd".repeat(16)))
                        .unwrap();
                    drop(unrelated);
                    match change {
                        "setup" => {
                            let mut next = environment.clone();
                            next.revision += 1;
                            atomic_json(&environment.root.join("environment.json"), &next).unwrap();
                        }
                        "root" => {
                            fs::rename(&root, root.with_extension("retained")).unwrap();
                            private_dir(&root).unwrap();
                        }
                        "publication" => {
                            let mut registry = manager.registry().unwrap();
                            registry.classes.get_mut(&class).unwrap().publication =
                                Publication::Removed;
                            atomic_json(&manager.root.join("registry.json"), &registry).unwrap();
                        }
                        _ => {}
                    }
                    drop(admission);
                    // A refresh may already own the registry when the copy
                    // finishes. It must delay completion, not discard it.
                    let reader = matches!(change, "reader" | "reader_unwaited")
                        .then(|| manager.lock("registry.lock").unwrap());
                    resume.send(()).unwrap();
                    if reader.is_some() {
                        std::thread::sleep(Duration::from_millis(250));
                    }
                    drop(reader);
                });
                // The unscoped counterexample reproduces the old transport
                // path. The public action must wait through the same reader.
                let dispatch = if change == "reader_unwaited" { answer_waiting } else { answer };
                let receipt = dispatch(
                    &f.m,
                    ui::Request {
                        schema: ui::OPERATOR_SCHEMA,
                        state_token: detail.state_token,
                        action,
                    },
                    None,
                    None,
                )
                .unwrap();
                observer.join().unwrap();
                assert_eq!(receipt.receipt.accepted, matches!(change, "none" | "reader"), "{change}");
                assert_eq!(
                    index(&f.m, &key).exists(),
                    matches!(change, "none" | "reader"),
                    "stale copy cannot publish recovery ownership"
                );
            });
        }
    }
    #[test]
    fn preset_publication_holds_rejoin_guard_and_refuses_preceding_competitor() {
        use files::PublicationBoundary::{Admitted, Prepared, Published};
        for kind in ["import", "restore", "link"] {
            for stale in [false, true] {
                let (f, key, profile) = fixture();
                files::with_profiles(vec![profile], || {
                    let root = f.r.environment.root.join("reference-user-presets");
                    let destination = root.join("fixture.lvbpreset");
                    let source = f.outer.join("selected.lvbpreset");
                    fs::write(&source, b"first party fixture").unwrap();
                    if kind == "restore" {
                        fs::write(&destination, b"first party fixture").unwrap();
                        let bound = binding(&f.m, &key).unwrap().unwrap();
                        retain_backup(&f.m, &bound, &"71".repeat(16)).unwrap();
                        fs::remove_file(&destination).unwrap();
                    }
                    let detail =
                        product_detail(&f.m, &key.environment, &key.module_sha256, &key.class_id)
                            .unwrap();
                    let mut action = detail
                        .product
                        .actions
                        .iter()
                        .find(|a| match &a.action {
                            ui::Action::PresetFiles { command, .. } => matches!(
                                (kind, command),
                                ("import", Command::Import)
                                    | ("restore", Command::Restore { .. })
                                    | ("link", Command::Link)
                            ),
                            _ => false,
                        })
                        .unwrap()
                        .action
                        .clone();
                    if let ui::Action::PresetFiles {
                        command: Command::Restore { confirmed, .. },
                        ..
                    } = &mut action
                    {
                        *confirmed = true;
                    }
                    let destination = if kind == "link" {
                        f.m.root
                            .ancestors()
                            .nth(4)
                            .unwrap()
                            .join("Music/Plug-in Presets")
                            .join(format!(
                                "{} ({}-{})",
                                detail.product.name,
                                &key.environment[..8],
                                &key.class_id[..8]
                            ))
                    } else {
                        destination
                    };
                    let manager = Manager::new(f.m.root.clone(), f.m.publications.clone());
                    let class = key.class_id.clone();
                    let (reached, boundary) = std::sync::mpsc::channel();
                    let (resume, resumed) = std::sync::mpsc::channel();
                    files::PUBLICATION_CHECKPOINT.with(|hook| {
                        *hook.borrow_mut() = Some(Box::new(move |alias, stage| {
                            if alias != (kind == "link") {
                                return;
                            }
                            reached.send(stage).unwrap();
                            resumed.recv_timeout(Duration::from_secs(10)).unwrap();
                        }));
                    });
                    let observer = std::thread::spawn(move || {
                        assert!(
                            boundary.recv_timeout(Duration::from_secs(10)).unwrap() == Prepared
                        );
                        {
                            // A real registry competitor can enter after copy/fsync,
                            // before the exact owner rejoin. Its change must refuse publication.
                            let _guard = manager.lock("registry.lock").unwrap();
                            assert!(!destination.try_exists().unwrap());
                            if stale {
                                let mut registry = manager.registry().unwrap();
                                registry.classes.get_mut(&class).unwrap().publication =
                                    Publication::Removed;
                                atomic_json(&manager.root.join("registry.json"), &registry)
                                    .unwrap();
                            }
                        }
                        resume.send(()).unwrap();
                        if stale {
                            return destination;
                        }
                        assert!(
                            boundary.recv_timeout(Duration::from_secs(10)).unwrap() == Admitted
                        );
                        assert!(!destination.try_exists().unwrap());
                        assert!(
                            manager.lock("registry.lock").is_err(),
                            "competitor entered before preset publication"
                        );
                        resume.send(()).unwrap();
                        assert!(
                            boundary.recv_timeout(Duration::from_secs(10)).unwrap() == Published
                        );
                        {
                            // Publication is visible before another registry writer enters;
                            // directory fsync and subsequent hashing have no global custody.
                            let _guard = manager.lock("registry.lock").unwrap();
                            assert!(destination.try_exists().unwrap());
                        }
                        resume.send(()).unwrap();
                        destination
                    });
                    let receipt = answer(
                        &f.m,
                        ui::Request {
                            schema: ui::OPERATOR_SCHEMA,
                            state_token: detail.state_token,
                            action,
                        },
                        (kind == "import").then(|| "fixture.lvbpreset".into()),
                        (kind == "import").then(|| fs::File::open(&source).unwrap()),
                    )
                    .unwrap();
                    files::PUBLICATION_CHECKPOINT.with(|hook| *hook.borrow_mut() = None);
                    let destination = observer.join().unwrap();
                    assert_eq!(
                        receipt.receipt.accepted, !stale,
                        "{kind} stale={stale}: {:?}",
                        receipt.receipt
                    );
                    assert_eq!(destination.try_exists().unwrap(), !stale);
                    assert_eq!(fs::read(&source).unwrap(), b"first party fixture");
                });
            }
        }
    }
    #[test]
    fn gated_maintenance_backup_rejoins_without_holding_global_registry() {
        let (f, key, profile) = fixture();
        files::with_profiles(vec![profile], || {
            let bound = binding(&f.m, &key).unwrap().unwrap();
            fs::write(bound.root.join("fixture.lvbpreset"), b"first party fixture").unwrap();
            let _setup =
                f.m.lock(&format!("operator-environment-{}.lock", key.environment))
                    .unwrap();
            let manager = Manager::new(f.m.root.clone(), f.m.publications.clone());
            let environment = f.r.environment.clone();
            files::COPY_CHECKPOINT.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move || {
                    assert!(capacity::status_observed(
                        &manager,
                        capacity::fixture_limits(),
                        0,
                        false
                    )
                    .is_ok());
                    let _registry =
                        capacity::reserve(&manager, &capacity::fixture_limits(), None, false)
                            .unwrap();
                    assert!(manager
                        .lock(&format!("operator-environment-{}.lock", environment.id))
                        .is_err());
                }))
            });
            let operation = "68".repeat(16);
            let ready = protect_environment(&f.m, &f.r.environment, &operation).unwrap();
            let _registry = f.m.lock("registry.lock").unwrap();
            ready.recheck(&f.m).unwrap();
            let mut changed = f.r.environment.clone();
            changed.revision += 1;
            atomic_json(&changed.root.join("environment.json"), &changed).unwrap();
            assert!(ready.recheck(&f.m).is_err());
        });
    }
    #[test]
    fn owner_page_import_backup_missing_restore_and_stale_action() {
        let (f, key, profile) = fixture();
        files::with_profiles(vec![profile], || {
            let root = f.r.environment.root.join("reference-user-presets");
            fs::write(root.join("existing.lvbpreset"), b"existing").unwrap();
            let selected = f.outer.join("selected.lvbpreset");
            fs::write(&selected, b"selected").unwrap();
            let run = |command: Command, name: Option<&str>, input: Option<fs::File>| {
                let detail =
                    product_detail(&f.m, &key.environment, &key.module_sha256, &key.class_id)
                        .unwrap();
                let mut action=detail.product.actions.iter().find(|a|matches!(&a.action,ui::Action::PresetFiles {command:c,..} if std::mem::discriminant(c)==std::mem::discriminant(&command))).unwrap().action.clone();
                if let ui::Action::PresetFiles {
                    command: Command::Restore { confirmed, .. },
                    ..
                } = &mut action
                {
                    *confirmed = true;
                }
                answer(
                    &f.m,
                    ui::Request {
                        schema: ui::OPERATOR_SCHEMA,
                        state_token: detail.state_token,
                        action,
                    },
                    name.map(str::to_owned),
                    input,
                )
                .unwrap()
            };
            assert!(
                run(
                    Command::Import,
                    Some("chosen.lvbpreset"),
                    Some(fs::File::open(&selected).unwrap())
                )
                .receipt
                .accepted
            );
            assert_eq!(fs::read(&selected).unwrap(), b"selected");
            assert!(run(Command::Backup, None, None).receipt.accepted);
            fs::remove_file(root.join("chosen.lvbpreset")).unwrap();
            assert_eq!(project_known(&f.m, &key).unwrap().missing, 1);
            let restore = Command::Restore {
                backup: String::new(),
                missing_digest: String::new(),
                confirmed: false,
            };
            assert!(run(restore, None, None).receipt.accepted);
            assert_eq!(
                fs::read(root.join("chosen.lvbpreset")).unwrap(),
                b"selected"
            );
            let stale = answer(
                &f.m,
                ui::Request {
                    schema: ui::OPERATOR_SCHEMA,
                    state_token: "stale".into(),
                    action: project_known(&f.m, &key).unwrap().actions[0].action.clone(),
                },
                None,
                None,
            )
            .unwrap();
            assert!(!stale.receipt.accepted);
        });
    }
    #[test]
    fn transition_is_required_for_successor_and_unrelated_setup_is_never_admitted() {
        let (f, key, profile) = fixture();
        files::with_profiles(vec![profile], || {
            let bound = binding(&f.m, &key).unwrap().unwrap();
            fs::write(bound.root.join("preset.lvbpreset"), b"source").unwrap();
            let mut record = retain_backup(&f.m, &bound, &"12".repeat(16)).unwrap();
            let mut successor = bound.clone();
            successor.product.module_sha256 = "bb".repeat(32);
            successor.profile.module_sha256 = successor.product.module_sha256.clone();
            assert!(!recovery_allowed(&f.m, &record, &successor).unwrap());
            record.transition = Some(Transition {
                operation: "34".repeat(16),
                before: bound.clone(),
                after: successor.clone(),
                backup: record.backup.clone(),
            });
            assert!(
                recovery_allowed(&f.m, &record, &successor).is_err(),
                "an asserted transition without its operation is an authority gap"
            );
            let t = record.transition.as_ref().unwrap();
            let dir = job_dir(&f.m, &t.operation).unwrap();
            private_dir(&dir).unwrap();
            atomic_json(
                &dir.join(format!("preset-transition-{}.json", super::key(&key))),
                t,
            )
            .unwrap();
            atomic_json(
                &protection_path(&f.m, &t.operation, &key.environment).unwrap(),
                &Protection {
                    schema: 1,
                    operation: t.operation.clone(),
                    environment: f.r.environment.clone(),
                    backups: vec![record.backup.clone()],
                    unknown_products: 0,
                },
            )
            .unwrap();
            assert!(recovery_allowed(&f.m, &record, &successor).unwrap());
            successor.product.environment = "cc".repeat(16);
            assert!(!recovery_allowed(&f.m, &record, &successor).unwrap());
            successor = bound;
            successor.inode += 1;
            assert!(!recovery_allowed(&f.m, &record, &successor).unwrap());
        });
    }
    #[test]
    fn recorded_update_and_rollback_keep_missing_offers_through_package_preparation() {
        let (f, original, profile) = fixture();
        let old_module = fs::read(&f.r.module.path).unwrap();
        let new_module = b"updated first-party module fixture";
        let mut successor = original.clone();
        successor.module_sha256 = hex(&sha2::Sha256::digest(new_module));
        let mut next_profile = profile.clone();
        next_profile.id = "reference-successor".into();
        next_profile.module_sha256 = successor.module_sha256.clone();
        files::with_profiles(vec![profile, next_profile], || {
            let root = f.r.environment.root.join("reference-user-presets");
            fs::write(root.join("saved.lvbpreset"), b"first-party sound bytes").unwrap();
            let update_scan = |product: &Product, bytes: &[u8]| {
                fs::write(&f.r.module.path, bytes).unwrap();
                let path =
                    f.m.root
                        .join("inventory")
                        .join(format!("{}.json", original.environment));
                let mut scan: inventory::Scan = read_json(&path).unwrap();
                for module in &mut scan.modules {
                    if module.artifact.path == f.r.module.path {
                        module.artifact.sha256 = product.module_sha256.clone();
                    }
                }
                atomic_json(&path, &scan).unwrap();
            };
            for (operation, product, bytes) in [
                ("91".repeat(16), &successor, new_module.as_slice()),
                ("92".repeat(16), &original, old_module.as_slice()),
            ] {
                protect_environment(&f.m, &f.r.environment, &operation).unwrap();
                update_scan(product, bytes);
                fs::remove_file(root.join("saved.lvbpreset")).unwrap();
                assert_eq!(
                    project_known(&f.m, product).unwrap().missing,
                    0,
                    "a fresh build observation alone cannot admit an old backup"
                );
                after_maintenance(&f.m, &f.r.environment, &operation).unwrap();
                let current = binding(&f.m, product).unwrap().unwrap();
                let record = retained(&f.m, product).unwrap().unwrap();
                assert!(recovery_allowed(&f.m, &record, &current).unwrap());
                assert_ne!(record.source.product.module_sha256, product.module_sha256);
                let offer = project_known(&f.m, product).unwrap();
                assert_eq!(offer.missing, 1);
                let pointer = fs::read(index(&f.m, product)).unwrap();
                let package_operation = random_id().unwrap();
                let package = protect_package(&f.m, &package_operation).unwrap();
                assert_eq!(package.len(), 1);
                verify_protection(&f.m, &package[0], &package_operation).unwrap();
                assert_ne!(package[0].backups[0], record.backup);
                assert_eq!(fs::read(index(&f.m, product)).unwrap(), pointer);
                assert_eq!(
                    project_known(&f.m, product).unwrap(),
                    offer,
                    "package-only preparation must preserve the exact pending recovery offer"
                );
                let backup = files::verify(&store(&f.m), &record.backup).unwrap();
                let missing = files::recovery(&current, &backup).unwrap();
                files::restore_missing(&store(&f.m), &current, &record.backup, &missing).unwrap();
                assert_eq!(
                    fs::read(root.join("saved.lvbpreset")).unwrap(),
                    b"first-party sound bytes"
                );
                assert_eq!(project_known(&f.m, product).unwrap().missing, 0);
            }
        });
    }
    #[test]
    fn package_preset_forward_retry_requires_the_same_current_files_and_root() {
        let (f, key, profile) = fixture();
        files::with_profiles(vec![profile], || {
            let bound = binding(&f.m, &key).unwrap().unwrap();
            fs::write(bound.root.join("preset.lvbpreset"), b"protected").unwrap();
            let operation = "67".repeat(16);
            let protection = protect_package(&f.m, &operation).unwrap();
            assert_eq!(protection.len(), 1);
            verify_package_sources(&f.m, &protection[0]).unwrap();
            fs::write(bound.root.join("preset.lvbpreset"), b"new save").unwrap();
            assert!(verify_package_sources(&f.m, &protection[0]).is_err());
            verify_protection(&f.m, &protection[0], &operation).unwrap();
            assert!(!job_dir(&f.m, &operation).unwrap().exists());
        });
    }
    #[test]
    fn maintenance_retains_completed_backup_before_update_and_records_unknown_separately() {
        let (f, key, profile) = fixture();
        files::with_profiles(vec![profile], || {
            let bound = binding(&f.m, &key).unwrap().unwrap();
            fs::write(bound.root.join("preset.lvbpreset"), b"protected").unwrap();
            let operation = "56".repeat(16);
            protect_environment(&f.m, &f.r.environment, &operation).unwrap();
            let saved: Protection =
                read_json(&protection_path(&f.m, &operation, &key.environment).unwrap()).unwrap();
            assert_eq!(saved.backups.len(), 1);
            let backup = files::verify(&store(&f.m), &saved.backups[0]).unwrap();
            assert_eq!(backup.files.len(), 1);
            fs::remove_file(bound.root.join("preset.lvbpreset")).unwrap();
            after_maintenance(&f.m, &f.r.environment, &operation).unwrap();
            assert_eq!(project_known(&f.m, &key).unwrap().missing, 1);
            files::with_profiles(vec![], || {
                let operation = "78".repeat(16);
                protect_environment(&f.m, &f.r.environment, &operation).unwrap();
                let unknown: Protection =
                    read_json(&protection_path(&f.m, &operation, &key.environment).unwrap())
                        .unwrap();
                assert!(unknown.backups.is_empty());
                assert!(unknown.unknown_products > 0);
                assert_eq!(
                    project_known(&f.m, &key).unwrap().location,
                    Location::Unknown
                );
            });
        });
    }
}
