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

pub fn native_access() -> Result<Profile> {
    Ok(serde_json::from_slice(NATIVE_ACCESS)?)
}

fn relative(value: &str) -> Result<&Path> {
    let path = Path::new(value);
    require(!path.is_absolute() && path.components().all(|part|
        matches!(part, std::path::Component::Normal(_))), "vendor_profile_relative_path")?;
    Ok(path)
}

fn image(path: &Path) -> Result<Image> {
    let metadata = fs::symlink_metadata(path)?;
    require(metadata.file_type().is_file() && metadata.nlink() == 1
        && path.canonicalize()? == path && metadata.len() > 0
        && metadata.len() <= 256 * 1024 * 1024, "vendor_profile_resource_alias_or_extent")?;
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

fn observe_structure(app: &Application, profile: &Profile) -> Result<Capabilities> {
    require(app.schema == 2 && app.user_selection.is_none(),
        "vendor_profile_observed_installation_required")?;
    let directory = app.executable.path.parent().ok_or("vendor_profile_application_directory")?;
    let drive = app.environment.root.join("compatdata/pfx/drive_c");
    require(directory.starts_with(&drive) && directory.canonicalize()? == directory,
        "vendor_profile_application_directory")?;
    executable(&app.executable.path, Some(&profile.architecture))?;
    let main = image(&app.executable.path)?;
    require(main.artifact == app.executable, "vendor_profile_application_changed")?;
    let mut resources = BTreeMap::from([(profile.executable.clone(), main)]);
    let mut total = resources.values().map(|i|i.size).sum::<u64>();
    for name in &profile.resources {
        let resource = image(&directory.join(relative(name)?))?;
        total = total.checked_add(resource.size).ok_or("vendor_profile_resource_budget")?;
        require(total <= 512 * 1024 * 1024, "vendor_profile_resource_budget")?;
        require(resources.insert(name.clone(), resource).is_none(), "vendor_profile_resource_duplicate")?;
    }
    let bundled = directory.join(relative(&profile.dependency.installer_directory)?);
    require(bundled.is_dir() && bundled.canonicalize()? == bundled,
        "vendor_profile_bundled_dependency_directory")?;
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
    require(candidates.len() == 1, "vendor_profile_bundled_dependency_missing_or_ambiguous")?;
    let daemon_path = drive.join(relative(&profile.dependency.daemon)?);
    let daemon = match fs::symlink_metadata(&daemon_path) {
        Ok(_) => {
            executable(&daemon_path, Some(&profile.dependency.architecture))?;
            Some(image(&daemon_path)?)
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let profile_value = serde_json::to_value(profile)?;
    Ok(Capabilities { schema: 1, profile: profile.clone(),
        profile_sha256: hex(&Sha256::digest(serde_json::to_vec(&profile_value)?)),
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
    match observe_structure(app, &profile) {
        Ok(capabilities) => Ok(Inspection { capabilities: Some(capabilities), missing_capability: None }),
        Err(error) => Ok(Inspection { capabilities: None,
            missing_capability: Some(format!("Assisted application setup needs a complete, unambiguous Windows application and bundled service: {error}")) }),
    }
}

impl Capabilities {
    pub fn verify(&self, app: &Application, managed: &Path) -> Result<()> {
        require(self.schema == 1 && self.profile == native_access()?, "vendor_profile_definition_changed")?;
        app.verify(managed)?;
        require(observe_structure(app, &self.profile)? == *self, "vendor_profile_observed_content_changed")
    }

    /// Browser return transfers an opaque payload to an already bound owner.
    /// That owner can have installed the service since launch admission; the
    /// application and bundled installer still must match all original bytes.
    pub fn verify_callback_inputs(&self, app: &Application, managed: &Path) -> Result<()> {
        require(self.schema == 1 && self.profile == native_access()?, "vendor_profile_definition_changed")?;
        app.verify(managed)?;
        let mut current = observe_structure(app, &self.profile)?;
        current.dependency.daemon = self.dependency.daemon.clone();
        require(current == *self, "vendor_profile_observed_content_changed")
    }
}
