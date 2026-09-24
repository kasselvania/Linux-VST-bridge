//! WD0: one typed Windows DAW workspace adjacent to, never inside, DSP admission.
//! The immutable executable/runtime closure and mutable FL data have distinct owners.
use super::*;
use serde_json::{json, Value};
use std::os::unix::fs::{FileTypeExt, MetadataExt};

const APP: &str = "fl-studio";
const STANDARD_RUNNER: &str = "proton-11.0-2c-25118279-slr4-4.0.20260805.254769";
const SESSION_SECONDS: u64 = 30;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ApplicationId {
    FlStudio,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum State {
    Imported,
    Installing,
    NeedsUserAction,
    Installed,
    Ready,
    Starting,
    Running,
    Stopping,
    Failed,
    CleanupUnconfirmed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InstalledApplication {
    executable: Artifact,
    installer_advertised_release: String,
    observed_file_version: Option<String>,
    resource_root: PathBuf,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AudioBackend {
    PulseAudio,
    WineAsio,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct AudioObservation {
    backend: AudioBackend,
    sample_rate: u32,
    buffer_frames: u32,
    linux_endpoint: String,
    operator_audible: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Workspace {
    schema: u32,
    id: String,
    revision: u64,
    application: ApplicationId,
    installer: String,
    installer_release: String,
    environment: Environment,
    projects: PathBuf,
    exports: PathBuf,
    preferences: PathBuf,
    state: State,
    installation_operation: Option<String>,
    installed: Option<InstalledApplication>,
    audio: Option<AudioObservation>,
    session_operation: Option<String>,
    first_failure: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Tooling {
    schema: u32,
    generation: String,
    source_head: String,
    source_tree: String,
    manager: Artifact,
    supervisor: Artifact,
    ownership: Artifact,
    importer: Artifact,
}

fn base(m: &Manager) -> PathBuf {
    m.root.join("daw-workspaces").join(APP)
}

fn record(m: &Manager) -> PathBuf {
    base(m).join("workspace.json")
}

fn private_exact(path: &Path) -> Result<()> {
    private_dir(path)?;
    verify_private_exact(path)
}

fn verify_private_exact(path: &Path) -> Result<()> {
    let md = fs::symlink_metadata(path)?;
    require(
        md.is_dir()
            && !md.file_type().is_symlink()
            && md.uid() == unsafe { libc::getuid() }
            && md.mode() & 0o077 == 0
            && md.nlink() >= 2,
        "workspace_directory_type",
    )?;
    require(path.canonicalize()? == path, "workspace_directory_alias")
}

fn owned_vendor_dir(path: &Path) -> Result<()> {
    let md = fs::symlink_metadata(path)?;
    require(
        md.is_dir()
            && !md.file_type().is_symlink()
            && md.uid() == unsafe { libc::getuid() }
            && path.canonicalize()? == path,
        "daw_workspace_vendor_directory",
    )
}

fn release_syntax(s: &str) -> bool {
    (3..=32).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && s.bytes().any(|b| b.is_ascii_digit())
}

fn audio_syntax(a: &AudioObservation) -> bool {
    (8_000..=192_000).contains(&a.sample_rate)
        && (16..=8_192).contains(&a.buffer_frames)
        && !a.linux_endpoint.is_empty()
        && a.linux_endpoint.len() <= 128
        && a.linux_endpoint
            .bytes()
            .all(|b| b.is_ascii_graphic() || b == b' ')
}

fn load(m: &Manager) -> Result<Workspace> {
    let w: Workspace = read_json(&record(m))?;
    let root = base(m);
    require(
        w.schema == 1
            && valid_hex(&w.id, 32)
            && w.revision >= 1
            && w.application == ApplicationId::FlStudio
            && valid_hex(&w.installer, 64)
            && release_syntax(&w.installer_release)
            && w.environment.id == w.id
            && w.environment.revision == 1
            && w.environment.root == root.join("environment")
            && w.environment.runner.id == STANDARD_RUNNER
            && w.environment.runner.policy.is_none()
            && w.projects == root.join("projects")
            && w.exports == root.join("exports")
            && w.preferences == root.join("preferences")
            && w.installation_operation
                .as_ref()
                .is_none_or(|v| valid_hex(v, 32))
            && w.session_operation
                .as_ref()
                .is_none_or(|v| valid_hex(v, 32)),
        "daw_workspace_identity",
    )?;
    for dir in [
        &m.root.join("daw-workspaces"),
        &root,
        &w.environment.root,
        &w.projects,
        &w.exports,
        &w.preferences,
    ] {
        verify_private_exact(dir)?;
    }
    require(
        read_json::<Environment>(&w.environment.root.join("environment.json"))? == w.environment,
        "daw_workspace_environment_changed",
    )?;
    if let Some(app) = &w.installed {
        let image_line = w
            .environment
            .root
            .join("compatdata/pfx/drive_c/Program Files/Image-Line");
        require(
            app.resource_root.starts_with(&image_line)
                && app.executable.path.parent() == Some(app.resource_root.as_path())
                && release_syntax(&app.installer_advertised_release)
                && app
                    .observed_file_version
                    .as_deref()
                    .is_none_or(release_syntax),
            "daw_workspace_application_binding",
        )?;
        owned_vendor_dir(&app.resource_root)?;
    }
    if let Some(a) = &w.audio {
        require(audio_syntax(a), "daw_workspace_audio_observation")?;
    }
    Ok(w)
}

fn save(m: &Manager, w: &mut Workspace) -> Result<()> {
    w.revision = w
        .revision
        .checked_add(1)
        .ok_or("daw_workspace_revision_overflow")?;
    atomic_json(&record(m), w)
}

fn unit(op: &str, install: bool) -> Result<String> {
    require(valid_hex(op, 32), "daw_workspace_operation_identity")?;
    Ok(format!(
        "linux-vst-bridge-{}-{op}",
        if install { "installer" } else { "daw-fl" }
    ))
}

fn unit_active(name: &str) -> Result<bool> {
    let output = Command::new("systemctl")
        .args(["--user", "show", name, "-p", "ActiveState", "--value"])
        .output()?;
    require(output.status.success(), "daw_workspace_unit_readback")?;
    Ok(std::str::from_utf8(&output.stdout)?.trim() == "active")
}

fn result_path(m: &Manager, op: &str, install: bool) -> PathBuf {
    base(m)
        .join(if install { "installations" } else { "sessions" })
        .join(op)
        .join("result.json")
}

fn result(m: &Manager, op: &str, install: bool) -> Result<Option<Value>> {
    let path = result_path(m, op, install);
    if !path.try_exists()? {
        return Ok(None);
    }
    let v: Value = read_json(&path)?;
    require(
        v["operation"] == op || (!install && v["operation_id"] == op),
        "daw_workspace_result_operation",
    )?;
    Ok(Some(v))
}

fn retired(v: &Value) -> bool {
    v["cleanup_confirmed"] == true
        && v["owned_live"] == 0
        && matches!(
            v["state"].as_str(),
            Some("completed" | "failed" | "cancelled")
        )
}

fn selected_runner(m: &Manager) -> Result<Runner> {
    let mut selected = None;
    for (_, runner) in onboarding::runners(m)? {
        if runner.id == STANDARD_RUNNER && runner.policy.is_none() {
            require(selected.is_none(), "daw_workspace_runner_ambiguous")?;
            selected = Some(runner);
        }
    }
    selected.ok_or("daw_workspace_standard_runner_absent".into())
}

fn create(m: &Manager, installer: &str, release: &str) -> Result<()> {
    require(
        valid_hex(installer, 64) && release_syntax(release),
        "daw_workspace_import_identity",
    )?;
    let imported = installer_import::load(m, installer)?;
    require(
        imported.format == "pe_executable",
        "daw_workspace_requires_pe_installer",
    )?;
    let runner = selected_runner(m)?;
    let w = create_exact(m, installer, release, runner)?;
    println!("{}", serde_json::to_string(&w)?);
    Ok(())
}

fn create_exact(m: &Manager, installer: &str, release: &str, runner: Runner) -> Result<Workspace> {
    require(
        valid_hex(installer, 64)
            && release_syntax(release)
            && runner.id == STANDARD_RUNNER
            && runner.policy.is_none(),
        "daw_workspace_creation_authority",
    )?;
    runner.verify()?;
    let _guard = m.lock("daw-workspace.lock")?;
    require(!record(m).try_exists()?, "daw_workspace_already_exists")?;
    let root = base(m);
    for dir in [
        m.root.join("daw-workspaces"),
        root.clone(),
        root.join("environment"),
        root.join("projects"),
        root.join("exports"),
        root.join("preferences"),
        root.join("installations"),
        root.join("sessions"),
    ] {
        private_exact(&dir)?;
    }
    let env_root = root.join("environment");
    for name in [
        "compatdata",
        "runtime-var",
        "host-cache",
        "host-config",
        "host-data",
        "host-tmp",
        "client",
        "home",
    ] {
        private_exact(&env_root.join(name))?;
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(env_root.join("operation.lock"))?
        .sync_all()?;
    let id = random_id()?;
    let environment = Environment {
        id: id.clone(),
        root: env_root,
        runner,
        revision: 1,
    };
    atomic_json(&environment.root.join("environment.json"), &environment)?;
    let w = Workspace {
        schema: 1,
        id,
        revision: 1,
        application: ApplicationId::FlStudio,
        installer: installer.into(),
        installer_release: release.into(),
        environment,
        projects: root.join("projects"),
        exports: root.join("exports"),
        preferences: root.join("preferences"),
        state: State::Imported,
        installation_operation: None,
        installed: None,
        session_operation: None,
        audio: None,
        first_failure: None,
    };
    atomic_json(&record(m), &w)?;
    Ok(w)
}

fn supervisor(m: &Manager) -> Result<PathBuf> {
    // A workspace package is separate from the installed native-bridge service.
    // The executable and supervisor must be siblings in one selected generation.
    let tooling_root = m.root.join("daw-workspaces/tooling");
    let t: Tooling = read_json(&tooling_root.join("selection.json"))?;
    require(
        t.schema == 1
            && valid_hex(&t.generation, 64)
            && valid_hex(&t.source_head, 40)
            && valid_hex(&t.source_tree, 40),
        "daw_workspace_tooling_identity",
    )?;
    let dir = tooling_root.join("generations").join(&t.generation);
    verify_private_exact(&dir)?;
    require(
        t.manager.path == dir.join("linux-vst-bridge")
            && t.supervisor.path == dir.join("session.py")
            && t.ownership.path == dir.join("ownership.py")
            && t.importer.path == dir.join("import_installer.py")
            && std::env::current_exe()? == t.manager.path,
        "daw_workspace_unmanaged_tooling",
    )?;
    t.manager.verify()?;
    t.supervisor.verify()?;
    t.ownership.verify()?;
    t.importer.verify()?;
    Ok(t.supervisor.path)
}

fn install(m: &Manager) -> Result<()> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    require(
        w.state == State::Imported && w.installation_operation.is_none(),
        "daw_workspace_install_already_started",
    )?;
    let imported = installer_import::load(m, &w.installer)?;
    w.environment.runner.verify()?;
    let sw = software(m)?;
    let owner = supervisor(m)?;
    let op = random_id()?;
    let dir = result_path(m, &op, true)
        .parent()
        .ok_or("daw_workspace_install_dir")?
        .to_path_buf();
    private_exact(&dir)?;
    let spec = dir.join("spec.json");
    atomic_json(
        &spec,
        &json!({"schema":2,"operation":op,"environment":w.environment,
        "installer":imported.artifact,"format":imported.format,
        "installer_launch":sw.installer_launch,"report":result_path(m,&op,true)}),
    )?;
    w.installation_operation = Some(op.clone());
    w.state = State::Installing;
    save(m, &mut w)?;
    let status = Command::new("systemd-run")
        .args([
            "--user",
            "--collect",
            "--property=UMask=0077",
            "--property=KillMode=control-group",
            "--property=TimeoutStopSec=30",
            "--property=StandardOutput=null",
            "--property=StandardError=null",
        ])
        .arg(format!("--unit={}", unit(&op, true)?))
        .arg("/usr/bin/python3")
        .arg(owner)
        .arg("--install")
        .arg(spec)
        .status();
    if !status.is_ok_and(|s| s.success()) {
        require(
            !unit_active(&unit(&op, true)?)?,
            "daw_workspace_installer_owner_uncertain",
        )?;
        w.state = State::Failed;
        w.first_failure
            .get_or_insert_with(|| "installer_worker_launch_failed".into());
        save(m, &mut w)?;
        return Err("daw_workspace_installer_launch_failed".into());
    }
    println!(
        "{}",
        json!({"workspace":w.id,"operation":op,"state":"installing"})
    );
    Ok(())
}

fn pe_machine(path: &Path) -> Result<u16> {
    use std::io::{Seek, SeekFrom};
    let mut f = file(path)?;
    let mut header = [0u8; 64];
    f.read_exact(&mut header)?;
    require(&header[..2] == b"MZ", "daw_workspace_installed_pe")?;
    let at = u32::from_le_bytes(header[60..64].try_into()?) as u64;
    require(
        at >= 64 && at + 24 <= f.metadata()?.len(),
        "daw_workspace_installed_pe",
    )?;
    f.seek(SeekFrom::Start(at))?;
    let mut coff = [0u8; 24];
    f.read_exact(&mut coff)?;
    require(&coff[..4] == b"PE\0\0", "daw_workspace_installed_pe")?;
    Ok(u16::from_le_bytes(coff[4..6].try_into()?))
}

fn discover(w: &Workspace) -> Result<InstalledApplication> {
    let root = w
        .environment
        .root
        .join("compatdata/pfx/drive_c/Program Files/Image-Line");
    owned_vendor_dir(&root)?;
    let mut images = Vec::new();
    for (i, entry) in fs::read_dir(&root)?.enumerate() {
        require(i < 64, "daw_workspace_installation_roster_bound")?;
        let path = entry?.path();
        if !fs::symlink_metadata(&path)?.is_dir() {
            continue;
        }
        owned_vendor_dir(&path)?;
        for (j, file) in fs::read_dir(&path)?.enumerate() {
            require(j < 1024, "daw_workspace_application_roster_bound")?;
            let file = file?.path();
            let name = file.file_name().and_then(|v| v.to_str()).unwrap_or("");
            if !matches!(name, "FL64.exe" | "FL Studio.exe") {
                continue;
            }
            if pe_machine(&file)? == 0x8664 {
                images.push(file);
            }
        }
    }
    require(
        images.len() == 1,
        "daw_workspace_installed_image_ambiguous_or_absent",
    )?;
    let executable = Artifact {
        path: images.remove(0),
        sha256: String::new(),
    };
    let executable = Artifact {
        sha256: digest(&executable.path)?,
        ..executable
    };
    Ok(InstalledApplication {
        resource_root: executable
            .path
            .parent()
            .ok_or("daw_workspace_resource_root")?
            .into(),
        executable,
        installer_advertised_release: w.installer_release.clone(),
        observed_file_version: None,
    })
}

fn finalize_install(m: &Manager, w: &mut Workspace) -> Result<()> {
    if w.installed.is_some() {
        return Ok(());
    }
    let op = w
        .installation_operation
        .as_deref()
        .ok_or("daw_workspace_not_installed")?;
    require(
        !unit_active(&unit(op, true)?)?,
        "daw_workspace_installer_still_running",
    )?;
    let v = result(m, op, true)?.ok_or("daw_workspace_install_result_absent")?;
    require(retired(&v), "daw_workspace_installer_cleanup_unconfirmed")?;
    // A helper's nonzero exit does not erase an exact installed application.
    let installed = discover(w)?;
    installed.executable.verify()?;
    w.installed = Some(installed);
    w.state = if v["state"] == "completed" {
        State::Installed
    } else {
        State::NeedsUserAction
    };
    if v["state"] != "completed" {
        w.first_failure
            .get_or_insert_with(|| "installer_exit_nonzero_application_present".into());
    }
    save(m, w)
}

fn session_ready(m: &Manager, w: &Workspace) -> Result<bool> {
    let Some(op) = &w.session_operation else {
        return Ok(true);
    };
    if unit_active(&unit(op, false)?)? {
        return Ok(false);
    }
    let v = result(m, op, false)?.ok_or("daw_workspace_session_result_absent")?;
    require(retired(&v), "daw_workspace_cleanup_unconfirmed")?;
    Ok(true)
}

fn check_prerequisites(runtime: &Path, graphical: &str, authority: &Path) -> Result<()> {
    require(
        graphical.starts_with(':') && graphical.len() <= 16,
        "daw_workspace_graphical_session_unavailable",
    )?;
    let md = fs::symlink_metadata(authority).map_err(|_| "daw_workspace_xauthority_unavailable")?;
    require(
        md.is_file()
            && !md.file_type().is_symlink()
            && md.uid() == unsafe { libc::getuid() }
            && md.mode() & 0o077 == 0,
        "daw_workspace_xauthority_unavailable",
    )?;
    let pulse = fs::symlink_metadata(runtime.join("pulse/native"))
        .map_err(|_| "daw_workspace_audio_endpoint_unavailable")?;
    require(
        pulse.file_type().is_socket() && pulse.uid() == unsafe { libc::getuid() },
        "daw_workspace_audio_endpoint_unavailable",
    )
}

fn desktop_prerequisites() -> Result<()> {
    let output = Command::new("systemctl")
        .args(["--user", "show-environment"])
        .output()?;
    require(
        output.status.success(),
        "daw_workspace_session_environment_unavailable",
    )?;
    let value = std::str::from_utf8(&output.stdout)?;
    let field = |name: &str| -> Result<String> {
        value
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .map(str::to_owned)
            .ok_or_else(|| "daw_workspace_session_environment_unavailable".into())
    };
    let runtime = PathBuf::from(field("XDG_RUNTIME_DIR")?);
    require(
        runtime == Path::new(&format!("/run/user/{}", unsafe { libc::getuid() })),
        "daw_workspace_runtime_identity",
    )?;
    check_prerequisites(
        &runtime,
        &field("DISPLAY")?,
        Path::new(&field("XAUTHORITY")?),
    )
}

fn launch(m: &Manager) -> Result<()> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    finalize_install(m, &mut w)?;
    require(
        session_ready(m, &w)?,
        "daw_workspace_already_running_use_focus",
    )?;
    desktop_prerequisites()?;
    w.environment.runner.verify()?;
    let app = w.installed.as_ref().ok_or("daw_workspace_not_installed")?;
    app.executable.verify()?;
    let owner = supervisor(m)?;
    let op = random_id()?;
    let dir = result_path(m, &op, false)
        .parent()
        .ok_or("daw_workspace_session_dir")?
        .to_path_buf();
    private_exact(&dir)?;
    let spec = dir.join("spec.json");
    atomic_json(
        &spec,
        &json!({"kind":"daw_workspace_application","application":{
        "id":"fl_studio","environment":w.environment,"executable":app.executable,"helpers":[],
        "preferences":w.preferences,"projects":w.projects,"exports":w.exports},
        "report":result_path(m,&op,false),"mode":"normal","operation_id":op}),
    )?;
    w.session_operation = Some(op.clone());
    w.state = State::Starting;
    save(m, &mut w)?;
    let status = Command::new("systemd-run")
        .args([
            "--user",
            "--collect",
            "--property=UMask=0077",
            "--property=KillMode=control-group",
            "--property=TimeoutStopSec=30",
            "--property=StandardOutput=null",
            "--property=StandardError=null",
        ])
        .arg(format!("--unit={}", unit(&op, false)?))
        .arg("/usr/bin/python3")
        .arg(owner)
        .arg("--vendor-application")
        .arg(spec)
        .status();
    if !status.is_ok_and(|s| s.success()) {
        require(
            !unit_active(&unit(&op, false)?)?,
            "daw_workspace_application_owner_uncertain",
        )?;
        w.state = State::Failed;
        w.first_failure
            .get_or_insert_with(|| "application_worker_launch_failed".into());
        save(m, &mut w)?;
        return Err("daw_workspace_application_launch_failed".into());
    }
    println!(
        "{}",
        json!({"workspace":w.id,"operation":op,"state":"starting"})
    );
    Ok(())
}

fn request(m: &Manager, action: &str) -> Result<()> {
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    let op = w
        .session_operation
        .as_deref()
        .ok_or("daw_workspace_no_session")?
        .to_owned();
    require(
        unit_active(&unit(&op, false)?)?,
        "daw_workspace_no_live_session",
    )?;
    let id = random_id()?;
    let file = result_path(m, &op, false)
        .parent()
        .ok_or("daw_workspace_session_dir")?
        .join(if action == "focus" {
            "focus.json"
        } else {
            "close.json"
        });
    require(!file.try_exists()?, "daw_workspace_control_pending")?;
    atomic_json(&file, &json!({"operation_id":op,"request":id}))?;
    if action == "stop" {
        w.state = State::Stopping;
        save(m, &mut w)?;
    }
    let deadline = Instant::now()
        + Duration::from_secs(if action == "focus" {
            4
        } else {
            SESSION_SECONDS
        });
    loop {
        if let Some(v) = result(m, &op, false)? {
            if action == "focus" && v["focus_result"]["request"] == id {
                require(
                    v["focus_result"]["result"] == "focused",
                    "daw_workspace_focus_refused",
                )?;
                println!("{}", v["focus_result"]);
                return Ok(());
            }
            if action == "stop" && retired(&v) && !unit_active(&unit(&op, false)?)? {
                println!(
                    "{}",
                    json!({"operation":op,"state":"retired","cleanup_confirmed":true})
                );
                return Ok(());
            }
        }
        require(
            Instant::now() < deadline,
            if action == "focus" {
                "daw_workspace_focus_timeout"
            } else {
                "daw_workspace_graceful_stop_pending"
            },
        )?;
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn status(m: &Manager) -> Result<()> {
    let w = load(m)?;
    let install_result = match &w.installation_operation {
        Some(op) => result(m, op, true)?,
        None => None,
    };
    let session_result = match &w.session_operation {
        Some(op) => result(m, op, false)?,
        None => None,
    };
    let owner_active = match &w.session_operation {
        Some(op) => unit_active(&unit(op, false)?)?,
        None => false,
    };
    let install_active = match &w.installation_operation {
        Some(op) => unit_active(&unit(op, true)?)?,
        None => false,
    };
    let detected_installed =
        if w.installed.is_none() && !install_active && install_result.as_ref().is_some_and(retired)
        {
            discover(&w).ok()
        } else {
            None
        };
    let cleanup = if owner_active || install_active {
        "running"
    } else if (w.session_operation.is_some() && !session_result.as_ref().is_some_and(retired))
        || (w.installation_operation.is_some() && !install_result.as_ref().is_some_and(retired))
    {
        "cleanup_unconfirmed"
    } else {
        "confirmed"
    };
    let effective_state = if cleanup == "cleanup_unconfirmed" {
        State::CleanupUnconfirmed
    } else if owner_active && w.state == State::Stopping {
        State::Stopping
    } else if owner_active
        && session_result
            .as_ref()
            .is_some_and(|v| v["state"] == "running" || v["state"] == "unknown")
    {
        State::Running
    } else if owner_active {
        State::Starting
    } else if install_active {
        State::Installing
    } else if session_result
        .as_ref()
        .is_some_and(|v| v["state"] == "failed")
    {
        State::Failed
    } else if detected_installed.is_some() {
        if install_result
            .as_ref()
            .is_some_and(|v| v["state"] == "completed")
        {
            if desktop_prerequisites().is_ok() {
                State::Ready
            } else {
                State::Installed
            }
        } else {
            State::NeedsUserAction
        }
    } else if w.installation_operation.is_some() && w.installed.is_none() {
        State::Failed
    } else if w.installed.is_some() && w.state == State::NeedsUserAction {
        State::NeedsUserAction
    } else if w.installed.is_some() {
        if desktop_prerequisites().is_ok() {
            State::Ready
        } else {
            State::Installed
        }
    } else {
        w.state
    };
    let observed_failure = w
        .first_failure
        .as_deref()
        .or_else(|| session_result.as_ref().and_then(|v| v["error"].as_str()))
        .or_else(|| install_result.as_ref().and_then(|v| v["error"].as_str()));
    println!(
        "{}",
        json!({"schema":1,"workspace":w,"installer_result":install_result,
        "installer_active":install_active,"session_result":session_result,
        "owner_active":owner_active,"cleanup":cleanup,"effective_state":effective_state,
        "detected_installed_not_committed":detected_installed,
        "first_useful_failure":observed_failure})
    );
    Ok(())
}

fn record_audio(m: &Manager, args: &[String]) -> Result<()> {
    let [backend, rate, block, endpoint, audible] = args else {
        return Err(
            "Usage: workspace record-audio BACKEND RATE BUFFER ENDPOINT audible|not-audible".into(),
        );
    };
    let backend = match backend.as_str() {
        "pulseaudio" => AudioBackend::PulseAudio,
        "wineasio" => AudioBackend::WineAsio,
        _ => return Err("daw_workspace_audio_backend_unsupported".into()),
    };
    let observation = AudioObservation {
        backend,
        sample_rate: rate.parse()?,
        buffer_frames: block.parse()?,
        linux_endpoint: endpoint.clone(),
        operator_audible: match audible.as_str() {
            "audible" => true,
            "not-audible" => false,
            _ => return Err("daw_workspace_audio_witness_invalid".into()),
        },
    };
    require(
        audio_syntax(&observation),
        "daw_workspace_audio_observation",
    )?;
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    require(
        w.installed.is_some(),
        "daw_workspace_audio_before_installation",
    )?;
    w.audio = Some(observation);
    save(m, &mut w)?;
    println!("{}", serde_json::to_string(&w.audio)?);
    Ok(())
}

fn record_installed_version(m: &Manager, version: &str, image_sha: &str) -> Result<()> {
    require(
        release_syntax(version) && valid_hex(image_sha, 64),
        "daw_workspace_installed_version_syntax",
    )?;
    let _guard = m.lock("daw-workspace.lock")?;
    let mut w = load(m)?;
    let installed = w.installed.as_mut().ok_or("daw_workspace_not_installed")?;
    require(
        installed.executable.sha256 == image_sha,
        "daw_workspace_installed_version_image_changed",
    )?;
    installed.executable.verify()?;
    require(
        installed
            .observed_file_version
            .as_deref()
            .is_none_or(|v| v == version),
        "daw_workspace_installed_version_conflict",
    )?;
    installed.observed_file_version = Some(version.to_owned());
    save(m, &mut w)?;
    println!(
        "{}",
        json!({"file_version":version,"image_sha256":image_sha})
    );
    Ok(())
}

pub fn run(m: &Manager, args: &[String]) -> Result<()> {
    match args {
        [action] if action == "import" => {
            let source = fs::File::from(std::io::stdin().as_fd().try_clone_to_owned()?);
            println!("{}", serde_json::to_string(&installer_import::import(m, source)?)?);
            Ok(())
        }
        [action, id, release] if action == "create" => create(m, id, release),
        [action] if action == "install" => install(m),
        [action] if action == "launch" => launch(m),
        [action] if action == "focus" => request(m, "focus"),
        [action] if action == "stop" => request(m, "stop"),
        [action] if action == "status" => status(m),
        [action, rest @ ..] if action == "record-audio" => record_audio(m, rest),
        [action, version, image_sha] if action == "record-installed-version" => record_installed_version(m, version, image_sha),
        _ => Err("Usage: workspace import | create INSTALLER_SHA RELEASE | install | launch | focus | status | stop | record-audio BACKEND RATE BUFFER ENDPOINT audible|not-audible | record-installed-version VERSION IMAGE_SHA".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;
    #[test]
    fn closed_workspace_and_release_grammar() {
        assert_eq!(APP, "fl-studio");
        assert!(release_syntax("26.1.6.0"));
        for value in ["", "latest", "26.1;run", "26/1"] {
            assert!(!release_syntax(value));
        }
        assert!(unit("a".repeat(32).as_str(), false)
            .unwrap()
            .starts_with("linux-vst-bridge-daw-fl-"));
        assert!(unit("other", false).is_err());
    }
    #[test]
    fn installer_and_application_are_distinct() {
        assert!(!retired(
            &json!({"state":"running","cleanup_confirmed":false,"owned_live":1})
        ));
        assert!(!retired(
            &json!({"state":"completed","cleanup_confirmed":false,"owned_live":0})
        ));
        assert!(retired(
            &json!({"state":"completed","cleanup_confirmed":true,"owned_live":0})
        ));
    }
    #[test]
    fn workspace_roots_are_private_and_native_registry_is_unchanged() {
        let f = crate::test_fixture::Fixture::new();
        let before = fs::read(f.m.root.join("registry.json")).ok();
        let mut runner = f.r.environment.runner.clone();
        runner.id = STANDARD_RUNNER.into();
        let w = create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner.clone()).unwrap();
        assert_eq!(w.state, State::Imported);
        assert_eq!(w.environment.runner, runner);
        assert_eq!(load(&f.m).unwrap(), w);
        for path in [&w.projects, &w.exports, &w.preferences, &w.environment.root] {
            let md = fs::symlink_metadata(path).unwrap();
            assert!(md.is_dir());
            assert_eq!(md.mode() & 0o077, 0);
        }
        assert_eq!(fs::read(f.m.root.join("registry.json")).ok(), before);
        assert!(create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner).is_err());
    }
    #[test]
    fn foreign_workspace_root_refuses_without_registry_mutation() {
        let f = crate::test_fixture::Fixture::new();
        let mut runner = f.r.environment.runner.clone();
        runner.id = STANDARD_RUNNER.into();
        private_exact(&f.m.root.join("daw-workspaces/fl-studio")).unwrap();
        symlink(&f.outer, base(&f.m).join("projects")).unwrap();
        let before = fs::read(f.m.root.join("registry.json")).ok();
        assert!(create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner).is_err());
        assert_eq!(fs::read(f.m.root.join("registry.json")).ok(), before);
        assert!(!record(&f.m).exists());
    }
    #[test]
    fn graphical_and_audio_prerequisites_are_separate() {
        let f = crate::test_fixture::Fixture::new();
        let runtime = f.outer.join("runtime");
        private_exact(&runtime).unwrap();
        let auth = runtime.join("xauthority");
        fs::write(&auth, b"fixture").unwrap();
        assert!(check_prerequisites(&runtime, "", &auth).is_err());
        assert!(check_prerequisites(&runtime, ":0", &auth).is_err());
        private_exact(&runtime.join("pulse")).unwrap();
        let _socket = UnixListener::bind(runtime.join("pulse/native")).unwrap();
        assert!(check_prerequisites(&runtime, ":0", &auth).is_ok());
        fs::remove_file(&auth).unwrap();
        symlink(&f.outer, &auth).unwrap();
        assert!(check_prerequisites(&runtime, ":0", &auth).is_err());
    }
    #[test]
    fn installed_image_requires_exact_x64_under_fl_resource_root() {
        let f = crate::test_fixture::Fixture::new();
        let mut runner = f.r.environment.runner.clone();
        runner.id = STANDARD_RUNNER.into();
        let mut w = create_exact(&f.m, &"ab".repeat(32), "26.1.6.0", runner).unwrap();
        let parent = w
            .environment
            .root
            .join("compatdata/pfx/drive_c/Program Files/Image-Line/FL Studio 2025");
        fs::create_dir_all(&parent).unwrap();
        let pe = parent.join("FL64.exe");
        let mut bytes = vec![0u8; 192];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        fs::write(&pe, bytes).unwrap();
        let app = discover(&w).unwrap();
        assert_eq!(app.executable.path, pe);
        assert_eq!(app.installer_advertised_release, "26.1.6.0");
        assert_eq!(app.observed_file_version, None);
        let image_sha = app.executable.sha256.clone();
        w.installed = Some(app);
        save(&f.m, &mut w).unwrap();
        assert!(record_installed_version(&f.m, "26.1.6.5639", &"cd".repeat(32)).is_err());
        record_installed_version(&f.m, "26.1.6.5639", &image_sha).unwrap();
        assert_eq!(
            load(&f.m)
                .unwrap()
                .installed
                .unwrap()
                .observed_file_version
                .as_deref(),
            Some("26.1.6.5639")
        );
    }
}
