use linux_vst_bridge::*;
mod managed_cli;
#[cfg(test)]
mod test_fixture;
mod transport_storage;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{
    fs,
    io::{Read, Write},
    os::fd::{FromRawFd, IntoRawFd},
    os::unix::{
        fs::{OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Software {
    manager: Artifact,
    supervisor: Artifact,
    ownership: Artifact,
    host: Artifact,
    source_manifest: Artifact,
    source_sha256: String,
    #[serde(default)]
    native_catalogue: Option<Artifact>,
}
#[derive(Serialize, Deserialize)]
struct SessionSpec {
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
    #[serde(default)]
    transport: Option<transport_storage::MemoryTransport>,
}
// Unexposed admission owns only a reservation. Once the native binding is
// exposed, only positive supervisor retirement can release that ownership.
struct PendingAdmission(Option<PathBuf>, Option<Arc<AtomicBool>>);
impl Drop for PendingAdmission {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            if let Some(blocked) = &self.1 {
                // Native binding was exposed. Without a supervisor receipt its
                // lifetime is unresolved; keep the lease and refuse admission.
                blocked.store(true, Ordering::Release);
            } else {
                let _ = fs::remove_file(path);
            }
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct ClassSelection {
    class_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
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
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionRequest {
    environment_id: String,
    module: Artifact,
    class_id: String,
    compatibility: Compatibility,
}
fn software(m: &Manager) -> Result<Software> {
    let s: Software = read_json(&m.root.join("software.json"))?;
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
    if let Some(a) = &s.native_catalogue {
        a.verify()?;
    }
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
    let _lock = m.lock("setup.lock")?;
    let _registry = m.lock("registry.lock")?;
    // A stopped prior service may leave a positively retired keeper lease.
    // Use the existing ownership receipt rule before the inactivity check;
    // never erase an unresolved lease merely to make setup succeed.
    if package.is_some() {
        let _unresolved = reconcile_leases(m)?;
    }
    m.require_inactive(None)?;
    let me = std::env::current_exe()?;
    let (catalogue, files, source) = if let Some(package) = package {
        let profiles = profiles::installed_profiles()?;
        let catalogue = if m.registry()?.classes.is_empty() {
            None
        } else {
            Some(catalogue::adoption(m, &profiles)?)
        };
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
            read_json::<String>(&package.join("host-source.json"))?,
        )
    } else {
        let accepted = acceptance::prepare(m)?;
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
    require(
        valid_hex(&source, 64) && digest(&files[4].1)? == source,
        "host source manifest hash differs",
    )?;
    let mut identity = String::new();
    for (_, p) in &files {
        identity.push_str(&digest(p)?);
    }
    if let Some(c) = &catalogue {
        identity.push_str(&hex(&sha2::Sha256::digest(serde_json::to_vec(c)?)));
    }
    if package.is_none() {
        identity.push_str(&hex(&sha2::Sha256::digest(acceptance::REVIEW)));
    }
    let id = hex(&sha2::Sha256::digest(identity.as_bytes()));
    let dest = m.root.join("software").join(&id);
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
                fs::Permissions::from_mode(if *name == "linux-vst-bridge" {
                    0o500
                } else {
                    0o400
                }),
            )?;
            fs::File::open(to)?.sync_all()?;
        }
        if let Some(mut c) = catalogue.clone() {
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
            fs::write(&path, acceptance::REVIEW)?;
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
            fs::read(dest.join("acceptance-review.json"))? == acceptance::REVIEW,
            "acceptance_review_identity",
        )?;
    }
    if let Some(mut expected) = catalogue.clone() {
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
    let previous = read_json::<Software>(&m.root.join("software.json")).ok();
    publication::install_command(
        &home.join(".local/bin/linux-vst-bridge"),
        &installed.manager.path,
        previous.as_ref().map(|s| s.manager.path.as_path()),
    )?;
    atomic_json(&m.root.join("software.json"), &installed)?;
    let units = home.join(".config/systemd/user");
    fs::create_dir_all(&units)?;
    let service=format!("[Unit]\nDescription=Linux VST Bridge registered host\nAfter=graphical-session.target\n\n[Service]\nType=simple\nExecStart={} serve\nUMask=0077\nRestart=on-failure\nRestartSec=2\nKillMode=control-group\nTimeoutStopSec=30\n\n[Install]\nWantedBy=default.target\n",systemd(installed.manager.path.to_str().ok_or("executable path encoding")?));
    let unit = units.join("linux-vst-bridge.service");
    if unit.exists() {
        let old = fs::read_to_string(&unit)?;
        require(
            old.starts_with("[Unit]\nDescription=Linux VST Bridge registered host\n"),
            "service name belongs to another owner",
        )?;
    }
    let temp = units.join(".linux-vst-bridge.service.tmp");
    {
        let mut f = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temp)?;
        f.write_all(service.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(temp, unit)?;
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
    let s = SessionSpec {
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
        transport: None,
    };
    let path = directory.join("owner.json");
    atomic_json(&path, &s)?;
    Ok((s, path))
}
fn spawn(s: &Software, path: &Path, peer: Option<UnixStream>) -> Result<Child> {
    s.supervisor.verify()?;
    s.ownership.verify()?;
    let bound_peer = peer.is_some();
    let stdin = if let Some(p) = peer {
        unsafe { Stdio::from_raw_fd(p.into_raw_fd()) }
    } else {
        Stdio::null()
    };
    let job: SessionSpec = read_json(path)?;
    atomic_json(&job.lease, &job.report)?;
    let child = Command::new("/usr/bin/python3")
        .arg(&s.supervisor.path)
        .arg(path)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn();
    if child.is_err() && !bound_peer {
        fs::remove_file(&job.lease)?;
    }
    Ok(child?)
}
fn reconcile_leases(m: &Manager) -> Result<bool> {
    let directory = m.root.join("runtime/leases");
    private_dir(&directory)?;
    let mut unconfirmed = false;
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let report: PathBuf = read_json(&path)?;
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
            _ => unconfirmed = true,
        }
    }
    Ok(unconfirmed)
}
fn ensure_keeper(
    m: &Manager,
    s: &Software,
    r: &HostBinding,
    keepers: &Mutex<Vec<(String, Child)>>,
) -> Result<()> {
    let mut active = keepers
        .lock()
        .map_err(|_| "environment ownership lock poisoned")?;
    if let Some((_, child)) = active.iter_mut().find(|(id, _)| *id == r.environment.id) {
        return require(
            child.try_wait()?.is_none(),
            "environment owner exited; restart service after closing devices",
        );
    }
    // Shared environment infrastructure belongs to installed product software,
    // independently of the exact per-class host retained by a rollback.
    let mut keeper_binding = r.clone();
    keeper_binding.host = s.host.clone();
    keeper_binding.host_source_sha256 = s.source_sha256.clone();
    let (mut job, path) = spec(m, keeper_binding, true, false, true)?;
    job.shared_runtime = true;
    atomic_json(&path, &job)?;
    let child = spawn(s, &path, None)?;
    // Retain ownership even when readiness or its report fails.
    active.push((r.environment.id.clone(), child));
    let child = &mut active.last_mut().unwrap().1;
    let deadline = Instant::now() + Duration::from_secs(60);
    while !job.report.exists() {
        require(child.try_wait()?.is_none(), "environment startup failed")?;
        if Instant::now() >= deadline {
            // The supervisor owns descendants; never SIGKILL it and abandon them.
            unsafe {
                libc::kill(child.id() as i32, libc::SIGTERM);
            }
            return Err("environment startup deadline".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let status: serde_json::Value = read_json(&job.report)?;
    require(status["ready"] == true, "environment not ready")?;
    Ok(())
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
    // A service restart cannot turn missing cleanup into a fresh admission.
    // Clean reports retire their leases; uncertain ones remain inspectable.
    let blocked = Arc::new(AtomicBool::new(reconcile_leases(&manager)?));
    let keepers = Arc::new(Mutex::new(Vec::new()));
    let mut threads: Vec<std::thread::JoinHandle<()>> = Vec::new();
    for peer in listener.incoming() {
        let mut peer = peer?;
        threads.retain(|t| !t.is_finished());
        if threads.len() >= 8 || blocked.load(Ordering::Acquire) {
            continue;
        }
        let m = manager.clone();
        let s = s.clone();
        let blocked = blocked.clone();
        let keepers = keepers.clone();
        threads.push(std::thread::spawn(move || {
            let outcome = (|| -> Result<()> {
                peer.set_read_timeout(Some(Duration::from_secs(5)))?;
                let mut greeting = [0; 53];
                peer.read_exact(&mut greeting[..5])?;
                if &greeting[..5] == b"LVI1\n" || &greeting[..5] == b"LVQ1\n" {
                    let mut size=[0;4];peer.read_exact(&mut size)?;
                    let size=u32::from_le_bytes(size) as usize;require(size<=65536,"inspection_request_bound")?;
                    let mut bytes=vec![0;size];peer.read_exact(&mut bytes)?;
                    let _admission=m.lock("registry.lock")?;
                    m.require_inactive(None)?;
                    let request=serde_json::from_slice(&bytes)?;
                    let r=if &greeting[..5]==b"LVQ1\n" {qualification_binding(&m,request)?} else {inspection_binding(&m,request)?};
                    ensure_keeper(&m,&s,&r,&keepers)?;
                    let (mut job,path)=spec(&m,r,true,false,false)?;
                    let mut pending=PendingAdmission(Some(job.lease.clone()), None);
                    job.shared_inspection=true;atomic_json(&path,&job)?;
                    let mut child=spawn(&s,&path,None)?;
                    pending.0=None;
                    let status=child.wait()?;
                    let mut disposition=String::new();if let Some(stdout)=child.stdout.take(){stdout.take(128).read_to_string(&mut disposition)?;}
                    if !status.success()||disposition!=format!("LVO1 {} retired\n",job.session){blocked.store(true,Ordering::Release);return Err("inspection_cleanup_unconfirmed".into());}
                    fs::remove_file(&job.lease)?;
                    let reply=serde_json::to_vec(&job.report)?;
                    require(reply.len()<=4096,"inspection_reply_bound")?;
                    peer.write_all(&(reply.len() as u32).to_le_bytes())?;peer.write_all(&reply)?;
                    return Ok(());
                }
                if &greeting[..5]==b"LVA1\n" {
                    let mut size=[0;4];peer.read_exact(&mut size)?;
                    let size=u32::from_le_bytes(size) as usize;require(size<=65536,"vendor access request bound")?;
                    let mut bytes=vec![0;size];peer.read_exact(&mut bytes)?;
                    let r=inspection_binding(&m,serde_json::from_slice(&bytes)?)?;
                    ensure_keeper(&m,&s,&r,&keepers)?;
                    let (mut job,path)=spec(&m,r,false,false,false)?;
                    job.vendor_access=true;atomic_json(&path,&job)?;
                    let mut child=spawn(&s,&path,None)?;
                    let _=peer.write_all(format!("Vendor access {}: editor only; no DAW audio or project recall. Close its window to finish.\n",job.session).as_bytes());
                    let status=child.wait()?;
                    let mut disposition=String::new();if let Some(stdout)=child.stdout.take(){stdout.take(128).read_to_string(&mut disposition)?;}
                    if !status.success()||disposition!=format!("LVO1 {} retired\n",job.session){blocked.store(true,Ordering::Release);return Err("vendor access cleanup unconfirmed".into());}
                    fs::remove_file(&job.lease)?;peer.write_all(b"Vendor access retired.\n")?;return Ok(());
                }
                peer.read_exact(&mut greeting[5..])?;
                let version2 = &greeting[..5] == b"LVB2\n";
                require(version2 || &greeting[..5] == b"LVB1\n", "registration protocol mismatch")?;
                let (r, performance, job, path, mut admission, mut storage) = {
                    let _admission = m.lock("registry.lock")?;
                    let registration = m.resolve(&greeting[5..])?;
                    m.verify_served_host(&registration, &s.host, &s.source_sha256, &profiles::installed_profiles()?)?;
                    let r: HostBinding = registration.into();
                    let performance = m.performance(&r.metadata.class_id)?;
                    require(version2 || performance.added_frames == 512,
                        "selected delay requires a version-2 native binding")?;
                    // Validate the DAW's view of the fixed memory root before
                    // creating or exposing a session. A Flatpak's /dev/shm is
                    // not assumed to be the host's shared memory mount.
                    transport_storage::visible_to_peer(&peer, &transport_storage::root())?;
                    let (mut job, path) = spec(&m, r.clone(), false, false, false)?;
                    let storage = transport_storage::PendingTransport::new(&job.session)?;
                    job.directory = storage.directory.clone();
                    job.transport = Some(storage.identity.clone());
                    job.shared_runtime = true;
                    atomic_json(&path, &job)?;
                    atomic_json(&job.lease, &job.report)?;
                    let admission = PendingAdmission(Some(job.lease.clone()), None);
                    (r, performance, job, path, admission, storage)
                };
                // Deliver the private binding inside the native greeting deadline.
                // Cold Wine startup then uses the existing bounded transport accept.
                let reply = if version2 {
                    format!("{}\n{}\n{}", job.session, performance.added_frames, job.directory.display())
                } else {
                    format!("{}\n{}", job.session, job.directory.display())
                };
                require(reply.len() <= 1024, "session binding size")?;
                peer.set_write_timeout(Some(Duration::from_secs(5)))?;
                admission.1 = Some(blocked.clone());
                storage.expose();
                peer.write_all(&(reply.len() as u16).to_le_bytes())?;
                peer.write_all(reply.as_bytes())?;
                ensure_keeper(&m, &s, &r, &keepers)?;
                let mut child = spawn(&s, &path, Some(peer))?;
                admission.0 = None;
                let status = child.wait()?;
                let mut disposition=String::new();
                if let Some(stdout)=child.stdout.take(){stdout.take(128).read_to_string(&mut disposition)?;}
                if !status.success() || disposition!=format!("LVO1 {} retired\n",job.session) {
                    blocked.store(true, Ordering::Release);
                    return Err(
                        "instance owner did not confirm cleanup; new admissions blocked".into(),
                    );
                }
                fs::remove_file(&job.lease)?;
                Ok(())
            })();
            if let Err(e) = outcome {
                eprintln!("Bridge instance: {e}");
            }
        }));
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
    let r = HostBinding {
        metadata: ClassSelection {
            class_id: request.class_id,
        },
        environment,
        module: request.module,
        host: sw.host.clone(),
        host_source_sha256: sw.source_sha256.clone(),
        compatibility: request.compatibility,
    };
    Ok(r)
}
// Engineering inspection has a distinct admission selector. Its host can only
// come from the sealed installed candidate roster, never from request data.
fn qualification_binding(m: &Manager, request: InspectionRequest) -> Result<HostBinding> {
    let candidates = qualification::installed(m)?;
    let r = inspection_binding(m, request)?;
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
    m.check_editor_qualification_parent(&c.profile, &registration)?;
    Ok(registration.into())
}
fn inspect(m: &Manager, path: &Path) -> Result<()> {
    let r = inspection_binding(m, read_json(path)?)?;
    let sw = software(m)?;
    let (job, path) = spec(m, r, true, false, false)?;
    let status = spawn(&sw, &path, None)?.wait()?;
    println!("{}", job.report.display());
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
    let m = Manager::installed()?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str){
  Some("setup") if args.len()==2=>setup(&m,Some(Path::new(&args[1]))),
  Some("accept-editor") if args.len()==1=>managed_cli::run_acceptance(&m),
  Some("managed")=>managed_cli::run(&m,&args[1..]),
  Some("qualify-editor")=>managed_cli::run_qualification(&m,&args[1..]),
  Some("environment-create") if args.len()==2=>environment_create(&m,Path::new(&args[1])),
  Some("environment-import") if args.len()==2=>environment_import(&m,Path::new(&args[1])),
  Some("install") if args.len()==4=>install(&m,&args[1],Path::new(&args[2]),&args[3]),
  Some("register") if args.len()==2=>m.register(read_json(Path::new(&args[1]))?),
  Some("unpublish") if args.len()==2=>m.unpublish(&args[1]),
  Some("set-delay") if args.len()==3=>m.select_delay(&args[1],args[2].parse()?),
  Some("serve") if args.len()==1=>serve(m),
  Some("status") if args.len()==1=>status(&m),
  Some("reconcile") if args.len()==1=>m.reconcile(),
  Some("inspect") if args.len()==2=>inspect(&m,Path::new(&args[1])),
  Some("vendor-editor") if args.len()==2=>vendor_editor(&m,Path::new(&args[1])),
  _=>Err("Usage: linux-vst-bridge setup PACKAGE | environment-create RUNNER.json | environment-import ENVIRONMENT.json | install ENV_ID INSTALLER SHA256 | inspect INSPECTION.json | vendor-editor INSPECTION.json | register REGISTRATION.json | status | set-delay CLASS_ID FRAMES | reconcile | unpublish CLASS_ID | serve".into())
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
            "added_frames":performance.map(|p|p.added_frames)})
    }).collect();
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exposed_binding_cannot_erase_an_unresolved_retirement() {
        let f = test_fixture::Fixture::new();
        let lease = f.outer.join("lease.json");
        fs::write(&lease, b"reservation").unwrap();
        drop(PendingAdmission(Some(lease.clone()), None));
        assert!(!lease.exists());
        fs::write(&lease, b"bound native owner").unwrap();
        let blocked = Arc::new(AtomicBool::new(false));
        drop(PendingAdmission(Some(lease.clone()), Some(blocked.clone())));
        assert!(lease.exists());
        assert!(blocked.load(Ordering::Acquire));
        let blocked = Arc::new(AtomicBool::new(false));
        drop(PendingAdmission(None, Some(blocked.clone())));
        assert!(!blocked.load(Ordering::Acquire));
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
        let lease = m.root.join("runtime/leases/one.json");
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
        atomic_json(&receipt,&serde_json::json!({"session":"one","cleanup_confirmed":true,"transport_retired":true,"reporting_error":"disk refusal"})).unwrap();
        fs::remove_file(&report).unwrap();
        assert!(!reconcile_leases(&m).unwrap());
        assert!(!lease.exists());
        fs::remove_dir_all(outer).unwrap();
    }
}
