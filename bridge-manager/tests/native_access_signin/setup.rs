//! A Native Access setup whose app is the first-party stand-in.
//!
//! Everything here is product state as a completed installation leaves it,
//! written into the account's own bridge root: a runtime acquired by the
//! product's delivery code, a setup with a prefix the runtime created, the
//! stand-in in Native Access's install location under the names the bundled
//! profile declares, the installer import and onboarding receipts, the app
//! selection, and a package generation holding the real manager, supervisor
//! and launch adapter. Only the vendor's files are replaced.
use linux_vst_bridge::{catalogue::Software, vendor_application::{Application, ApplicationId}, *};
use serde_json::json;
use std::{
    fs,
    io::Write,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use crate::report::Report;

const MANAGER: &str = env!("CARGO_BIN_EXE_linux-vst-bridge");
const SUPERVISOR: &[u8] = include_bytes!("../../runtime/session.py");
const OWNERSHIP: &[u8] = include_bytes!("../../runtime/ownership.py");
/// Where Native Access installs itself; its profile locates files from here.
pub const APP_DIRECTORY: &str = "Program Files/Native Instruments/Native Access";
/// The stand-in's own fixture directory in the prefix (`C:\LVBStandIn`).
const STAND_IN_DIRECTORY: &str = "LVBStandIn";
const BUNDLED_INSTALLER_VERSION: &str = "1.0.0";
/// Sizes of the Native Access build the bridge was first qualified with
/// (session.py's RENDERER_PRODUCTION and NAD1_INSTALLER). The stand-in's files
/// are padded to them with stored bytes, so the handler and supervisor hash
/// as many bytes as for the real app.
const SIZES: [(&str, u64); 11] = [
    ("Native Access.exe", 225_757_168),
    ("chrome_100_percent.pak", 119_905),
    ("chrome_200_percent.pak", 197_089),
    ("icudtl.dat", 10_876_560),
    ("libEGL.dll", 474_624),
    ("libGLESv2.dll", 8_024_064),
    ("resources.pak", 7_148_145),
    ("resources/app.asar", 76_143_407),
    ("snapshot_blob.bin", 365_488),
    ("v8_context_snapshot.bin", 740_048),
    ("bundled installer", 35_769_456),
];

const DISPOSABLE: &str = "This test writes the account's own bridge state, starts its user \
    units and runs a Wine prefix. Run it only in a disposable account, such as a CI runner or a \
    throwaway VM user, with LVB_DISPOSABLE_ACCOUNT=1.";

/// What the workflow (or a developer) supplies; see main.rs.
pub struct Inputs {
    pub revision: String,
    pub archives: PathBuf,
    pub adapter: PathBuf,
    pub stand_in: PathBuf,
    pub other_user: Option<String>,
    pub report: Option<PathBuf>,
}
impl Inputs {
    pub fn from_env() -> Self {
        let path = |name: &str| -> PathBuf {
            let value = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required; see main.rs"));
            let path = PathBuf::from(value);
            assert!(path.is_absolute(), "{name} must be an absolute path");
            path
        };
        Self {
            revision: std::env::var("LVB_SIGNIN_RUNTIME").expect("LVB_SIGNIN_RUNTIME names a runtime revision"),
            archives: path("LVB_SIGNIN_ARCHIVES"),
            adapter: path("LVB_SIGNIN_ADAPTER"),
            stand_in: path("LVB_SIGNIN_STAND_IN"),
            other_user: std::env::var("LVB_SIGNIN_OTHER_USER").ok().filter(|user| !user.is_empty()),
            report: std::env::var_os("LVB_SIGNIN_REPORT").map(PathBuf::from),
        }
    }
}

pub struct Setup {
    pub m: Manager,
    pub app: Application,
    pub selector: String,
    /// `vendor-applications/<selector>`: the app's launch records.
    pub directory: PathBuf,
    pub software: Software,
    pub prefix: PathBuf,
    /// `C:\LVBStandIn`: the stand-in's mode setting and what it observed.
    pub stand_in: PathBuf,
    pub stand_in_bytes: Vec<u8>,
}

impl Setup {
    pub fn create(inputs: &Inputs, report: &mut Report) -> Self {
        assert_eq!(std::env::var("LVB_DISPOSABLE_ACCOUNT").as_deref(), Ok("1"), "{DISPOSABLE}");
        unsafe { libc::umask(0o077); }
        let m = Manager::installed().unwrap();
        assert!(!m.root.exists(), "{} already exists. {DISPOSABLE} It must start with no bridge state.",
            m.root.display());
        private_dir(&m.root).unwrap();

        let started = Instant::now();
        let runner = runtime_delivery::install_revision(&m, &inputs.revision, &inputs.archives)
            .unwrap_or_else(|error| panic!("runtime {}: {error}", inputs.revision));
        report.stage("runtime download, unpack and tree verification", started.elapsed());

        let environment = environment(&m, runner);
        let started = Instant::now();
        initialize_prefix(&environment);
        report.stage("prefix creation by the runtime", started.elapsed());

        let prefix = environment.root.join("compatdata/pfx");
        let drive = prefix.join("drive_c");
        let install = drive.join(APP_DIRECTORY);
        let stand_in_bytes = fs::read(&inputs.stand_in).unwrap();
        let profile = vendor_profile::native_access().unwrap();
        let size = |name: &str| SIZES.iter().find(|(file, _)| *file == name).unwrap().1;
        write_padded(&install.join(&profile.executable), &stand_in_bytes, size(&profile.executable));
        for name in &profile.resources {
            write_padded(&install.join(name), format!("LVB stand-in {name}\n").as_bytes(), size(name));
        }
        let bundled = install.join(&profile.dependency.installer_directory).join(format!("{}{}{}",
            profile.dependency.installer_prefix, BUNDLED_INSTALLER_VERSION, profile.dependency.installer_suffix));
        write_padded(&bundled, &stand_in_bytes, size("bundled installer"));
        let stand_in = drive.join(STAND_IN_DIRECTORY);
        private_dir(&stand_in).unwrap();

        let (installer_sha256, installer_result) = installation_receipts(&m, &environment, &stand_in_bytes);
        let artifact = |path: PathBuf| Artifact { sha256: digest(&path).unwrap(), path };
        let app = Application { schema: 2, id: ApplicationId::Selected, environment: environment.clone(),
            executable: artifact(install.join(&profile.executable)), helpers: vec![artifact(bundled)],
            installer_sha256: Some(installer_sha256), installer_result: Some(installer_result),
            observed_installer_version: String::new(), user_selection: None };
        let selector = app.selector().unwrap();
        let directory = m.root.join("vendor-applications").join(&selector);
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("application.json"), &app).unwrap();
        let software = package_generation(&m, &inputs.adapter);
        Self { m, app, selector, directory, software, prefix, stand_in, stand_in_bytes }
    }

    pub fn app_executable(&self) -> &Path { &self.app.executable.path }
    pub fn drive(&self) -> PathBuf { self.prefix.join("drive_c") }

    /// The second copy's behaviour for the next return; a fixture setting.
    pub fn stand_in_mode(&self, mode: &str) {
        fs::write(self.stand_in.join("mode"), format!("{mode}\n")).unwrap();
    }
    /// A `slow` second copy hands over as soon as this exists.
    pub fn release_slow_copy(&self, released: bool) {
        let marker = self.stand_in.join("release");
        if released { fs::write(marker, b"").unwrap() } else { let _ = fs::remove_file(marker); }
    }

    /// Point the prefix's C: drive at a mirror of drive_c that differs only
    /// in the app's executable, which is another build. Every Linux path the
    /// handler and supervisor check is unchanged; a Windows open of the app's
    /// path now finds the other file. Undone on drop.
    pub fn other_build_at_app_path(&self) -> DriveRedirect {
        let drive = self.drive();
        let mirror = self.prefix.join("drive_c-other-build");
        let (mut original, mut copy) = (drive.clone(), mirror.clone());
        private_dir(&copy).unwrap();
        for component in self.app_executable().strip_prefix(&drive).unwrap().components() {
            for entry in fs::read_dir(&original).unwrap().flatten() {
                if entry.file_name() != component.as_os_str() {
                    symlink(entry.path(), copy.join(entry.file_name())).unwrap();
                }
            }
            original.push(component);
            copy.push(component);
            if original.is_dir() { private_dir(&copy).unwrap(); }
        }
        let mut other = self.stand_in_bytes.clone();
        other.extend_from_slice(b"another build of the app");
        fs::write(&copy, other).unwrap();
        let link = self.prefix.join("dosdevices/c:");
        let target = fs::read_link(&link).unwrap();
        replace_link(&link, Path::new("../drive_c-other-build"));
        DriveRedirect { link, target, mirror }
    }
    pub fn stand_in_lines(&self, log: &str) -> Vec<String> {
        match fs::read_to_string(self.stand_in.join(log)) {
            Ok(text) => text.lines().map(str::to_owned).collect(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => vec![],
            Err(error) => panic!("{log}: {error}"),
        }
    }
}

pub struct DriveRedirect { link: PathBuf, target: PathBuf, mirror: PathBuf }
impl Drop for DriveRedirect {
    fn drop(&mut self) {
        replace_link(&self.link, &self.target);
        fs::remove_dir_all(&self.mirror).unwrap();
    }
}
fn replace_link(link: &Path, target: &Path) {
    let staged = link.with_file_name(".lvb-signin-c");
    symlink(target, &staged).unwrap();
    fs::rename(staged, link).unwrap();
}

fn environment(m: &Manager, runner: Runner) -> Environment {
    let id = random_id().unwrap();
    let root = m.root.join("environments").join(&id);
    for directory in ["home", "compatdata", "runtime-var", "host-cache", "host-config", "host-data",
        "host-tmp", "client"] {
        private_dir(&root.join(directory)).unwrap();
    }
    let environment = Environment { id, root, runner, revision: 1 };
    atomic_json(&environment.root.join("environment.json"), &environment).unwrap();
    environment
}

/// The runtime creates the prefix exactly as the installer supervisor has it
/// do before an installer runs (`proton getcompatpath /`), in a transient
/// unit so that no Wine process outlives it.
fn initialize_prefix(environment: &Environment) {
    let root = &environment.root;
    let runner = &environment.runner;
    let mut command = Command::new("systemd-run");
    command.args(["--user", "--wait", "--collect", "--quiet", "--property=UMask=0077"])
        .arg(format!("--unit=lvb-signin-prefix-{}.service", environment.id))
        .arg("/usr/bin/env").arg("-i");
    for (key, value) in runtime_environment(root) {
        command.arg(format!("{key}={value}"));
    }
    let status = command.arg(&runner.entry_point).args(["--verb=run", "--"]).arg(&runner.proton)
        .args(["getcompatpath", "/"]).status().unwrap();
    assert!(status.success(), "prefix creation failed: {status}");
    assert!(root.join("compatdata/pfx/system.reg").is_file(), "the runtime created no prefix");
}

/// The closed launch environment session.py's `environment()` builds.
fn runtime_environment(root: &Path) -> Vec<(String, String)> {
    let user = std::env::var("USER").unwrap_or_default();
    let uid = unsafe { libc::getuid() };
    let mut values: Vec<(String, String)> = [
        ("HOME", root.join("home").display().to_string()), ("USER", user.clone()), ("LOGNAME", user),
        ("PATH", "/usr/bin:/bin".into()), ("LANG", "C.UTF-8".into()),
        ("XDG_RUNTIME_DIR", format!("/run/user/{uid}")),
        ("STEAM_COMPAT_DATA_PATH", root.join("compatdata").display().to_string()),
        ("STEAM_COMPAT_CLIENT_INSTALL_PATH", root.join("client").display().to_string()),
        ("STEAM_COMPAT_APP_ID", "0".into()), ("SteamAppId", "0".into()), ("SteamGameId", "0".into()),
        ("STEAM_ZENITY", String::new()),
        ("PRESSURE_VESSEL_VARIABLE_DIR", root.join("runtime-var").display().to_string()),
        // Proton is Python; without this it writes caches into the runtime.
        ("PYTHONDONTWRITEBYTECODE", "1".into()),
        ("XDG_CACHE_HOME", root.join("host-cache").display().to_string()),
        ("XDG_CONFIG_HOME", root.join("host-config").display().to_string()),
        ("XDG_DATA_HOME", root.join("host-data").display().to_string()),
        ("TMPDIR", root.join("host-tmp").display().to_string()),
        ("PROTON_LOG", "0".into()), ("WINEDEBUG", "-all".into()),
    ].into_iter().map(|(key, value)| (key.to_owned(), value)).collect();
    let shown = Command::new("systemctl").args(["--user", "show-environment"]).output().unwrap();
    for line in String::from_utf8_lossy(&shown.stdout).lines() {
        if let Some((key, value)) = line.split_once('=') {
            if matches!(key, "DISPLAY" | "XAUTHORITY" | "WAYLAND_DISPLAY" | "DBUS_SESSION_BUS_ADDRESS") {
                values.push((key.into(), value.into()));
            }
        }
    }
    assert!(values.iter().any(|(key, value)| key == "DISPLAY" && !value.is_empty()),
        "the user manager has no DISPLAY; Wine needs an X server (Xvfb is enough)");
    values
}

/// Pad with stored, varied bytes, never a sparse hole. None is ASCII, so the
/// padding can hold no code for the privacy search to find.
fn write_padded(path: &Path, bytes: &[u8], size: u64) {
    private_dir(path.parent().unwrap()).unwrap();
    let mut file = fs::File::create(path).unwrap();
    file.write_all(bytes).unwrap();
    let mut state = 0x9e37_79b9_7f4a_7c15_u64 ^ size;
    let block: Vec<u8> = (0..1 << 20).map(|_| {
        state ^= state << 13; state ^= state >> 7; state ^= state << 17;
        state as u8 | 0x80
    }).collect();
    let mut left = size.saturating_sub(bytes.len() as u64);
    while left > 0 {
        let n = left.min(block.len() as u64) as usize;
        file.write_all(&block[..n]).unwrap();
        left -= n as u64;
    }
    file.sync_all().unwrap();
}

/// The import and onboarding receipts a completed Native Access installation
/// leaves behind (installer_import.rs, onboarding.rs); the installer journey
/// itself is outside this test. Returns the installer digest and receipt.
fn installation_receipts(m: &Manager, environment: &Environment, installer: &[u8]) -> (String, Artifact) {
    let installers = m.root.join("installers");
    private_dir(&installers).unwrap();
    let staged = installers.join(format!(".import-{}", random_id().unwrap()));
    fs::write(&staged, installer).unwrap();
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o400)).unwrap();
    let sha = digest(&staged).unwrap();
    let path = installers.join(format!("{sha}.exe"));
    fs::rename(&staged, &path).unwrap();
    let now = observation::now().unwrap();
    atomic_json(&installers.join(format!("{sha}.json")), &json!({"schema":1,"id":sha,
        "artifact":{"path":path,"sha256":sha},"byte_size":installer.len(),"format":"pe_executable",
        "name_hint":"Windows installer","source_class":"operator_selected_file","import_result":"imported",
        "created_at":now,"vendor":null,"product":null})).unwrap();
    let operation = random_id().unwrap();
    let onboarding = m.root.join("onboarding").join(&environment.id);
    private_dir(&onboarding).unwrap();
    atomic_json(&onboarding.join("record.json"), &json!({"schema":1,"id":environment.id,"installer":sha,
        "environment":environment,"created_at":now,"creation_operation":random_id().unwrap(),
        "installation_operation":operation,"published":true})).unwrap();
    let receipt = onboarding.join(format!("{operation}-result.json"));
    atomic_json(&receipt, &json!({"schema":2,"operation":operation,"state":"completed","raw_exit":0,
        "owned_live":0,"cleanup_confirmed":true,"error":null})).unwrap();
    (sha, Artifact { sha256: digest(&receipt).unwrap(), path: receipt })
}

/// The selected package generation, every artifact read-only under
/// software/, as package activation leaves it: the manager Cargo built for
/// this test, the supervisor and ownership sources it ships, and the launch
/// adapter built from tools/is2.
fn package_generation(m: &Manager, adapter: &Path) -> Software {
    let generation = m.root.join("software").join(format!("generation-{}", random_id().unwrap()));
    private_dir(&generation).unwrap();
    let install = |name: &str, bytes: &[u8], mode: u32| {
        let path = generation.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        Artifact { sha256: digest(&path).unwrap(), path }
    };
    let source_manifest = install("host-source-manifest.json", b"{\"fixture\":\"browser return\"}\n", 0o400);
    let software = Software {
        manager: install("linux-vst-bridge", &fs::read(MANAGER).unwrap(), 0o500),
        supervisor: install("session.py", SUPERVISOR, 0o400),
        ownership: install("ownership.py", OWNERSHIP, 0o400),
        host: install("host.exe", b"MZ browser-return fixture host; never executed\n", 0o400),
        installer_launch: Some(install("installer-launch.exe", &fs::read(adapter).unwrap(), 0o400)),
        source_sha256: source_manifest.sha256.clone(),
        source_manifest,
        preparation_kit: None, operator_frontend: None, native_catalogue: None,
    };
    atomic_json(&m.root.join("software.json"), &software).unwrap();
    software
}
