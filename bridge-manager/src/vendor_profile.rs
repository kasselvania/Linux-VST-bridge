//! Declarative compatibility requirements for an already authorized application.
//! Structure selects behavior; actual content and installation bind execution.
use crate::{renderer_application::Image, vendor_application::Application, *};
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};

pub const NATIVE_ACCESS: &[u8] = include_bytes!("vendor_profiles/native_access.json");

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceRequirements {
    pub installer_directory: String,
    pub installer_prefix: String,
    pub installer_suffix: String,
    pub service: String,
    pub daemon: String,
    pub architecture: String,
    pub listeners: Vec<u16>,
}

/// A Windows folder resolved for the prefix's own user. The value written is
/// never taken from profile text, user input or the environment.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum KnownFolder {
    Downloads,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistryType {
    #[serde(rename = "REG_SZ")]
    String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegistryDefaultValue {
    pub known_folder: KnownFolder,
}

/// A per-user setting the app needs but leaves empty in a fresh prefix. The
/// supervisor fills it only when absent or empty; a chosen value is kept.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegistryDefault {
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: RegistryType,
    pub value: RegistryDefaultValue,
}

pub const REGISTRY_DEFAULTS_LIMIT: usize = 4;

fn registry_word(value: &str) -> bool {
    (1..=64).contains(&value.len()) && value.trim() == value
        && value.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric())
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'.' | b'_' | b'-'))
}

/// Current-user keys only, with a bounded plain charset. No expansion, no
/// other hives, no types beyond a plain string.
pub fn check_registry_defaults(defaults: &[RegistryDefault]) -> Result<()> {
    require(defaults.len() <= REGISTRY_DEFAULTS_LIMIT, "vendor_profile_registry_default_count")?;
    let mut seen = std::collections::BTreeSet::new();
    for default in defaults {
        let path = default.key.strip_prefix("HKCU\\").ok_or("vendor_profile_registry_default_hive")?;
        let parts: Vec<_> = path.split('\\').collect();
        require(default.key.len() <= 255 && (1..=8).contains(&parts.len())
            && parts.iter().all(|part| registry_word(part)), "vendor_profile_registry_default_key")?;
        require(registry_word(&default.name), "vendor_profile_registry_default_name")?;
        require(seen.insert((default.key.to_ascii_lowercase(), default.name.to_ascii_lowercase())),
            "vendor_profile_registry_default_duplicate")?;
    }
    Ok(())
}

/// The binding part of a vendor profile. It is saved inside every launch
/// record, so its shape stays readable by older and newer managers alike;
/// settings a launch only applies live outside it (see `Bundled`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: u32,
    pub id: String,
    pub executable: String,
    pub architecture: String,
    pub resources: Vec<String>,
    pub dependency: ServiceRequirements,
    pub rendering: operator_model::RendererPolicy,
    pub browser_scheme: String,
}

/// A bundled profile file: the saved binding plus launch-time settings that
/// are handed to the supervisor beside the launch record, never inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundled {
    pub profile: Profile,
    pub registry_defaults: Vec<RegistryDefault>,
}

impl Bundled {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut value: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(bytes)?;
        let registry_defaults: Vec<RegistryDefault> = value.remove("registry_defaults")
            .map(serde_json::from_value).transpose()?.unwrap_or_default();
        check_registry_defaults(&registry_defaults)?;
        Ok(Self { profile: serde_json::from_value(serde_json::Value::Object(value))?, registry_defaults })
    }
}

/// Launch-time settings for one operation, written beside its operation.json.
/// Managers that predate them never open this file, so rollback keeps working.
pub const LAUNCH_SETTINGS: &str = "profile-defaults.json";

pub fn launch_settings(profile: &Profile, operation_id: &str) -> Result<serde_json::Value> {
    let bundled = Bundled::parse(NATIVE_ACCESS)?;
    require(bundled.profile.id == profile.id, "vendor_profile_definition_changed")?;
    Ok(serde_json::json!({"schema": 1, "operation_id": operation_id, "profile": profile.id,
        "registry_defaults": bundled.registry_defaults}))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceBinding {
    pub installer: Image,
    pub daemon: Option<Image>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub schema: u32,
    pub profile: Profile,
    pub profile_sha256: String,
    pub application: String,
    pub resources: BTreeMap<String, Image>,
    pub dependency: ServiceBinding,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManagedLaunch {
    pub capabilities: Capabilities,
    pub software: catalogue::Software,
}
impl std::fmt::Debug for ManagedLaunch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedLaunch").field("capabilities", &self.capabilities).finish_non_exhaustive()
    }
}
impl ManagedLaunch {
    pub fn new(capabilities: Capabilities, software: catalogue::Software) -> Result<Self> {
        for artifact in [&software.manager, &software.supervisor, &software.ownership,
            software.installer_launch.as_ref().ok_or("Assisted application setup needs the managed launch adapter")?] {
            artifact.verify()?;
        }
        Ok(Self { capabilities, software })
    }
}

pub struct Inspection {
    pub capabilities: Option<Capabilities>,
    pub missing_capability: Option<String>,
}

#[derive(Debug)]
struct IntegrityFailure(&'static str);
impl std::fmt::Display for IntegrityFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let problem = match self.0 {
            "vendor_profile_structure_unavailable" => "The app's support files could not be read.",
            "vendor_profile_bundled_dependency_ambiguous" => "More than one support installer was found for this app.",
            "vendor_profile_application_changed" => "The app's recorded files changed.",
            _ => "The app's support files point outside this setup or share another file's identity.",
        };
        write!(f, "{problem} Run its full installer again, or use Choose program to open it directly.")
    }
}
impl std::error::Error for IntegrityFailure {}
fn integrity(ok: bool, reason: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(Box::new(IntegrityFailure(reason))) }
}

pub fn native_access() -> Result<Profile> {
    Ok(Bundled::parse(NATIVE_ACCESS)?.profile)
}

/// Bundled profiled applications whose first start must be their managed
/// launch, for the installer supervisor. Every profile declares a service
/// dependency that only its managed launch prepares, so a copy an installer
/// starts by itself ("launch after install") would run without it. The
/// supervisor ends such a copy once the installer has finished; the file name
/// and resources only locate it, its mapped file identity joins processes.
pub fn installer_first_launch() -> Result<serde_json::Value> {
    let profile = native_access()?;
    Ok(serde_json::json!([{"profile": profile.id, "executable": profile.executable,
        "resources": profile.resources}]))
}

/// Digest of the saved binding, as the supervisor pins it.
pub fn profile_sha256(profile: &Profile) -> Result<String> {
    Ok(hex(&Sha256::digest(serde_json::to_vec(&serde_json::to_value(profile)?)?)))
}

fn relative(value: &str) -> Result<&Path> {
    let path = Path::new(value);
    require(!path.is_absolute() && path.components().all(|part|
        matches!(part, std::path::Component::Normal(_))), "vendor_profile_relative_path")?;
    Ok(path)
}

fn image(path: &Path) -> Result<Image> {
    let metadata = fs::symlink_metadata(path)?;
    integrity(metadata.file_type().is_file() && metadata.nlink() == 1
        && path.canonicalize()? == path, "vendor_profile_resource_alias")?;
    require(metadata.len() > 0 && metadata.len() <= 256 * 1024 * 1024,
        "vendor_profile_resource_extent")?;
    let image = Image { artifact: Artifact { path: path.into(), sha256: digest(path)? },
        size: metadata.len() };
    crate::renderer_application::verify_image(&image)?;
    Ok(image)
}

fn executable(path: &Path, architecture: Option<&str>) -> Result<()> {
    let mut source = file(path)?;
    let mut header = [0_u8;64]; source.read_exact(&mut header)?;
    require(&header[..2] == b"MZ", "vendor_profile_windows_executable_required")?;
    let offset = u32::from_le_bytes(header[60..64].try_into()?) as u64;
    let size = source.metadata()?.len();
    require(offset >= 64 && offset.checked_add(24).is_some_and(|end|
        end <= size), "vendor_profile_pe_extent")?;
    source.seek(SeekFrom::Start(offset))?;
    let mut pe = [0_u8;24]; source.read_exact(&mut pe)?;
    let machine = u16::from_le_bytes(pe[4..6].try_into()?);
    let flags = u16::from_le_bytes(pe[22..24].try_into()?);
    require(&pe[..4] == b"PE\0\0" && flags & 2 != 0 && flags & 0x2000 == 0
        && matches!(machine, 0x14c | 0x8664), "vendor_profile_windows_executable_required")?;
    require(architecture.is_none_or(|expected|
        (expected == "x64" && machine == 0x8664) || (expected == "x86" && machine == 0x14c)),
        "vendor_profile_processor_capability_missing")
}

fn check_structure(directory: &Path, drive: &Path, profile: &Profile) -> Result<()> {
    let bundled=directory.join(relative(&profile.dependency.installer_directory)?);
    let mut paths:Vec<_>=profile.resources.iter().map(|name|directory.join(name)).collect();
    paths.extend([bundled.clone(),drive.join(relative(&profile.dependency.daemon)?)]);
    // Missing capabilities must not conceal a later alias or ambiguous input.
    // This bounded pass inspects structure only; content is hashed at admission.
    for path in paths {
        let mut current=drive.to_path_buf();
        for component in path.strip_prefix(drive)?.components() {
            current.push(component);
            match fs::symlink_metadata(&current) {
                Ok(metadata)=>integrity(!metadata.file_type().is_symlink()
                    && current.canonicalize()? == current
                    && (!metadata.is_file() || metadata.nlink()==1),"vendor_profile_resource_alias")?,
                Err(error) if error.kind()==std::io::ErrorKind::NotFound=>break,
                Err(error)=>return Err(error.into()),
            }
        }
    }
    if bundled.is_dir() {
        let mut candidates=0;
        for (index,entry) in fs::read_dir(&bundled)?.enumerate() {
            integrity(index<64,"vendor_profile_dependency_candidate_bound")?;
            let entry=entry?;let name=entry.file_name();
            if name.to_str().is_some_and(|name|name.starts_with(&profile.dependency.installer_prefix)
                && name.ends_with(&profile.dependency.installer_suffix)) {
                candidates+=1;
                integrity(candidates<=1,"vendor_profile_bundled_dependency_ambiguous")?;
                let metadata=fs::symlink_metadata(entry.path())?;
                integrity(metadata.is_file() && metadata.nlink()==1 && entry.path().canonicalize()?==entry.path(),
                    "vendor_profile_resource_alias")?;
            }
        }
    }
    Ok(())
}

fn observe_structure(app: &Application, profile: &Profile) -> Result<Capabilities> {
    require(app.schema == 2 && app.user_selection.is_none(),
        "vendor_profile_observed_installation_required")?;
    let directory = app.executable.path.parent().ok_or("vendor_profile_application_directory")?;
    let drive = app.environment.root.join("compatdata/pfx/drive_c");
    require(directory.starts_with(&drive) && directory.canonicalize()? == directory,
        "vendor_profile_application_directory")?;
    check_structure(directory,&drive,profile).map_err(|error| {
        if error.downcast_ref::<IntegrityFailure>().is_some() {error}
        else {Box::new(IntegrityFailure("vendor_profile_structure_unavailable"))
            as Box<dyn std::error::Error + Send + Sync>}
    })?;
    executable(&app.executable.path, Some(&profile.architecture))?;
    let main = image(&app.executable.path)?;
    integrity(main.artifact == app.executable, "vendor_profile_application_changed")?;
    let mut resources = BTreeMap::from([(profile.executable.clone(), main)]);
    let mut total = resources.values().map(|i|i.size).sum::<u64>();
    for name in &profile.resources {
        let resource = image(&directory.join(relative(name)?))?;
        total = total.checked_add(resource.size).ok_or("vendor_profile_resource_budget")?;
        require(total <= 512 * 1024 * 1024, "vendor_profile_resource_budget")?;
        require(resources.insert(name.clone(), resource).is_none(), "vendor_profile_resource_duplicate")?;
    }
    let bundled = directory.join(relative(&profile.dependency.installer_directory)?);
    let metadata = fs::symlink_metadata(&bundled)?;
    integrity(metadata.file_type().is_dir() && bundled.canonicalize()? == bundled,
        "vendor_profile_bundled_dependency_directory_alias")?;
    let mut candidates = Vec::new();
    for (index, entry) in fs::read_dir(&bundled)?.enumerate() {
        require(index < 64, "vendor_profile_dependency_candidate_bound")?;
        let entry = entry?;
        let name = entry.file_name(); let name = name.to_str().ok_or("vendor_profile_dependency_filename")?;
        // Names locate a candidate. No file is admitted before its actual bytes,
        // executable kind, location and installation context are bound.
        if name.starts_with(&profile.dependency.installer_prefix)
            && name.ends_with(&profile.dependency.installer_suffix) {
            let installer = image(&entry.path())?;
            executable(&installer.artifact.path, None)?;
            candidates.push(installer);
        }
    }
    integrity(candidates.len() <= 1, "vendor_profile_bundled_dependency_ambiguous")?;
    require(candidates.len() == 1, "vendor_profile_bundled_dependency_missing")?;
    let daemon_path = drive.join(relative(&profile.dependency.daemon)?);
    let daemon = match fs::symlink_metadata(&daemon_path) {
        Ok(_) => {
            executable(&daemon_path, Some(&profile.dependency.architecture))?;
            Some(image(&daemon_path)?)
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    Ok(Capabilities { schema: 1, profile: profile.clone(), profile_sha256: profile_sha256(profile)?,
        application: app.selector()?, resources,
        dependency: ServiceBinding { installer: candidates.remove(0), daemon } })
}

/// A nonmatch leaves ordinary application launch available. A structure problem
/// explains why assisted dependency/rendering behavior could not be selected.
pub fn inspect(app: &Application, managed: &Path) -> Result<Inspection> {
    app.verify(managed)?;
    let profile = native_access()?;
    if !app.executable.path.file_name().is_some_and(|name|
        name.to_string_lossy().eq_ignore_ascii_case(&profile.executable)) {
        return Ok(Inspection { capabilities: None, missing_capability: None });
    }
    if app.user_selection.is_some() {
        return Ok(Inspection { capabilities: None, missing_capability: Some(
            "This chosen program opens normally. Run its full installer in this setup to prepare its bundled support service automatically.".into()) });
    }
    match observe_structure(app, &profile) {
        Ok(capabilities) => Ok(Inspection { capabilities: Some(capabilities), missing_capability: None }),
        Err(error) if error.downcast_ref::<IntegrityFailure>().is_some() => Err(error),
        Err(error) => Ok(Inspection { capabilities: None,
            missing_capability: Some(format!("This app can open normally, but its bundled support files do not provide automatic setup. Run its full installer in this setup if it cannot open. Details: {error}")) }),
    }
}

impl Capabilities {
    /// A saved launch record is manager-owned data. Profile data improves with
    /// updates, so a record saved under an earlier revision of the same
    /// profile stays valid: it is rechecked against its own bound files, and
    /// the next launch binds to the current bundled profile.
    fn same_profile(&self) -> Result<()> {
        require(self.schema == 1 && self.profile.id == native_access()?.id, "vendor_profile_definition_changed")
    }

    pub fn verify(&self, app: &Application, managed: &Path) -> Result<()> {
        self.same_profile()?;
        app.verify(managed)?;
        require(observe_structure(app, &self.profile)? == *self, "vendor_profile_observed_content_changed")
    }

    /// Browser return transfers an opaque payload to an already bound owner.
    /// That owner can have installed the service since launch admission; the
    /// application and bundled installer still must match all original bytes.
    pub fn verify_callback_inputs(&self, app: &Application, managed: &Path) -> Result<()> {
        self.same_profile()?;
        app.verify(managed)?;
        let mut current = observe_structure(app, &self.profile)?;
        current.dependency.daemon = self.dependency.daemon.clone();
        require(current == *self, "vendor_profile_observed_content_changed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_defaults(defaults: serde_json::Value) -> Vec<u8> {
        let mut value: serde_json::Value = serde_json::from_slice(NATIVE_ACCESS).unwrap();
        value["registry_defaults"] = defaults;
        serde_json::to_vec(&value).unwrap()
    }

    #[test]
    fn bundled_profile_declares_only_a_current_user_known_folder_default() {
        let bundled = Bundled::parse(NATIVE_ACCESS).unwrap();
        assert_eq!(bundled.registry_defaults, vec![RegistryDefault {
            key: "HKCU\\Software\\Native Instruments\\Native Access".into(),
            name: "DownloadLocation".into(), kind: RegistryType::String,
            value: RegistryDefaultValue { known_folder: KnownFolder::Downloads } }]);
        // The saved binding is the profile without launch-time settings.
        let mut value: serde_json::Value = serde_json::from_slice(NATIVE_ACCESS).unwrap();
        let defaults = value.as_object_mut().unwrap().remove("registry_defaults").unwrap();
        assert_eq!(serde_json::to_value(&bundled.profile).unwrap(), value);
        assert_eq!(native_access().unwrap(), bundled.profile);
        // A profile file without settings stays valid.
        let plain = Bundled::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(plain.registry_defaults.is_empty());
        assert_eq!(plain.profile, bundled.profile);
        let operation = "ab".repeat(16);
        assert_eq!(launch_settings(&bundled.profile, &operation).unwrap(), serde_json::json!({"schema": 1,
            "operation_id": operation, "profile": "native_access_desktop", "registry_defaults": defaults}));
        let mut other = bundled.profile.clone(); other.id = "other_vendor".into();
        assert!(launch_settings(&other, &operation).is_err());
    }

    /// The saved profile shape exactly as main defined it before launch-time
    /// settings existed, for both rollback directions.
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct MainEraServiceRequirements {
        installer_directory: String, installer_prefix: String, installer_suffix: String,
        service: String, daemon: String, architecture: String, listeners: Vec<u16>,
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct MainEraProfile {
        schema: u32, id: String, executable: String, architecture: String, resources: Vec<String>,
        dependency: MainEraServiceRequirements, rendering: operator_model::RendererPolicy, browser_scheme: String,
    }
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct MainEraCapabilities {
        schema: u32, profile: MainEraProfile, profile_sha256: String, application: String,
        resources: BTreeMap<String, Image>, dependency: ServiceBinding,
    }

    fn saved_capabilities() -> Capabilities {
        let image = Image { artifact: Artifact { path: "/managed/environments/x/Setup.exe".into(), sha256: "cd".repeat(32) }, size: 7 };
        let profile = native_access().unwrap();
        Capabilities { schema: 1, profile_sha256: profile_sha256(&profile).unwrap(), profile, application: "ef".repeat(32),
            resources: BTreeMap::from([("Native Access.exe".into(), image.clone())]),
            dependency: ServiceBinding { installer: image, daemon: None } }
    }

    #[test]
    fn saved_launch_records_read_across_update_and_rollback() {
        // New manager writes; a rolled-back (main-era) manager reads it.
        let written = serde_json::to_value(saved_capabilities()).unwrap();
        assert!(written["profile"].get("registry_defaults").is_none());
        let old: MainEraCapabilities = serde_json::from_value(written.clone()).unwrap();
        // The saved binding and its digest are unchanged, so the supervisor's pin is too.
        assert_eq!(old.profile_sha256, "dba4302039d2d0d780c8e31d373dd17a56384c5b60469bfa0adc7178a10fb186");
        // Main-era manager wrote; the new manager reads and accepts the same profile.
        let main_era = serde_json::to_value(&old).unwrap();
        assert_eq!(main_era, written);
        let reread: Capabilities = serde_json::from_value(main_era).unwrap();
        reread.same_profile().unwrap();
        // A record saved under an earlier revision of the same profile stays usable;
        // a record for another profile does not.
        let mut earlier = saved_capabilities();
        earlier.profile.rendering = operator_model::RendererPolicy::Inherited;
        earlier.profile.resources.pop();
        earlier.same_profile().unwrap();
        earlier.profile.id = "other_vendor".into();
        assert!(earlier.same_profile().is_err());
    }

    #[test]
    fn registry_defaults_refuse_other_hives_values_types_and_extents() {
        let entry = |key: &str, name: &str| serde_json::json!(
            {"key": key, "name": name, "type": "REG_SZ", "value": {"known_folder": "Downloads"}});
        let long = format!("HKCU\\{}", "A".repeat(65));
        let deep = format!("HKCU{}", "\\A".repeat(9));
        for (case, defaults) in [
            ("hklm", serde_json::json!([entry("HKLM\\Software\\Vendor", "Value")])),
            ("hive spelled out", serde_json::json!([entry("HKEY_CURRENT_USER\\Software", "Value")])),
            ("bare hive", serde_json::json!([entry("HKCU\\", "Value")])),
            ("empty part", serde_json::json!([entry("HKCU\\Software\\\\Vendor", "Value")])),
            ("overlong part", serde_json::json!([entry(&long, "Value")])),
            ("too deep", serde_json::json!([entry(&deep, "Value")])),
            ("expansion", serde_json::json!([entry("HKCU\\Software\\%USERPROFILE%", "Value")])),
            ("name charset", serde_json::json!([entry("HKCU\\Software", "Value;x")])),
            ("part starts with a dot", serde_json::json!([entry("HKCU\\Software\\..", "Value")])),
            ("name starts with a dash", serde_json::json!([entry("HKCU\\Software", "-Value")])),
            ("overlong name", serde_json::json!([entry("HKCU\\Software", &"N".repeat(65))])),
            ("duplicate", serde_json::json!([entry("HKCU\\Software", "Value"), entry("hkcu\\software", "VALUE")])),
            ("count", serde_json::json!((0..5).map(|i| entry("HKCU\\Software", &format!("V{i}"))).collect::<Vec<_>>())),
            ("unknown folder", serde_json::json!([{"key": "HKCU\\Software", "name": "Value", "type": "REG_SZ",
                "value": {"known_folder": "Desktop"}}])),
            ("free string", serde_json::json!([{"key": "HKCU\\Software", "name": "Value", "type": "REG_SZ",
                "value": "C:\\users\\steamuser\\Downloads"}])),
            ("expand type", serde_json::json!([{"key": "HKCU\\Software", "name": "Value", "type": "REG_EXPAND_SZ",
                "value": {"known_folder": "Downloads"}}])),
            ("extra field", serde_json::json!([{"key": "HKCU\\Software", "name": "Value", "type": "REG_SZ",
                "value": {"known_folder": "Downloads"}, "command": "reg add"}])),
            ("extra value field", serde_json::json!([{"key": "HKCU\\Software", "name": "Value", "type": "REG_SZ",
                "value": {"known_folder": "Downloads", "suffix": "x"}}])),
        ] {
            assert!(Bundled::parse(&with_defaults(defaults)).is_err(), "{case}");
        }
        assert!(Bundled::parse(&with_defaults(serde_json::json!(
            [entry("HKCU\\Software\\Vendor App 2.0", "Download_Location-1")]))).is_ok());
    }
}
