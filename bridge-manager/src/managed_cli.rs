//! The ordinary typed management route. No caller supplies registration facts.
use super::*;
use linux_vst_bridge::{catalogue::*, observation::*, profiles::*};

use linux_vst_bridge::readback::refusal;
#[derive(Serialize)]
struct Selection {
    id: String,
    revision: u32,
    claim: Claim,
}
impl From<&Profile> for Selection {
    fn from(p: &Profile) -> Self {
        Self {
            id: p.id.clone(),
            revision: p.revision,
            claim: p.claim.clone(),
        }
    }
}
#[derive(Serialize)]
struct Planned {
    selection: usize,
    name: String,
    role: Role,
    class_id: String,
    profile: Selection,
    module_sha256: String,
    build: String,
    census: String,
    environment: String,
    environment_revision: u64,
    runner: String,
    native_sha256: String,
    external_ids: [String; 2],
    capabilities: Capabilities,
    limitations: Vec<Limitation>,
}
struct Plan {
    profile: Profile,
    census: Census,
    registration: Registration,
}
impl Plan {
    fn view(&self, selection: usize) -> Result<Planned> {
        Ok(Planned {
            selection,
            name: self.census.selected.name.clone(),
            role: role(&self.census.selected)?,
            class_id: self.registration.key(),
            profile: (&self.profile).into(),
            module_sha256: self.census.module.sha256.clone(),
            build: self.census.selected.version.clone(),
            census: self.census.id.clone(),
            environment: self.registration.environment.id.clone(),
            environment_revision: self.registration.environment.revision,
            runner: self.registration.environment.runner.id.clone(),
            native_sha256: self.registration.native.sha256.clone(),
            external_ids: external_ids(&self.registration.key())?,
            capabilities: self.profile.capabilities.clone(),
            limitations: self.profile.limitations.clone(),
        })
    }
}
fn catalogue(m: &Manager, sw: &Software) -> Result<Catalogue> {
    let a = sw
        .native_catalogue
        .as_ref()
        .ok_or("native_catalogue_absent_run_product_setup")?;
    require(
        a.path.parent() == sw.manager.path.parent(),
        "catalogue_software_binding",
    )?;
    a.verify()?;
    require(
        file(&a.path)?.metadata()?.len() <= 512 * 1024,
        "catalogue_size",
    )?;
    let c: Catalogue = read_json(&a.path)?;
    c.validate(&m.root)?;
    Ok(c)
}
fn environment(c: &Catalogue, index: Option<&str>) -> Result<EnvironmentBinding> {
    let index = if let Some(index) = index {
        index.parse::<usize>()?
    } else {
        require(c.environments.len() == 1, "environment_selection_required")?;
        1
    };
    c.environments
        .get(
            index
                .checked_sub(1)
                .ok_or("environment_selection_required")?,
        )
        .cloned()
        .ok_or_else(|| "environment_selection_required".into())
}
fn product(m: &Manager, index: &str) -> Result<String> {
    let index = index.parse::<usize>()?;
    m.registry()?
        .classes
        .keys()
        .nth(index.checked_sub(1).ok_or("product_selection_required")?)
        .cloned()
        .ok_or_else(|| "product_selection_required".into())
}
fn modules(environment: &Environment) -> Result<Vec<Artifact>> {
    let root = environment
        .root
        .join("compatdata/pfx/drive_c/Program Files/Common Files/VST3");
    require(root.canonicalize()? == root, "module_directory_symlink")?;
    let mut pending = vec![(root, 0)];
    let mut result = Vec::new();
    let mut visited = 0;
    let mut total = 0u64;
    while let Some((dir, depth)) = pending.pop() {
        require(depth <= 8, "module_discovery_depth")?;
        for item in fs::read_dir(dir)? {
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
    Ok(result)
}
#[derive(Clone, Copy)]
enum InspectionRoute {
    Current,
    Ap15Editor,
    Ap17Capacity,
    Ap18Pigments,
}
fn inspect_through_owner(
    m: &Manager,
    r: &InspectionRequest,
    route: InspectionRoute,
) -> Result<Artifact> {
    let bytes = serde_json::to_vec(r)?;
    require(bytes.len() <= 65536, "inspection_request_bound")?;
    let mut peer = UnixStream::connect(m.root.join("runtime/owner.sock"))?;
    peer.set_read_timeout(Some(Duration::from_secs(240)))?;
    peer.set_write_timeout(Some(Duration::from_secs(5)))?;
    peer.write_all(match route {
        InspectionRoute::Current => b"LVI1\n",
        InspectionRoute::Ap15Editor => b"LVQ1\n",
        InspectionRoute::Ap17Capacity => b"LVQ2\n",
        InspectionRoute::Ap18Pigments => b"LVQ3\n",
    })?;
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
    route: InspectionRoute,
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
            require(
                sw.host.sha256 == policy.requirements.host_sha256
                    && sw.source_sha256 == policy.requirements.host_source_sha256,
                "installed_host_mismatch",
            )?;
            let native = catalogue.native(policy)?;
            let cache = m
                .root
                .join("censuses/current")
                .join(format!("{class}.json"));
            let saved = read_json::<Census>(&cache).ok().filter(|c| {
                c.module == module
                    && c.environment == environment
                    && c.verify_current(&m.root, &sw.host, &sw.source_sha256, now().unwrap_or(0))
                        .is_ok()
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
                let report = inspect_through_owner(m, &request, route)?;
                let census = Census::from_report(
                    environment.clone(),
                    module.clone(),
                    stamp,
                    sw.host.clone(),
                    sw.source_sha256.clone(),
                    report,
                    &class,
                )?;
                census.verify_current(&m.root, &sw.host, &sw.source_sha256, now()?)?;
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
fn status(m: &Manager) -> Result<serde_json::Value> {
    let sw = software(m)?;
    Ok(serde_json::to_value(
        m.managed_status(&sw.host, &sw.source_sha256)?,
    )?)
}
fn execute(m: &Manager, args: &[String], profiles: &[Profile]) -> Result<serde_json::Value> {
    match args.first().map(String::as_str){
            Some("status") if args.len()==1=>status(m),
            Some("reconcile") if args.len()==1=>{m.reconcile()?;status(m)},
            Some("rollback") if args.len()==3=>{let key=product(m,&args[1])?;m.rollback(&key,&args[2],None)?;status(m)},
            Some("unpublish") if args.len()==2=>{let key=product(m,&args[1])?;m.unpublish(&key)?;status(m)},
            Some("environments") if args.len()==1=>{
                let sw=software(m)?;let c=catalogue(m,&sw)?;
                Ok(serde_json::json!({"schema":1,"environments":c.environments.iter().enumerate().map(|(i,e)|
                    serde_json::json!({"selection":i+1,"family":e.family,"id":e.environment.id,"revision":e.environment.revision,"runner":e.environment.runner.id})).collect::<Vec<_>>()}))
            },
            Some("preview"|"publish") if args.len()<=2=>{
                let sw=software(m)?;let c=catalogue(m,&sw)?;
                let e=environment(&c,args.get(1).map(String::as_str))?;
                let plans=plans(m,&sw,&c,e,profiles,SelectionPurpose::Activation,InspectionRoute::Current)?;
                let views=plans.iter().enumerate().map(|(i,p)|p.view(i+1)).collect::<Result<Vec<_>>>()?;
                if args[0]=="publish"{
                    // Validate all selected plans before the first class changes.
                    { let _lock=m.lock("registry.lock")?;for p in &plans{m.require_inactive(Some(&p.registration.key()))?;} }
                    for p in &plans{p.census.verify_current(&m.root,&sw.host,&sw.source_sha256,now()?)?;p.registration.verify(&m.root)?;}
                    for p in plans{m.managed_publish(&p.profile,&p.census,p.registration,&sw.host,&sw.source_sha256,None)?;}
                    status(m)
                }else{Ok(serde_json::json!({"schema":1,"matches":views}))}
            },
            Some("fault-check") if args.len()==2=>{
                // One fixed preactivation fault for disposable candidate files.
                // No caller can inject executable code or force an active swap.
                let key=product(m,&args[1])?;
                let sw=software(m)?;let c=catalogue(m,&sw)?;
                let selected=plans(m,&sw,&c,environment(&c,None)?,profiles,SelectionPurpose::Activation,InspectionRoute::Current)?.into_iter().find(|p|p.registration.key()==key).ok_or("profile_no_match")?;
                let error=m.managed_publish(&selected.profile,&selected.census,selected.registration,&sw.host,&sw.source_sha256,
                    Some(linux_vst_bridge::publication::Boundary::CandidateReady)).err().ok_or("fault_not_reached")?;
                require(error.to_string()=="injected_CandidateReady","fault_not_reached")?;
                Ok(serde_json::json!({"schema":1,"injected":"candidate_ready_before_activation","reconcile_required":true,"status":status(m)?}))
            },
            Some("check-candidate") if args.len()==2=>{
                let p=Path::new(&args[1]);let f=file(p)?;require(f.metadata()?.len()<=PROFILE_LIMIT as u64,"profile_size")?;
                let mut bytes=Vec::new();f.take(PROFILE_LIMIT as u64+1).read_to_end(&mut bytes)?;
                let profile=Profile::parse(&bytes).map_err(|e|format!("profile_invalid: {e}"))?;
                let sw=software(m)?;let c=catalogue(m,&sw)?;let e=environment(&c,None)?;
                let plans=plans(m,&sw,&c,e,&[profile],SelectionPurpose::Qualification,InspectionRoute::Current)?;
                Ok(serde_json::json!({"schema":1,"matches":plans.iter().enumerate().map(|(i,p)|p.view(i+1)).collect::<Result<Vec<_>>>()?,"activation_permitted":false}))
            },
            _=>Err("Usage: managed environments | preview [ENVIRONMENT_NUMBER] | publish [ENVIRONMENT_NUMBER] | status | reconcile | rollback PRODUCT_NUMBER REVISION | unpublish PRODUCT_NUMBER | check-candidate PROFILE | fault-check PRODUCT_NUMBER".into()),
        }
}
pub(super) fn run(m: &Manager, args: &[String]) -> Result<()> {
    let result = installed_profiles().and_then(|profiles| execute(m, args, &profiles));
    render(result)
}
fn render(result: Result<serde_json::Value>) -> Result<()> {
    match result {
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value)?);
            Ok(())
        }
        Err(e) => {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"schema":1,"ok":false,"refusal":refusal(e.as_ref())})
                )?
            );
            Err(e)
        }
    }
}

// This separate command is intentionally absent from ordinary managed
// preview/publish. Profile and artifact inputs are fixed by the compiled roster.
fn execute_qualification(m: &Manager, args: &[String]) -> Result<serde_json::Value> {
    execute_qualification_for(m, args, publication::Qualification::Ap15Editor)
}
fn execute_qualification_for(
    m: &Manager,
    args: &[String],
    purpose: publication::Qualification,
) -> Result<serde_json::Value> {
    match args.first().map(String::as_str) {
        Some("stage") if args.len() == 2 => {
            qualification::stage_for(m, Path::new(&args[1]), purpose)?;
            status(m)
        }
        Some("restore") if args.len() == 1 => {
            m.reconcile()?;
            status(m)
        }
        Some("publish") if args.len() == 1 => {
            let candidates = qualification::installed_for(m, purpose)?;
            let mut sw = software(m)?;
            let mut c = catalogue(m, &sw)?;
            c.natives = candidates.iter().map(|c| c.native.clone()).collect();
            let mut planned = Vec::new();
            for candidate in &candidates {
                sw.host = candidate.host.clone();
                sw.source_manifest = candidate.source_manifest.clone();
                sw.source_sha256 = candidate.source_manifest.sha256.clone();
                planned.extend(plans(
                    m,
                    &sw,
                    &c,
                    environment(&c, None)?,
                    std::slice::from_ref(&candidate.profile),
                    SelectionPurpose::Qualification,
                    match purpose {
                        publication::Qualification::Ap15Editor => InspectionRoute::Ap15Editor,
                        publication::Qualification::Ap17Capacity => InspectionRoute::Ap17Capacity,
                        publication::Qualification::Ap18Pigments => InspectionRoute::Ap18Pigments,
                    },
                )?);
            }
            {
                let _lock = m.lock("registry.lock")?;
                m.require_inactive(None)?;
                for p in &planned {
                    m.check_qualification_parent_for(&p.profile, &p.registration, purpose)?;
                }
            }
            for p in &planned {
                m.qualify_for(&p.census, None, purpose)?;
            }
            status(m)
        }
        _ => Err("Usage: qualify-editor stage EXACT_PRODUCT_PACKAGE | publish | restore".into()),
    }
}
pub(super) fn run_qualification(m: &Manager, args: &[String]) -> Result<()> {
    render(execute_qualification(m, args))
}

pub(super) fn run_capacity_qualification(m: &Manager, args: &[String]) -> Result<()> {
    render(execute_qualification_for(
        m,
        args,
        publication::Qualification::Ap17Capacity,
    ))
}

pub(super) fn run_acceptance(m: &Manager) -> Result<()> {
    // Setup starts the owner, which may immediately hold registry.lock while
    // reconciling/creating its keeper. Do not turn a completed installation
    // into a false failure by racing that lock for optional status readback.
    render(setup(m, None).and_then(|()| acceptance_receipt(m)))
}
pub(super) fn run_capacity_acceptance(m: &Manager) -> Result<()> {
    render(setup_selected(m, None, true).and_then(|()| acceptance_receipt(m)))
}
fn acceptance_receipt(m: &Manager) -> Result<serde_json::Value> {
    Ok(serde_json::json!({"schema": 1, "software_installed": true,
        "software": software(m)?, "publication_command": "managed publish",
        "readback_command": "managed status"}))
}

pub(super) fn run_pigments_qualification(m: &Manager, args: &[String]) -> Result<()> {
    if args == ["restore"] { return render(pigments::restore(m).and_then(|()| status(m))); }
    render(execute_qualification_for(m, args, publication::Qualification::Ap18Pigments))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::{prepared, snapshot};
    #[test]
    fn completed_setup_receipt_does_not_race_service_startup_registry_lock() {
        let (f, _, c, _) = prepared();
        let sw = Software {
            manager: c.host.clone(),
            supervisor: c.host.clone(),
            ownership: c.host.clone(),
            host: c.host.clone(),
            source_manifest: Artifact {
                path: c.host.path.with_file_name("host-source-manifest.json"),
                sha256: c.host_source_sha256.clone(),
            },
            source_sha256: c.host_source_sha256,
            native_catalogue: None,
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let _startup = f.m.lock("registry.lock").unwrap();
        let before = snapshot(&f.outer);
        let receipt = acceptance_receipt(&f.m).unwrap();
        assert_eq!(receipt["software_installed"], true);
        assert_eq!(receipt["software"]["host"]["sha256"], sw.host.sha256);
        assert_eq!(snapshot(&f.outer), before);
    }
    #[test]
    fn missing_exact_sealed_package_cannot_mutate_installed_state() {
        let (f, _, _, _) = prepared();
        let before = snapshot(&f.outer);
        for command in [
            vec!["publish".into()],
            vec!["stage".into(), f.outer.to_str().unwrap().into()],
        ] {
            let error = execute_qualification(&f.m, &command).unwrap_err();
            assert_ne!(
                refusal(error.as_ref()).code,
                linux_vst_bridge::readback::RefusalCode::ReviewCandidateNotActivatable
            );
            assert_eq!(snapshot(&f.outer), before);
        }
    }

    #[test]
    fn actual_candidate_command_qualifies_without_activation_or_durable_mutation() {
        let (f, mut p, c, n) = prepared();
        let catalogue_path = f.m.root.join("software/native-catalogue.json");
        atomic_json(
            &catalogue_path,
            &Catalogue {
                schema: 1,
                natives: vec![n],
                environments: vec![c.environment.clone()],
            },
        )
        .unwrap();
        let sw = Software {
            manager: c.host.clone(),
            supervisor: c.host.clone(),
            ownership: c.host.clone(),
            host: c.host.clone(),
            source_manifest: Artifact {
                path: c.host.path.with_file_name("host-source-manifest.json"),
                sha256: c.host_source_sha256.clone(),
            },
            source_sha256: c.host_source_sha256.clone(),
            native_catalogue: Some(Artifact {
                sha256: digest(&catalogue_path).unwrap(),
                path: catalogue_path,
            }),
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        private_dir(&f.m.root.join("censuses/current")).unwrap();
        atomic_json(
            &f.m.root
                .join("censuses/current")
                .join(format!("{}.json", p.class.class_id)),
            &c,
        )
        .unwrap();
        let profile_path = f.outer.join("candidate.json");
        p.claim = Claim::ReviewCandidate;
        atomic_json(&profile_path, &p).unwrap();
        let before = snapshot(&f.outer);
        let result = execute(
            &f.m,
            &[
                "check-candidate".into(),
                profile_path.to_str().unwrap().into(),
            ],
            std::slice::from_ref(&p),
        )
        .unwrap();
        assert_eq!(result["activation_permitted"], false);
        assert_eq!(result["matches"][0]["profile"]["claim"], "review_candidate");
        assert_eq!(snapshot(&f.outer), before);
        for command in ["preview", "publish"] {
            let error = execute(&f.m, &[command.into()], std::slice::from_ref(&p)).unwrap_err();
            assert_eq!(
                refusal(error.as_ref()).code,
                linux_vst_bridge::readback::RefusalCode::ReviewCandidateNotActivatable
            );
            assert_eq!(snapshot(&f.outer), before);
        }
        assert!(f.m.resolve(&f.identity()).is_ok());
        // Verified ordinary preview reaches the same observed facts. Candidate
        // checking remains nonactivating even when the input is verified.
        p.claim = Claim::VerifiedExactFixture;
        let preview = execute(&f.m, &["preview".into()], std::slice::from_ref(&p)).unwrap();
        assert_eq!(
            preview["matches"][0]["profile"]["claim"],
            "verified_exact_fixture"
        );
        assert_eq!(snapshot(&f.outer), before);
        p.claim = Claim::Withdrawn;
        let error = execute(&f.m, &["publish".into()], &[p]).unwrap_err();
        assert_eq!(
            refusal(error.as_ref()).code,
            linux_vst_bridge::readback::RefusalCode::ProfileWithdrawn
        );
        assert_eq!(snapshot(&f.outer), before);
    }
}
