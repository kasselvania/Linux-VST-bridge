//! Module discovery and recommended-revision republication for operator actions.
//! No caller supplies registration facts.
use super::*;
use linux_vst_bridge::{catalogue::*, observation::*, profiles::*};
struct Plan {
    profile: Profile,
    census: Census,
    registration: Registration,
}
fn catalogue(m: &Manager, sw: &Software) -> Result<Catalogue> {
    sw.catalogue(m)
}
pub(super) fn modules(environment: &Environment) -> Result<Vec<Artifact>> {
    Ok(module_discovery(environment)?.modules)
}
pub(super) struct ModuleDiscovery {
    pub modules: Vec<Artifact>,
    directories: Vec<(PathBuf, Option<crate::DigestFileIdentity>)>,
}
impl ModuleDiscovery {
    pub fn recheck(&self) -> Result<()> {
        for (path, expected) in &self.directories {
            let actual = match fs::symlink_metadata(path) {
                Ok(metadata) => Some(crate::DigestFileIdentity::from(&metadata)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            require(actual == *expected, "inventory_module_directory_changed")?;
        }
        Ok(())
    }
}
/// Capture the directory identities as well as the module bytes. A later
/// registry phase can reject additions/removals without walking or hashing
/// modules while unrelated setup admission waits.
pub(super) fn module_discovery(environment: &Environment) -> Result<ModuleDiscovery> {
    let root = environment
        .root
        .join("compatdata/pfx/drive_c/Program Files/Common Files/VST3");
    if !root.try_exists()? {
        return Ok(ModuleDiscovery { modules: vec![], directories: vec![(root, None)] });
    }
    require(root.canonicalize()? == root, "module_directory_symlink")?;
    let mut pending = vec![(root, 0)];
    let mut result = Vec::new();
    let mut directories = Vec::new();
    let mut visited = 0;
    let mut total = 0u64;
    while let Some((dir, depth)) = pending.pop() {
        require(depth <= 8, "module_discovery_depth")?;
        let identity = crate::DigestFileIdentity::from(&fs::symlink_metadata(&dir)?);
        directories.push((dir.clone(), Some(identity)));
        for item in fs::read_dir(&dir)? {
            let path = item?.path();
            let md = fs::symlink_metadata(&path)?;
            visited += 1;
            require(visited <= 4096, "module_discovery_count")?;
            if md.is_dir() {
                pending.push((path, depth + 1));
                continue;
            }
            if !path
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("vst3"))
            {
                continue;
            }
            require(
                md.is_file() && !md.file_type().is_symlink() && md.len() <= 512 * 1024 * 1024,
                "module_discovery_type_or_size",
            )?;
            total = total.checked_add(md.len()).ok_or("module_discovery_size")?;
            require(
                total <= 2 * 1024 * 1024 * 1024 && result.len() < 256,
                "module_discovery_size",
            )?;
            result.push(Artifact {
                sha256: digest(&path)?,
                path,
            });
        }
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    let found = ModuleDiscovery { modules: result, directories };
    found.recheck()?;
    Ok(found)
}
fn inspect_through_owner(m: &Manager, r: &InspectionRequest) -> Result<Artifact> {
    let bytes = serde_json::to_vec(r)?;
    require(bytes.len() <= 65536, "inspection_request_bound")?;
    let mut peer = UnixStream::connect(m.root.join("runtime/owner.sock"))?;
    peer.set_read_timeout(Some(Duration::from_secs(240)))?;
    peer.set_write_timeout(Some(Duration::from_secs(5)))?;
    peer.write_all(b"LVI1\n")?;
    peer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    peer.write_all(&bytes)?;
    let mut size = [0; 4];
    peer.read_exact(&mut size)?;
    let size = u32::from_le_bytes(size) as usize;
    require(size <= 4096, "inspection_reply_bound")?;
    let mut bytes = vec![0; size];
    peer.read_exact(&mut bytes)?;
    let path: PathBuf = serde_json::from_slice(&bytes)?;
    require(
        path.parent() == Some(m.root.join("runtime/results").as_path()),
        "inspection_report_location",
    )?;
    Ok(Artifact {
        sha256: digest(&path)?,
        path,
    })
}
fn plans(
    m: &Manager,
    sw: &Software,
    catalogue: &Catalogue,
    environment: EnvironmentBinding,
    profiles: &[Profile],
    purpose: SelectionPurpose,
) -> Result<Vec<Plan>> {
    validate_set(profiles)?;
    for p in profiles {
        p.claim.require(purpose)?;
    }
    let current = environment_record(m, &environment.environment.id)?;
    require(current == environment.environment, "environment_mismatch")?;
    for p in profiles {
        p.verify_environment(&current, &environment.family)?;
    }
    let found = modules(&current)?;
    // An existing binding makes a replacement digest explicit rather than
    // silently omitting a previously supported product from discovery.
    for e in m
        .registry()?
        .classes
        .values()
        .filter(|e| e.registration.environment.id == current.id)
    {
        let r = &e.registration;
        if profiles.iter().any(|p| p.class.class_id == r.key()) {
            require(
                found
                    .iter()
                    .any(|a| a.path == r.module.path && a.sha256 == r.module.sha256),
                "module_digest_changed",
            )?;
        }
    }
    let mut result = Vec::new();
    let mut selected = std::collections::BTreeSet::new();
    for module in found {
        let eligible: Vec<_> = profiles
            .iter()
            .filter(|p| p.module_sha256 == module.sha256 && p.claim.permits(purpose))
            .collect();
        let classes: std::collections::BTreeSet<_> =
            eligible.iter().map(|p| p.class.class_id.clone()).collect();
        for class in classes {
            require(selected.insert(class.clone()), "profile_ambiguous")?;
            let policy: Vec<_> = eligible
                .iter()
                .filter(|p| p.class.class_id == class)
                .collect();
            require(policy.len() == 1, "profile_ambiguous")?;
            let policy = policy[0];
            let host = catalogue.host(policy, &sw.host, &sw.source_sha256)?;
            let native = catalogue.native(policy)?;
            let cache = m
                .root
                .join("censuses/current")
                .join(format!("{class}.json"));
            let saved = read_json::<Census>(&cache).ok().filter(|c| {
                c.module == module && c.environment == environment
                    && c.verify_current(&m.root,&host.host,&host.source_manifest.sha256,
                        now().unwrap_or(0)).is_ok()
            });
            let census = if let Some(saved) = saved {
                saved
            } else {
                let stamp = ModuleStamp::read(&module.path)?;
                let request = InspectionRequest {
                    environment_id: current.id.clone(),
                    module: module.clone(),
                    class_id: class.clone(),
                    compatibility: policy.capabilities.compatibility(),
                };
                let report = inspect_through_owner(m, &request)?;
                let census = Census::from_report(
                    environment.clone(),
                    module.clone(),
                    stamp,
                    host.host.clone(),
                    host.source_manifest.sha256.clone(),
                    report,
                    &class,
                )?;
                census.verify_current(&m.root, &host.host, &host.source_manifest.sha256, now()?)?;
                private_dir(&m.root.join("censuses/current"))?;
                atomic_json(
                    &m.root.join("censuses").join(format!("{}.json", census.id)),
                    &census,
                )?;
                atomic_json(&cache, &census)?;
                census
            };
            let p = select_for(profiles, &census, purpose)?.clone();
            let registration = derive_for(&p, &census, native, purpose)?;
            result.push(Plan {
                profile: p,
                census,
                registration,
            });
        }
    }
    require(!result.is_empty(), "profile_no_match")?;
    result.sort_by_key(|p| p.registration.key());
    Ok(result)
}
pub(super) fn restore_recommended(m: &Manager, key: &str) -> Result<()> {
    let sw = software(m)?;
    let c = catalogue(m, &sw)?;
    let r = m.registry()?.classes.get(key).ok_or("registration_absent")?.registration.clone();
    let e = c.environments.iter().find(|e| e.environment == r.environment).ok_or("environment_absent")?.clone();
    let profiles: Vec<_> = installed_profiles()?.into_iter().filter(|p| p.class.class_id == key).collect();
    require(profiles.len() == 1, "profile_ambiguous")?;
    let mut matched = plans(m, &sw, &c, e, &profiles, SelectionPurpose::Activation)?;
    require(matched.len() == 1, "profile_ambiguous")?;
    let p = matched.remove(0);
    m.managed_publish_inactive(&p.profile, &p.census, p.registration, &p.census.host, &p.census.host_source_sha256, None)?;
    Ok(())
}
