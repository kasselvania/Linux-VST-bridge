//! Installed Linux frontend identity, selected-file custody and report export.
use super::{exchange, request, FileSelection, Query, Reply, SupportReport};
use lvb_model::contract;
use std::{io::Write, path::{Path, PathBuf}};

// Capture this immutable frontend's content once, before its first request.
// Selection readback then remains a small record read even when an older
// manager is restored and cannot send the new superseded refusal itself.
fn frontend_identity(
    home: &std::path::Path,
    executable: &std::path::Path,
) -> Result<Option<linux_vst_bridge::Artifact>, String> {
    use std::os::unix::fs::PermissionsExt;
    let root = home.join(".local/share/linux-vst-bridge/managed/software");
    if executable
        .parent()
        .is_none_or(|dir| dir.parent() != Some(root.as_path()))
        || executable.file_name() != Some(std::ffi::OsStr::new("linux-audio-compatibility-manager"))
    {
        return Ok(None);
    }
    let metadata = linux_vst_bridge::file(executable)
        .and_then(|f| Ok(f.metadata()?))
        .map_err(|_| "Frontend identity unavailable")?;
    if executable.canonicalize().ok().as_deref() != Some(executable)
        || metadata.permissions().mode() & 0o222 != 0
        || metadata.len() > 128 * 1024 * 1024
    {
        return Err("Managed frontend identity changed".into());
    }
    let sha256 =
        linux_vst_bridge::digest(executable).map_err(|_| "Frontend identity unavailable")?;
    Ok(Some(linux_vst_bridge::Artifact {
        path: executable.to_owned(),
        sha256,
    }))
}
fn frontend_selected(
    home: &std::path::Path,
    frontend: &linux_vst_bridge::Artifact,
) -> Result<bool, String> {
    use std::os::unix::fs::PermissionsExt;
    let root = home.join(".local/share/linux-vst-bridge/managed");
    let selected: linux_vst_bridge::catalogue::Software =
        linux_vst_bridge::read_json(&root.join("software.json"))
            .map_err(|_| "Selected application identity is unavailable")?;
    let current = selected
        .operator_frontend
        .as_ref()
        .ok_or("Selected frontend is unavailable")?;
    current
        .validate_record()
        .map_err(|_| "Selected frontend identity is invalid")?;
    if current.path.parent() != selected.manager.path.parent()
        || current.path.parent().and_then(|dir| dir.parent())
            != Some(root.join("software").as_path())
        || current.path.canonicalize().ok().as_ref() != Some(&current.path)
        || std::fs::metadata(&current.path)
            .map_err(|_| "Selected frontend is unavailable")?
            .permissions()
            .mode()
            & 0o222
            != 0
    {
        return Err("Selected frontend identity is invalid".into());
    }
    Ok(current.sha256 == frontend.sha256)
}
/// The exact frontend process and home whose requests this client sends.
/// Keep one client for the process lifetime so an update cannot replace its identity.
pub struct Client {
    home: PathBuf,
    executable: PathBuf,
    frontend: Option<linux_vst_bridge::Artifact>,
}
impl Client {
    /// Preserve the existing frontend behavior, including development executables.
    /// Packaged live frontends should use [`Self::managed`].
    pub fn new(home: PathBuf, executable: PathBuf) -> Result<Self, String> {
        let frontend = frontend_identity(&home, &executable)?;
        Ok(Self { home, executable, frontend })
    }

    /// Require the installed frontend identity used by packaged live windows.
    pub fn managed(home: PathBuf, executable: PathBuf) -> Result<Self, String> {
        let client = Self::new(home, executable)?;
        if client.frontend.is_none() {
            return Err("Open the installed Bridge manager to continue".into());
        }
        Ok(client)
    }

    /// Whether this immutable frontend is still selected. Updater adapters use
    /// the same admission check before touching saved links or staging a build.
    pub fn current_frontend(&self) -> Result<bool, String> {
        match &self.frontend {
            Some(frontend) => frontend_selected(&self.home, frontend),
            None => Err("Open the installed Bridge manager to continue".into()),
        }
    }

    /// Read the exact completed export through the same custody check as Save.
    /// This is local file I/O and belongs on the frontend's persistent worker.
    pub fn preview_support_report(&self, report: &SupportReport) -> Result<super::report::Contents, String> {
        if let Some(frontend) = &self.frontend {
            if !frontend_selected(&self.home, frontend)? {
                return Err("Open the current manager to read this report.".into());
            }
        }
        super::report::Contents::validated(report, support_report(&self.home, report)?)
    }

    /// Dispatch through the existing service/installed-manager contract.
    pub fn call(&self, query: Query, selection: &impl FileSelection) -> Result<Reply, String> {
        self.call_with(query, selection, contract::Installed::home(&self.home))
    }

    fn call_with(
        &self,
        mut query: Query,
        selection: &impl FileSelection,
        transport: impl contract::Transport,
    ) -> Result<Reply, String> {
        let home = &self.home;
        if matches!(query, Query::OpenCurrentManager) {
            launch_selected_with(home, &self.executable, |path| {
                std::process::Command::new(path).spawn().map(|_| ())
                    .map_err(|_| "Selected application could not be opened".into())
            })?;
            return Ok(Reply::CurrentManagerOpened);
        }
        let identity = &self.frontend;
        if let Some(frontend) = identity {
            if !frontend_selected(home, frontend)? {
                return Ok(Reply::FrontendSuperseded);
            }
        }
        if let Query::Action(ref action) = query {
            if let lvb_model::operator_model::Action::PresetFiles {command,..} = &action.action {
                let (name,input) = if matches!(command,lvb_model::operator_model::PresetCommand::Import) {
                    let Some(path)=selection.pick_preset() else {return Ok(Reply::Cancelled)};
                    let name=path.file_name().and_then(|name|name.to_str()).ok_or("Selected preset name is unavailable")?.to_owned();
                    let source=open_selected(&path)?;
                    if let Some(frontend)=identity {
                        if !frontend_selected(home,frontend)? {return Ok(Reply::FrontendSuperseded)}
                    }
                    (Some(name),Some(source))
                } else {(None,None)};
                let request=contract::Request::Preset {request:action.clone(),name};
                let reply=match contract::Client::new(transport).call(&request,input) {
                    Ok(reply)=>reply,
                    Err(contract::Failure::Error(text))=>return Err(text),
                    Err(contract::Failure::Unavailable)=>return Err("Bridge service is unavailable".into()),
                    Err(contract::Failure::FrontendSuperseded)=>return Ok(Reply::FrontendSuperseded),
                };
                let contract::Reply::Preset(result)=reply else {return Err("Unexpected preset response".into())};
                if let Some(folder)=result.folder {
                    if let Some(frontend)=identity {
                        if !frontend_selected(home,frontend)? {return Ok(Reply::FrontendSuperseded)}
                    }
                    if !folder.is_absolute() {return Err("Preset folder response is invalid".into())}
                    std::process::Command::new("xdg-open").arg(&folder).spawn()
                        .map_err(|_|"The file manager could not open the preset folder")?;
                }
                return Ok(Reply::Receipt(result.receipt));
            }
        }
        if let Query::SaveSupportReport(report) = &query {
            let value = support_report(home, report)?;
            let Some(path) = selection.save_report(report) else {
                return Ok(Reply::Cancelled);
            };
            if let Some(frontend) = identity {
                if !frontend_selected(home, frontend)? { return Ok(Reply::FrontendSuperseded); }
            }
            save_support_report(&path, &value)?;
            return Ok(Reply::ReportSaved(path));
        }
        if let Query::Action(request) = &mut query {
            if let lvb_model::operator_model::Action::ProgramChoose {
                executable: program,
                ..
            } = &mut request.action
            {
                if program.path.as_os_str().is_empty() {
                    let Some(path) = selection.pick_program() else {
                        return Ok(Reply::Cancelled);
                    };
                    let path = path
                        .canonicalize()
                        .map_err(|_| "Selected program is unavailable")?;
                    let source = open_selected(&path)?;
                    if source
                        .metadata()
                        .map_err(|_| "Selected program is unavailable")?
                        .len()
                        > 256 * 1024 * 1024
                    {
                        return Err("Choose a Windows program smaller than 256 MiB".into());
                    }
                    *program = lvb_model::operator_model::ProgramExecutable {
                        sha256: linux_vst_bridge::digest(&path)
                            .map_err(|_| "Selected program could not be read")?,
                        path,
                    };
                    if let Some(frontend) = identity {
                        if !frontend_selected(home, frontend)? {
                            return Ok(Reply::FrontendSuperseded);
                        }
                    }
                }
            }
        }
        let (request, input) = if matches!(query, Query::PickInstaller) {
            let selected = selection.pick_installer();
            let Some(path) = selected else {
                return Ok(Reply::Cancelled);
            };
            let basename = path
                .file_name()
                .and_then(|part| part.to_str())
                .ok_or("Selected installer name is unavailable")?
                .to_owned();
            let source = open_selected(&path)?;
            // A file picker can outlive a package update or restoration. No import
            // has started yet: recheck selection after the dialog, before dispatch.
            if let Some(frontend) = identity {
                if !frontend_selected(home, frontend)? {
                    return Ok(Reply::FrontendSuperseded);
                }
            }
            (
                contract::Request::ImportInstaller { name: basename },
                Some(source),
            )
        } else {
            (request(query), None)
        };
        // Exchange through the owner contract. An unavailable service does not
        // authorize a command fallback or replay an uncertain action.
        exchange(transport, &request, input)
    }
}
/// Open only the owner-selected, verified frontend through its installed route.
pub fn launch_selected_with(home: &Path, executable: &Path,
    launch: impl FnOnce(&Path) -> Result<(), String>) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let route = home.join(".local/bin/linux-audio-compatibility-manager");
    let target = std::fs::canonicalize(&route)
        .map_err(|_| "Selected application route is unavailable".to_owned())?;
    if target == Path::new("/usr/bin/linux-audio-compatibility-manager")
        || executable == target {
        return Err("Selected application route points back to the package launcher".into());
    }
    let root = home.join(".local/share/linux-vst-bridge/managed");
    let software_path = root.join("software.json");
    let (software, identity) = linux_vst_bridge::read_json_identity::<
        linux_vst_bridge::catalogue::Software>(&software_path)
        .map_err(|_| "Selected application identity is invalid".to_owned())?;
    software.validate_record().map_err(|_| "Selected application identity is invalid".to_owned())?;
    let frontend = software.operator_frontend.as_ref()
        .ok_or_else(|| "Selected application is unavailable".to_owned())?;
    let metadata = std::fs::metadata(&frontend.path)
        .map_err(|_| "Selected application identity is unavailable".to_owned())?;
    if frontend.path.parent() != software.manager.path.parent()
        || !frontend.path.starts_with(root.join("software"))
        || frontend.path.canonicalize().ok().as_ref() != Some(&frontend.path)
        || target != frontend.path
        || metadata.permissions().mode() & 0o222 != 0 {
        return Err("Selected application identity is invalid".into());
    }
    frontend.verify().map_err(|_| "Selected application changed; reopen Setup".to_owned())?;
    let unchanged = linux_vst_bridge::read_json_identity::<
        linux_vst_bridge::catalogue::Software>(&software_path);
    if !unchanged.is_ok_and(|(again, digest)|again==software && digest==identity)
        || std::fs::canonicalize(&route).ok().as_ref() != Some(&frontend.path) {
        return Err("Selected application changed; reopen Setup".into());
    }
    launch(&frontend.path)
}


// Read only the manager-created, bounded export named by the exact completion.
// A stale/replaced report, symlink or caller-selected source is never shared.
fn support_report(home: &std::path::Path, report: &SupportReport) -> Result<Vec<u8>, String> {
    if !linux_vst_bridge::valid_hex(&report.id, 32)
        || !linux_vst_bridge::valid_hex(&report.sha256, 64)
    {
        return Err("Problem report identity is unavailable. Create a new report.".into());
    }
    let path = home
        .join(".local/share/linux-vst-bridge/managed/support-exports")
        .join(format!("{}.json", report.id));
    let (value, digest): (serde_json::Value, String) = linux_vst_bridge::read_json_identity(&path)
        .map_err(|_| "Problem report is unavailable. Create a new report.")?;
    if digest != report.sha256 || value["schema"] != 2 {
        return Err("Problem report changed. Create a new report.".into());
    }
    // Preview and Save share this exact bounded formatting of owner data.
    super::report::encode(&value)
}
fn save_support_report(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| "Could not save the report. Choose a new, writable filename.")?;
    if file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        let _ = std::fs::remove_file(path);
        return Err("Could not finish saving the report. Choose another location.".into());
    }
    Ok(())
}
#[path = "../../service/src/owner/installer_source.rs"]
mod installer_source;

fn open_selected(path: &std::path::Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "Choose a regular local file; links are not accepted")?;
    let md = f.metadata().map_err(|_| "Cannot read selected file")?;
    if !md.is_file()
        || !installer_source::accepted_owner(md.uid(), md.mode(), unsafe { libc::getuid() })
    {
        return Err("Choose a regular file owned by you or protected by the system".into());
    }
    Ok(f)
}
#[cfg(test)]
mod tests {
    use super::*;

    struct NeverTransport;
    impl contract::Transport for NeverTransport {
        fn exchange(
            &self,
            _: &contract::Request,
            _: Option<std::fs::File>,
        ) -> Result<Vec<u8>, contract::Failure> {
            panic!("no request may be dispatched")
        }
    }
    struct Choose<'a>(&'a dyn Fn() -> Option<PathBuf>);
    impl FileSelection for Choose<'_> {
        fn pick_preset(&self)->Option<PathBuf> {self.0()}
        fn pick_installer(&self) -> Option<PathBuf> {
            self.0()
        }
        fn pick_program(&self) -> Option<PathBuf> {
            self.0()
        }
        fn save_report(&self, _: &SupportReport) -> Option<PathBuf> {
            self.0()
        }
    }
    fn preset_query()->Query {
        use lvb_model::operator_model::*;
        Query::Action(Request {schema:OPERATOR_SCHEMA,state_token:"exact-state".into(),action:Action::PresetFiles {
            product:PresetProduct {environment:"ab".repeat(16),module_sha256:"cd".repeat(32),class_id:"ef".repeat(16)},
            expected_root:"11".repeat(32),command:PresetCommand::Import}})
    }
    #[test]
    fn preset_selection_cancel_never_dispatches() {
        let client=Client::new(PathBuf::from("/tmp"),PathBuf::from("/development/frontend")).unwrap();
        assert!(matches!(client.call_with(preset_query(),&Choose(&||None),NeverTransport),Ok(Reply::Cancelled)));
    }
    fn program_query() -> Query {
        Query::Action(lvb_model::operator_model::Request {
            schema: lvb_model::operator_model::OPERATOR_SCHEMA,
            state_token: "exact-state".into(),
            action: lvb_model::operator_model::Action::ProgramChoose {
                environment: "exact-setup".into(),
                expected_environment: "exact-revision".into(),
                executable: lvb_model::operator_model::ProgramExecutable {
                    path: PathBuf::new(),
                    sha256: String::new(),
                },
            },
        })
    }

    #[test]
    fn problem_report_reads_only_exact_bounded_owned_export_and_never_overwrites() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let home = std::env::temp_dir().join(format!(
            "lvb-report-{}",
            linux_vst_bridge::random_id().unwrap()
        ));
        let reports = home.join(".local/share/linux-vst-bridge/managed/support-exports");
        std::fs::create_dir_all(&reports).unwrap();
        let path = reports.join(format!("{}.json", "ab".repeat(16)));
        std::fs::write(&path, b"{\"schema\":2,\"setups\":[]}").unwrap();
        let mut report = SupportReport {
            id: "ab".repeat(16),
            sha256: linux_vst_bridge::digest(&path).unwrap(),
        };
        let bytes = support_report(&home, &report).unwrap();
        let client = Client::new(home.clone(), home.join("development-frontend")).unwrap();
        assert_eq!(client.preview_support_report(&report).unwrap().json.as_bytes(), bytes);
        let destination = home.join("report.json");
        save_support_report(&destination, &bytes).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), bytes);
        assert_eq!(
            std::fs::metadata(&destination)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert!(save_support_report(&destination, b"overwrite").is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), bytes);
        std::fs::write(&path, b"{\"schema\":2,\"changed\":true}").unwrap();
        assert!(support_report(&home, &report).is_err());
        assert!(client.preview_support_report(&report).is_err());
        report.sha256 = linux_vst_bridge::digest(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        symlink(&destination, &path).unwrap();
        assert!(support_report(&home, &report).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(support_report(&home, &report).is_err());
        std::fs::remove_dir(&path).unwrap();
        std::fs::File::create(&path)
            .unwrap()
            .set_len(8 * 1024 * 1024 + 1)
            .unwrap();
        assert!(support_report(&home, &report).is_err());
        report.id = "../private".into();
        assert!(support_report(&home, &report).is_err());
        std::fs::remove_dir_all(&home).unwrap();
    }
    #[test]
    fn frontend_content_binding_detects_restore_without_help_from_old_manager() {
        use std::os::unix::fs::PermissionsExt;
        let home = std::env::temp_dir().join(format!(
            "frontend-selection-{}",
            linux_vst_bridge::random_id().unwrap()
        ));
        std::fs::create_dir(&home).unwrap();
        let home = home.canonicalize().unwrap();
        let root = home.join(".local/share/linux-vst-bridge/managed");
        let dir = root.join("software/current");
        std::fs::create_dir_all(&dir).unwrap();
        let executable = dir.join("linux-audio-compatibility-manager");
        std::fs::write(&executable, b"current frontend content").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o400)).unwrap();
        let identity = frontend_identity(&home, &executable).unwrap().unwrap();
        let artifact = identity.clone();
        let mut selected = linux_vst_bridge::catalogue::Software {
            installer_launch: None,
            preparation_kit: None,
            operator_frontend: Some(artifact.clone()),
            manager: artifact.clone(),
            supervisor: artifact.clone(),
            ownership: artifact.clone(),
            host: artifact.clone(),
            source_manifest: artifact.clone(),
            source_sha256: artifact.sha256,
            native_catalogue: None,
        };
        let selection = root.join("software.json");
        linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
        assert!(frontend_selected(&home, &identity).unwrap());
        let client = Client::managed(home.clone(), executable.clone()).unwrap();
        let current = selected.clone();
        let chosen = home.join("selected.exe");
        std::fs::write(&chosen, b"selected fixture bytes").unwrap();
        let previous = root.join("software/previous/linux-audio-compatibility-manager");
        std::fs::create_dir_all(previous.parent().unwrap()).unwrap();
        std::fs::write(&previous, b"previous frontend content").unwrap();
        std::fs::set_permissions(&previous, std::fs::Permissions::from_mode(0o400)).unwrap();
        let predecessor = linux_vst_bridge::Artifact {
            path: previous,
            sha256: linux_vst_bridge::digest(
                &root.join("software/previous/linux-audio-compatibility-manager"),
            )
            .unwrap(),
        };
        selected.operator_frontend = Some(predecessor.clone());
        selected.manager = predecessor;
        linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
        assert!(matches!(
            client.call_with(
                Query::PickInstaller,
                &Choose(&|| panic!("superseded frontend cannot open a picker")),
                NeverTransport
            ),
            Ok(Reply::FrontendSuperseded)
        ));
        // Exercise the real dispatch boundary for both native pickers. A package
        // restore while the picker is open must prevent the subsequent request.
        for query in [Query::PickInstaller, program_query()] {
            linux_vst_bridge::atomic_json(&selection, &current).unwrap();
            let picker = || {
                linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
                Some(chosen.clone())
            };
            assert!(matches!(
                client.call_with(query, &Choose(&picker), NeverTransport),
                Ok(Reply::FrontendSuperseded)
            ));
        }
        let reports = root.join("support-exports");
        std::fs::create_dir(&reports).unwrap();
        let source = reports.join(format!("{}.json", "ab".repeat(16)));
        std::fs::write(&source, br#"{"schema":2,"setups":[]}"#).unwrap();
        let report = SupportReport { id: "ab".repeat(16), sha256: linux_vst_bridge::digest(&source).unwrap() };
        assert!(client.preview_support_report(&report).is_err(), "superseded window cannot preview");
        linux_vst_bridge::atomic_json(&selection, &current).unwrap();
        assert!(client.preview_support_report(&report).is_ok());
        let destination = home.join("saved-report.json");
        let picker = || {
            linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
            Some(destination.clone())
        };
        assert!(matches!(client.call_with(Query::SaveSupportReport(report), &Choose(&picker), NeverTransport),
            Ok(Reply::FrontendSuperseded)));
        assert!(!destination.exists(), "supersession during Save cannot create a file");
        linux_vst_bridge::atomic_json(&selection,&current).unwrap();
        assert!(matches!(client.call_with(preset_query(),&Choose(&|| {
            linux_vst_bridge::atomic_json(&selection,&selected).unwrap();Some(chosen.clone())
        }),NeverTransport),Ok(Reply::FrontendSuperseded)));
        selected.operator_frontend = Some(identity.clone());
        selected.manager = identity.clone();
        linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
        assert!(frontend_selected(&home, &identity).unwrap());
        assert!(frontend_identity(&home, &home.join("unmanaged/frontend"))
            .unwrap()
            .is_none());
        assert!(Client::managed(home.clone(), home.join("unmanaged/frontend")).is_err());
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(frontend_identity(&home, &executable).is_err());
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn chosen_installer_reaches_transport_as_an_open_descriptor() {
        use std::io::Read;
        let root = std::env::temp_dir().join(format!(
            "lvb-client-import-{}",
            linux_vst_bridge::random_id().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let selected = root.join("selected.msi");
        std::fs::write(&selected, b"exact selected installer bytes").unwrap();
        let client = Client::new(root.clone(), root.join("development-frontend")).unwrap();
        struct Reading;
        impl contract::Transport for Reading {
            fn exchange(
                &self,
                request: &contract::Request,
                input: Option<std::fs::File>,
            ) -> Result<Vec<u8>, contract::Failure> {
                assert_eq!(
                    request,
                    &contract::Request::ImportInstaller {
                        name: "selected.msi".into()
                    }
                );
                let mut bytes = Vec::new();
                input.unwrap().read_to_end(&mut bytes).unwrap();
                assert_eq!(bytes, b"exact selected installer bytes");
                Err(contract::Failure::FrontendSuperseded)
            }
        }
        assert!(matches!(
            client.call_with(
                Query::PickInstaller,
                &Choose(&|| Some(selected.clone())),
                Reading
            ),
            Ok(Reply::FrontendSuperseded)
        ));
        assert!(matches!(
            client.call_with(Query::PickInstaller, &Choose(&|| None), NeverTransport),
            Ok(Reply::Cancelled)
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn selected_descriptor_refuses_links_and_directories() {
        let root = std::env::temp_dir().join(format!("mf2-picker-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let p = root.join("selected.exe");
        std::fs::write(&p, b"bytes").unwrap();
        assert!(open_selected(&p).is_ok());
        let l = root.join("link");
        std::os::unix::fs::symlink(&p, &l).unwrap();
        assert!(open_selected(&l).is_err());
        assert!(open_selected(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
