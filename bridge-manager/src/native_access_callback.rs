//! Opaque browser return transport. Never an operator request or launch authority.
use crate::*;
#[cfg(any(target_os = "linux", test))]
use std::io::{Read, Write};
#[cfg(any(target_os = "linux", test))]
use std::os::unix::net::UnixStream;
#[cfg(any(target_os = "linux", test))]
use std::time::Duration;
pub const LIMIT: usize = 2048;
/// Keep the URL opaque. Admit a single ASCII URI argument, never shell syntax.
pub fn valid_uri(value: &[u8]) -> bool {
    if value.len() <= 14 || value.len() > LIMIT || !value.starts_with(b"native-access:") {
        return false;
    }
    let mut i = 14;
    while i < value.len() {
        let c = value[i];
        if !(0x21..=0x7e).contains(&c) || matches!(c, b'"' | b'\'' | b'\\' | b'`' | b'<' | b'>') {
            return false;
        }
        if c == b'%' {
            if i + 2 >= value.len() {
                return false;
            }
            let hex = |c: u8| match c {
                b'0'..=b'9' => Some(c - b'0'),
                b'a'..=b'f' => Some(c - b'a' + 10),
                b'A'..=b'F' => Some(c - b'A' + 10),
                _ => None,
            };
            let (Some(a), Some(b)) = (hex(value[i + 1]), hex(value[i + 2])) else {
                return false;
            };
            if a * 16 + b < 0x20 || a * 16 + b == 0x7f {
                return false;
            }
            i += 2;
        }
        i += 1;
    }
    true
}
#[cfg(any(target_os = "linux", test))]
fn exchange(peer: &mut UnixStream, operation: &str, url: &[u8]) -> Result<()> {
    require(
        valid_hex(operation, 32) && valid_uri(url),
        "native_access_callback_invalid",
    )?;
    peer.set_read_timeout(Some(Duration::from_secs(25)))?;
    peer.set_write_timeout(Some(Duration::from_secs(3)))?;
    peer.write_all(operation.as_bytes())?;
    let mut ready = [0; 4];
    peer.read_exact(&mut ready)?;
    require(&ready == b"NAC1", "native_access_callback_not_ready")?;
    peer.write_all(&(url.len() as u32).to_le_bytes())?;
    peer.write_all(url)?;
    // The supervisor allows 60 seconds for the vendor secondary, five for
    // retirement and two for acknowledgement. Transport retains a small margin.
    peer.set_read_timeout(Some(Duration::from_secs(70)))?;
    let mut result = [0; 1];
    peer.read_exact(&mut result)?;
    require(result == [0], "native_access_callback_delivery_unconfirmed")
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Target {
    operation: String,
    unit: String,
    selector: Option<String>,
    binding: String,
}

fn assisted_target(m: &Manager, directory: &Path, verify_content: bool) -> Result<Option<Target>> {
    let result_path = directory.join("managed-session.json");
    if !result_path.exists() { return Ok(None); }
    let result: serde_json::Value = read_json(&result_path)?;
    if result["state"] != "running" { return Ok(None); }
    let job: vendor_application::Operation = read_json(&directory.join("operation.json"))?;
    let Some(managed) = &job.managed else { return Ok(None); };
    let app = &job.application;
    let selector = app.selector()?;
    require(valid_hex(&job.operation_id, 32)
        && directory == m.root.join("vendor-applications").join(&selector)
        && directory.canonicalize()? == directory
        && job.report == directory.join("operation-result.json")
        && job.mode == vendor_application::LaunchMode::Normal,
        "native_access_callback_operation_binding")?;
    let current: vendor_application::OperationResult = read_json(&job.report)?;
    require(current.operation_id.as_deref()==Some(job.operation_id.as_str()), "native_access_callback_result_changed")?;
    if current.retired() && current.dependency.as_ref().is_some_and(vendor_application::dependency_retired) { return Ok(None); }
    let mut selected: vendor_application::Application = read_json(&directory.join("application.json"))?;
    require(selected.environment.id == app.environment.id && selected.environment.root == app.environment.root,
        "native_access_callback_environment")?;
    selected.environment = read_json(&app.environment.root.join("environment.json"))?;
    require(selected == *app, "native_access_callback_selection_changed")?;
    if verify_content {app.verify_for_running_launch(&m.root)?;}
    let profile = vendor_profile::native_access()?;
    let capabilities = &managed.capabilities;
    // The session was launched with its own saved profile. A manager update
    // that changes the bundled profile's data (same id) must not cut off the
    // sign-in return of a session that was already running.
    require(capabilities.schema == 1 && capabilities.profile.id == profile.id
        && capabilities.application == selector && capabilities.profile.browser_scheme == "native-access",
        "native_access_callback_capability")?;
    // Callback transport does not admit a service or install anything. The daemon
    // may have been installed by this owned operation after its initial binding.
    // Application resources and its bundled installer must still be exact.
    if verify_content {capabilities.verify_callback_inputs(app, &m.root)?;}
    let software = crate::preparation::build::launch_software_record(m)?;
    require(managed.software == software && job.launcher == software.installer_launch,
        "native_access_callback_software_changed")?;
    for artifact in [&software.manager, &software.supervisor, &software.ownership,
        software.installer_launch.as_ref().ok_or("native_access_callback_adapter")?] { if verify_content {artifact.verify()?;} }
    require(result["operation"] == job.operation_id
        && result["application_identity"] == selector
        && result["effective"]["operation"] == job.operation_id
        && result["renderer"]["launch_binding"]["status"] == "bound"
        && result["dependency"]["ready_tested"] == true,
        "native_access_callback_not_live")?;
    let binding=hex(&Sha256::digest(serde_json::to_vec(&job)?));
    Ok(Some(Target { operation: job.operation_id, unit: format!("linux-vst-bridge-vendor-{selector}.service"),
        selector: Some(selector),binding }))
}

fn target(m: &Manager) -> Result<Target> {
    current_target(m,true)
}
fn current_target(m: &Manager, verify_content: bool) -> Result<Target> {
    let root = m.root.join("vendor-applications");
    let mut targets = vec![];
    if root.exists() {
        for (index, entry) in fs::read_dir(&root)?.enumerate() {
            require(index < 128, "native_access_callback_application_bound")?;
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.file_name().to_str().is_some_and(|s| valid_hex(s, 64)) {
                if let Some(found) = assisted_target(m, &entry.path(),verify_content)? { targets.push(found); }
            }
        }
    }
    if let Some(legacy) = legacy_target(m,verify_content)? { targets.push(legacy); }
    require(targets.len() == 1, "native_access_callback_no_unique_live_operation")?;
    Ok(targets.remove(0))
}

fn legacy_target(m: &Manager, verify_content: bool) -> Result<Option<Target>> {
    let c = renderer_session::current(m)?;
    let Some(op) = c["operation"].as_str() else { return Ok(None); };
    require(valid_hex(op, 32), "native_access_callback_operation")?;
    let result = renderer_session::result(m, op)?;
    if result["state"] != "running" { return Ok(None); }
    let d = renderer_session::operation_dir(m, op)?;
    let spec: serde_json::Value = read_json(&d.join("spec.json"))?;
    let sw = crate::preparation::build::launch_software_record(m)?;
    for artifact in [&sw.manager, &sw.supervisor, &sw.ownership,
        sw.installer_launch.as_ref().ok_or("native_access_callback_adapter")?] {if verify_content {artifact.verify()?;} }
    require(spec["software"] == serde_json::to_value(&sw)? && spec["operation"] == op
        && spec["kind"] == "renderer_application" && spec["application_identity"] == c["application"],
        "native_access_callback_generation")?;
    let app: renderer_application::Application = serde_json::from_value(spec["application"].clone())?;
    if verify_content {app.verify_for_running_launch(&m.root)?;}
    require(app.identity()? == c["application"], "native_access_callback_application")?;
    require(result["effective"]["operation"] == op && result["renderer"]["launch_binding"]["status"] == "bound",
        "native_access_callback_not_live")?;
    Ok(Some(Target { operation: op.into(), unit: renderer_session::unit(op)?, selector: None,
        binding:hex(&Sha256::digest(serde_json::to_vec(&spec)?)) }))
}
#[cfg(target_os = "linux")]
pub fn deliver(m: &Manager, url: &str) -> Result<()> {
    use std::os::unix::net::SocketAddr;
    use std::os::{fd::AsRawFd, linux::net::SocketAddrExt};
    require(valid_uri(url.as_bytes()), "native_access_callback_invalid")?;
    let selected = target(m)?;
    let op = &selected.operation;
    let uid = unsafe { libc::getuid() };
    let address = SocketAddr::from_abstract_name(format!("lvb-native-access-{uid}-{op}"))?;
    let mut peer = UnixStream::connect_addr(&address)?;
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of_val(&cred) as libc::socklen_t;
    require(
        unsafe {
            libc::getsockopt(
                peer.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut cred as *mut libc::ucred).cast(),
                &mut length,
            )
        } == 0
            && cred.uid == uid
            && cred.pid > 0,
        "native_access_callback_peer",
    )?;
    // The receiver must be a member of this exact application operation, not merely
    // another same-user socket. No Linux/Windows PID equivalence is involved.
    let mut bytes = Vec::new();
    fs::File::open(format!("/proc/{}/cgroup", cred.pid))?
        .take(16385)
        .read_to_end(&mut bytes)?;
    require(bytes.len() <= 16384, "native_access_callback_cgroup_extent")?;
    let suffix = format!("/{}", selected.unit);
    require(
        std::str::from_utf8(&bytes)?
            .lines()
            .any(|l| l.starts_with("0::") && l.ends_with(&suffix)),
        "native_access_callback_owner",
    )?;
    // Bulk file/runtime verification happened before connect. The receiver's
    // five-second idle window only contains bounded exact-record/owner checks.
    require(current_target(m,false)? == selected, "native_access_callback_operation_changed")?;
    exchange(&mut peer, op, url.as_bytes())
}
#[cfg(not(target_os = "linux"))]
pub fn deliver(_m: &Manager, _url: &str) -> Result<()> {
    Err("native_access_callback_linux_only".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn managed_application(f: &crate::test_fixture::Fixture, mut app: vendor_application::Application, name: &str)
        -> (PathBuf, vendor_application::Operation) {
        let folder = app.environment.root.join("compatdata/pfx/drive_c").join(name);
        private_dir(&folder).unwrap();
        let executable = folder.join("Native Access.exe");
        let mut pe = [0_u8; 1024];
        pe[..2].copy_from_slice(b"MZ"); pe[60..64].copy_from_slice(&128_u32.to_le_bytes());
        pe[128..132].copy_from_slice(b"PE\0\0"); pe[132..134].copy_from_slice(&0x8664_u16.to_le_bytes()); pe[150] = 2;
        fs::write(&executable, pe).unwrap();
        app.executable = Artifact { path: executable.clone(), sha256: digest(&executable).unwrap() };
        let profile = vendor_profile::native_access().unwrap();
        for name in &profile.resources {
            let path = folder.join(name); private_dir(path.parent().unwrap()).unwrap(); fs::write(path, b"first-party resource").unwrap();
        }
        let bundled = folder.join(&profile.dependency.installer_directory);
        private_dir(&bundled).unwrap(); fs::write(bundled.join("NTKDaemon new-version Setup PC.exe"), pe).unwrap();
        let source = f.m.root.join("software/source"); fs::write(&source, b"first-party source").unwrap();
        let source = Artifact { path: source.clone(), sha256: digest(&source).unwrap() };
        let software = catalogue::Software { manager: f.r.host.clone(), supervisor: f.r.host.clone(), ownership: f.r.host.clone(),
            host: f.r.host.clone(), installer_launch: Some(f.r.host.clone()), source_manifest: source.clone(), source_sha256: source.sha256,
            preparation_kit: None, operator_frontend: None, native_catalogue: None };
        atomic_json(&f.m.root.join("software.json"), &software).unwrap();
        let capability = vendor_profile::inspect(&app, &f.m.root).unwrap().capabilities.unwrap();
        let directory = f.m.root.join("vendor-applications").join(app.selector().unwrap()); private_dir(&directory).unwrap();
        atomic_json(&directory.join("application.json"), &app).unwrap();
        let job = vendor_application::Operation { application: app, mode: vendor_application::LaunchMode::Normal,
            operation_id: random_id().unwrap(), report: directory.join("operation-result.json"), launcher: software.installer_launch.clone(),
            managed: Some(vendor_profile::ManagedLaunch::new(capability, software).unwrap()), capability_notice: None };
        atomic_json(&directory.join("operation.json"), &job).unwrap();
        atomic_json(&job.report,&serde_json::json!({"schema":3,"operation_id":job.operation_id,"state":"running",
            "launcher_exit":null,"owned_live":1,"cleanup_confirmed":false,"error":null,"discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        atomic_json(&directory.join("managed-session.json"), &serde_json::json!({"schema":1,"state":"running",
            "operation":job.operation_id,"application_identity":job.application.selector().unwrap(),
            "effective":{"operation":job.operation_id},"renderer":{"launch_binding":{"status":"bound"}},"dependency":{"ready_tested":true}})).unwrap();
        (directory, job)
    }

    #[test]
    fn browser_return_selects_owned_generic_operation_and_refuses_stale_or_ambiguous_targets() {
        let (f, app) = vendor_application::tests::selected_application();
        let (directory, job) = managed_application(&f, app.clone(), "First application");
        let expected = target(&f.m).unwrap();
        assert_eq!(current_target(&f.m,false).unwrap(),expected);
        let mut changed=job.clone();changed.capability_notice=Some("changed launch binding".into());
        atomic_json(&directory.join("operation.json"),&changed).unwrap();
        assert_ne!(current_target(&f.m,false).unwrap(),expected);
        atomic_json(&directory.join("operation.json"),&job).unwrap();
        assert_eq!(expected.operation, job.operation_id);
        assert_eq!(expected.unit, format!("linux-vst-bridge-vendor-{}.service", job.application.selector().unwrap()));
        assert!(!renderer_session::directory(&f.m).exists());
        // A daemon created by this operation does not rewrite its application identity.
        let daemon = job.application.environment.root.join("compatdata/pfx/drive_c").join(&job.managed.as_ref().unwrap().capabilities.profile.dependency.daemon);
        private_dir(daemon.parent().unwrap()).unwrap(); fs::copy(&job.application.executable.path, &daemon).unwrap();
        assert_eq!(target(&f.m).unwrap(), expected);
        let resource = job.managed.as_ref().unwrap().capabilities.resources["resources/app.asar"].artifact.path.clone();
        let before = fs::read(&resource).unwrap(); fs::write(&resource, b"changed resource").unwrap();
        assert!(target(&f.m).is_err()); fs::write(&resource, before).unwrap();
        let mut result: serde_json::Value = read_json(&directory.join("managed-session.json")).unwrap();
        result["state"] = "completed".into(); atomic_json(&directory.join("managed-session.json"), &result).unwrap();
        assert!(target(&f.m).is_err());
        result["state"] = "running".into(); atomic_json(&directory.join("managed-session.json"), &result).unwrap();
        managed_application(&f, app, "Second application");
        assert!(target(&f.m).unwrap_err().to_string().contains("no_unique_live_operation"));
        // Generic recovery is authoritative even when the old renderer detail
        // was left marked running by its crashed writer.
        atomic_json(&job.report,&serde_json::json!({"schema":3,"operation_id":job.operation_id,"state":"failed",
            "launcher_exit":null,"owned_live":0,"cleanup_confirmed":true,"error":"interrupted",
            "discarded_diagnostic_bytes":0,"account_posture":"unknown",
            "dependency":{"process_cleanup_confirmed":true,"dependency_cleanup_disposition":"not_started","service_actions_started":false}})).unwrap();
        assert_ne!(target(&f.m).unwrap().operation,job.operation_id);
    }
    #[test]
    fn a_browser_return_finds_its_session_while_the_supervisor_rewrites_its_records() {
        // The supervisor replaces its session record twice a second, and its
        // report as the session progresses. A return that lands during a
        // replacement still finds its session (#524).
        let (f, app) = vendor_application::tests::selected_application();
        let (directory, job) = managed_application(&f, app, "Rewritten session");
        let expected = target(&f.m).unwrap();
        let session: serde_json::Value = read_json(&directory.join("managed-session.json")).unwrap();
        let report: serde_json::Value = read_json(&job.report).unwrap();
        use std::sync::atomic::{AtomicU64, Ordering};
        // Far more often than a supervisor does, while returns are read back to back.
        let replaced = AtomicU64::new(0);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for heartbeat in 1..=200_u64 {
                    let mut session = session.clone();
                    session["heartbeat"] = heartbeat.into();
                    atomic_json(&directory.join("managed-session.json"), &session).unwrap();
                    let mut report = report.clone();
                    report["discarded_diagnostic_bytes"] = heartbeat.into();
                    atomic_json(&job.report, &report).unwrap();
                    replaced.store(heartbeat, Ordering::Relaxed);
                    std::thread::sleep(Duration::from_millis(1));
                }
            });
            while replaced.load(Ordering::Relaxed) < 200 {
                assert_eq!(current_target(&f.m, false).unwrap(), expected);
            }
        });
        assert_eq!(target(&f.m).unwrap(), expected);
    }
    #[test]
    fn opaque_uri_has_closed_extent_and_no_argument_injection() {
        for good in [
            "native-access://auth?code=fixture&state=test",
            "native-access:fixture%2Btest",
        ] {
            assert!(valid_uri(good.as_bytes()));
        }
        for bad in [
            "https://example.test/",
            "native-access:",
            "native-access:x --no-sandbox",
            "native-access:\"x",
            "native-access:x\\",
            "native-access:x\n",
            "native-access:%00",
            "native-access:%7f",
            "native-access:%a",
            "native-access:%xz",
        ] {
            assert!(!valid_uri(bad.as_bytes()), "{bad}");
        }
        assert!(!valid_uri(
            format!("native-access:{}", "a".repeat(LIMIT)).as_bytes()
        ));
    }
    #[test]
    fn only_live_owner_acknowledgment_allows_payload_transfer() {
        for accepted in [false, true] {
            let (mut a, mut b) = UnixStream::pair().unwrap();
            let t = std::thread::spawn(move || {
                let mut operation = [0; 32];
                b.read_exact(&mut operation).unwrap();
                assert_eq!(&operation, b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
                b.write_all(if accepted { b"NAC1" } else { b"NOPE" })
                    .unwrap();
                if accepted {
                    let mut n = [0; 4];
                    b.read_exact(&mut n).unwrap();
                    let mut value = vec![0; u32::from_le_bytes(n) as usize];
                    b.read_exact(&mut value).unwrap();
                    assert_eq!(value, b"native-access:fixture");
                    b.write_all(&[0]).unwrap();
                }
            });
            assert_eq!(
                exchange(&mut a, &"a".repeat(32), b"native-access:fixture").is_ok(),
                accepted
            );
            t.join().unwrap();
        }
    }
}
