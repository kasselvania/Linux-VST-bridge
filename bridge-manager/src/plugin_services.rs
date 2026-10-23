//! Declared plug-in service dependencies, selected only when registered in this setup.
use super::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependency { service: String, name: String, installer: String, provider: String }
fn registered(hive: &str, service: &str) -> bool {
    hive.lines().filter_map(|line| line.strip_prefix('[')?.split_once(']').map(|(key,_)| key))
        .any(|key| {
            let parts: Vec<_> = key.split("\\\\").collect();
            parts.len()==4 && parts[0].eq_ignore_ascii_case("System")
                && (parts[1].eq_ignore_ascii_case("CurrentControlSet")
                    || parts[1].strip_prefix("ControlSet").is_some_and(|suffix|
                        suffix.len()==3 && suffix.bytes().all(|c|c.is_ascii_digit())))
                && parts[2].eq_ignore_ascii_case("Services") && parts[3].eq_ignore_ascii_case(service)
        })
}
fn registry(environment: &Environment) -> Result<Option<String>> {
    let path=environment.root.join("compatdata/pfx/system.reg");
    if !path.try_exists()? {return Ok(None)}
    let mut hive=String::new();
    file(&path)?.take(16*1024*1024+1).read_to_string(&mut hive)?;
    linux_vst_bridge::record_opened(&path,0,hive.len() as u64);
    require(hive.len()<=16*1024*1024,"plugin_service_registry_bound")?;
    Ok(Some(hive))
}
/// The declared services this hive registers.
fn registrations(hive: &str) -> Result<BTreeSet<String>> {
    let dependencies:Vec<Dependency>=serde_json::from_str(include_str!("plugin_services.json"))?;
    Ok(dependencies.into_iter().filter(|d| registered(hive,&d.service)).map(|d|d.service).collect())
}
/// A hive file as it is on disk: any write to it, or a replacement, changes
/// one of these. (The library's file identity is private to its digest
/// cache; this also needs the change time on its own.)
#[derive(Clone, Copy, PartialEq, Eq)]
struct HiveIdentity { device: u64, inode: u64, size: u64, modified: (i64, i64), changed: (i64, i64) }
/// How long before a read a hive must have last changed for its facts to be
/// kept: a filesystem's timestamps can be a second coarse, and the kernel's
/// clock tick lags the system clock.
const SETTLED: Duration = Duration::from_secs(2);
impl HiveIdentity {
    fn of(path: &Path) -> std::io::Result<Self> {
        use std::os::unix::fs::MetadataExt;
        let m = fs::symlink_metadata(path)?;
        Ok(Self { device: m.dev(), inode: m.ino(), size: m.len(),
            modified: (m.mtime(), m.mtime_nsec()), changed: (m.ctime(), m.ctime_nsec()) })
    }
    /// Whether it last changed `SETTLED` before `read`, so a later write
    /// cannot share its change time. An absurd change time never is.
    fn settled_before(self, read: std::time::SystemTime) -> bool {
        let (seconds, nanoseconds) = self.changed;
        u64::try_from(seconds).ok().zip(u32::try_from(nanoseconds).ok().filter(|n| *n < 1_000_000_000))
            .and_then(|(seconds, nanoseconds)| std::time::UNIX_EPOCH.checked_add(Duration::new(seconds, nanoseconds)))
            .and_then(|changed| changed.checked_add(SETTLED))
            .is_some_and(|settled| settled < read)
    }
}
/// What this process learnt from each setup's hive, with the hive's identity
/// when it was read. A hive is read again once any part of that changes.
static REGISTRATIONS: std::sync::Mutex<BTreeMap<PathBuf, (HiveIdentity, BTreeSet<String>)>> =
    std::sync::Mutex::new(BTreeMap::new());
/// The declared services this setup's hive registers, for dependency help.
/// A Windows registry hive can be 15 MB, and the service shows this on every
/// recomposition, so it reads a hive again only after it changed. Launch
/// selection (`selected`) always reads the hive itself.
fn registered_services(environment: &Environment) -> Result<Option<BTreeSet<String>>> {
    registered_services_read_at(environment, std::time::SystemTime::now())
}
/// As `registered_services`, reading at `read`.
fn registered_services_read_at(environment: &Environment, read: std::time::SystemTime)
    -> Result<Option<BTreeSet<String>>> {
    let path = environment.root.join("compatdata/pfx/system.reg");
    let mut kept = REGISTRATIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let before = match HiveIdentity::of(&path) {
        Ok(identity) => identity,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            kept.remove(&path);
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    if let Some((_, services)) = kept.get(&path).filter(|(identity, _)| *identity == before) {
        // The page still depends on this file: name it for the view's watch.
        linux_vst_bridge::record_opened(&path, 0, 0);
        return Ok(Some(services.clone()));
    }
    drop(kept);
    let Some(hive) = registry(environment)? else { return Ok(None) };
    let services = registrations(&hive)?;
    // Keep only what was read from one unchanged, settled file.
    if HiveIdentity::of(&path)? == before && before.settled_before(read) {
        REGISTRATIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(path, (before, services.clone()));
    }
    Ok(Some(services))
}
pub(super) fn selected(environment: &Environment) -> Result<Vec<String>> {
    let Some(hive)=registry(environment)? else {return Ok(vec![])};
    let dependencies:Vec<Dependency>=serde_json::from_str(include_str!("plugin_services.json"))?;
    Ok(dependencies.into_iter().filter(|d| !d.name.is_empty() && registered(&hive,&d.service))
        .map(|d|d.service).collect())
}

/// Advice uses only this setup's registration and its current, bound scan.
/// An absent registration on an unfamiliar module is not a missing requirement.
pub(super) fn assistance(m: &Manager, environment: &Environment, host: &Artifact,
    source: &str) -> Result<Vec<String>> {
    // Optional display readback may be unavailable. Launch selection above
    // still propagates errors; unreadable registration never becomes absence.
    let registered=registered_services(environment).ok().flatten();
    let path=m.root.join("inventory").join(format!("{}.json",environment.id));
    let mut observations=vec![];
    if path.try_exists()? {
        let scan:inventory::Scan=read_json(&path)?;
        if scan.schema==1 && scan.environment==*environment && scan.host==*host
            && scan.host_source_sha256==source {
            for module in &scan.modules {
                if inventory::record_stale_reason(module,&scan.environment,&scan.host,
                    &scan.host_source_sha256,environment,host,source).is_some() {continue;}
                // Only small owned report bytes are read; never inspect vendor
                // binaries, licence data or another setup to infer a dependency.
                let report=module.report.record_bytes(4*1024*1024).ok()
                    .and_then(|bytes|serde_json::from_slice::<Value>(&bytes).ok());
                if let Some(rows)=report.as_ref().and_then(|report|report["records"].as_array()) {
                    observations.extend(rows.iter().filter(|row|row["state"]=="required_service").cloned());
                }
            }
        }
    }
    let dependencies:Vec<Dependency>=serde_json::from_str(include_str!("plugin_services.json"))?;
    Ok(dependencies.iter().filter_map(|dependency|
        advice(dependency,registered.as_ref(),&observations)).collect())
}

fn advice(dependency: &Dependency, registered: Option<&BTreeSet<String>>, observations: &[Value]) -> Option<String> {
    let rows:Vec<_>=observations.iter().filter(|row|row["name"]==dependency.service).collect();
    let registration=registered.map(|services|services.contains(&dependency.service));
    if registration!=Some(true) && rows.is_empty() {return None;}
    let running=rows.iter().any(|row|row["error"]==0 && row["after"]==4);
    let not_running=rows.iter().any(|row|row["error"]==0
        && row["after"].as_u64().is_some_and(|state|(1..=7).contains(&state) && state!=4));
    let fact=match registration {
        Some(false)=>format!("The {} is not registered in this setup now; a retained plug-in check requested it.",dependency.name),
        None=>format!("The {} registration could not be read in this setup.",dependency.name),
        Some(true) if rows.is_empty()=>format!("The {} is registered in this setup; whether it runs has not been checked.",dependency.name),
        Some(true) if running && not_running=>format!("Retained plug-in checks observed the {} both running and not running in this setup. Its current state has not been checked.",dependency.name),
        Some(true) if not_running=>format!("A retained plug-in check observed the {} not running in this setup. Its current state has not been checked.",dependency.name),
        Some(true) if running=>format!("A retained plug-in check observed the {} running in this setup. Its current state has not been checked.",dependency.name),
        Some(true)=>format!("The {} is registered in this setup; retained plug-in checks could not confirm it was running.",dependency.name),
    };
    Some(format!("{fact} If your plug-in asks for {}, download {} from {} and choose its installer to run in this setup. Complete any activation in the vendor's window, then open the plug-in to finish setup or retry its check.",
        dependency.provider,dependency.installer,dependency.provider))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn service_advice_distinguishes_registration_observation_and_unknown_requirement() {
        let mut dependencies:Vec<Dependency>=serde_json::from_str(include_str!("plugin_services.json")).unwrap();
        let dependency=dependencies.remove(0);
        let hive=r"[System\\ControlSet001\\Services\\PaceLicenseDServices] 1";
        assert!(advice(&dependency,Some(&registrations("").unwrap()),&[]).is_none());
        assert!(advice(&dependency,None,&[]).is_none());
        let unknown=advice(&dependency,Some(&registrations(hive).unwrap()),&[]).unwrap();
        assert!(unknown.contains("registered in this setup; whether it runs has not been checked"));
        for (error,after,expected) in [(0,1,"not running in this setup"),
            (0,4,"running in this setup"),(5,0,"could not confirm")] {
            let rows=[json!({"name":dependency.service,"error":error,"after":after})];
            let help=advice(&dependency,Some(&registrations(hive).unwrap()),&rows).unwrap();
            assert!(help.contains(expected),"{help}");
            assert!(help.contains("If your plug-in asks for iLok"));
            assert!(help.contains("download iLok License Manager from iLok"));
            assert!(help.contains("activation in the vendor's window"));
            assert!(!help.contains("last plug-in check"));
        }
        let absent=advice(&dependency,Some(&registrations("").unwrap()),&[json!({"name":dependency.service,"error":1060,"after":0})]).unwrap();
        assert!(absent.contains("not registered in this setup now"));
        assert!(!absent.contains("requires"));
        assert!(advice(&dependency,Some(&registrations("").unwrap()),&[json!({"name":"OtherService","after":1,"error":0})]).is_none());
    }
    #[test]
    fn retried_module_does_not_make_retained_sibling_observation_the_latest_state() {
        let f=test_fixture::Fixture::new();let environment=&f.r.environment;
        fs::write(environment.root.join("compatdata/pfx/system.reg"),
            r"[System\\ControlSet001\\Services\\PaceLicenseDServices] 1").unwrap();
        let report=f.outer.join("older-sibling-report.json");
        let stopped=f.outer.join("original-retried-module-report.json");
        let running=f.outer.join("new-retried-module-report.json");
        for (path,after,session) in [(&report,1,"sibling"),(&stopped,1,"original"),(&running,4,"retry")] {
            atomic_json(path,&json!({"session":session,"records":[{"state":"required_service",
                "name":"PaceLicenseDServices","after":after,"error":0}]})).unwrap();
        }
        let second=f.outer.join("second-first-party-module.vst3");fs::write(&second,b"second module").unwrap();
        let mut scan=inventory::Scan {schema:1,id:"ab".repeat(16),environment:environment.clone(),
            host:f.r.host.clone(),host_source_sha256:f.r.host_source_sha256.clone(),completed_at:1,
            modules:vec![inventory::Module {artifact:f.r.module.clone(),classes:vec![],
                report:Artifact {sha256:digest(&report).unwrap(),path:report},inspection_error:None,quarantine_reason:None},
                inventory::Module {artifact:Artifact {sha256:digest(&second).unwrap(),path:second},classes:vec![],
                    report:Artifact {sha256:digest(&stopped).unwrap(),path:stopped},inspection_error:None,quarantine_reason:None}],
            changes:Default::default()};
        let path=f.m.root.join("inventory");private_dir(&path).unwrap();
        let path=path.join(format!("{}.json",environment.id));
        let help=||assistance(&f.m,environment,&f.r.host,&f.r.host_source_sha256).unwrap();
        atomic_json(&path,&scan).unwrap();assert!(help()[0].contains("not running in this setup"));
        let sibling=scan.modules[0].report.clone();
        // An exact retry replaces one report and retains the sibling's older one.
        scan.id="cd".repeat(16);scan.modules[1].report=Artifact {sha256:digest(&running).unwrap(),path:running};
        atomic_json(&path,&scan).unwrap();let mixed=help();
        assert!(mixed[0].contains("both running and not running"));
        assert!(mixed[0].contains("current state has not been checked"));
        assert!(!mixed[0].contains("last plug-in check"));
        assert_eq!(scan.modules[0].report,sibling);
        scan.modules.reverse();atomic_json(&path,&scan).unwrap();assert_eq!(help(),mixed);
    }
    #[test]
    fn advice_never_borrows_another_setup_or_stale_or_modified_report() {
        let f=test_fixture::Fixture::new();
        let environment=&f.r.environment;
        fs::write(environment.root.join("compatdata/pfx/system.reg"),"").unwrap();
        let report=f.outer.join("service-report.json");
        atomic_json(&report,&json!({"records":[{"state":"required_service",
            "name":"PaceLicenseDServices","after":1,"error":0}]})).unwrap();
        let mut scan=inventory::Scan {schema:1,id:"ab".repeat(16),environment:environment.clone(),
            host:f.r.host.clone(),host_source_sha256:f.r.host_source_sha256.clone(),completed_at:1,
            modules:vec![inventory::Module {artifact:f.r.module.clone(),classes:vec![],
                report:Artifact {sha256:digest(&report).unwrap(),path:report.clone()},inspection_error:Some("TimeoutError: Windows call deadline: load_library".into()),
                quarantine_reason:Some("inventory_factory_absent_or_duplicate".into())}],changes:Default::default()};
        let path=f.m.root.join("inventory");private_dir(&path).unwrap();
        let path=path.join(format!("{}.json",environment.id));
        let help=||assistance(&f.m,environment,&f.r.host,&f.r.host_source_sha256).unwrap();
        assert!(help().is_empty());
        atomic_json(&path,&scan).unwrap();
        assert!(help()[0].contains("not registered"));
        assert!(selected(environment).unwrap().is_empty(),"A scan observation cannot authorize service creation");
        for change in ["environment","revision","runner","host","source"] {
            let original=scan.clone();
            match change {
                "environment"=>scan.environment.id="different-setup".into(),
                "revision"=>scan.environment.revision+=1,
                "runner"=>scan.environment.runner.id="different-runner".into(),
                "host"=>scan.host.sha256="bb".repeat(32),
                "source"=>scan.host_source_sha256="cc".repeat(32),
                _=>unreachable!(),
            }
            atomic_json(&path,&scan).unwrap();assert!(help().is_empty(),"{change}");scan=original;
        }
        atomic_json(&path,&scan).unwrap();fs::write(report,b"changed report").unwrap();
        assert!(help().is_empty());
        fs::write(environment.root.join("compatdata/pfx/system.reg"),
            r"[System\\ControlSet001\\Services\\PaceLicenseDServices] 1").unwrap();
        assert!(help()[0].contains("whether it runs has not been checked"));
        assert_eq!(selected(environment).unwrap(),vec!["PaceLicenseDServices"]);
    }
    #[test]
    fn only_exact_registered_declared_service_selects_dependency() {
        for key in [r"System\\CurrentControlSet\\Services\\PaceLicenseDServices",
            r"System\\ControlSet001\\Services\\PaceLicenseDServices"] {
            assert!(registered(&format!("[{key}] 1\n"),"PaceLicenseDServices"));
        }
        for key in [r"Software\\PaceLicenseDServices",r"System\\ControlSet001\\Services\\Bonjour",
            r"System\\ControlSet001\\Services\\PaceLicenseDServices\\Parameters"] {
            assert!(!registered(&format!("[{key}] 1\n"),"PaceLicenseDServices"));
        }
    }
    #[test]
    fn dependency_help_reads_a_hive_again_only_once_it_changed() {
        let f=crate::test_fixture::Fixture::new();
        let environment=&f.r.environment;
        let path=environment.root.join("compatdata/pfx/system.reg");
        fs::write(&path,"[System\\\\ControlSet001\\\\Services\\\\PaceLicenseDServices] 1\n").unwrap();
        let written=std::time::SystemTime::now();
        let read_at=|read| {
            let (services,opened)=linux_vst_bridge::with_opened_reads(||
                registered_services_read_at(environment,read).unwrap());
            (services.unwrap(),opened.unwrap().get(&path).map(|(_,bytes)|*bytes))
        };
        let pace=BTreeSet::from(["PaceLicenseDServices".to_owned()]);
        // Read within two seconds of its last change, a hive is read every
        // time: a later write could still share its change time.
        assert!(matches!(read_at(written),(ref services,Some(bytes)) if *services==pace && bytes>0));
        assert!(matches!(read_at(written),(ref services,Some(bytes)) if *services==pace && bytes>0));
        let later=written+Duration::from_secs(3);
        assert!(matches!(read_at(later),(ref services,Some(bytes)) if *services==pace && bytes>0));
        assert_eq!(read_at(later),(pace.clone(),Some(0)),
            "an unchanged, settled hive is not read again, but is still named for the view's watch");
        // A replacement of the same size and modification time is read again.
        let replacement=path.with_file_name("system.reg.new");
        fs::write(&replacement,"[System\\\\ControlSet001\\\\Services\\\\PaceLicenseDServicex] 1\n").unwrap();
        let modified=fs::metadata(&path).unwrap().modified().unwrap();
        fs::File::options().write(true).open(&replacement).unwrap().set_modified(modified).unwrap();
        assert_eq!(fs::metadata(&replacement).unwrap().len(),fs::metadata(&path).unwrap().len());
        fs::rename(&replacement,&path).unwrap();
        assert!(matches!(read_at(later),(ref services,Some(bytes)) if services.is_empty() && bytes>0),
            "a replaced hive is read again");
        fs::write(&path,"[System\\\\ControlSet001\\\\Services\\\\Other] 1\n").unwrap();
        assert!(matches!(read_at(later),(ref services,Some(bytes)) if services.is_empty() && bytes>0),
            "a rewritten hive is read again");
        fs::remove_file(&path).unwrap();
        assert_eq!(registered_services_read_at(environment,later).unwrap(),None);
        assert!(!REGISTRATIONS.lock().unwrap().contains_key(&path),"a removed hive leaves nothing kept");
    }
    #[test]
    fn a_hive_is_settled_only_two_seconds_after_its_last_change() {
        let changed=|seconds,nanoseconds| HiveIdentity {device:0,inode:0,size:0,modified:(0,0),
            changed:(seconds,nanoseconds)};
        let at=|seconds| std::time::UNIX_EPOCH+Duration::from_secs(seconds);
        assert!(!changed(100,0).settled_before(at(101)));
        assert!(!changed(100,0).settled_before(at(102)));
        assert!(changed(100,0).settled_before(at(103)));
        assert!(!changed(-1,0).settled_before(at(103)),"a change time before 1970 never is");
        assert!(!changed(i64::MAX,0).settled_before(at(103)));
        assert!(!changed(100,2_000_000_000).settled_before(at(103)));
    }
}
