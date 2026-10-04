use linux_vst_bridge::*;
mod managed_cli;
#[cfg(test)]
mod test_fixture;
mod vendor_cli;
mod renderer_cli;
mod native_access_runner;
mod experimental_runner;
mod vendor_product_cli;
mod operator_cli;
mod readiness;
mod graphics_cli;
mod installer_import;
mod installer_source;
mod onboarding;
mod daw_workspace;
mod preparation_cli;
mod setup_install;
mod package_authority;
mod audio_scheduling;
#[cfg(feature = "pb0-r3-audit")]
mod pb0_r3_audit;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{
    fs,
    io::{Read, Write},
    os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd},
    os::unix::{
        fs::{OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use catalogue::Software;

struct KeeperOwner {
    session: String,
    environment: String,
    graphical_session: Option<transport_storage::GraphicalSession>,
    child: Child,
    report: PathBuf,
    lease: PathBuf,
    retiring: bool,
    failed: bool,
    failure_pending: bool,
    started: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeeperAvailability { Starting, Retiring, Ready, Failed }

const KEEPER_OWNER_STARTUP_SECONDS: u64 = 60;
const KEEPER_MANAGER_RETIRE_SECONDS: u64 = 62;

fn prepare_recovery_bindings(
    m: &Manager,
    software: &Software,
    registrations: Vec<Registration>,
    verification: &LaunchVerification,
    deadline: Instant,
) -> Result<(Vec<(Registration, Software)>, LaunchSnapshot)> {
    verification.prepare(deadline, || {
        registrations.into_iter().map(|registration| {
            registration.verify(&m.root)?;
            let execution = package_authority::paired_components(m, software, &registration)?;
            Ok((registration, execution))
        }).collect()
    })
}

fn require_recovery_registration(m: &Manager, registration: &Registration) -> Result<()> {
    m.require_inactive(None)?;
    require(m.registry()?.classes.get(&registration.metadata.class_id)
        .is_some_and(|entry| entry.registration == *registration),
        "service_recovery_registration_changed")
}

fn verify_recovery_binding(m: &Manager, selected: &Software,
    registration: &Registration, execution: &Software) -> Result<()> {
    require_recovery_registration(m, registration)?;
    registration.verify(&m.root)?;
    require(package_authority::paired_components(m, selected, registration)? == *execution,
        "service_recovery_components_changed")
}

fn stage_recovery_keeper(m: &Manager, execution: &Software, registration: &Registration,
    keepers: &Keepers, ready_owners: &mut std::collections::BTreeMap<String, String>)
    -> Result<KeeperAvailability> {
    require_recovery_registration(m, registration)?;
    let binding: HostBinding = registration.clone().into();
    let readiness = stage_keeper(m, execution, &binding, keepers, None)?;
    if readiness == KeeperAvailability::Ready {
        let session = keeper_session(keepers, &registration.environment.id)?
            .ok_or("ready_keeper_ownership_missing")?;
        let expected = ready_owners.entry(registration.environment.id.clone())
            .or_insert_with(|| session.clone());
        require(*expected == session, "service_recovery_owner_changed")?;
    }
    Ok(readiness)
}

fn restore_recovery_bindings<R>(
    bindings: &[(Registration, Software)], deadline: Instant,
    mut reserve: impl FnMut() -> Result<R>,
    mut stage: impl FnMut(&Registration, &Software) -> Result<KeeperAvailability>,
    mut verify: impl FnMut(&Registration, &Software) -> Result<()>,
    mut pause: impl FnMut(Duration),
) -> Result<()> {
    // Start each exact environment before waiting for any one Wine startup.
    // Starting is retained ownership, never a readiness acknowledgment.
    for (registration, execution) in bindings {
        require(Instant::now() < deadline, "service_recovery_deadline")?;
        let _admission = reserve()?;
        if stage(registration, execution)? == KeeperAvailability::Failed {
            return Err(capacity::Refusal::BindingInvalid.into());
        }
    }
    for (registration, execution) in bindings {
        let _admission = wait_for_keeper(deadline, &mut reserve,
            || stage(registration, execution), &mut pause)?;
    }
    // Upstream startup creates hard links into its mutable runtime copy. Full
    // byte verification belongs after every owned keeper is ready, outside
    // registry ownership; neither a timestamp nor a busy readback is readiness.
    for (registration, execution) in bindings {
        require(Instant::now() < deadline, "service_recovery_deadline")?;
        verify(registration, execution)?;
    }
    // Take fresh authority after verification. The stage callback rechecks the
    // exact registration and owner; any changed/starting owner refuses the ack.
    let _admission = reserve()?;
    for (registration, execution) in bindings {
        require(stage(registration, execution)? == KeeperAvailability::Ready,
            "service_recovery_owner_changed")?;
    }
    require(Instant::now() < deadline, "service_recovery_deadline")
}

// Startup work runs on a control worker. Waiting for the shared environment
// owner releases the registry reservation, so scans and status requests cannot
// be serialized behind cold Wine initialization. Every retry rechecks capacity
// and exact publication authority before a DSP lease can become visible.
fn wait_for_keeper<R>(deadline: Instant, mut reserve: impl FnMut() -> Result<R>,
    mut stage: impl FnMut() -> Result<KeeperAvailability>,
    mut pause: impl FnMut(Duration)) -> Result<R> {
    loop {
        if Instant::now() >= deadline { return Err(capacity::Refusal::ServiceBusy.into()); }
        match reserve() {
            Ok(reservation) => match stage()? {
                KeeperAvailability::Ready => return Ok(reservation),
                KeeperAvailability::Failed => return Err(capacity::Refusal::BindingInvalid.into()),
                KeeperAvailability::Starting | KeeperAvailability::Retiring => drop(reservation),
            },
            Err(error) if error.downcast_ref::<capacity::Refusal>()
                == Some(&capacity::Refusal::ServiceBusy) => {},
            Err(error) => return Err(error),
        }
        pause(Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())));
    }
}

struct NativeStartup<'a> {
    manager: &'a Manager,
    request: [u8; 16],
    class: String,
    started: Instant,
    phases: Vec<(&'static str, u64)>,
    finished: bool,
}
impl<'a> NativeStartup<'a> {
    fn new(manager: &'a Manager, request: [u8; 16], class: String) -> Self {
        Self { manager, request, class, started: Instant::now(),
            phases: vec![("request_received", 0)], finished: false }
    }
    fn phase(&mut self, name: &'static str) {
        if self.phases.len()<16 {
            self.phases.push((name, self.started.elapsed().as_millis() as u64));
        }
    }
    fn finish(&mut self, outcome: &'static str) {
        self.finished = true;
        let directory = self.manager.root.join("runtime/native-startup");
        let retained = (|| -> Result<()> {
            private_dir(&directory)?;
            atomic_json(&directory.join(format!("{}.json", self.class.to_ascii_lowercase())),
                &serde_json::json!({"schema":1,"class_id":self.class,
                    "request":hex(&self.request),"observed_at":observation::now()?,
                    "outcome":outcome,"phases":self.phases,
                    "total_ms":self.started.elapsed().as_millis() as u64}))
        })();
        if retained.is_err() { eprintln!("native startup measurement unavailable"); }
    }
}
impl Drop for NativeStartup<'_> {
    fn drop(&mut self) { if !self.finished { self.finish("refused"); } }
}

fn retain_admission_incident(m:&Manager,source:&str,request:Option<[u8;16]>,
    class_id:Option<&str>,environment:Option<&str>,keeper_session:Option<&str>)->Result<()> {
    require(matches!(source,"registry_busy"|"worker_ceiling"|"worker_unavailable"|"keeper_starting"|
        "keeper_retiring"|"keeper_failed"),"admission incident class")?;
    if let Some(class_id)=class_id {require(valid_hex(class_id,32),"admission incident identity")?;}
    if let Some(keeper_session)=keeper_session {require(valid_hex(keeper_session,32),"admission incident identity")?;}
    if let Some(environment)=environment {
        require(!environment.is_empty() && environment.len()<=128
            && environment.bytes().all(|value|value.is_ascii_alphanumeric() || value==b'-' || value==b'_'),
            "admission incident identity")?;
    }
    let directory=m.root.join("runtime/admission-incidents");
    private_dir(&directory)?;
    let key=if let Some(environment)=environment {
        format!("environment-{}",environment.to_ascii_lowercase())
    } else if let Some(class_id)=class_id {
        format!("class-{}",class_id.to_ascii_lowercase())
    } else {"service".into()};
    atomic_json(&directory.join(format!("{key}.json")),&serde_json::json!({
        "schema":1,
        "observed_at":observation::now()?,
        "source":source,
        "request":request.map(|value|hex(&value)),
        "class_id":class_id,
        "environment":environment,
        "keeper_session":keeper_session,
    }))
}

fn keeper_incident_source(status:KeeperAvailability)->Option<&'static str> {
    match status {
        KeeperAvailability::Starting=>Some("keeper_starting"),
        KeeperAvailability::Retiring=>Some("keeper_retiring"),
        KeeperAvailability::Failed=>Some("keeper_failed"),
        KeeperAvailability::Ready=>None,
    }
}

fn keeper_session(keepers:&Keepers,environment:&str)->Result<Option<String>> {
    let active=keepers.lock().map_err(|_|"environment ownership lock poisoned")?;
    Ok(active.iter().find(|owner|owner.environment==environment)
        .map(|owner|owner.session.clone()))
}

fn retain_keeper_incident(m:&Manager,keepers:&Keepers,status:KeeperAvailability,
    request:[u8;16],class_id:&str,environment:&str)->Result<()> {
    let Some(source)=keeper_incident_source(status) else {return Ok(())};
    let session=keeper_session(keepers,environment)?;
    retain_admission_incident(m,source,Some(request),Some(class_id),Some(environment),
        session.as_deref())
}

type Keepers = Mutex<Vec<KeeperOwner>>;
fn refuse_unclassified_worker(manager: &Manager, peer: Option<&mut UnixStream>, source: &str) {
    // No worker exists to authenticate/classify this request. Retain a service
    // incident and grant no session; the same bounded refusal covers the
    // configured worker ceiling and an actual OS thread-creation failure.
    if let Err(error) = retain_admission_incident(manager, source, None, None, None, None) {
        eprintln!("admission incident unavailable: {error}");
    }
    if let Some(peer) = peer {
        if peer.set_write_timeout(Some(Duration::from_millis(100))).is_ok() {
            let _ = startup_reply(peer,
                &ap1_native_client::admission::refused([0; 16], capacity::Refusal::ServiceBusy));
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SessionSpec {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    onboarding_home: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    graphics_assessment: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    crash_capture: Option<crash_capture::Capture>,
    registration: HostBinding,
    session: String,
    directory: PathBuf,
    report: PathBuf,
    lease: PathBuf,
    inspect: bool,
    first_audio: bool,
    keeper: bool,
    binding_sent: bool,
    #[serde(default)]
    vendor_access: bool,
    #[serde(default)]
    shared_inspection: bool,
    #[serde(default)]
    shared_runtime: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runner_key: Option<String>,
    #[serde(default)]
    transport: Option<transport_storage::MemoryTransport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    graphical_session: Option<transport_storage::GraphicalSession>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    keeper_startup_seconds: Option<u64>,
}
// Unexposed admission owns only a reservation. Once the native binding is
// exposed, only positive supervisor retirement can release that ownership.
struct PendingAdmission {
    lease: Option<PathBuf>,
    blocked: Arc<AtomicBool>,
    exposed: bool,
}
impl PendingAdmission {
    fn new(lease: PathBuf, blocked: Arc<AtomicBool>) -> Self {
        Self {
            lease: Some(lease),
            blocked,
            exposed: false,
        }
    }
    fn expose(&mut self) {
        self.exposed = true;
    }
    fn complete(
        &mut self,
        session: &str,
        success: bool,
        receipt: &str,
        transport: Option<&Path>,
    ) -> Result<()> {
        require(self.exposed, "admission retirement before exposure")?;
        require(
            success && receipt == format!("LVO1 {session} retired\n"),
            "instance owner did not confirm cleanup; new admissions blocked",
        )?;
        if let Some(directory) = transport {
            require(!directory.try_exists()?, "transport retirement unconfirmed")?;
        }
        if let Some(lease) = &self.lease {
            fs::remove_file(lease)?;
            self.lease = None;
        }
        Ok(())
    }
}
impl Drop for PendingAdmission {
    fn drop(&mut self) {
        if let Some(path) = &self.lease {
            if self.exposed {
                self.blocked.store(true, Ordering::Release);
            } else if let Err(e) = fs::remove_file(path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    // Failed unexposed cleanup cannot silently return a unit.
                    self.blocked.store(true, Ordering::Release);
                }
            }
        }
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ClassSelection {
    class_id: String,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
struct HostBinding {
    metadata: ClassSelection,
    environment: Environment,
    module: Artifact,
    host: Artifact,
    host_source_sha256: String,
    compatibility: Compatibility,
}
impl From<Registration> for HostBinding {
    fn from(r: Registration) -> Self {
        Self {
            metadata: ClassSelection {
                class_id: r.metadata.class_id,
            },
            environment: r.environment,
            module: r.module,
            host: r.host,
            host_source_sha256: r.host_source_sha256,
            compatibility: r.compatibility,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionRequest {
    environment_id: String,
    module: Artifact,
    class_id: String,
    compatibility: Compatibility,
}
// One decoder owns both admission and dispatch; a new exact route cannot be
// dispatched in a match arm that an earlier greeting filter never admits.
fn inspection_purpose(greeting: &[u8]) -> Option<Option<publication::Qualification>> {
    use publication::Qualification::*;
    Some(match greeting {
        b"LVI1\n" => None,
        b"LVQ1\n" => Some(Ap15Editor),
        b"LVQ2\n" => Some(Ap17Capacity),
        b"LVQ3\n" => Some(Ap18Pigments),
        b"LVQ4\n" => Some(Uir1Input),
        b"LVQ5\n" => Some(If1Failure),
        b"LVQ6\n" => Some(Frg1Ubuntu),
        _ => return None,
    })
}
#[test]
fn exact_inspection_greetings_share_admission_and_dispatch() {
    use publication::Qualification::*;
    assert_eq!(inspection_purpose(b"LVI1\n"), Some(None));
    for (bytes, purpose) in [(b"LVQ1\n", Ap15Editor), (b"LVQ2\n", Ap17Capacity),
        (b"LVQ3\n", Ap18Pigments), (b"LVQ4\n", Uir1Input), (b"LVQ5\n", If1Failure),
        (b"LVQ6\n", Frg1Ubuntu)] {
        assert_eq!(inspection_purpose(bytes), Some(Some(purpose)));
    }
    for invalid in [b"LVQ7\n".as_slice(), b"LVQ6", b"lvq6\n", b"LVQ6\nextra"] {
        assert_eq!(inspection_purpose(invalid), None);
    }
}

#[test]
fn cold_keeper_wait_releases_reservation_and_rechecks_each_attempt() {
    use std::cell::Cell;
    struct Reservation<'a>(&'a Cell<bool>);
    impl Drop for Reservation<'_> { fn drop(&mut self) { self.0.set(false); } }
    let held=Cell::new(false);
    let stages=Cell::new(0);
    let pauses=Cell::new(0);
    let reservation=wait_for_keeper(Instant::now()+Duration::from_secs(1),|| {
        assert!(!held.replace(true));
        Ok(Reservation(&held))
    },|| {
        assert!(held.get());
        stages.set(stages.get()+1);
        Ok(if stages.get()<3 {KeeperAvailability::Starting} else {KeeperAvailability::Ready})
    }, |_| {
        assert!(!held.get());
        pauses.set(pauses.get()+1);
    }).unwrap();
    assert!(held.get());
    assert_eq!((stages.get(),pauses.get()),(3,2));
    drop(reservation);
    assert!(!held.get());
}

#[test]
fn keeper_wait_retries_only_busy_and_never_hides_failure_or_deadline() {
    let mut reservations=0;
    let mut pauses=0;
    wait_for_keeper(Instant::now()+Duration::from_secs(1),|| {
        reservations+=1;
        if reservations==1 {Err(capacity::Refusal::ServiceBusy.into())} else {Ok(())}
    },|| Ok(KeeperAvailability::Ready),|_| pauses+=1).unwrap();
    assert_eq!((reservations,pauses),(2,1));
    let error=wait_for_keeper(Instant::now()+Duration::from_secs(1),
        || Ok(()),|| Ok(KeeperAvailability::Failed),|_| panic!("failed keeper retry")).unwrap_err();
    assert_eq!(error.downcast_ref::<capacity::Refusal>(),Some(&capacity::Refusal::BindingInvalid));
    let error=wait_for_keeper::<()>(Instant::now(),|| panic!("expired reservation"),
        || panic!("expired preparation"),|_| panic!("expired pause")).unwrap_err();
    assert_eq!(error.downcast_ref::<capacity::Refusal>(),Some(&capacity::Refusal::ServiceBusy));
    assert!(wait_for_keeper::<()>(Instant::now()+Duration::from_secs(1),
        || Err("capacity unavailable".into()),|| panic!("no admission"),
        |_| panic!("non-busy retry")).is_err());
}
fn software(m: &Manager) -> Result<Software> {
    let s = software_record(m)?;
    for a in [
        &s.manager,
        &s.supervisor,
        &s.ownership,
        &s.host,
        &s.source_manifest,
    ] {
        a.verify()?;
    }
    require(
        s.source_manifest.sha256 == s.source_sha256,
        "host source manifest differs",
    )?;
    if let Some(a) = &s.preparation_kit { a.verify()?; }
    if let Some(a) = &s.installer_launch { a.verify()?; }
    if let Some(a) = &s.operator_frontend {
        a.verify()?;
    }
    if let Some(a) = &s.native_catalogue {
        a.verify()?;
    }
    Ok(s)
}
fn software_record(m: &Manager) -> Result<Software> {
    let s: Software = read_json(&m.root.join("software.json"))?;
    s.validate_record()?;
    Ok(s)
}
fn systemd(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
    )
}
fn setup(m: &Manager, package: Option<&Path>) -> Result<()> {
    setup_selected(m, package, Acceptance::Editor)
}
#[derive(Clone, Copy)]
enum Acceptance { Editor, Capacity, Pigments, Uir1, Uio2 }
fn setup_selected(m: &Manager, package: Option<&Path>, acceptance: Acceptance) -> Result<()> {
    let review = match acceptance {
        Acceptance::Editor => acceptance::REVIEW,
        Acceptance::Capacity => acceptance::CAPACITY_REVIEW,
        Acceptance::Pigments => acceptance::pigments::REVIEW,
        Acceptance::Uir1 => acceptance::uir1::REVIEW,
        Acceptance::Uio2 => acceptance::uio2::REVIEW,
    };
    let _lock = m.lock("setup.lock")?;
    let _registry = m.lock("registry.lock")?;
    // A stopped prior service may leave a positively retired keeper lease.
    // Use the existing ownership receipt rule before the inactivity check;
    // never erase an unresolved lease merely to make setup succeed.
    if package.is_some() {
        let _unresolved = reconcile_leases(m)?;
    }
    m.require_inactive(None)?;
    let previous = if m.root.join("software.json").try_exists()? {
        Some(read_json::<Software>(&m.root.join("software.json"))?)
    } else {
        None
    };
    let me = std::env::current_exe()?;
    let (catalogue, files, source) = if let Some(package) = package {
        let profiles = profiles::installed_profiles()?;
        let source: String = read_json(&package.join("host-source.json"))?;
        let catalogue = catalogue::setup_adoption_for_host(m, &profiles, &digest(&package.join("host.exe"))?, &source)?;
        (
            catalogue,
            [
                ("linux-vst-bridge", me),
                ("session.py", package.join("session.py")),
                ("ownership.py", package.join("ownership.py")),
                ("host.exe", package.join("host.exe")),
                (
                    "host-source-manifest.json",
                    package.join("host-source-manifest.json"),
                ),
            ],
            source,
        )
    } else {
        let accepted = match acceptance {
            Acceptance::Editor => acceptance::prepare(m)?,
            Acceptance::Capacity => acceptance::prepare_capacity(m)?,
            Acceptance::Pigments => acceptance::pigments::prepare(m)?,
            Acceptance::Uir1 => acceptance::uir1::prepare(m)?,
            Acceptance::Uio2 => acceptance::uio2::prepare(m)?,
        };
        let old = software(m)?;
        for artifact in [&old.supervisor, &old.ownership] {
            require(
                artifact.path.parent() == old.manager.path.parent()
                    && artifact.path.starts_with(m.root.join("software"))
                    && artifact.path.canonicalize()? == artifact.path
                    && file(&artifact.path)?.metadata()?.permissions().mode() & 0o222 == 0,
                "acceptance_runtime_identity",
            )?;
        }
        // This transition reuses retained immutable runtime helpers only when
        // they exactly equal this manager's compiled source. No runtime input.
        require(
            old.supervisor.sha256
                == hex(&sha2::Sha256::digest(include_bytes!(
                    "../runtime/session.py"
                )))
                && old.ownership.sha256
                    == hex(&sha2::Sha256::digest(include_bytes!(
                        "../runtime/ownership.py"
                    ))),
            "acceptance_runtime_identity",
        )?;
        (
            Some(accepted.catalogue),
            [
                ("linux-vst-bridge", me),
                ("session.py", old.supervisor.path),
                ("ownership.py", old.ownership.path),
                ("host.exe", accepted.host.path),
                ("host-source-manifest.json", accepted.source_manifest.path),
            ],
            accepted.source_manifest.sha256,
        )
    };
    let mut files=files.to_vec();
    let frontend = package
        .map(|p| p.join("linux-audio-compatibility-manager"))
        .filter(|p| p.exists());
    let retained_frontend =
        setup_install::retained_frontend(frontend.as_deref(), previous.as_ref())?;
    if let Some(path)=frontend {files.push(("linux-audio-compatibility-manager",path));}
    require(
        valid_hex(&source, 64) && digest(&files[4].1)? == source,
        "host source manifest hash differs",
    )?;
    let kit=package.map(|p|p.join("preparation-kit.zip")).filter(|p|p.exists());
    let retained_kit=previous.as_ref().and_then(|s|s.preparation_kit.clone()).filter(|_|kit.is_none());
    if let Some(a)=&retained_kit {a.verify()?;}
    if let Some(path)=kit {files.push(("preparation-kit.zip",path));}
    let (adapter, retained_adapter) =
        setup_install::installer_launch_inputs(package, previous.as_ref())?;
    if let Some(path)=adapter {files.push(("installer-launch.exe",path));}
    let mut identity = String::new();
    if let Some(a)=&retained_adapter {identity.push_str(&serde_json::to_string(a)?);}
    if let Some(a)=&retained_kit {identity.push_str(&serde_json::to_string(a)?);}

    for (_, p) in &files {
        identity.push_str(&digest(p)?);
    }
    if let Some(c) = &catalogue {
        identity.push_str(&hex(&sha2::Sha256::digest(serde_json::to_vec(c)?)));
    }
    if package.is_none() {
        identity.push_str(&hex(&sha2::Sha256::digest(review)));
    }
    if let Some(a) = &retained_frontend {
        identity.push_str(&serde_json::to_string(a)?);
    }
    let id = hex(&sha2::Sha256::digest(identity.as_bytes()));
    let dest = m.root.join("software").join(&id);
    catalogue::verify_host_path(&dest.join("host.exe"))?;
    if let Some(c) = &catalogue {
        for h in &c.hosts {
            catalogue::verify_host_path(&dest.join("hosts").join(h.directory_name()).join("host.exe"))?;
        }
    }
    if let Ok(old) = read_json::<Software>(&m.root.join("software.json")) {
        if old.manager.path != dest.join("linux-vst-bridge") {
            let running = Command::new("systemctl")
                .args(["--user", "is-active", "--quiet", "linux-vst-bridge.service"])
                .status()?;
            require(
                !running.success(),
                "close devices and stop the bridge service before replacing installed software",
            )?;
        }
    }
    if !dest.try_exists()? {
        private_dir(dest.parent().unwrap())?;
        let stage = dest.with_file_name(format!("stage-{}", random_id()?));
        private_dir(&stage)?;
        for (name, p) in &files {
            let to = stage.join(name);
            fs::copy(p, &to)?;
            require(digest(&to)? == digest(p)?, "software copy differs")?;
            fs::set_permissions(
                &to,
                fs::Permissions::from_mode(if matches!(*name, "linux-vst-bridge" | "linux-audio-compatibility-manager") {
                    0o500
                } else {
                    0o400
                }),
            )?;
            fs::File::open(to)?.sync_all()?;
        }
        if let Some(mut c) = catalogue.clone() {
            for h in &mut c.hosts {
                let relative = PathBuf::from("hosts").join(h.directory_name());
                private_dir(&stage.join(&relative))?;
                for (name, a) in [("host.exe", &mut h.host), ("host-source-manifest.json", &mut h.source_manifest)] {
                    a.verify()?;
                    let to = stage.join(&relative).join(name);
                    fs::copy(&a.path, &to)?;
                    require(digest(&to)? == a.sha256, "acceptance_artifact_identity")?;
                    fs::set_permissions(&to, fs::Permissions::from_mode(0o400))?;
                    fs::File::open(&to)?.sync_all()?;
                    a.path = dest.join(&relative).join(name);
                }
                fs::File::open(stage.join(&relative))?.sync_all()?;
            }
            if !c.hosts.is_empty() { fs::File::open(stage.join("hosts"))?.sync_all()?; }
            private_dir(&stage.join("proxies"))?;
            for n in &mut c.natives {
                let name = format!("{}.so", n.artifact.sha256);
                let copy = stage.join("proxies").join(&name);
                fs::copy(&n.artifact.path, &copy)?;
                require(
                    digest(&copy)? == n.artifact.sha256,
                    "native_package_copy_changed",
                )?;
                fs::set_permissions(&copy, fs::Permissions::from_mode(0o500))?;
                fs::File::open(&copy)?.sync_all()?;
                n.artifact.path = dest.join("proxies").join(name);
            }
            atomic_json(&stage.join("native-catalogue.json"), &c)?;
            fs::set_permissions(
                stage.join("native-catalogue.json"),
                fs::Permissions::from_mode(0o400),
            )?;
            fs::File::open(stage.join("proxies"))?.sync_all()?;
        }
        if package.is_none() {
            let path = stage.join("acceptance-review.json");
            fs::write(&path, review)?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o400))?;
            fs::File::open(path)?.sync_all()?;
        }
        fs::File::open(&stage)?.sync_all()?;
        fs::rename(&stage, &dest)?;
        fs::File::open(dest.parent().unwrap())?.sync_all()?;
    }
    let a = |n: &str| -> Result<Artifact> {
        let path = dest.join(n);
        Ok(Artifact {
            sha256: digest(&path)?,
            path,
        })
    };
    for (name, input) in &files {
        require(
            digest(&dest.join(name))? == digest(input)?,
            "software copy differs",
        )?;
    }
    if package.is_none() {
        require(
            fs::read(dest.join("acceptance-review.json"))? == review,
            "acceptance_review_identity",
        )?;
    }
    if let Some(mut expected) = catalogue.clone() {
        for h in &mut expected.hosts {
            let dir = dest.join("hosts").join(h.directory_name());
            h.host.path = dir.join("host.exe");
            h.source_manifest.path = dir.join("host-source-manifest.json");
        }
        for n in &mut expected.natives {
            n.artifact.path = dest
                .join("proxies")
                .join(format!("{}.so", n.artifact.sha256));
        }
        let actual: catalogue::Catalogue = read_json(&dest.join("native-catalogue.json"))?;
        require(actual == expected, "native_artifact_mismatch")?;
        actual.validate(&m.root)?;
    }
    let installed = Software {
        installer_launch: if files.iter().any(|(n,_)|*n=="installer-launch.exe"){Some(a("installer-launch.exe")?)}else{retained_adapter},
        preparation_kit: if files.iter().any(|(n,_)|*n=="preparation-kit.zip"){Some(a("preparation-kit.zip")?)}else{retained_kit},
        operator_frontend: if files.iter().any(|(n,_)|*n=="linux-audio-compatibility-manager") {Some(a("linux-audio-compatibility-manager")?)}else{retained_frontend},
        manager: a("linux-vst-bridge")?,
        supervisor: a("session.py")?,
        ownership: a("ownership.py")?,
        host: a("host.exe")?,
        source_manifest: a("host-source-manifest.json")?,
        source_sha256: source,
        native_catalogue: if catalogue.is_some() {
            Some(a("native-catalogue.json")?)
        } else {
            None
        },
    };
    require(
        installed.source_manifest.sha256 == installed.source_sha256,
        "host source manifest hash differs",
    )?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    setup_install::commit(m, &home, &installed, previous.as_ref(), None)?;
    // Publication/setup records are complete. Let the service's startup
    // reconcile acquire the same nonblocking admission lock immediately.
    drop(_registry);
    for args in [
        &["--user", "daemon-reload"][..],
        &["--user", "enable", "--now", "linux-vst-bridge.service"][..],
    ] {
        require(
            Command::new("systemctl").args(args).status()?.success(),
            "automatic service setup failed",
        )?;
    }
    Ok(())
}
fn spec(
    m: &Manager,
    r: HostBinding,
    inspect: bool,
    first_audio: bool,
    keeper: bool,
) -> Result<(SessionSpec, PathBuf)> {
    let sid = random_id()?;
    let directory = r
        .environment
        .root
        .join("compatdata/pfx/drive_c/bridge/sessions")
        .join(&sid);
    private_dir(&directory)?;
    let results = m.root.join("runtime/results");
    private_dir(&results)?;
    let leases = m.root.join("runtime/leases");
    private_dir(&leases)?;
    let onboarding_home = r.metadata.class_id == managed_candidate::candidate()?.class.class_id
        || m.root.join("onboarding").join(&r.environment.id).join("record.json").exists();
    if onboarding_home {
        if keeper {
            let current:Software=read_json(&m.root.join("software.json"))?;
            let paired=if r.host==current.host && r.host_source_sha256==current.source_sha256 {current}
                else {package_authority::paired_host_components(m,&current,&r.host,&r.host_source_sha256)?};
            require(r.host==paired.host && r.host_source_sha256==paired.source_sha256,"keeper_software_binding_changed")?;
            require(onboarding::history_records(m)?.iter().any(|h|h.environment==r.environment),"keeper_environment_binding_changed")?;
        } else if inspect && onboarding::history_records(m)?.iter().any(|h|h.environment==r.environment) {
            // The maintenance inspector is independently installed and may be
            // newer than a retained product runtime. It grants no DSP authority.
        } else if preparation::session_binding(m,&r.metadata.class_id,&r.environment,&r.module,&r.host,&r.host_source_sha256)? {
            // Exact managed preparation/acceptance authority, including adoption.
        } else if r.metadata.class_id == managed_candidate::candidate()?.class.class_id {
            managed_candidate::check_session(m,&r.metadata.class_id,&r.environment,&r.module,&r.host,&r.host_source_sha256)?;
        } else {
            return Err("managed_session_preparation_required".into());
        }
    }
    let runner_key = catalogue::runner_key(&r.environment.runner)?;
    let s = SessionSpec {
        onboarding_home,
        graphics_assessment: false,
        crash_capture: None,
        registration: r,
        session: sid.clone(),
        directory: directory.clone(),
        report: results.join(format!(
            "{}-{sid}.json",
            if keeper { "environment" } else { "windows" }
        )),
        lease: leases.join(format!("{sid}.json")),
        inspect,
        first_audio,
        keeper,
        binding_sent: !inspect,
        vendor_access: false,
        shared_inspection: false,
        shared_runtime: false,
        runner_key: Some(runner_key),
        transport: None,
        graphical_session: None,
        keeper_startup_seconds: keeper.then_some(KEEPER_OWNER_STARTUP_SECONDS),
    };
    let path = directory.join("owner.json");
    atomic_json(&path, &s)?;
    Ok((s, path))
}
fn spawn(m: &Manager, s: &Software, path: &Path, peer: Option<UnixStream>) -> Result<Child> {
    s.supervisor.verify()?;
    s.ownership.verify()?;
    let bound_peer = peer.is_some();
    let stdin = if let Some(p) = peer {
        unsafe { Stdio::from_raw_fd(p.into_raw_fd()) }
    } else {
        Stdio::null()
    };
    let job: SessionSpec = read_json(path)?;
    record_lease_generation(&job)?;
    m.retain_lease_owner(path)?;
    atomic_json(&job.lease, &job.report)?;
    let mut command = Command::new("/usr/bin/python3");
    command.arg(&s.supervisor.path);
    if job.registration.compatibility.graphics.is_some() && !job.keeper {
        // Older retained supervisors expect the first argument to be the job
        // file. A versioned entry point makes them refuse before preflight or
        // Windows launch, rather than discovering unsupported settings after
        // an owner may already have spawned children.
        command.arg("--graphics-settings-v1");
    }
    let child = command.arg(path)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        // The supervisor invokes this verified product executable on its
        // control plane. No scheduling subprocess belongs in a DAW callback.
        .env("LVB_AUDIO_SCHEDULER", serde_json::to_string(&s.manager)?)
        .spawn();
    if child.is_err() && !bound_peer {
        fs::remove_file(&job.lease)?;
    }
    Ok(child?)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LeaseGeneration {
    schema: u32,
    session: String,
    report: PathBuf,
    kernel_boot: String,
    basis: LeaseGenerationBasis,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LeaseGenerationBasis { BeforeLaunch, StoppedServiceObservation }
fn valid_kernel_boot(value: &str) -> bool {
    value.len() == 36 && value.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) { byte == b'-' }
        else { byte.is_ascii_digit() || matches!(byte, b'a'..=b'f') }
    })
}
fn kernel_boot() -> Result<Option<String>> {
    #[cfg(target_os = "linux")]
    {
        let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")?.trim().to_owned();
        require(valid_kernel_boot(&boot), "kernel_generation_identity")?;
        Ok(Some(boot))
    }
    #[cfg(not(target_os = "linux"))]
    { Ok(None) }
}
fn record_lease_generation(job: &SessionSpec) -> Result<()> {
    let boot = kernel_boot()?.ok_or("session_requires_linux_kernel_identity")?;
    let runtime = job.lease.parent().and_then(Path::parent).ok_or("lease_generation_path")?;
    require(job.lease == runtime.join("leases").join(format!("{}.json", job.session))
        && job.report.parent() == Some(runtime.join("results").as_path())
        && valid_hex(&job.session, 32), "lease_generation_binding")?;
    let directory = runtime.join("lease-generations");
    private_dir(&directory)?;
    let path = directory.join(format!("{}.json", job.session));
    require(!path.try_exists()?, "lease_generation_already_owned")?;
    // Persist the kernel lifetime before a lease or process can exist. A later
    // manager may prove interruption without recycling a PID or inventing a
    // successful plug-in result. Historical leases receive no inferred boot.
    atomic_json(&path, &LeaseGeneration {schema:1, session:job.session.clone(),
        report:job.report.clone(), kernel_boot:boot, basis:LeaseGenerationBasis::BeforeLaunch})
}
// Caller has stopped the exact selected service and holds registry.lock. This
// is a current observation, never an inference about a historical launch. Only
// a subsequent kernel restart can retire these unresolved keeper leases.
fn observe_stopped_leases(m: &Manager, boot: Option<&str>) -> Result<bool> {
    let Some(boot) = boot else { return Ok(false); };
    require(valid_kernel_boot(boot), "kernel_generation_identity")?;
    let owners = capacity::owners(m)?;
    require(owners.iter().all(|owner| owner.kind == capacity::Kind::Keeper),
        "package_owner_active")?;
    let directory = m.root.join("runtime/lease-generations");
    private_dir(&directory)?;
    for owner in &owners {
        let lease = m.root.join("runtime/leases").join(format!("{}.json", owner.session));
        let report: PathBuf = read_json(&lease)?;
        let path = directory.join(format!("{}.json", owner.session));
        if !path.try_exists()? {
            atomic_json(&path, &LeaseGeneration { schema:1, session:owner.session.clone(),
                report, kernel_boot:boot.into(), basis:LeaseGenerationBasis::StoppedServiceObservation })?;
        }
    }
    Ok(!owners.is_empty())
}
fn reconcile_leases(m: &Manager) -> Result<bool> {
    reconcile_leases_in_kernel(m, kernel_boot()?.as_deref())
}
fn reconcile_leases_in_kernel(m: &Manager, boot: Option<&str>) -> Result<bool> {
    require(boot.is_none_or(valid_kernel_boot), "kernel_generation_identity")?;
    let directory = m.root.join("runtime/leases");
    private_dir(&directory)?;
    let mut unconfirmed = false;
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let report: PathBuf = read_json(&path)?;
        let session = path.file_stem().and_then(|name| name.to_str()).ok_or("lease_session_identity")?;
        require(valid_hex(session, 32) && path.file_name().and_then(|name| name.to_str())
            == Some(format!("{session}.json").as_str()), "lease_session_identity")?;
        require(
            report.parent() == Some(m.root.join("runtime/results").as_path()),
            "lease report outside owned results",
        )?;
        let receipt = report.with_extension("ownership.json");
        let proof = if receipt.exists() {
            read_json::<serde_json::Value>(&receipt).and_then(|r| {
                require(
                    r["session"].as_str() == path.file_stem().and_then(|s| s.to_str())
                        && r["transport_retired"] == true,
                    "ownership receipt identity/retirement",
                )?;
                Ok(r)
            })
        } else {
            read_json::<serde_json::Value>(&report).and_then(|r| {
                require(
                    r["ownership_schema"].is_null(),
                    "new owner requires minimal retirement receipt",
                )?;
                Ok(r)
            })
        };
        match proof {
            Ok(r) if r["cleanup_confirmed"] == true => fs::remove_file(path)?,
            _ => {
                let generation_path = m.root.join("runtime/lease-generations")
                    .join(format!("{session}.json"));
                if generation_path.try_exists()? {
                    let generation: LeaseGeneration = read_json(&generation_path)?;
                    require(generation.schema == 1 && generation.session == session
                        && generation.report == report && valid_kernel_boot(&generation.kernel_boot),
                        "lease_generation_binding")?;
                    if boot.is_some_and(|current| current != generation.kernel_boot) {
                        let interrupted = generation_path.with_extension("interrupted.json");
                        atomic_json(&interrupted, &serde_json::json!({"schema":1,
                            "session":session,"retirement_basis":"kernel_generation_ended",
                            "observation_basis":generation.basis,
                            "cleanup_confirmed":true,"transport_retired":true,
                            "successful_session":false}))?;
                        fs::remove_file(path)?;
                        continue;
                    }
                }
                unconfirmed = true;
            },
        }
    }
    Ok(unconfirmed)
}
fn retired_keeper(owner: &KeeperOwner, status: ExitStatus) -> Result<()> {
    let report: serde_json::Value = read_json(&owner.report)?;
    require(report["ready"] == false && report["cleanup_confirmed"] == true,
        "environment owner exited without confirmed cleanup")?;
    require(status.success(), "environment owner exited unsuccessfully")?;
    if owner.lease.exists() { fs::remove_file(&owner.lease)?; }
    Ok(())
}

fn observe_keeper(environment: &str, active: &mut Vec<KeeperOwner>)
    -> Result<Option<KeeperAvailability>> {
    let Some(index)=active.iter().position(|owner|owner.environment==environment) else {
        return Ok(None);
    };
    if active[index].failed {
        return Ok(Some(KeeperAvailability::Failed));
    }
    if let Some(status)=active[index].child.try_wait()? {
        retired_keeper(&active[index],status)?;
        if active[index].retiring && !active[index].failure_pending {
            active.remove(index);
            return Ok(None);
        }
        // A cleanly contained unexpected keeper exit is a failed generation,
        // not permission to create another process on every native retry.
        active[index].failed=true;
        return Ok(Some(KeeperAvailability::Failed));
    }
    if active[index].retiring {
        return Ok(Some(KeeperAvailability::Retiring));
    }
    if !active[index].report.exists() {
        return Ok(Some(KeeperAvailability::Starting));
    }
    let status:serde_json::Value=read_json(&active[index].report)?;
    require(status["environment"]==environment,"environment readiness binding differs")?;
    if status["ready"]==true { Ok(Some(KeeperAvailability::Ready)) }
    else { Err("environment owner published a non-ready live result".into()) }
}

fn retire_mismatched_graphical_keeper(m:&Manager,environment:&str,
    requested:Option<&transport_storage::GraphicalSession>,active:&mut [KeeperOwner])
    -> Result<bool> {
    let Some(requested)=requested else {return Ok(false)};
    let Some(owner)=active.iter_mut().find(|owner|owner.environment==environment) else {
        return Ok(false);
    };
    if owner.graphical_session.as_ref().is_some_and(|bound|bound.same_display_context(requested))
        || owner.child.try_wait()?.is_some() {return Ok(false)}
    if owner.retiring {return Ok(true)}
    // Each DAW request retains its authenticated peer generation, while the
    // keeper belongs to the exact display context shared by those requests.
    m.require_inactive(None)?;
    require(unsafe{libc::kill(owner.child.id() as i32,libc::SIGTERM)}==0,
        "graphical keeper retirement request")?;
    owner.retiring=true;
    Ok(true)
}

fn stage_keeper(m:&Manager,s:&Software,r:&HostBinding,keepers:&Keepers,
    graphical_session:Option<&transport_storage::GraphicalSession>)
    -> Result<KeeperAvailability> {
    stage_keeper_with_history(m,s,r,keepers,graphical_session,
        experimental_runner::verify_selected_bg1_history,|software,path|spawn(m,software,path,None))
}

fn stage_keeper_with_history(m:&Manager,s:&Software,r:&HostBinding,keepers:&Keepers,
    graphical_session:Option<&transport_storage::GraphicalSession>,
    verify_history:impl Fn(&Manager,&Environment)->Result<()>,
    start_keeper:impl Fn(&Software,&Path)->Result<Child>)
    -> Result<KeeperAvailability> {
    // All keeper callers, including inspection and vendor access, must prove
    // selected V4 history before reusing an owner or creating session state.
    verify_history(m, &r.environment)?;
    let mut active=keepers.lock().map_err(|_|"environment ownership lock poisoned")?;
    if let Some(owner)=active.iter_mut().find(|owner|owner.environment==r.environment.id) {
        if !owner.retiring && !owner.report.exists()
            && owner.started.elapsed()>=Duration::from_secs(KEEPER_MANAGER_RETIRE_SECONDS)
            && owner.child.try_wait()?.is_none() {
            require(unsafe{libc::kill(owner.child.id() as i32,libc::SIGTERM)}==0,
                "environment startup retirement request")?;
            owner.retiring=true;
            owner.failure_pending=true;
            return Ok(KeeperAvailability::Retiring);
        }
    }
    let mut observed=observe_keeper(&r.environment.id,&mut active)?;
    if observed==Some(KeeperAvailability::Failed) {
        let index=active.iter().position(|owner|owner.environment==r.environment.id)
            .ok_or("failed environment owner absent")?;
        let same_context=match (active[index].graphical_session.as_ref(),graphical_session) {
            (Some(bound),Some(requested))=>bound.same_display_context(requested),
            (None,None)=>true,
            _=>false,
        };
        if same_context {return Ok(KeeperAvailability::Failed)}
        // A later, different authenticated graphical context may replace one
        // already reaped and cleanly retired failed generation exactly once.
        active.remove(index);
        observed=None;
    }
    if retire_mismatched_graphical_keeper(m,&r.environment.id,graphical_session,&mut active)? {
        return Ok(KeeperAvailability::Retiring);
    }
    if let Some(status)=observed {return Ok(status)}
    let mut keeper_binding=r.clone();
    keeper_binding.host=s.host.clone();
    keeper_binding.host_source_sha256=s.source_sha256.clone();
    let (mut job,path)=spec(m,keeper_binding,true,false,true)?;
    job.shared_runtime=true;
    job.graphical_session=graphical_session.cloned();
    atomic_json(&path,&job)?;
    let child=start_keeper(s,&path)?;
    // The keeper is retained before any retryable refusal. No native binding,
    // transport, or DSP lease has been exposed at this point.
    active.push(KeeperOwner{session:job.session.clone(),environment:r.environment.id.clone(),
        graphical_session:graphical_session.cloned(),child,report:job.report,lease:job.lease,
        retiring:false,failed:false,failure_pending:false,started:Instant::now()});
    Ok(KeeperAvailability::Starting)
}

// Worker admission and DSP ownership are separate. Full musical capacity still
// leaves bounded classifier capacity for truthful refusals and status.
struct WorkerCount(Arc<AtomicUsize>);
impl Drop for WorkerCount {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
fn startup_reply(peer: &mut UnixStream, bytes: &[u8]) -> Result<()> {
    require(
        bytes.len() <= ap1_native_client::admission::MAX_REPLY,
        "session binding size",
    )?;
    peer.write_all(&(bytes.len() as u16).to_le_bytes())?;
    peer.write_all(bytes)?;
    Ok(())
}
fn with_loaded_native_caller<T>(
    peer: &UnixStream,
    process: &transport_storage::PeerProcess,
    registration: &Registration,
    loaded: &ap1_native_client::admission::ExecutionIdentity,
    admitted: impl FnOnce() -> T,
) -> Result<T> {
    if registration.verify_loaded_execution(loaded).is_err()
        || transport_storage::peer_maps_artifact(peer, process, &registration.native.path).is_err() {
        return Err(capacity::Refusal::StaleNativeCaller.into());
    }
    Ok(admitted())
}
fn supervisor_ready(child: &mut Child, job: &SessionSpec, timeout: Duration) -> Result<()> {
    supervisor_ready_for_configuration(child, &job.session, job.registration.compatibility.graphics, timeout)
}
fn supervisor_ready_for_configuration(child: &mut Child, session: &str,
    graphics: Option<operator_model::GraphicsBackend>, timeout: Duration) -> Result<()> {
    let fd = child
        .stdout
        .as_ref()
        .ok_or("supervisor readiness output absent")?
        .as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    require(flags >= 0, "supervisor readiness output flags")?;
    require(
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == 0,
        "supervisor readiness output nonblocking",
    )?;
    // A retained supervisor may predate typed launch settings. Require its
    // capability acknowledgment before exposing the native instance. Existing
    // capable generations remain usable across manager/package updates.
    let capability = if graphics.is_some() { " graphics-v1" } else { "" };
    let expected = format!("LVO0 {session} ready{capability}\n").into_bytes();
    let mismatch = if graphics.is_some() {
        "Graphics-aware supervisor readiness was not confirmed; check compatibility to prepare a current configuration"
    } else { "supervisor readiness receipt differs" };
    let mut received = Vec::with_capacity(expected.len());
    let deadline = Instant::now() + timeout;
    while received.len() < expected.len() {
        let now = Instant::now();
        require(now < deadline, "supervisor readiness deadline")?;
        let remaining = deadline.saturating_duration_since(now);
        let mut descriptor = libc::pollfd {
            fd,
            events: libc::POLLIN | libc::POLLHUP | libc::POLLERR,
            revents: 0,
        };
        let millis = remaining.as_millis().min(i32::MAX as u128) as i32;
        let result = unsafe { libc::poll(&mut descriptor, 1, millis.max(1)) };
        require(result >= 0, "supervisor readiness poll")?;
        if result == 0 {
            continue;
        }
        let mut bytes = [0u8; 128];
        match child
            .stdout
            .as_mut()
            .ok_or("supervisor readiness output absent")?
            .read(&mut bytes[..(expected.len() - received.len()).min(128)])
        {
            // Process exit may race the parent, but a complete readiness
            // record already buffered in the pipe is still authoritative.
            // Drain the pipe before classifying its closure as pre-readiness
            // failure; checking try_wait() first discards that valid record.
            Ok(0) => return Err(if graphics.is_some() { mismatch }
                else { "supervisor readiness output closed" }.into()),
            Ok(count) => received.extend_from_slice(&bytes[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(error) => return Err(error.into()),
        }
        require(
            expected.starts_with(&received),
            mismatch,
        )?;
    }
    require(received == expected, mismatch)
}

fn retire_unready_supervisor(child: &mut Child, owner: &Path) -> Result<()> {
    if child.try_wait()?.is_none() {
        child.kill()?;
    }
    child.wait()?;
    if owner.try_exists()? {
        fs::remove_file(owner)?;
    }
    let directory = owner.parent().ok_or("session owner parent absent")?;
    if directory.try_exists()? {
        fs::remove_dir(directory)?;
    }
    Ok(())
}
struct SupervisorDelivery<'a> {
    manager: &'a Manager,
    class_id: &'a str,
    report: &'a Path,
    session: &'a str,
    transport: Option<&'a Path>,
}
fn finish_supervised_delivery(context:SupervisorDelivery<'_>,delivery:Result<()>,
    child:&mut Child,admission:&mut PendingAdmission)->Result<()> {
    // Delivery failure does not abandon the exact process generation that was
    // already retained before exposure. Let its bounded socket-loss path
    // finish, publish cleanup, and retire this lease before returning the
    // delivery error to the service worker.
    let retirement=(||->Result<()>{
        let status=child.wait()?;
        let mut disposition=String::new();
        if let Some(stdout)=child.stdout.take(){stdout.take(128).read_to_string(&mut disposition)?;}
        // A sanitized terminal summary is user-facing incident evidence, not
        // physical-cleanup authority. Preserve its error for the caller, but
        // never leave an already retired exact owner exposed merely because
        // that additional projection could not be written.
        let terminal=capacity::retain_terminal_summary(context.manager,context.session,
            context.class_id,context.report);
        let (_registry, _) = context.manager.lock_bounded(operator_model::OperatorLock::Registry,
            operator_model::LockPurpose::ServiceRecovery, None, Duration::from_secs(2))?;
        admission.complete(context.session,status.success(),&disposition,context.transport)?;
        terminal
    })();
    delivery?;
    retirement
}
fn capacity_reply(m: &Manager) -> Result<serde_json::Value> {
    let mut peer = UnixStream::connect(m.root.join("runtime/owner.sock"))?;
    peer.set_read_timeout(Some(Duration::from_secs(5)))?;
    peer.set_write_timeout(Some(Duration::from_secs(1)))?;
    peer.write_all(b"LVC1\n")?;
    let mut length = [0; 4];
    peer.read_exact(&mut length)?;
    let size = u32::from_le_bytes(length) as usize;
    require(size <= 65536, "capacity_readback_extent")?;
    let mut bytes = vec![0; size];
    peer.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}
fn capacity_read(m: &Manager) -> Result<()> {
    let value = capacity_reply(m)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    require(value["ok"] == true, "capacity_readback_unavailable")
}
fn serve(m: Manager) -> Result<()> {
    let _lock = m.lock("service.lock")?;
    m.reconcile()?;
    let s = software(&m)?;
    transport_storage::initialize()?;
    let runtime = m.root.join("runtime");
    private_dir(&runtime)?;
    private_dir(&runtime.join("results"))?;
    let address = runtime.join("owner.sock");
    if fs::symlink_metadata(&address).is_ok() {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        let md = fs::symlink_metadata(&address)?;
        require(
            md.file_type().is_socket() && md.uid() == unsafe { libc::getuid() },
            "endpoint ownership differs",
        )?;
        fs::remove_file(&address)?;
    }
    let listener = UnixListener::bind(address)?;
    let manager = Arc::new(m);
    let launch_verification = Arc::new(LaunchVerification::default());
    // Warm the owned runtime outside the interactive request and registry
    // locks. The listener still serves setup/recovery while bytes are read.
    // A concurrent native request shares this bounded preparation; it does
    // not start an independent multi-gigabyte verification race.
    let warm_manager = manager.clone();
    let warm_verification = launch_verification.clone();
    let _runtime_preparation = std::thread::Builder::new().spawn(move || {
        let result = warm_verification.prepare(
            Instant::now()+Duration::from_secs(KEEPER_OWNER_STARTUP_SECONDS),
            || runtime_delivery::installed(&warm_manager).map(|_| ()));
        if let Err(error)=result {
            eprintln!("owned runtime preparation did not authorize execution; each launch remains subject to its own exact admission: {error}");
        }
    });
    if let Err(error) = &_runtime_preparation {
        eprintln!("owned runtime preparation worker unavailable: {error}");
    }
    // A service restart cannot turn missing cleanup into a fresh admission.
    // Clean reports retire their leases; uncertain ones remain inspectable.
    let blocked = Arc::new(AtomicBool::new(reconcile_leases(&manager)?));
    let keepers = Arc::new(Mutex::new(Vec::new()));
    let limits = capacity::service_limits()?;
    let workers = Arc::new(AtomicUsize::new(0));
    let mut threads: Vec<std::thread::JoinHandle<()>> = Vec::new();
    for peer in listener.incoming() {
        let mut peer = peer?;
        threads.retain(|t| !t.is_finished());
        if threads.len() >= limits.service_workers {
            // Classification itself is unavailable. This bounded zero-token
            // refusal cannot convey a session or acknowledge a stale request.
            refuse_unclassified_worker(&manager, Some(&mut peer), "worker_ceiling");
            continue;
        }
        // Keep a control-plane descriptor for refusal if the OS cannot create
        // the worker. Failure to duplicate it still grants no execution and
        // must not stop the service's already-owned sessions.
        let mut refusal_peer = peer.try_clone().ok();
        let m = manager.clone();
        let s = s.clone();
        let blocked = blocked.clone();
        let keepers = keepers.clone();
        let launch_verification = launch_verification.clone();
        let limits = limits.clone();
        let workers = workers.clone();
        workers.fetch_add(1, Ordering::AcqRel);
        let worker_count = WorkerCount(workers.clone());
        let launched = std::thread::Builder::new().spawn(move || {
            let _worker_count=worker_count;
            let outcome = (|| -> Result<()> {
                peer.set_read_timeout(Some(Duration::from_secs(5)))?;
                let mut greeting = [0; 117];
                peer.read_exact(&mut greeting[..5])?;
                if &greeting[..5]==b"LVE2\n" {
                    // Maintenance admitted no active DSP. Restore control
                    // service availability only; native admission owns exact
                    // environment initialization and processing readiness.
                    peer.read_exact(&mut greeting[5..37])?;
                    let operation=std::str::from_utf8(&greeting[5..37])?;
                    require(valid_hex(operation,32),"operator_resume_request_identity")?;
                    let _admission=capacity::reserve_maintenance_until(&m,&limits,
                        || blocked.load(Ordering::Acquire),
                        Instant::now()+Duration::from_secs(KEEPER_OWNER_STARTUP_SECONDS))?;
                    m.require_inactive(None)?;
                    operator_cli::authorize_service_resume(&m,operation,&s)?;
                    peer.set_write_timeout(Some(Duration::from_secs(1)))?;
                    peer.write_all(b"LVE2 control ready\n")?;
                    return Ok(());
                }
                if &greeting[..5]==b"LVE1\n" {
                    // MF1 resumes keeper ownership after exclusive vendor work.
                    // Selection comes only from current registered environments.
                    let deadline=Instant::now()+Duration::from_secs(KEEPER_OWNER_STARTUP_SECONDS);
                    let registrations={
                        let _admission=capacity::reserve_maintenance_until(&m,&limits,
                            || blocked.load(Ordering::Acquire),deadline)?;
                        m.require_inactive(None)?;
                        let mut environments=std::collections::BTreeSet::new();
                        m.registry()?.classes.into_values().map(|entry|entry.registration)
                            .filter(|r|environments.insert(r.environment.id.clone()))
                            .collect::<Vec<_>>()
                    };
                    // Restoration launches the same exact environments as native
                    // admission. Share this process's byte preparation with its
                    // startup warm-up; never hash the entire runtime independently
                    // for every registration or keeper-stage retry. Preparation
                    // owns no registry reservation, and its snapshot reopens and
                    // rechecks file identity throughout the subsequent launch.
                    let (bindings,verified)=prepare_recovery_bindings(&m,&s,
                        registrations,&launch_verification,deadline)?;
                    return verified.run(|| {
                        let mut ready_owners=std::collections::BTreeMap::new();
                        restore_recovery_bindings(&bindings, deadline,
                                || capacity::reserve_maintenance_until(&m,&limits,
                                    || blocked.load(Ordering::Acquire),deadline),
                                |registration, execution| {
                                    stage_recovery_keeper(&m,execution,registration,&keepers,&mut ready_owners)
                                }, |registration, execution| {
                                    // Commit fresh observations after upstream
                                    // preparation, without holding registry
                                    // ownership or reusing a persisted cache.
                                    launch_verification.prepare(deadline, ||
                                        verify_recovery_binding(&m,&s,registration,execution))?;
                                    Ok(())
                                },
                                std::thread::sleep)?;
                        peer.set_write_timeout(Some(Duration::from_secs(1)))?;
                        peer.write_all(b"LVE1 ready\n")?;
                        Ok(())
                    });
                }
                if &greeting[..5]==b"LVC1\n" {
                    let value=match capacity::status(&m,limits.clone(),workers.load(Ordering::Acquire),blocked.load(Ordering::Acquire)) {
                        Ok(status)=>serde_json::json!({"ok":true,"capacity":status}),
                        Err(e)=>serde_json::json!({"ok":false,"refusal":readback::refusal(e.as_ref())}),
                    };
                    let bytes=serde_json::to_vec(&value)?;
                    require(bytes.len()<=65536,"capacity_readback_extent")?;
                    peer.set_write_timeout(Some(Duration::from_secs(1)))?;
                    peer.write_all(&(bytes.len() as u32).to_le_bytes())?;
                    peer.write_all(&bytes)?;return Ok(());
                }
                if &greeting[..5]==b"LVP1\n" {
                    peer.set_write_timeout(Some(Duration::from_millis(200)))?;
                    peer.write_all(&[u8::from(blocked.load(Ordering::Acquire))])?;
                    return Ok(());
                }
                if let Some(purpose) = inspection_purpose(&greeting[..5]) {
                    let mut size=[0;4];peer.read_exact(&mut size)?;
                    let size=u32::from_le_bytes(size) as usize;require(size<=65536,"inspection_request_bound")?;
                    let mut bytes=vec![0;size];peer.read_exact(&mut bytes)?;
                    let request:InspectionRequest=serde_json::from_slice(&bytes)?;
                    let select=|| match purpose {
                        Some(purpose) => qualification_binding(&m, request.clone(), purpose),
                        None => inspection_binding(&m, request.clone()),
                    };
                    let (r,execution)={
                        let _admission=capacity::reserve_maintenance(&m,&limits,
                            || blocked.load(Ordering::Acquire))?;
                        m.require_inactive(None)?;
                        let r=select()?;
                        let execution=package_authority::paired_host_components(&m,&s,&r.host,&r.host_source_sha256)?;
                        (r,execution)
                    };
                    let _admission=wait_for_keeper(
                        Instant::now()+Duration::from_secs(KEEPER_OWNER_STARTUP_SECONDS),
                        || capacity::reserve_maintenance(&m,&limits,
                            || blocked.load(Ordering::Acquire)),
                        || {
                            m.require_inactive(None)?;
                            require(select()?==r,"inspection_binding_changed")?;
                            stage_keeper(&m,&execution,&r,&keepers,None)
                        },std::thread::sleep)?;
                    let (mut job,path)=spec(&m,r,true,false,false)?;
                    let mut pending=PendingAdmission::new(job.lease.clone(),blocked.clone());
                    job.shared_inspection=true;atomic_json(&path,&job)?;
                    let mut child=spawn(&m,&execution,&path,None)?;
                    if let Err(readiness)=supervisor_ready(&mut child,&job,
                        Duration::from_secs(4)) {
                        retire_unready_supervisor(&mut child,&path)?;
                        return Err(readiness);
                    }
                    pending.expose();
                    drop(_admission);
                    let status=child.wait()?;
                    let mut disposition=String::new();if let Some(stdout)=child.stdout.take(){stdout.take(128).read_to_string(&mut disposition)?;}
                    let (_registry, _) = m.lock_bounded(operator_model::OperatorLock::Registry,
                        operator_model::LockPurpose::ServiceRecovery, None, Duration::from_secs(2))?;
                    pending.complete(&job.session,status.success(),&disposition,None)?;
                    let reply=serde_json::to_vec(&job.report)?;
                    require(reply.len()<=4096,"inspection_reply_bound")?;
                    peer.write_all(&(reply.len() as u32).to_le_bytes())?;peer.write_all(&reply)?;
                    return Ok(());
                }
                if &greeting[..5]==b"LVA1\n" {
                    let mut size=[0;4];peer.read_exact(&mut size)?;
                    let size=u32::from_le_bytes(size) as usize;require(size<=65536,"vendor access request bound")?;
                    let mut bytes=vec![0;size];peer.read_exact(&mut bytes)?;
                    let request:InspectionRequest=serde_json::from_slice(&bytes)?;
                    let (r,execution)={
                        let _admission=capacity::reserve_maintenance(&m,&limits,
                            || blocked.load(Ordering::Acquire))?;
                        m.require_inactive(None)?;
                        let r=inspection_binding(&m,request.clone())?;
                        let execution=package_authority::paired_host_components(&m,&s,&r.host,&r.host_source_sha256)?;
                        (r,execution)
                    };
                    let _admission=wait_for_keeper(
                        Instant::now()+Duration::from_secs(KEEPER_OWNER_STARTUP_SECONDS),
                        || capacity::reserve_maintenance(&m,&limits,
                            || blocked.load(Ordering::Acquire)),
                        || {
                            m.require_inactive(None)?;
                            require(inspection_binding(&m,request.clone())?==r,"vendor_access_binding_changed")?;
                            stage_keeper(&m,&execution,&r,&keepers,None)
                        },std::thread::sleep)?;
                    let (mut job,path)=spec(&m,r,false,false,false)?;
                    job.vendor_access=true;atomic_json(&path,&job)?;
                    let mut pending=PendingAdmission::new(job.lease.clone(),blocked.clone());
                    let mut child=spawn(&m,&execution,&path,None)?;
                    if let Err(readiness)=supervisor_ready(&mut child,&job,
                        Duration::from_secs(4)) {
                        retire_unready_supervisor(&mut child,&path)?;
                        return Err(readiness);
                    }
                    pending.expose();
                    drop(_admission);
                    let _=peer.write_all(format!("Vendor access {}: editor only; no DAW audio or project recall. Close its window to finish.\n",job.session).as_bytes());
                    let status=child.wait()?;
                    let mut disposition=String::new();if let Some(stdout)=child.stdout.take(){stdout.take(128).read_to_string(&mut disposition)?;}
                    let (_registry, _) = m.lock_bounded(operator_model::OperatorLock::Registry,
                        operator_model::LockPurpose::ServiceRecovery, None, Duration::from_secs(2))?;
                    pending.complete(&job.session,status.success(),&disposition,None)?;
                    peer.write_all(b"Vendor access retired.\n")?;return Ok(());
                }
                let version5 = &greeting[..5] == ap1_native_client::admission::GREETING_V5;
                let extent = if version5 { 117 } else { 53 };
                peer.read_exact(&mut greeting[5..extent])?;
                let version4 = &greeting[..5] == ap1_native_client::admission::GREETING_V4;
                let version3 = version5 || version4 || &greeting[..5] == ap1_native_client::admission::GREETING;
                let version2 = version3 || &greeting[..5] == b"LVB2\n";
                require(version2 || &greeting[..5] == b"LVB1\n", "registration protocol mismatch")?;
                let mut request=[0u8;16];
                if version3 {peer.read_exact(&mut request)?;require(request!=[0;16],"admission request identity")?;}
                peer.set_write_timeout(Some(Duration::from_secs(5)))?;
                let class=hex(&greeting[5..21]).to_uppercase();
                let loaded=version5.then(|| ap1_native_client::admission::ExecutionIdentity {
                    engine:greeting[53..85].try_into().unwrap(),
                    descriptor:greeting[85..117].try_into().unwrap(),
                });
                let mut startup=NativeStartup::new(&m,request,class.clone());
                // Leave room for the four-second supervisor handshake within
                // the existing 65-second native admission budget.
                let keeper_deadline=startup.started+Duration::from_secs(KEEPER_OWNER_STARTUP_SECONDS);
                let prepared=(|| -> Result<_> {
                    let peer_process=match transport_storage::peer_process(&peer) {
                        Ok(process)=>process,
                        Err(_) if version5=>return Err(capacity::Refusal::StaleNativeCaller.into()),
                        Err(error)=>return Err(error),
                    };
                    let ((registration, execution), verified)=launch_verification.prepare(keeper_deadline, || {
                        let registration = m.resolve(&greeting[5..53])?;
                        let prepare_execution=|| -> Result<_> {
                            experimental_runner::verify_selected_bg1_history(&m, &registration.environment)?;
                            m.verify_served_host(&registration, &s.host, &s.source_sha256, &profiles::installed_profiles()?)?;
                            package_authority::paired_components(&m,&s,&registration)
                        };
                        let execution=if let Some(loaded)=&loaded {
                            with_loaded_native_caller(&peer,&peer_process,&registration,loaded,
                                prepare_execution)??
                        } else {prepare_execution()?};
                        Ok((registration, execution))
                    })?;
                    // Shared byte preparation is complete. All keeper and DSP
                    // work proceeds independently, rechecking those observations
                    // in this exact admission before exposing the binding.
                    verified.run(|| -> Result<_> {
                    let full_registration = registration.clone();
                    let r: HostBinding = registration.into();
                    let performance = m.performance(&r.metadata.class_id)?;
                    require(version2 || performance.added_frames == 512,
                        "selected delay requires a version-2 native binding")?;
                    require(version5 || version4 || performance.delivery_mode == DeliveryMode::Buffered,
                        "selected delivery requires a version-4 native binding")?;
                    require(performance.delivery_mode != DeliveryMode::SameCallback
                        || preparation::build::supports_audio_completion(&m, &full_registration)?,
                        "selected pair lacks the audio completion contract")?;
                    // Validate the DAW's view of the fixed memory root before
                    // creating or exposing a session. A Flatpak's /dev/shm is
                    // not assumed to be the host's shared memory mount.
                    transport_storage::visible_to_peer(&peer,&peer_process,&transport_storage::root())?;
                    let graphical_session=transport_storage::graphical_session(&peer,&peer_process)?;
                    startup.phase("binding_verified");
                    let mut observed=None;
                    let reservation=wait_for_keeper(keeper_deadline,
                        || capacity::reserve(&m,&limits,Some(&class),blocked.load(Ordering::Acquire)),
                        || {
                            let registry=m.registry()?;
                            require(registry.classes.get(&class).is_some_and(|entry|
                                entry.publication==Publication::Published
                                    && entry.registration==full_registration),
                                "publication_changed_during_startup")?;
                            require(m.performance(&class)?==performance,"performance_changed_during_startup")?;
                            let mut keeper=if observed==Some(KeeperAvailability::Starting) {
                                let mut active=keepers.lock().map_err(|_|"environment ownership lock poisoned")?;
                                observe_keeper(&r.environment.id,&mut active)?.unwrap_or(KeeperAvailability::Retiring)
                            } else {
                                stage_keeper(&m,&execution,&r,&keepers,Some(&graphical_session))?
                            };
                            if keeper==KeeperAvailability::Ready {
                                keeper=stage_keeper(&m,&execution,&r,&keepers,Some(&graphical_session))?;
                            }
                            if observed!=Some(keeper) {
                                if keeper!=KeeperAvailability::Ready {
                                    let _=retain_keeper_incident(&m,&keepers,keeper,request,&class,&r.environment.id);
                                    startup.phase(match keeper {
                                        KeeperAvailability::Starting=>"keeper_starting",
                                        KeeperAvailability::Retiring=>"keeper_retiring",
                                        _=>"keeper_failed",
                                    });
                                }
                                observed=Some(keeper);
                            }
                            Ok(keeper)
                        },std::thread::sleep)?;
                    startup.phase("keeper_ready");
                    let ready_keeper_session=keeper_session(&keepers,&r.environment.id)?
                        .ok_or("ready_keeper_ownership_missing")?;
                    drop(reservation);
                    // Runtime-copy preparation may invalidate file observations.
                    // Reverify exact bytes only after readiness, without the
                    // registry reservation, and publish the fresh process-owned
                    // observations so the next load need not redo this work.
                    let (_, ready_verified)=launch_verification.prepare(keeper_deadline, || {
                        require(m.resolve(&greeting[5..53])?==full_registration,
                            "publication_changed_during_startup")?;
                        let verify_components=|| -> Result<()> {require(
                            package_authority::paired_components(&m,&s,&full_registration)?==execution,
                            "publication_components_changed_during_startup")};
                        if let Some(loaded)=&loaded {
                            with_loaded_native_caller(&peer,&peer_process,&full_registration,loaded,
                                verify_components)??
                        } else {verify_components()?;}
                        Ok(())
                    })?;
                    startup.phase("ready_binding_verified");
                    ready_verified.run(|| {
                    let _reservation=wait_for_keeper(keeper_deadline,
                        || capacity::reserve(&m,&limits,Some(&class),blocked.load(Ordering::Acquire)),
                        || {
                            let registry=m.registry()?;
                            require(registry.classes.get(&class).is_some_and(|entry|
                                entry.publication==Publication::Published
                                    && entry.registration==full_registration),
                                "publication_changed_during_startup")?;
                            require(m.performance(&class)?==performance,
                                "performance_changed_during_startup")?;
                            require(keeper_session(&keepers,&r.environment.id)?.as_ref()
                                ==Some(&ready_keeper_session),
                                "keeper_changed_during_verification")?;
                            stage_keeper(&m,&execution,&r,&keepers,Some(&graphical_session))
                        },std::thread::sleep)?;
                    let (mut job, path) = spec(&m, r.clone(), false, false, false)?;
                    let storage = transport_storage::PendingTransport::new(&job.session)?;
                    job.directory = storage.directory.clone();
                    job.transport = Some(storage.identity.clone());
                    job.shared_runtime = true;
                    job.graphical_session=Some(graphical_session.clone());
                    job.crash_capture = crash_capture::claim(&m,&full_registration,&job.session)
                        .unwrap_or_else(|e| { eprintln!("CA1 capture unavailable: {e}"); None });
                    atomic_json(&path, &job)?;
                    let admission = PendingAdmission::new(job.lease.clone(),blocked.clone());
                    m.retain_lease_owner(&path)?;
                    atomic_json(&job.lease, &job.report)?;
                    // Recheck after all fallible preparation in this same
                    // fresh-byte verification scope. Reopening every file
                    // still checks its full identity; leaving the scope here
                    // would hash the complete runtime again before delivery.
                    let keeper=stage_keeper(&m,&execution,&r,&keepers,Some(&graphical_session))?;
                    if keeper!=KeeperAvailability::Ready {
                        if let Err(incident)=retain_keeper_incident(&m,&keepers,keeper,request,
                            &r.metadata.class_id,&r.environment.id) {
                            eprintln!("admission incident unavailable: {incident}");
                        }
                        return Err(match keeper {
                            KeeperAvailability::Failed=>capacity::Refusal::BindingInvalid,
                            _=>capacity::Refusal::ServiceBusy,
                        }.into());
                    }
                    startup.phase("transport_prepared");
                    Ok((execution,r,performance,job,path,admission,storage))
                    })
                    })
                })();
                let (execution,r,performance,job,path,mut admission,mut storage)=match prepared {
                    Ok(value)=>value,
                    Err(e)=>{
                        if version3 {
                            let reason=e.downcast_ref::<capacity::Refusal>().copied()
                                .unwrap_or(capacity::Refusal::BindingInvalid);
                            startup_reply(&mut peer,&(if version5 {ap1_native_client::admission::refused_v5}
                                else if version4 {ap1_native_client::admission::refused_v4}
                                else {ap1_native_client::admission::refused})(request,reason))?;
                        }
                        return Err(e);
                    }
                };
                // Deliver the private binding inside the native greeting deadline.
                // Cold Wine startup then uses the existing bounded transport accept.
                let reply = if version3 {
                    let session=std::array::from_fn(|i|u8::from_str_radix(&job.session[i*2..i*2+2],16).unwrap());
                    (if version5 {ap1_native_client::admission::accepted_v5}
                        else if version4 {ap1_native_client::admission::accepted_v4}
                        else {ap1_native_client::admission::accepted})(request,&ap1_native_client::admission::Binding {
                        session,added_frames:performance.added_frames,delivery_mode:performance.delivery_mode as u32,
                        directory:job.directory.to_str().ok_or("session directory encoding")?.into(),
                    })?
                } else if version2 {
                    format!("{}\n{}\n{}", job.session, performance.added_frames, job.directory.display()).into_bytes()
                } else {
                    format!("{}\n{}", job.session, job.directory.display()).into_bytes()
                };
                require(reply.len() <= 1024, "session binding size")?;
                peer.set_write_timeout(Some(Duration::from_secs(5)))?;
                // Retain a concrete supervisor generation before telling the
                // DAW that an accepted binding exists. Process creation alone
                // is not ownership: the exact supervisor must first validate
                // its immutable inputs and graphical peer and install its
                // outer finalizer.
                let mut child=spawn(&m,&execution,&path,Some(peer.try_clone()?))?;
                if let Err(readiness) = supervisor_ready(
                    &mut child,
                    &job,
                    Duration::from_secs(4),
                ) {
                    let cleanup = retire_unready_supervisor(&mut child, &path);
                    if version3 {
                        let _ = startup_reply(
                            &mut peer,
                            &(if version5 {ap1_native_client::admission::refused_v5}
                                else if version4 {ap1_native_client::admission::refused_v4}
                                else {ap1_native_client::admission::refused})(
                                request,
                                capacity::Refusal::BindingInvalid,
                            ),
                        );
                    }
                    cleanup?;
                    return Err(readiness);
                }
                startup.phase("supervisor_ready");
                admission.expose();
                storage.expose();
                let delivery=startup_reply(&mut peer,&reply);
                startup.phase("binding_delivered");
                startup.finish(if delivery.is_ok() {"accepted"} else {"delivery_failed"});
                drop(peer);
                finish_supervised_delivery(SupervisorDelivery{manager:&m,
                    class_id:&r.metadata.class_id,report:&job.report,
                    session:&job.session,transport:Some(&job.directory)},delivery,
                    &mut child,&mut admission)?;
                Ok(())
            })();
            if let Err(e) = outcome {
                eprintln!("Bridge instance: {e}");
            }
        });
        match launched {
            Ok(thread) => threads.push(thread),
            Err(error) => {
                eprintln!("bridge request worker unavailable: {error}");
                // The unlaunched closure drops its WorkerCount and peer; no
                // owner or admission can be exposed from this failed spawn.
                refuse_unclassified_worker(&manager, refusal_peer.as_mut(), "worker_unavailable");
            }
        }
    }
    Ok(())
}
fn environment_record(m: &Manager, id: &str) -> Result<Environment> {
    require(
        !id.is_empty() && id.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-'),
        "environment ID syntax",
    )?;
    let e: Environment = read_json(
        &m.root
            .join("environments")
            .join(id)
            .join("environment.json"),
    )?;
    require(
        e.id == id && e.root == m.root.join("environments").join(id),
        "environment identity differs",
    )?;
    e.runner.verify()?;
    Ok(e)
}
fn environment_create(m: &Manager, runner: &Path) -> Result<()> {
    let runner: Runner = read_json(runner)?;
    runner.verify()?;
    let id = random_id()?;
    let root = m.root.join("environments").join(&id);
    private_dir(&root)?;
    for d in [
        "compatdata",
        "runtime-var",
        "host-cache",
        "host-config",
        "host-data",
        "host-tmp",
        "client",
    ] {
        private_dir(&root.join(d))?;
    }
    let e = Environment {
        id,
        root: root.clone(),
        runner,
        revision: 1,
    };
    atomic_json(&root.join("environment.json"), &e)?;
    println!("{}", e.id);
    Ok(())
}
fn environment_import(m: &Manager, path: &Path) -> Result<()> {
    let e: Environment = read_json(path)?;
    require(
        !e.id.is_empty() && e.id.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-'),
        "environment ID syntax",
    )?;
    require(
        e.root == m.root.join("environments").join(&e.id) && e.revision > 0,
        "environment binding differs",
    )?;
    private_dir(&e.root)?;
    e.runner.verify()?;
    let marker = e.root.join("environment.json");
    if let Ok(old) = read_json::<Environment>(&marker) {
        require(old == e, "existing environment revision differs")?;
    } else if marker.try_exists()? {
        // Explicit import preserves earlier setup notes; vendor state is untouched.
        let backup = e
            .root
            .join(format!("environment-before-import-{}.json", random_id()?));
        fs::copy(&marker, &backup)?;
        fs::File::open(backup)?.sync_all()?;
    }
    atomic_json(&marker, &e)?;
    Ok(())
}
fn inspection_binding(m: &Manager, request: InspectionRequest) -> Result<HostBinding> {
    inspection_binding_for(m, request, true)
}
fn inspection_binding_for(m: &Manager, request: InspectionRequest, ordinary: bool) -> Result<HostBinding> {
    require(
        valid_hex(&request.class_id, 32),
        "inspection class ID syntax",
    )?;
    let environment = environment_record(m, &request.environment_id)?;
    require(
        request
            .module
            .path
            .starts_with(environment.root.join("compatdata/pfx/drive_c"))
            && request.module.path.canonicalize()? == request.module.path,
        "inspection module outside exact environment",
    )?;
    request.module.verify()?;
    let sw = software(m)?;
    let mut host = sw.host.clone();
    let mut source = sw.source_sha256.clone();
    if ordinary {
        if let Some(p) = profiles::installed_profiles()?.iter().find(|p| p.class.class_id == request.class_id) {
            require(p.module_sha256 == request.module.sha256 && p.capabilities.compatibility() == request.compatibility,
                "installed_host_mismatch")?;
            p.verify_environment(&environment, &p.requirements.environment_family)?;
            let selected = catalogue::current_host(m, &sw.host, &sw.source_sha256, p)?;
            host = selected.host;
            source = selected.source_manifest.sha256;
        }
    }
    let r = HostBinding {
        metadata: ClassSelection {
            class_id: request.class_id,
        },
        environment,
        module: request.module,
        host,
        host_source_sha256: source,
        compatibility: request.compatibility,
    };
    Ok(r)
}
// Engineering inspection has a distinct admission selector. Its host can only
// come from the sealed installed candidate roster, never from request data.
fn qualification_binding(
    m: &Manager,
    request: InspectionRequest,
    purpose: publication::Qualification,
) -> Result<HostBinding> {
    if purpose == publication::Qualification::Frg1Ubuntu {
        let requested = inspection_binding_for(m, request, false)?;
        let exact = frg1::binding(m)?;
        require(requested.metadata.class_id == exact.metadata.class_id
            && requested.environment == exact.environment && requested.module == exact.module
            && requested.compatibility == exact.compatibility,
            "qualification_exact_candidate_required")?;
        return Ok(exact.into());
    }
    if purpose == publication::Qualification::Ap18Pigments {
        let requested = inspection_binding_for(m, request, false)?;
        let r = pigments::binding(m)?;
        require(requested.metadata.class_id == r.metadata.class_id
            && requested.environment == r.environment && requested.module == r.module
            && requested.compatibility == r.compatibility, "qualification_exact_candidate_required")?;
        return Ok(r.into());
    }
    let candidates = qualification::installed_for(m, purpose)?;
    let r = inspection_binding_for(m, request, false)?;
    let matching: Vec<_> = candidates
        .iter()
        .filter(|c| c.profile.class.class_id == r.metadata.class_id)
        .collect();
    require(
        matching.len() == 1,
        "qualification_exact_candidate_required",
    )?;
    let c = matching[0];
    let db = m.registry()?;
    let prior = db
        .classes
        .get(&r.metadata.class_id)
        .ok_or("qualification_verified_parent_required")?;
    let mut registration = prior.registration.clone();
    registration.module = r.module;
    registration.environment = r.environment;
    registration.compatibility = r.compatibility;
    registration.host = c.host.clone();
    registration.native = c.native.artifact.clone();
    registration.host_source_sha256 = c.source_manifest.sha256.clone();
    m.check_qualification_parent_for(&c.profile, &registration, purpose)?;
    Ok(registration.into())
}
fn inspect(m: &Manager, path: &Path) -> Result<()> {
    let r = inspection_binding(m, read_json(path)?)?;
    let sw = software(m)?;
    let (job, path) = spec(m, r, true, false, false)?;
    let status = spawn(m,&sw, &path, None)?.wait()?;
    println!("{}", job.report.display());
    finish_inspection(m, &job, status)
}
fn finish_inspection(m: &Manager, job: &SessionSpec, status: ExitStatus) -> Result<()> {
    // A refused prelaunch still has an owned reservation. Retire it through
    // the existing positive receipt rule before reporting the inspection error.
    {
        let _registry = m.lock("registry.lock")?;
        let _unresolved = reconcile_leases(m)?;
        require(!job.lease.try_exists()?, "inspection cleanup unconfirmed")?;
    }
    require(status.success(), "inspection cleanup failed")?;
    let result: serde_json::Value = read_json(&job.report)?;
    require(
        result["error"].is_null(),
        "inspection failed; see retained result",
    )
}
fn install(m: &Manager, id: &str, path: &Path, sha: &str) -> Result<()> {
    let e = environment_record(m, id)?;
    let artifact = Artifact {
        path: path.canonicalize()?,
        sha256: sha.into(),
    };
    artifact.verify()?;
    let sw = software(m)?;
    let jobs = e.root.join("installations");
    private_dir(&jobs)?;
    let token = random_id()?;
    let job = jobs.join(format!("{token}.json"));
    let report = jobs.join(format!("{token}-result.json"));
    atomic_json(
        &job,
        &serde_json::json!({"environment":e,"installer":artifact,"report":report}),
    )?;
    let status = Command::new("systemd-run")
        .args([
            "--user",
            "--wait",
            "--pipe",
            "--property=KillMode=control-group",
        ])
        .arg(format!("--unit=linux-vst-bridge-setup-{token}"))
        .arg("/usr/bin/python3")
        .arg(&sw.supervisor.path)
        .arg("--install")
        .arg(&job)
        .status()?;
    println!("{}", report.display());
    require(status.success(), "installer did not complete successfully")
}
fn main() -> Result<()> {
    unsafe {
        libc::umask(0o077);
    }
    // Browser return URIs are ephemeral secret-bearing inputs, never dump data.
    if std::env::args_os().nth(1).as_deref()==Some(std::ffi::OsStr::new("native-access-callback")) {
        unsafe { let limit=libc::rlimit{rlim_cur:0,rlim_max:0};require(libc::setrlimit(libc::RLIMIT_CORE,&limit)==0,"callback_privacy")?; }
        #[cfg(target_os="linux")]
        unsafe { require(libc::prctl(libc::PR_SET_DUMPABLE,0,0,0,0)==0,"callback_privacy")?; }
    }
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["owned-audio-scheduling"] {
        return audio_scheduling::run();
    }
    let m = Manager::installed()?;
    match args.first().map(String::as_str){
  Some("setup") if args.len()==2=>setup(&m,Some(Path::new(&args[1]))),
  Some("package-adopt") if args.len()==1=>package_authority::adopt(&m),
  Some("package-rollback") if args.len()==1=>package_authority::rollback(&m),
  Some("package-recover") if args.len()==1=>package_authority::recover(&m).map(|_|()),
  Some("package-activation-status") if args.len()==1=>package_authority::activation_status(&m),
  Some("package-activate") if args.len()==1=>package_authority::activate(&m),
  Some("package-bootstrap-status") if args.len()==1=>package_authority::bootstrap_status(&m),
  Some("package-stop-for-repair") if args.len()==1=>package_authority::stop_for_repair(&m),
  Some("package-user-inspect") if args.len()==2=>{
    let mut bundle=portable_package::Bundle::open(Path::new(&args[1]),portable_package::Trust::compiled()?)?;
    bundle.verify_payload()?;
    println!("{}",serde_json::to_string(&serde_json::json!({"schema":1,"format":"native_user_installer",
      "key_class":bundle.key_class(),"release":bundle.release(),"staged":false,"selected":false}))?);
    Ok(())
  },
  #[cfg(feature = "pb0-r3-audit")]
  Some("pb0-r3-audit") if args.len()==1=>pb0_r3_audit::run(&m),
  Some("accept-editor") if args.len()==1=>managed_cli::run_acceptance(&m),
  Some("accept-capacity") if args.len()==1=>managed_cli::run_capacity_acceptance(&m),
  Some("accept-pigments") if args.len()==1=>managed_cli::run_pigments_acceptance(&m),
  Some("accept-pigments-ui") if args.len()==1=>managed_cli::run_uio2_acceptance(&m),
  Some("accept-ui") if args.len()==1=>managed_cli::run_ui_acceptance(&m),
  Some("managed")=>managed_cli::run(&m,&args[1..]),
  Some("capture")=>crash_capture::run(&m,&args[1..]),
  Some("operator")=>operator_cli::run(&m,&args[1..]),
  Some("native-access-callback") if args.len()==2=>native_access_callback::deliver(&m,&args[1]).map_err(|_|"Native Access login return could not be delivered. Open Native Access through the manager and start a fresh sign-in.".into()),
  Some("import-installer") if args.len()==1=>{let source=fs::File::from(std::io::stdin().as_fd().try_clone_to_owned()?);println!("{}",serde_json::to_string(&installer_import::import(&m,source)?)?);Ok(())},
  Some("import-installer") if args.len()==2=>{let source=fs::File::from(std::io::stdin().as_fd().try_clone_to_owned()?);println!("{}",serde_json::to_string(&installer_import::import_named(&m,source,&args[1])?)?);Ok(())},
  Some("workspace")=>daw_workspace::run(&m,&args[1..]),
  Some("vendor-app")=>vendor_cli::run(&m,&args[1..]),
  Some("vendor-product")=>vendor_product_cli::run(&m,&args[1..]),
  Some("qualify-instrument")=>managed_cli::run_installer_qualification(&m,&args[1..]),
  Some("qualify-editor")=>managed_cli::run_qualification(&m,&args[1..]),
  Some("qualify-ui")=>managed_cli::run_ui_qualification(&m,&args[1..]),
  Some("qualify-failure")=>managed_cli::run_failure_qualification(&m,&args[1..]),
  Some("qualify-pigments")=>managed_cli::run_pigments_qualification(&m,&args[1..]),
  Some("qualify-capacity")=>managed_cli::run_capacity_qualification(&m,&args[1..]),
  Some("qualify-frg1")=>managed_cli::run_frg1_qualification(&m,&args[1..]),
  Some("environment-create") if args.len()==2=>environment_create(&m,Path::new(&args[1])),
  Some("environment-import") if args.len()==2=>environment_import(&m,Path::new(&args[1])),
  Some("native-access-runner") if args.len()==2=>native_access_runner::update(&m,Path::new(&args[1])),
  Some("experimental-runner") if args.len()==2=>experimental_runner::update(&m,Path::new(&args[1])),
  Some("install") if args.len()==4=>install(&m,&args[1],Path::new(&args[2]),&args[3]),
  Some("register") if args.len()==2=>m.register(read_json(Path::new(&args[1]))?),
  Some("unpublish") if args.len()==2=>m.unpublish(&args[1]),
  Some("set-delay") if args.len()==3=>m.select_delay(&args[1],args[2].parse()?),
  Some("serve") if args.len()==1=>serve(m),
  Some("status") if args.len()==1=>status(&m),
  Some("capacity") if args.len()==1=>capacity_read(&m),
  Some("reconcile") if args.len()==1=>m.reconcile(),
  Some("inspect") if args.len()==2=>inspect(&m,Path::new(&args[1])),
  Some("graphics-assess") if args.len()==2=>graphics_cli::run(&m,Path::new(&args[1])),
  Some("vendor-editor") if args.len()==2=>vendor_editor(&m,Path::new(&args[1])),
  _=>Err("Usage: linux-vst-bridge setup PACKAGE | environment-create RUNNER.json | environment-import ENVIRONMENT.json | install ENV_ID INSTALLER SHA256 | inspect INSPECTION.json | graphics-assess INSPECTION.json | vendor-editor INSPECTION.json | register REGISTRATION.json | status | set-delay CLASS_ID FRAMES | reconcile | unpublish CLASS_ID | serve".into())
 }
}
fn vendor_editor(m: &Manager, path: &Path) -> Result<()> {
    let request: InspectionRequest = read_json(path)?;
    let bytes = serde_json::to_vec(&request)?;
    require(bytes.len() <= 65536, "vendor access request bound")?;
    let mut peer = UnixStream::connect(m.root.join("runtime/owner.sock"))?;
    peer.set_write_timeout(Some(Duration::from_secs(5)))?;
    peer.set_read_timeout(Some(Duration::from_secs(1900)))?;
    peer.write_all(b"LVA1\n")?;
    peer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    peer.write_all(&bytes)?;
    let mut reply = peer.take(1024);
    std::io::copy(&mut reply, &mut std::io::stdout())?;
    Ok(())
}
fn status(m: &Manager) -> Result<()> {
    let db = m.registry()?;
    let rows: Vec<_> = db.classes.values().map(|e| {
        let refusal = e.registration.verify(&m.root).err().map(|e| e.to_string());
        let performance = m.performance(&e.registration.key());
        let performance_refusal = performance.as_ref().err().map(|e| e.to_string());
        let performance = performance.ok();
        serde_json::json!({"class":e.registration.metadata,"publication":e.publication,
            "environment":e.registration.environment.id,"revision":e.registration.environment.revision,
            "compatibility":e.registration.compatibility,"artifacts_valid":refusal.is_none(),
            "refusal":refusal,"performance":performance,
            "performance_refusal":performance_refusal,
            "added_frames":performance.as_ref().map(|p|p.effective_frames()),
            "buffered_frames":performance.as_ref().map(|p|p.added_frames),
            "delivery_mode":performance.as_ref().map(|p|p.delivery_mode)})
    }).collect();
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(target_os="linux")]
    #[test]
    fn loaded_native_gate_uses_one_peer_generation_and_never_runs_rejected_continuations() {
        use std::os::unix::fs::MetadataExt;

        let f = test_fixture::Fixture::new();
        let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
        let descriptor_path = f.outer.join("loaded-descriptor.json");
        fs::write(&descriptor_path, b"exact cached descriptor").unwrap();
        let mut selected = f.r.clone();
        selected.native = Artifact {
            sha256: digest(&executable).unwrap(),
            path: executable.clone(),
        };
        selected.descriptor = Some(Artifact {
            sha256: digest(&descriptor_path).unwrap(),
            path: descriptor_path,
        });
        let parse = |value: &str| -> [u8; 32] {
            std::array::from_fn(|index| {
                u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap()
            })
        };
        let loaded = ap1_native_client::admission::ExecutionIdentity {
            engine: parse(&selected.native.sha256),
            descriptor: parse(&selected.descriptor.as_ref().unwrap().sha256),
        };
        let (_client, peer) = UnixStream::pair().unwrap();
        let process = transport_storage::peer_process(&peer).unwrap();
        let admitted = std::cell::Cell::new(false);
        assert_eq!(
            with_loaded_native_caller(&peer, &process, &selected, &loaded, || {
                admitted.set(true);
                17
            })
            .unwrap(),
            17
        );
        assert!(admitted.get());

        let refuse = |registration: &Registration,
                      execution: &ap1_native_client::admission::ExecutionIdentity,
                      owner: &transport_storage::PeerProcess| {
            let continued = std::cell::Cell::new(false);
            let error = with_loaded_native_caller(&peer, owner, registration, execution, || {
                continued.set(true);
            })
            .unwrap_err();
            assert_eq!(
                error.downcast_ref::<capacity::Refusal>(),
                Some(&capacity::Refusal::StaleNativeCaller)
            );
            assert!(!continued.get(), "refused admission reached resource continuation");
        };
        let mut stale_engine = loaded;
        stale_engine.engine[0] ^= 1;
        refuse(&selected, &stale_engine, &process);
        let mut stale_descriptor = loaded;
        stale_descriptor.descriptor[0] ^= 1;
        refuse(&selected, &stale_descriptor, &process);

        let unmapped = f.outer.join("same-bytes-unmapped-native");
        fs::copy(&executable, &unmapped).unwrap();
        assert_ne!(fs::metadata(&executable).unwrap().ino(), fs::metadata(&unmapped).unwrap().ino());
        let mut unmapped_selection = selected.clone();
        unmapped_selection.native.path = unmapped;
        assert_eq!(digest(&unmapped_selection.native.path).unwrap(), selected.native.sha256);
        refuse(&unmapped_selection, &loaded, &process);

        let mut replaced_generation = process;
        replaced_generation.start_ticks = replaced_generation.start_ticks.checked_add(1).unwrap();
        refuse(&selected, &loaded, &replaced_generation);
    }

    fn recovery_software(f: &test_fixture::Fixture) -> Software {
        let source_path = f.r.host.path.with_file_name("host-source-manifest.json");
        let source = Artifact { sha256: digest(&source_path).unwrap(), path: source_path };
        for artifact in [&f.r.host, &source] {
            fs::set_permissions(&artifact.path, fs::Permissions::from_mode(0o500)).unwrap();
        }
        Software {
            installer_launch: None, preparation_kit: None,
            operator_frontend: Some(f.r.host.clone()),
            manager: f.r.host.clone(), supervisor: f.r.host.clone(),
            ownership: f.r.host.clone(), host: f.r.host.clone(),
            source_sha256: source.sha256.clone(), source_manifest: source,
            native_catalogue: None,
        }
    }
    #[test]
    fn service_recovery_shares_launch_preparation_but_rejects_changed_bytes() {
        let (f, _, _, _) = test_fixture::prepared();
        let software = recovery_software(&f);
        let shared = LaunchVerification::default();
        let registration = f.m.registry().unwrap().classes[&f.r.key()].registration.clone();
        let deadline = || Instant::now() + Duration::from_secs(2);
        // The service's startup preparation and restoration use one process's
        // observations, while retaining the exact historical execution pair.
        shared.prepare(deadline(), || f.r.environment.runner.verify()).unwrap();
        let (bindings, snapshot) = prepare_recovery_bindings(&f.m, &software,
            vec![registration.clone()], &shared, deadline()).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].1.host, f.r.host);
        snapshot.run(|| verify_recovery_binding(&f.m, &software, &registration,
            &bindings[0].1)).unwrap();
        let before = test_fixture::snapshot(&f.m.root);
        fs::write(&f.r.module.path, b"changed bytes").unwrap();
        assert!(snapshot.run(|| verify_recovery_binding(&f.m, &software, &registration,
            &bindings[0].1)).is_err());
        assert!(prepare_recovery_bindings(&f.m, &software,
            vec![registration], &shared, deadline()).is_err());
        fs::write(&f.r.module.path, b"vendor module").unwrap();
        assert_eq!(test_fixture::snapshot(&f.m.root), before,
            "preparation cannot publish, launch, or write environment state");
    }
    #[test]
    fn service_recovery_rechecks_registration_after_shared_preparation() {
        let (f, _, _, _) = test_fixture::prepared();
        let software = recovery_software(&f);
        let shared = LaunchVerification::default();
        let registration = f.m.registry().unwrap().classes[&f.r.key()].registration.clone();
        let (_, snapshot) = prepare_recovery_bindings(&f.m, &software,
            vec![registration.clone()], &shared, Instant::now() + Duration::from_secs(2)).unwrap();
        snapshot.run(|| require_recovery_registration(&f.m, &registration)).unwrap();
        let mut registry = f.m.registry().unwrap();
        registry.classes.remove(&f.r.key());
        atomic_json(&f.m.root.join("registry.json"), &registry).unwrap();
        assert_eq!(snapshot.run(|| require_recovery_registration(&f.m, &registration))
            .unwrap_err().to_string(), "service_recovery_registration_changed");
        assert!(!f.m.root.join("runtime/leases").exists());
    }
    #[test]
    fn service_recovery_starts_both_environments_before_waiting_and_confirms_each() {
        use std::cell::RefCell;
        let (f,_,_,_)=test_fixture::prepared();
        let software=recovery_software(&f);
        let first=f.r.clone();
        let mut second=first.clone();
        second.environment.id="55".repeat(16);
        let bindings=vec![(first,software.clone()),(second,software)];
        let started=RefCell::new(std::collections::BTreeSet::new());
        let rounds=std::cell::Cell::new(0);
        let confirmed=RefCell::new(Vec::new());
        restore_recovery_bindings(&bindings,Instant::now()+Duration::from_secs(1),
            || Ok(()), |registration,_| {
                started.borrow_mut().insert(registration.environment.id.clone());
                Ok(if rounds.get()>=2 {KeeperAvailability::Ready} else {KeeperAvailability::Starting})
            }, |registration,_| {
                if rounds.get()>=2 {confirmed.borrow_mut().push(registration.environment.id.clone());}
                Ok(())
            }, |_| {
                assert_eq!(started.borrow().len(),2,
                    "one environment cannot consume the next environment's startup budget");
                rounds.set(rounds.get()+1);
            }).unwrap();
        assert_eq!(rounds.get(),2);
        assert_eq!(confirmed.borrow().len(),2,"every environment must actually report ready");
    }
    #[test]
    fn recovery_acknowledgment_cannot_inherit_a_replacement_keeper_generation() {
        let (f,_,_,_)=test_fixture::prepared();
        let registration=f.m.registry().unwrap().classes[&f.r.key()].registration.clone();
        let software=recovery_software(&f);
        let (first,report,lease)=fixture_keeper(&f,"exec sleep 1");
        let keepers=Mutex::new(vec![first]);
        let mut ready_owners=std::collections::BTreeMap::new();
        let _registry=f.m.lock("registry.lock").unwrap();
        atomic_json(&report,&serde_json::json!({"environment":registration.environment.id,"ready":true})).unwrap();
        assert_eq!(stage_recovery_keeper(&f.m,&software,&registration,&keepers,&mut ready_owners).unwrap(),
            KeeperAvailability::Ready);
        {
            let mut active=keepers.lock().unwrap();
            active[0].retiring=true;
            assert!(active[0].child.wait().unwrap().success());
            atomic_json(&report,&serde_json::json!({"ready":false,"cleanup_confirmed":true})).unwrap();
            assert_eq!(observe_keeper(&registration.environment.id,&mut active).unwrap(),None);
            assert!(!lease.exists());
            let (next,next_report,_)=fixture_keeper(&f,"exec sleep 1");
            atomic_json(&next_report,&serde_json::json!({"environment":registration.environment.id,"ready":true})).unwrap();
            active.push(next);
        }
        assert_eq!(stage_recovery_keeper(&f.m,&software,&registration,&keepers,&mut ready_owners)
            .unwrap_err().to_string(),"service_recovery_owner_changed");
        let mut active=keepers.lock().unwrap();
        assert!(active[0].lease.exists(),"refusal cannot release a replacement owner");
        active[0].retiring=true;
        assert!(active[0].child.wait().unwrap().success());
        atomic_json(&active[0].report,&serde_json::json!({"ready":false,"cleanup_confirmed":true})).unwrap();
        assert_eq!(observe_keeper(&registration.environment.id,&mut active).unwrap(),None);
    }
    #[test]
    fn recovery_verification_releases_registry_and_refuses_changed_readiness() {
        let (f,_,_,_)=test_fixture::prepared();
        let bindings=vec![(f.r.clone(),recovery_software(&f))];
        let verified=std::cell::Cell::new(false);
        let deadline=Instant::now()+Duration::from_secs(1);
        let result=restore_recovery_bindings(&bindings,deadline,
            || f.m.lock("registry.lock"), |_,_| {
                Ok(if verified.get() {KeeperAvailability::Starting} else {KeeperAvailability::Ready})
            }, |_,_| {
                // An independent readback must acquire the actual registry
                // while full verification is running, then final authority
                // must refuse the readiness change before acknowledging LVE1.
                let _readback=f.m.lock("registry.lock")?;
                verified.set(true);
                Ok(())
            }, |_| panic!("ready owner cannot require a startup pause"));
        assert_eq!(result.unwrap_err().to_string(),"service_recovery_owner_changed");
        assert!(verified.get());
    }
    #[test]
    fn service_recovery_never_confirms_a_failed_or_changed_binding() {
        let (f,_,_,_)=test_fixture::prepared();
        let bindings=vec![(f.r.clone(),recovery_software(&f))];
        let error=restore_recovery_bindings(&bindings,Instant::now()+Duration::from_secs(1),
            || Ok(()), |_,_| Ok(KeeperAvailability::Failed),
            |_,_| panic!("failed environment confirmed"), |_| panic!("failed environment retried")).unwrap_err();
        assert_eq!(error.downcast_ref::<capacity::Refusal>(),Some(&capacity::Refusal::BindingInvalid));
        let error=restore_recovery_bindings(&bindings,Instant::now()+Duration::from_secs(1),
            || Ok(()), |_,_| Ok(KeeperAvailability::Ready),
            |_,_| Err("service_recovery_components_changed".into()),
            |_| panic!("changed binding retried")).unwrap_err();
        assert_eq!(error.to_string(),"service_recovery_components_changed");
        assert!(restore_recovery_bindings::<()>(&bindings,Instant::now(),
            || panic!("expired reservation"), |_,_| panic!("expired staging"),
            |_,_| panic!("expired confirmation"), |_| panic!("expired wait")).is_err());
    }
    fn fixture_keeper(f:&test_fixture::Fixture,command:&str)->(KeeperOwner,PathBuf,PathBuf) {
        let report=f.outer.join(format!("keeper-{}.json",random_id().unwrap()));
        let lease=f.outer.join(format!("keeper-{}.lease",random_id().unwrap()));
        fs::write(&lease,b"keeper").unwrap();
        let child=Command::new("/bin/sh").args(["-c",command]).spawn().unwrap();
        (KeeperOwner{session:random_id().unwrap(),environment:f.r.environment.id.clone(),graphical_session:None,child,
            report:report.clone(),lease:lease.clone(),retiring:false,failed:false,failure_pending:false,
            started:Instant::now()},
            report,lease)
    }
    fn graphical(display:&str,generation:u64)->transport_storage::GraphicalSession {
        transport_storage::GraphicalSession{schema:1,peer_pid:77,peer_start_ticks:generation,
            display:display.into(),wayland_display:Some("wayland-session".into()),
            xauthority:None,dbus_session_bus_address:None}
    }
    #[test]
    fn keeper_is_retired_before_reuse_on_a_different_graphical_session() {
        let f=test_fixture::Fixture::new();
        let (mut owner,_,_)=fixture_keeper(&f,"sleep 5");
        owner.graphical_session=Some(graphical(":1",1));
        let mut active=vec![owner];
        // A different requester in the same display context shares the keeper.
        assert!(!retire_mismatched_graphical_keeper(&f.m,&f.r.environment.id,
            Some(&graphical(":1",2)),&mut active).unwrap());
        assert!(retire_mismatched_graphical_keeper(&f.m,&f.r.environment.id,
            Some(&graphical(":2",2)),&mut active).unwrap());
        assert!(active[0].child.wait().unwrap().code().is_none());
    }
    #[test]
    fn graphical_transition_never_retires_a_keeper_while_dsp_ownership_exists() {
        let f=test_fixture::Fixture::new();
        let (job,_)=spec(&f.m,f.r.clone().into(),false,false,false).unwrap();
        atomic_json(&job.lease,&job.report).unwrap();
        let (mut owner,_,_)=fixture_keeper(&f,"sleep 5");
        owner.graphical_session=Some(graphical(":1",1));
        let mut active=vec![owner];
        assert!(retire_mismatched_graphical_keeper(&f.m,&f.r.environment.id,
            Some(&graphical(":2",2)),&mut active).is_err());
        assert!(active[0].child.try_wait().unwrap().is_none());
        active[0].child.kill().unwrap();active[0].child.wait().unwrap();
        fs::remove_file(job.lease).unwrap();
    }
    #[test]
    fn keeper_generation_is_ready_only_while_live_and_exactly_bound() {
        let f=test_fixture::Fixture::new();
        let (owner,report,lease)=fixture_keeper(&f,"sleep 0.2");
        let mut active=vec![owner];
        assert_eq!(observe_keeper(&f.r.environment.id,&mut active).unwrap(),
            Some(KeeperAvailability::Starting));
        atomic_json(&report,&serde_json::json!({"ready":true,
            "environment":f.r.environment.id})).unwrap();
        assert_eq!(observe_keeper(&f.r.environment.id,&mut active).unwrap(),
            Some(KeeperAvailability::Ready));
        std::thread::sleep(Duration::from_millis(250));
        atomic_json(&report,&serde_json::json!({"ready":false,
            "cleanup_confirmed":true})).unwrap();
        assert_eq!(observe_keeper(&f.r.environment.id,&mut active).unwrap(),
            Some(KeeperAvailability::Failed));
        assert_eq!(observe_keeper(&f.r.environment.id,&mut active).unwrap(),
            Some(KeeperAvailability::Failed));
        assert_eq!(active.len(),1);
        assert!(!lease.exists());
    }
    #[test]
    fn shared_keeper_gate_refuses_v4_before_create_or_reuse_on_all_routes() {
        let f=test_fixture::Fixture::new();
        let mut binding:HostBinding=f.r.clone().into();
        binding.environment.runner.id=experimental_runner::BG1_V4_RUNNER.into();
        let software=Software{installer_launch:None,preparation_kit:None,operator_frontend:None,
            manager:f.r.host.clone(),supervisor:f.r.host.clone(),ownership:f.r.host.clone(),
            host:f.r.host.clone(),source_manifest:f.r.host.clone(),
            source_sha256:f.r.host_source_sha256.clone(),native_catalogue:None};
        let keepers=Keepers::new(Vec::new());
        let sessions=binding.environment.root.join("compatdata/pfx/drive_c/bridge/sessions");
        assert!(stage_keeper(&f.m,&software,&binding,&keepers,None).is_err());
        // Inspection and vendor access use the same fresh startup coordinator.
        // Its owner must refuse before an instance or keeper is materialized.
        assert!(wait_for_keeper(Instant::now()+Duration::from_secs(KEEPER_OWNER_STARTUP_SECONDS),
            || Ok(()), || stage_keeper(&f.m,&software,&binding,&keepers,None),
            std::thread::sleep).is_err());
        assert!(!sessions.exists());
        assert!(!f.m.root.join("runtime/leases").exists());
        assert!(keepers.lock().unwrap().is_empty());

        let (owner,report,lease)=fixture_keeper(&f,"sleep 30");
        atomic_json(&report,&serde_json::json!({"ready":true,
            "environment":f.r.environment.id})).unwrap();
        keepers.lock().unwrap().push(owner);
        assert!(stage_keeper(&f.m,&software,&binding,&keepers,None).is_err());
        assert_eq!(keepers.lock().unwrap().len(),1);
        assert!(keepers.lock().unwrap()[0].child.try_wait().unwrap().is_none());
        assert_eq!(read_json::<serde_json::Value>(&report).unwrap()["ready"],true);
        assert!(lease.exists());

        for runner in ["exact-runner-1","proton-11.0-2c-x11-touch-routing-v2",
            "proton-11.0-2c-dcomp-reference-v1"] {
            binding.environment.runner.id=runner.into();
            assert_eq!(stage_keeper(&f.m,&software,&binding,&keepers,None).unwrap(),
                KeeperAvailability::Ready);
        }
        let mut active=keepers.lock().unwrap();
        active[0].child.kill().unwrap();active[0].child.wait().unwrap();
    }
    #[test]
    fn service_resumed_keeper_retires_before_one_graphical_generation() {
        let f=test_fixture::Fixture::new();
        let (owner,report,lease)=fixture_keeper(&f,
            "exec python3 -c 'import signal,sys,time; signal.signal(signal.SIGTERM,lambda *_:sys.exit(0)); time.sleep(5)'");
        atomic_json(&report,&serde_json::json!({"ready":true,
            "environment":f.r.environment.id})).unwrap();
        let mut active=vec![owner];
        assert_eq!(observe_keeper(&f.r.environment.id,&mut active).unwrap(),
            Some(KeeperAvailability::Ready));
        std::thread::sleep(Duration::from_millis(50));
        assert!(retire_mismatched_graphical_keeper(&f.m,&f.r.environment.id,
            Some(&graphical(":1",1)),&mut active).unwrap());
        assert!(active[0].retiring);
        let status=active[0].child.wait().unwrap();
        atomic_json(&report,&serde_json::json!({"ready":false,
            "cleanup_confirmed":true})).unwrap();
        // Preserve the already reaped status shape used by observe_keeper.
        assert!(status.success());
        assert_eq!(observe_keeper(&f.r.environment.id,&mut active).unwrap(),None);
        assert!(active.is_empty()&&!lease.exists());
    }
    #[test]
    fn timed_out_keeper_retirement_becomes_a_stable_failure() {
        let f=test_fixture::Fixture::new();
        let (mut owner,report,lease)=fixture_keeper(&f,
            "exec python3 -c 'import signal,sys,time; signal.signal(signal.SIGTERM,lambda *_:sys.exit(0)); time.sleep(5)'");
        std::thread::sleep(Duration::from_millis(50));
        owner.retiring=true;owner.failure_pending=true;
        unsafe{libc::kill(owner.child.id() as i32,libc::SIGTERM)};
        assert!(owner.child.wait().unwrap().success());
        atomic_json(&report,&serde_json::json!({"ready":false,
            "cleanup_confirmed":true})).unwrap();
        let mut active=vec![owner];
        assert_eq!(observe_keeper(&f.r.environment.id,&mut active).unwrap(),
            Some(KeeperAvailability::Failed));
        assert_eq!(observe_keeper(&f.r.environment.id,&mut active).unwrap(),
            Some(KeeperAvailability::Failed));
        assert_eq!(active.len(),1);assert!(!lease.exists());
    }
    #[test]
    fn keeper_wrong_binding_or_unconfirmed_exit_never_becomes_ready() {
        let f=test_fixture::Fixture::new();
        let (owner,report,_)=fixture_keeper(&f,"sleep 0.2");
        let mut active=vec![owner];
        atomic_json(&report,&serde_json::json!({"ready":true,"environment":"wrong"})).unwrap();
        assert!(observe_keeper(&f.r.environment.id,&mut active).is_err());
        std::thread::sleep(Duration::from_millis(250));
        atomic_json(&report,&serde_json::json!({"ready":false,
            "cleanup_confirmed":false})).unwrap();
        assert!(observe_keeper(&f.r.environment.id,&mut active).is_err());
        assert_eq!(active.len(),1);
    }
    #[test]
    fn keeper_admission_incident_names_the_private_failure_source() {
        let f=test_fixture::Fixture::new();
        let request=[7;16];
        retain_admission_incident(&f.m,"keeper_failed",Some(request),
            Some(&f.r.metadata.class_id),Some(&f.r.environment.id),Some(&"ab".repeat(16)))
            .unwrap();
        let incident:serde_json::Value=read_json(&f.m.root.join("runtime/admission-incidents")
            .join(format!("environment-{}.json",f.r.environment.id.to_ascii_lowercase()))).unwrap();
        assert_eq!(incident["source"],"keeper_failed");
        assert_eq!(incident["request"],hex(&request));
        assert_eq!(incident["environment"],f.r.environment.id);
    }
    #[test]
    fn keeper_spec_binds_the_shared_startup_deadline() {
        let f=test_fixture::Fixture::new();
        let (job,_)=spec(&f.m,f.r.clone().into(),true,false,true).unwrap();
        assert_eq!(job.keeper_startup_seconds,Some(KEEPER_OWNER_STARTUP_SECONDS));
        assert_eq!(job.runner_key.as_deref(), Some(catalogue::runner_key(&f.r.environment.runner).unwrap().as_str()));
        assert_eq!(KEEPER_MANAGER_RETIRE_SECONDS,KEEPER_OWNER_STARTUP_SECONDS+2);
    }
    #[test]
    fn missing_candidate_onboarding_cannot_fall_back_to_operator_home() {
        let f=super::test_fixture::Fixture::new();
        let mut r:super::HostBinding=f.r.clone().into();
        r.metadata.class_id=super::managed_candidate::candidate().unwrap().class.class_id;
        assert!(super::spec(&f.m,r,false,false,false).is_err());
    }
    use super::*;
    #[test]
    fn exposed_binding_cannot_erase_an_unresolved_retirement() {
        let f = test_fixture::Fixture::new();
        let lease = f.outer.join("lease.json");
        fs::write(&lease, b"reservation").unwrap();
        let blocked = Arc::new(AtomicBool::new(false));
        drop(PendingAdmission::new(lease.clone(), blocked.clone()));
        assert!(!lease.exists());
        fs::write(&lease, b"bound native owner").unwrap();
        let blocked = Arc::new(AtomicBool::new(false));
        let mut pending = PendingAdmission::new(lease.clone(), blocked.clone());
        pending.expose();
        drop(pending);
        assert!(lease.exists());
        assert!(blocked.load(Ordering::Acquire));
        let blocked = Arc::new(AtomicBool::new(false));
        let mut pending = PendingAdmission::new(lease.clone(), blocked.clone());
        pending.expose();
        pending
            .complete("exact", true, "LVO1 exact retired\n", None)
            .unwrap();
        pending
            .complete("exact", true, "LVO1 exact retired\n", None)
            .unwrap();
        drop(pending);
        assert!(!lease.exists());
        assert!(!blocked.load(Ordering::Acquire));
    }
    #[test]
    fn failed_binding_delivery_still_retires_the_retained_supervisor_owner() {
        let f=test_fixture::Fixture::new();
        let session="aa".repeat(16);
        let lease=f.outer.join("delivery-lease.json");
        let report=f.m.root.join("runtime/results/delivery.json");
        private_dir(report.parent().unwrap()).unwrap();
        atomic_json(&report,&serde_json::json!({"session":session,"error":null,
            "gated":true,"cleanup_confirmed":true,"transport_retired":true,
            "fault_status":null})).unwrap();
        fs::write(&lease,b"owner").unwrap();
        let blocked=Arc::new(AtomicBool::new(false));
        let mut pending=PendingAdmission::new(lease.clone(),blocked.clone());
        pending.expose();
        let command=format!("printf 'LVO1 {session} retired\\n'");
        let mut child=Command::new("/bin/sh").args(["-c",&command])
            .stdout(Stdio::piped()).spawn().unwrap();
        let class="01".repeat(16);
        assert_eq!(finish_supervised_delivery(SupervisorDelivery{manager:&f.m,class_id:&class,
            report:&report,session:&session,transport:None},Err("delivery failed".into()),
            &mut child,&mut pending)
            .unwrap_err().to_string(),"delivery failed");
        assert!(!lease.exists());
        assert!(!blocked.load(Ordering::Acquire));
    }
    #[test]
    fn terminal_projection_failure_cannot_abandon_a_retired_owner() {
        let f=test_fixture::Fixture::new();
        let session="ac".repeat(16);
        let class="01".repeat(16);
        let lease=f.outer.join("projection-lease.json");
        let report=f.m.root.join("runtime/results/projection.json");
        private_dir(report.parent().unwrap()).unwrap();
        atomic_json(&report,&serde_json::json!({
            "session":session,"error":"Windows host exited",
            "gated":true,"cleanup_confirmed":true,"transport_retired":true,
            "fault_status":{"before_containment":{"terminal_instance":{
                "schema":1,"session":session,"failure_class":1,
                "producer":2,"status_domain":3
            }}}
        })).unwrap();
        private_dir(&f.m.root.join("runtime")).unwrap();
        fs::write(f.m.root.join("runtime/terminal-summaries"),b"not a directory").unwrap();
        fs::write(&lease,b"owner").unwrap();
        let blocked=Arc::new(AtomicBool::new(false));
        let mut pending=PendingAdmission::new(lease.clone(),blocked.clone());
        pending.expose();
        let command=format!("printf 'LVO1 {session} retired\\n'");
        let mut child=Command::new("/bin/sh").args(["-c",&command])
            .stdout(Stdio::piped()).spawn().unwrap();
        assert!(finish_supervised_delivery(SupervisorDelivery{manager:&f.m,class_id:&class,
            report:&report,session:&session,transport:None},Ok(()),&mut child,
            &mut pending).is_err());
        assert!(!lease.exists());
        assert!(!blocked.load(Ordering::Acquire));
    }
    #[test]
    fn exact_supervisor_readiness_precedes_exposure() {
        let session="ab".repeat(16);
        let command=format!("printf 'LVO0 {session} ready\\n'; sleep 5");
        let mut child=Command::new("/bin/sh").args(["-c",&command])
            .stdout(Stdio::piped()).spawn().unwrap();
        supervisor_ready_for_configuration(&mut child,&session,None,Duration::from_secs(1)).unwrap();
        child.kill().unwrap();child.wait().unwrap();

        let mut wrong=Command::new("/bin/sh").args(["-c","printf 'LVO0 wrong ready\\n'"])
            .stdout(Stdio::piped()).spawn().unwrap();
        assert!(supervisor_ready_for_configuration(&mut wrong,&session,None,Duration::from_secs(1)).is_err());
        wrong.wait().unwrap();

        let f=test_fixture::Fixture::new();
        let lease=f.outer.join("unready-lease.json");fs::write(&lease,b"unexposed").unwrap();
        let blocked=Arc::new(AtomicBool::new(false));
        drop(PendingAdmission::new(lease.clone(),blocked.clone()));
        assert!(!lease.exists()&&!blocked.load(Ordering::Acquire));
    }
    #[test]
    fn graphics_settings_require_capability_acknowledgment_from_any_retained_generation() {
        let session = "ac".repeat(16);
        for acknowledged in [false, true] {
            let capability = if acknowledged { " graphics-v1" } else { "" };
            let command = format!("printf 'LVO0 {session} ready{capability}\\n'");
            let mut child = Command::new("/bin/sh").args(["-c", &command])
                .stdout(Stdio::piped()).spawn().unwrap();
            let result = supervisor_ready_for_configuration(&mut child, &session,
                Some(operator_model::GraphicsBackend::WineD3d11), Duration::from_secs(1));
            assert_eq!(result.is_ok(), acknowledged);
            child.wait().unwrap();
        }
    }
    #[test]
    #[cfg(target_os="linux")]
    fn graphics_launch_refuses_legacy_entry_before_it_can_start_an_owner() {
        let (f, _, _, _) = test_fixture::prepared();
        let mut software = recovery_software(&f);
        let script = f.outer.join("legacy-supervisor.py");
        // The retained interface reads argv[1] as the job before launching.
        fs::write(&script, b"import json,pathlib,sys\njob=json.loads(pathlib.Path(sys.argv[1]).read_text())\npathlib.Path(job['directory'],'owner-started').write_text('legacy launch')\n").unwrap();
        software.supervisor = Artifact { sha256: digest(&script).unwrap(), path: script };
        for graphics in [None, Some(operator_model::GraphicsBackend::WineD3d11)] {
            let mut registration = f.r.clone();
            registration.compatibility.graphics = graphics;
            let (job, path) = spec(&f.m, registration.into(), true, false, false).unwrap();
            let mut child = spawn(&f.m, &software, &path, None).unwrap();
            let status = child.wait().unwrap();
            assert_eq!(status.success(), graphics.is_none());
            assert_eq!(job.directory.join("owner-started").exists(), graphics.is_none());
            if graphics.is_some() {
                assert!(supervisor_ready(&mut child, &job, Duration::from_secs(1)).unwrap_err()
                    .to_string().contains("check compatibility to prepare a current configuration"));
            }
        }
    }
    #[test]
    fn real_supervisor_preflight_refuses_before_exposure() {
        let f=test_fixture::Fixture::new();
        let (mut job,path)=spec(&f.m,f.r.clone().into(),false,false,false).unwrap();
        job.graphical_session=Some(transport_storage::GraphicalSession{
            schema:1,peer_pid:std::process::id() as i32,peer_start_ticks:1,
            display:":fixture".into(),wayland_display:None,xauthority:None,
            dbus_session_bus_address:None,
        });
        atomic_json(&path,&job).unwrap();
        atomic_json(&job.lease,&job.report).unwrap();
        let blocked=Arc::new(AtomicBool::new(false));
        let pending=PendingAdmission::new(job.lease.clone(),blocked.clone());
        let (_native,supervisor)=UnixStream::pair().unwrap();
        let stdin=unsafe{Stdio::from_raw_fd(supervisor.into_raw_fd())};
        let script=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime/session.py");
        let mut child=Command::new("/usr/bin/python3").arg(script).arg(&path)
            .stdin(stdin).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        assert!(supervisor_ready(&mut child,&job,Duration::from_secs(2)).is_err());
        retire_unready_supervisor(&mut child,&path).unwrap();
        drop(pending);
        assert!(!job.lease.exists());
        assert!(!job.directory.exists());
        assert!(!blocked.load(Ordering::Acquire));
    }
    #[cfg(target_os="linux")]
    #[test]
    fn stop_at_real_supervisor_readiness_completes_native_and_manager_retirement() {
        check_stop_at_real_supervisor_readiness(None);
        check_stop_at_real_supervisor_readiness(Some(operator_model::GraphicsBackend::WineD3d11));
    }
    #[cfg(target_os="linux")]
    fn check_stop_at_real_supervisor_readiness(graphics: Option<operator_model::GraphicsBackend>) {
        let f=test_fixture::Fixture::new();
        let mut registration = f.r.clone();
        registration.compatibility.graphics = graphics;
        let (job,path)=spec(&f.m,registration.into(),false,false,false).unwrap();
        let bin=f.outer.join("fixture-bin");private_dir(&bin).unwrap();
        let systemctl=bin.join("systemctl");
        fs::write(&systemctl,b"#!/bin/sh\nprintf 'DISPLAY=:fixture\\n'\n").unwrap();
        fs::set_permissions(&systemctl,fs::Permissions::from_mode(0o500)).unwrap();
        atomic_json(&job.lease,&job.report).unwrap();
        let blocked=Arc::new(AtomicBool::new(false));
        let mut pending=PendingAdmission::new(job.lease.clone(),blocked.clone());
        let (mut native,supervisor)=UnixStream::pair().unwrap();
        native.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let stdin=unsafe{Stdio::from_raw_fd(supervisor.into_raw_fd())};
        let script=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime/session.py");
        let mut command = Command::new("/usr/bin/python3");
        command.arg(script);
        if graphics.is_some() { command.arg("--graphics-settings-v1"); }
        let mut child=command.arg(&path)
            .env("PATH",format!("{}:/usr/bin:/bin",bin.display()))
            .stdin(stdin).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
        supervisor_ready(&mut child,&job,Duration::from_secs(2)).unwrap();
        pending.expose();
        assert_eq!(unsafe{libc::kill(child.id() as i32,libc::SIGTERM)},0);
        let mut byte=[0;1];native.read_exact(&mut byte).unwrap();assert_eq!(byte,[b'F']);
        assert!(job.directory.exists());
        native.shutdown(std::net::Shutdown::Write).unwrap();
        native.read_exact(&mut byte).unwrap();assert_eq!(byte,[b'R']);
        assert!(!job.directory.exists());
        let class=job.registration.metadata.class_id.clone();
        finish_supervised_delivery(SupervisorDelivery{manager:&f.m,class_id:&class,
            report:&job.report,session:&job.session,transport:None},Ok(()),&mut child,
            &mut pending).unwrap();
        let report:serde_json::Value=read_json(&job.report).unwrap();
        assert_eq!(report["cleanup_confirmed"],true);
        assert_eq!(report["transport_retired"],true);
        assert!(job.report.with_extension("ownership.json").exists());
        assert!(!job.lease.exists());
        assert!(!blocked.load(Ordering::Acquire));
    }
    #[test]
    fn only_exact_positive_retirement_releases_one_exposed_owner() {
        let f = test_fixture::Fixture::new();
        let lease = f.outer.join("lease.json");
        let sibling = f.outer.join("sibling.json");
        let transport = f.outer.join("transport");
        fs::write(&lease, b"owner").unwrap();
        fs::write(&sibling, b"sibling").unwrap();
        fs::create_dir(&transport).unwrap();
        let blocked = Arc::new(AtomicBool::new(false));
        let mut pending = PendingAdmission::new(lease.clone(), blocked.clone());
        assert!(pending
            .complete("a", true, "LVO1 a retired\n", None)
            .is_err());
        pending.expose();
        for (success, receipt) in [
            (false, "LVO1 a retired\n"),
            (true, "LVO1 b retired\n"),
            (true, ""),
        ] {
            assert!(pending
                .complete("a", success, receipt, Some(&transport))
                .is_err());
            assert!(lease.exists() && sibling.exists());
        }
        assert!(pending
            .complete("a", true, "LVO1 a retired\n", Some(&transport))
            .is_err());
        fs::remove_dir(&transport).unwrap();
        pending
            .complete("a", true, "LVO1 a retired\n", Some(&transport))
            .unwrap();
        // A duplicate completion cannot unlink a replacement or sibling.
        fs::write(&lease, b"replacement").unwrap();
        pending
            .complete("a", true, "LVO1 a retired\n", Some(&transport))
            .unwrap();
        drop(pending);
        assert_eq!(fs::read(&lease).unwrap(), b"replacement");
        assert_eq!(fs::read(&sibling).unwrap(), b"sibling");
        assert!(!blocked.load(Ordering::Acquire));
    }
    #[test]
    fn failed_unlink_or_post_spawn_error_keeps_admission_blocked() {
        let f = test_fixture::Fixture::new();
        let lease = f.outer.join("lease.json");
        fs::create_dir(&lease).unwrap(); // deterministic remove_file failure
        let blocked = Arc::new(AtomicBool::new(false));
        drop(PendingAdmission::new(lease.clone(), blocked.clone()));
        assert!(blocked.load(Ordering::Acquire) && lease.exists());
        blocked.store(false, Ordering::Release);
        let mut pending = PendingAdmission::new(lease.clone(), blocked.clone());
        pending.expose();
        assert!(pending
            .complete("a", true, "LVO1 a retired\n", None)
            .is_err());
        drop(pending);
        assert!(blocked.load(Ordering::Acquire) && lease.exists());
        fs::remove_dir(&lease).unwrap();
        fs::write(&lease, b"owner").unwrap();
        blocked.store(false, Ordering::Release);
        fn after_spawn(lease: PathBuf, blocked: Arc<AtomicBool>) -> Result<()> {
            let mut pending = PendingAdmission::new(lease, blocked);
            pending.expose();
            Err("post-spawn wait/read failure".into())
        }
        let failure = after_spawn(lease.clone(), blocked.clone());
        assert!(failure.is_err() && blocked.load(Ordering::Acquire) && lease.exists());
    }
    #[test]
    #[cfg(target_os="linux")]
    fn real_inspection_lock_refusal_releases_reservation_but_retains_failure() {
        let f = test_fixture::Fixture::new();
        let (job, path) = spec(&f.m, f.r.clone().into(), true, false, false).unwrap();
        let marker = f.r.environment.root.join("retained-vendor-state");
        fs::write(&marker, b"untouched").unwrap();
        let keeper = fs::OpenOptions::new().read(true).write(true).create(true)
            .truncate(false).open(f.r.environment.root.join("operation.lock")).unwrap();
        assert_eq!(unsafe { libc::flock(keeper.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) }, 0);
        let bin = f.outer.join("fixture-bin");
        private_dir(&bin).unwrap();
        let systemctl = bin.join("systemctl");
        fs::write(&systemctl, b"#!/bin/sh\nprintf 'DISPLAY=:fixture\\n'\n").unwrap();
        fs::set_permissions(&systemctl, fs::Permissions::from_mode(0o500)).unwrap();
        atomic_json(&job.lease, &job.report).unwrap();
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime/session.py");
        let status = Command::new("python3").arg(script).arg(&path)
            .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()))
            .stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap();
        assert!(status.success()); // Exact retirement does not make inspection successful.
        assert!(finish_inspection(&f.m, &job, status).unwrap_err().to_string()
            .contains("inspection failed"));
        let result: serde_json::Value = read_json(&job.report).unwrap();
        assert!(result["error"].as_str().unwrap().contains("BlockingIOError"));
        assert!(!job.directory.exists() && !job.lease.exists());
        assert!(job.report.with_extension("ownership.json").exists());
        assert!(capacity::owners(&f.m).unwrap().is_empty());
        assert_eq!(fs::read(marker).unwrap(), b"untouched");
    }
    #[test]
    fn successful_inspection_exit_cannot_release_an_unconfirmed_reservation() {
        let f = test_fixture::Fixture::new();
        let (job, _) = spec(&f.m, f.r.clone().into(), true, false, false).unwrap();
        atomic_json(&job.lease, &job.report).unwrap();
        atomic_json(&job.report, &serde_json::json!({
            "ownership_schema":1,"error":null,"cleanup_confirmed":true,"transport_retired":true
        })).unwrap();
        atomic_json(&job.report.with_extension("ownership.json"), &serde_json::json!({
            "session":"wrong","cleanup_confirmed":true,"transport_retired":true
        })).unwrap();
        let status = Command::new("/bin/sh").args(["-c", "exit 0"]).status().unwrap();
        assert!(finish_inspection(&f.m, &job, status).unwrap_err().to_string()
            .contains("inspection cleanup unconfirmed"));
        assert!(job.directory.exists() && job.lease.exists());
    }
    #[test]
    fn kernel_restart_retires_bound_leases_and_preserves_failed_result() {
        let f = test_fixture::Fixture::new();
        let (job, _) = spec(&f.m, f.r.clone().into(), true, false, false).unwrap();
        atomic_json(&job.lease, &job.report).unwrap();
        let result = serde_json::json!({"ready":true,"cleanup_confirmed":false,
            "error":"interrupted before retirement","plugin_state":"not captured"});
        atomic_json(&job.report, &result).unwrap();
        let original = fs::read(&job.report).unwrap();
        let generations = f.m.root.join("runtime/lease-generations");
        private_dir(&generations).unwrap();
        let path = generations.join(format!("{}.json", job.session));
        atomic_json(&path, &LeaseGeneration {schema:1,session:job.session.clone(),
            report:job.report.clone(),kernel_boot:"11111111-1111-1111-1111-111111111111".into(),
            basis:LeaseGenerationBasis::BeforeLaunch}).unwrap();
        assert!(reconcile_leases_in_kernel(&f.m,
            Some("11111111-1111-1111-1111-111111111111")).unwrap());
        assert!(job.lease.exists());
        assert!(reconcile_leases_in_kernel(&f.m, None).unwrap());
        assert!(job.lease.exists());
        assert!(!reconcile_leases_in_kernel(&f.m,
            Some("22222222-2222-2222-2222-222222222222")).unwrap());
        assert!(!job.lease.exists());
        assert_eq!(fs::read(&job.report).unwrap(), original);
        let receipt: serde_json::Value = read_json(&path.with_extension("interrupted.json")).unwrap();
        assert_eq!(receipt["session"], job.session);
        assert_eq!(receipt["retirement_basis"], "kernel_generation_ended");
        assert_eq!(receipt["successful_session"], false);
    }
    #[test]
    fn kernel_restart_cannot_infer_legacy_ownership_or_accept_wrong_binding() {
        let f = test_fixture::Fixture::new();
        let (job, _) = spec(&f.m, f.r.clone().into(), true, false, false).unwrap();
        atomic_json(&job.lease, &job.report).unwrap();
        let current = Some("22222222-2222-2222-2222-222222222222");
        assert!(reconcile_leases_in_kernel(&f.m, current).unwrap());
        assert!(job.lease.exists());
        let directory = f.m.root.join("runtime/lease-generations");
        private_dir(&directory).unwrap();
        let path = directory.join(format!("{}.json", job.session));
        for (session, report, boot) in [
            ("wrong".to_owned(), job.report.clone(), "11111111-1111-1111-1111-111111111111"),
            (job.session.clone(), job.report.with_extension("other.json"), "11111111-1111-1111-1111-111111111111"),
            (job.session.clone(), job.report.clone(), "invalid"),
        ] {
            atomic_json(&path, &LeaseGeneration {schema:1,session,report,kernel_boot:boot.into(),
                basis:LeaseGenerationBasis::BeforeLaunch}).unwrap();
            assert!(reconcile_leases_in_kernel(&f.m, current).is_err());
            assert!(job.lease.exists());
            assert!(!path.with_extension("interrupted.json").exists());
        }
    }
    #[test]
    #[cfg(target_os="linux")]
    fn launch_generation_precedes_spawn_and_cannot_be_replaced() {
        let f = test_fixture::Fixture::new();
        let (job, _) = spec(&f.m, f.r.clone().into(), true, false, false).unwrap();
        assert!(!job.lease.exists());
        record_lease_generation(&job).unwrap();
        let path = f.m.root.join("runtime/lease-generations").join(format!("{}.json", job.session));
        let original = fs::read(&path).unwrap();
        let generation: LeaseGeneration = read_json(&path).unwrap();
        assert_eq!(generation.kernel_boot, kernel_boot().unwrap().unwrap());
        assert!(record_lease_generation(&job).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(!job.lease.exists());
    }
    #[test]
    fn stopped_legacy_keeper_needs_a_subsequent_restart_without_inferred_launch() {
        let f = test_fixture::Fixture::new();
        let (job, _) = spec(&f.m, f.r.clone().into(), true, false, true).unwrap();
        atomic_json(&job.lease, &job.report).unwrap();
        atomic_json(&job.report, &serde_json::json!({"ready":true,"cleanup_confirmed":false})).unwrap();
        let result = fs::read(&job.report).unwrap();
        let boot = "11111111-1111-1111-1111-111111111111";
        assert!(!observe_stopped_leases(&f.m, None).unwrap());
        assert!(observe_stopped_leases(&f.m, Some("invalid")).is_err());
        assert!(observe_stopped_leases(&f.m, Some(boot)).unwrap());
        let path = f.m.root.join("runtime/lease-generations").join(format!("{}.json", job.session));
        let observed = fs::read(&path).unwrap();
        let record: serde_json::Value = read_json(&path).unwrap();
        assert_eq!(record["basis"], "stopped_service_observation");
        assert!(reconcile_leases_in_kernel(&f.m, Some(boot)).unwrap());
        assert!(job.lease.exists());
        // Repeated recovery must not move the recorded observation to a newer
        // kernel or change the failed result into a successful session.
        assert!(observe_stopped_leases(&f.m,
            Some("22222222-2222-2222-2222-222222222222")).unwrap());
        assert_eq!(fs::read(&path).unwrap(), observed);
        assert!(!reconcile_leases_in_kernel(&f.m,
            Some("22222222-2222-2222-2222-222222222222")).unwrap());
        assert!(!job.lease.exists());
        assert_eq!(fs::read(&job.report).unwrap(), result);
        let retired: serde_json::Value = read_json(&path.with_extension("interrupted.json")).unwrap();
        assert_eq!(retired["observation_basis"], "stopped_service_observation");
        assert_eq!(retired["successful_session"], false);
    }
    #[test]
    fn stopped_observation_cannot_adopt_an_active_instance() {
        let f = test_fixture::Fixture::new();
        let (job, _) = spec(&f.m, f.r.clone().into(), false, false, false).unwrap();
        atomic_json(&job.lease, &job.report).unwrap();
        assert!(observe_stopped_leases(&f.m,
            Some("11111111-1111-1111-1111-111111111111")).is_err());
        assert!(!f.m.root.join("runtime/lease-generations").exists());
        assert!(job.lease.exists());
    }
    #[test]
    fn restart_requires_positive_prior_cleanup() {
        unsafe {
            libc::umask(0o077);
        }
        let outer = std::env::temp_dir().join(format!("ap12-leases-{}", random_id().unwrap()));
        let m = Manager {
            root: outer.join("managed"),
            publications: outer.join("vst3"),
        };
        private_dir(&m.root.join("runtime/results")).unwrap();
        private_dir(&m.root.join("runtime/leases")).unwrap();
        let report = m.root.join("runtime/results/one.json");
        let session = "01".repeat(16);
        let lease = m.root.join("runtime/leases").join(format!("{session}.json"));
        atomic_json(&lease, &report).unwrap();
        assert!(reconcile_leases(&m).unwrap());
        assert!(lease.exists());
        atomic_json(&report, &serde_json::json!({"cleanup_confirmed":false})).unwrap();
        assert!(reconcile_leases(&m).unwrap());
        atomic_json(&report, &serde_json::json!({"cleanup_confirmed":true})).unwrap();
        assert!(!reconcile_leases(&m).unwrap());
        assert!(!lease.exists());
        assert!(report.exists());
        atomic_json(&lease, &report).unwrap();
        atomic_json(
            &report,
            &serde_json::json!({"ownership_schema":1,"cleanup_confirmed":true}),
        )
        .unwrap();
        assert!(reconcile_leases(&m).unwrap()); // physical report alone cannot retire new transport
        let receipt = report.with_extension("ownership.json");
        atomic_json(&receipt,&serde_json::json!({"session":"wrong","cleanup_confirmed":true,"transport_retired":true})).unwrap();
        assert!(reconcile_leases(&m).unwrap());
        atomic_json(&receipt,&serde_json::json!({"session":session,"cleanup_confirmed":true,"transport_retired":true,"reporting_error":"disk refusal"})).unwrap();
        fs::remove_file(&report).unwrap();
        assert!(!reconcile_leases(&m).unwrap());
        assert!(!lease.exists());
        fs::remove_dir_all(outer).unwrap();
    }
}

mod dependency_cli;
