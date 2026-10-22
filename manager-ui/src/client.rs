//! Fixed manager entry point; explicit program choices use structured requests.
//! Every exchange goes through the manager's client contract.
use crate::model::{Activity, CurrentOverview, CurrentProductDetail, InstallerImportResult, InteractiveOverview, Pulse, Receipt, Request, Snapshot};
use crate::screen_model::ProductKey;
use linux_vst_bridge::service::contract;
use std::{
    io::Write,
    sync::{mpsc::Sender, Arc, OnceLock},
};
#[derive(Clone, Debug)]
pub struct SupportReport {
    pub id: String,
    pub sha256: String,
}
pub enum Reply {
    Overview(Box<InteractiveOverview>),
    CurrentOverview(Box<CurrentOverview>),
    Product(Box<CurrentProductDetail>),
    Snapshot(Box<Snapshot>),
    Activity(Activity),
    Pulse(Pulse),
    Receipt(Receipt),
    Imported(InstallerImportResult),
    ReportSaved(std::path::PathBuf),
    Cancelled,
    FrontendSuperseded,
    Error(String),
}
pub enum Query {
    SaveSupportReport(SupportReport),
    PickInstaller,
    Overview,
    CurrentOverview,
    Product(ProductKey),
    Snapshot,
    Activity,
    Pulse,
    Action(Request),
}
// Capture this immutable frontend's content once, before its first request.
// Selection readback then remains a small record read even when an older
// manager is restored and cannot send the new superseded refusal itself.
static FRONTEND_IDENTITY: OnceLock<Result<Option<linux_vst_bridge::Artifact>, String>> = OnceLock::new();
fn frontend_identity(home: &std::path::Path, executable: &std::path::Path)
    -> Result<Option<linux_vst_bridge::Artifact>, String> {
    use std::os::unix::fs::PermissionsExt;
    let root = home.join(".local/share/linux-vst-bridge/managed/software");
    if executable.parent().is_none_or(|dir| dir.parent() != Some(root.as_path()))
        || executable.file_name() != Some(std::ffi::OsStr::new("linux-audio-compatibility-manager")) {
        return Ok(None);
    }
    let metadata = linux_vst_bridge::file(executable).and_then(|f| Ok(f.metadata()?))
        .map_err(|_| "Frontend identity unavailable")?;
    if executable.canonicalize().ok().as_deref() != Some(executable)
        || metadata.permissions().mode() & 0o222 != 0 || metadata.len() > 128 * 1024 * 1024 {
        return Err("Managed frontend identity changed".into());
    }
    let sha256 = linux_vst_bridge::digest(executable).map_err(|_| "Frontend identity unavailable")?;
    Ok(Some(linux_vst_bridge::Artifact { path:executable.to_owned(), sha256 }))
}
fn frontend_selected(home: &std::path::Path, frontend: &linux_vst_bridge::Artifact)
    -> Result<bool, String> {
    use std::os::unix::fs::PermissionsExt;
    let root = home.join(".local/share/linux-vst-bridge/managed");
    let selected: linux_vst_bridge::catalogue::Software =
        linux_vst_bridge::read_json(&root.join("software.json"))
            .map_err(|_| "Selected application identity is unavailable")?;
    let current = selected.operator_frontend.as_ref().ok_or("Selected frontend is unavailable")?;
    current.validate_record().map_err(|_| "Selected frontend identity is invalid")?;
    if current.path.parent() != selected.manager.path.parent()
        || current.path.parent().and_then(|dir|dir.parent()) != Some(root.join("software").as_path())
        || current.path.canonicalize().ok().as_ref() != Some(&current.path)
        || std::fs::metadata(&current.path).map_err(|_| "Selected frontend is unavailable")?
            .permissions().mode() & 0o222 != 0 {
        return Err("Selected frontend identity is invalid".into());
    }
    Ok(current.sha256 == frontend.sha256)
}
impl From<contract::Reply> for Reply {
    fn from(reply: contract::Reply) -> Self {
        match reply {
            contract::Reply::Overview(overview) => Self::Overview(overview),
            contract::Reply::CurrentOverview(overview) => Self::CurrentOverview(overview),
            contract::Reply::Product(detail) => Self::Product(detail),
            contract::Reply::Snapshot(snapshot) => Self::Snapshot(snapshot),
            contract::Reply::Activity(activity) => Self::Activity(activity),
            contract::Reply::Pulse(pulse) => Self::Pulse(pulse),
            contract::Reply::Receipt(receipt) => Self::Receipt(receipt),
            contract::Reply::Imported(imported) => Self::Imported(imported),
            contract::Reply::Delivered => Self::Error("Unexpected manager reply".into()),
        }
    }
}
/// The manager's request for a query that leaves this window.
fn request(query: Query, installer: Option<&str>) -> contract::Request {
    use contract::Read;
    match query {
        Query::Overview => contract::Request::Read(Read::Overview),
        Query::CurrentOverview => contract::Request::Read(Read::CurrentOverview),
        Query::Product(key) => contract::Request::Read(Read::Product(contract::ProductKey {
            environment: key.environment, module_sha256: key.module_sha256, class_id: key.class_id })),
        Query::Snapshot => contract::Request::Read(Read::Snapshot),
        Query::Activity => contract::Request::Read(Read::Activity),
        Query::Pulse => contract::Request::Read(Read::Pulse),
        Query::Action(request) => contract::Request::Action(request),
        Query::PickInstaller => contract::Request::ImportInstaller { name: installer.unwrap_or_default().into() },
        Query::SaveSupportReport(_) => unreachable!("problem reports are saved locally"),
    }
}
/// One exchange with the manager through `transport`, as this window shows it.
fn exchange(transport: impl contract::Transport, request: &contract::Request,
    input: Option<std::fs::File>) -> Result<Reply, String> {
    match contract::Client::new(transport).call(request, input) {
        Ok(reply) => Ok(reply.into()),
        Err(contract::Failure::FrontendSuperseded) => Ok(Reply::FrontendSuperseded),
        Err(contract::Failure::Error(error)) => Err(error),
    }
}
fn call(mut query: Query) -> Result<Reply, String> {
    let home = std::env::var_os("HOME").ok_or("Home directory unavailable")?;
    let home = std::path::PathBuf::from(home);
    let identity = FRONTEND_IDENTITY.get_or_init(|| {
        let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
        frontend_identity(&home, &executable)
    }).as_ref().map_err(Clone::clone)?;
    if let Some(frontend) = identity {
        if !frontend_selected(&home, frontend)? { return Ok(Reply::FrontendSuperseded); }
    }
    if let Query::SaveSupportReport(report) = &query {
        let value = support_report(&home, report)?;
        let Some(path) = rfd::FileDialog::new().set_title("Save problem report to send")
            .set_file_name(format!("linux-vst-bridge-problem-{}.json", report.id))
            .add_filter("Problem report", &["json"]).save_file() else { return Ok(Reply::Cancelled) };
        save_support_report(&path, &value)?;
        return Ok(Reply::ReportSaved(path));
    }
    if let Query::Action(request) = &mut query {
        if let crate::model::Action::ProgramChoose {executable:program,..} = &mut request.action {
            if program.path.as_os_str().is_empty() {
                let Some(path) = rfd::FileDialog::new().set_title("Choose Windows program")
                    .add_filter("Windows programs", &["exe"]).pick_file() else {return Ok(Reply::Cancelled)};
                let path = path.canonicalize().map_err(|_|"Selected program is unavailable")?;
                let source = open_selected(&path)?;
                if source.metadata().map_err(|_|"Selected program is unavailable")?.len() > 256 * 1024 * 1024 {
                    return Err("Choose a Windows program smaller than 256 MiB".into());
                }
                *program = crate::model::ProgramExecutable {sha256:linux_vst_bridge::digest(&path)
                    .map_err(|_|"Selected program could not be read")?,path};
                if let Some(frontend) = identity {
                    if !frontend_selected(&home, frontend)? { return Ok(Reply::FrontendSuperseded); }
                }
            }
        }
    }
    let selected = if matches!(query, Query::PickInstaller) {
        let selected = rfd::FileDialog::new()
            .set_title("Choose Windows installer")
            .add_filter("Windows installers", &["exe", "msi"])
            .pick_file();
        let Some(path) = selected else {
            return Ok(Reply::Cancelled);
        };
        let basename = path.file_name().and_then(|part| part.to_str())
            .ok_or("Selected installer name is unavailable")?.to_owned();
        let source = open_selected(&path)?;
        // A file picker can outlive a package update or restoration. No import
        // has started yet: recheck selection after the dialog, before dispatch.
        if let Some(frontend) = identity {
            if !frontend_selected(&home, frontend)? { return Ok(Reply::FrontendSuperseded); }
        }
        Some((source, basename))
    } else {
        None
    };
    let request = request(query, selected.as_ref().map(|(_, name)| name.as_str()));
    exchange(contract::Subprocess::installed(&home), &request, selected.map(|(file, _)| file))
}
// Read only the manager-created, bounded export named by the exact completion.
// A stale/replaced report, symlink or caller-selected source is never shared.
fn support_report(home: &std::path::Path, report: &SupportReport) -> Result<Vec<u8>, String> {
    if !linux_vst_bridge::valid_hex(&report.id,32)
        || !linux_vst_bridge::valid_hex(&report.sha256,64) {
        return Err("Problem report identity is unavailable. Create a new report.".into());
    }
    let path = home.join(".local/share/linux-vst-bridge/managed/support-exports")
        .join(format!("{}.json",report.id));
    let (value, digest): (serde_json::Value, String) = linux_vst_bridge::read_json_identity(&path)
        .map_err(|_| "Problem report is unavailable. Create a new report.")?;
    if digest != report.sha256 || value["schema"] != 2 {
        return Err("Problem report changed. Create a new report.".into());
    }
    serde_json::to_vec(&value).map_err(|_| "Problem report could not be read.".into())
}
fn save_support_report(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
        .open(path).map_err(|_| "Could not save the report. Choose a new, writable filename.")?;
    if file.write_all(bytes).and_then(|()|file.sync_all()).is_err() {
        let _ = std::fs::remove_file(path);
        return Err("Could not finish saving the report. Choose another location.".into());
    }
    Ok(())
}
pub type WindowWake = Arc<dyn Fn() + Send + Sync>;

fn deliver(reply: Reply, sender: &Sender<Reply>, ctx: &eframe::egui::Context,
    window_wake: Option<&WindowWake>) {
    if sender.send(reply).is_ok() {
        ctx.request_repaint();
        // A completed request must wake the actual native window as well as
        // egui's pass-based repaint queue. Input must not be needed to consume
        // a reply that is already waiting in the channel.
        if let Some(wake) = window_wake { wake(); }
    }
}

pub fn send(query: Query, sender: Sender<Reply>, ctx: eframe::egui::Context,
    window_wake: Option<WindowWake>) {
    std::thread::spawn(move || {
        let result = call(query).unwrap_or_else(Reply::Error);
        deliver(result, &sender, &ctx, window_wake.as_ref());
    });
}

#[path = "../../bridge-manager/src/installer_source.rs"]
mod installer_source;

fn open_selected(path: &std::path::Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "Choose a regular local installer file; links are not accepted")?;
    let md = f.metadata().map_err(|_| "Cannot read selected file")?;
    if !md.is_file()
        || !installer_source::accepted_owner(md.uid(), md.mode(), unsafe { libc::getuid() })
    {
        return Err("Choose a regular installer owned by you or protected by the system".into());
    }
    Ok(f)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_report_reads_only_exact_bounded_owned_export_and_never_overwrites() {
        use std::os::unix::fs::{symlink,PermissionsExt};
        let home = std::env::temp_dir().join(format!("lvb-report-{}",linux_vst_bridge::random_id().unwrap()));
        let reports = home.join(".local/share/linux-vst-bridge/managed/support-exports");
        std::fs::create_dir_all(&reports).unwrap();
        let path = reports.join(format!("{}.json","ab".repeat(16)));
        std::fs::write(&path,b"{\"schema\":2,\"setups\":[]}").unwrap();
        let mut report = SupportReport {id:"ab".repeat(16),sha256:linux_vst_bridge::digest(&path).unwrap()};
        let bytes = support_report(&home,&report).unwrap();
        let destination = home.join("report.json");
        save_support_report(&destination,&bytes).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(),bytes);
        assert_eq!(std::fs::metadata(&destination).unwrap().permissions().mode() & 0o777,0o600);
        assert!(save_support_report(&destination,b"overwrite").is_err());
        assert_eq!(std::fs::read(&destination).unwrap(),bytes);
        std::fs::write(&path,b"{\"schema\":2,\"changed\":true}").unwrap();
        assert!(support_report(&home,&report).is_err());
        report.sha256=linux_vst_bridge::digest(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        symlink(&destination,&path).unwrap();
        assert!(support_report(&home,&report).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(support_report(&home,&report).is_err());
        std::fs::remove_dir(&path).unwrap();
        std::fs::File::create(&path).unwrap().set_len(8*1024*1024+1).unwrap();
        assert!(support_report(&home,&report).is_err());
        report.id="../private".into();
        assert!(support_report(&home,&report).is_err());
        std::fs::remove_dir_all(&home).unwrap();
    }
    #[test]
    fn completion_wakes_window_when_egui_repaint_is_already_queued() {
        use std::sync::{atomic::{AtomicUsize, Ordering}, mpsc, Mutex};
        let ctx = eframe::egui::Context::default();
        let repaint_count = Arc::new(AtomicUsize::new(0));
        let observed = repaint_count.clone();
        ctx.set_request_repaint_callback(move |_| { observed.fetch_add(1, Ordering::SeqCst); });
        ctx.request_repaint();
        let already_queued = repaint_count.load(Ordering::SeqCst);
        let (sender, receiver) = mpsc::channel();
        let receiver = Mutex::new(receiver);
        let wakes = Arc::new(AtomicUsize::new(0));
        let observed = wakes.clone();
        let wake: WindowWake = Arc::new(move || {
            // The native wake must see the completed result already queued.
            assert!(matches!(receiver.lock().unwrap().try_recv(), Ok(Reply::Cancelled)));
            observed.fetch_add(1, Ordering::SeqCst);
        });
        deliver(Reply::Cancelled, &sender, &ctx, Some(&wake));
        assert_eq!(repaint_count.load(Ordering::SeqCst), already_queued);
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn completed_request_does_not_wake_a_closed_client() {
        use std::sync::{atomic::{AtomicUsize, Ordering}, mpsc};
        let ctx = eframe::egui::Context::default();
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        let wakes = Arc::new(AtomicUsize::new(0));
        let observed = wakes.clone();
        let wake: WindowWake = Arc::new(move || { observed.fetch_add(1, Ordering::SeqCst); });
        deliver(Reply::Cancelled, &sender, &ctx, Some(&wake));
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn frontend_content_binding_detects_restore_without_help_from_old_manager() {
        use std::os::unix::fs::PermissionsExt;
        let home = std::env::temp_dir().join(format!("frontend-selection-{}",
            linux_vst_bridge::random_id().unwrap()));
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
            installer_launch:None, preparation_kit:None, operator_frontend:Some(artifact.clone()),
            manager:artifact.clone(), supervisor:artifact.clone(), ownership:artifact.clone(),
            host:artifact.clone(), source_manifest:artifact.clone(), source_sha256:artifact.sha256,
            native_catalogue:None,
        };
        let selection = root.join("software.json");
        linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
        assert!(frontend_selected(&home, &identity).unwrap());
        let previous = root.join("software/previous/linux-audio-compatibility-manager");
        std::fs::create_dir_all(previous.parent().unwrap()).unwrap();
        std::fs::write(&previous, b"previous frontend content").unwrap();
        std::fs::set_permissions(&previous, std::fs::Permissions::from_mode(0o400)).unwrap();
        let predecessor = linux_vst_bridge::Artifact { path:previous,
            sha256:linux_vst_bridge::digest(&root.join("software/previous/linux-audio-compatibility-manager")).unwrap() };
        selected.operator_frontend = Some(predecessor.clone());
        selected.manager = predecessor;
        linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
        // The initial picker request saw the current generation above. Its
        // later dispatch must observe this restored selection instead.
        let mut import_started = false;
        if frontend_selected(&home, &identity).unwrap() { import_started = true; }
        assert!(!import_started);
        selected.operator_frontend = Some(identity.clone());
        selected.manager = identity.clone();
        linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
        assert!(frontend_selected(&home, &identity).unwrap());
        assert!(frontend_identity(&home, &home.join("unmanaged/frontend")).unwrap().is_none());
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(frontend_identity(&home, &executable).is_err());
        std::fs::remove_dir_all(home).unwrap();
    }

    /// Answers every exchange with one fixed result, as a manager would.
    struct Answering(Result<Vec<u8>, contract::Failure>);
    impl contract::Transport for Answering {
        fn exchange(&self, _: &contract::Request, _: Option<std::fs::File>)
            -> Result<Vec<u8>, contract::Failure> { self.0.clone() }
    }
    #[test]
    fn window_queries_reach_the_manager_through_the_contract() {
        let snapshot: Snapshot = serde_json::from_str(
            include_str!("../examples/library-preview.json")).unwrap();
        let key = ProductKey::from(&snapshot.products[0]);
        assert_eq!(request(Query::Product(key.clone()), None).command(),
            ["operator", "product", &key.environment, &key.module_sha256, &key.class_id]);
        assert_eq!(request(Query::PickInstaller, Some("Setup.exe")).command(), ["import-installer", "Setup.exe"]);
        let pulse = request(Query::Pulse, None);
        let reply = serde_json::to_vec(&serde_json::json!({"schema":crate::model::OPERATOR_SCHEMA,
            "service_state":"active","dsp":0,"keepers":2,"maintenance":0,
            "pending_transactions":0,"cleanup_unconfirmed":false,"current_generation":"exact-generation",
            "operation":null,"operation_live":false})).unwrap();
        assert!(matches!(exchange(Answering(Ok(reply)), &pulse, None), Ok(Reply::Pulse(_))));
        assert!(matches!(exchange(Answering(Err(contract::Failure::FrontendSuperseded)), &pulse, None),
            Ok(Reply::FrontendSuperseded)));
        let refused = "Error: \"operator_state_changed_refresh\"\n";
        assert_eq!(exchange(Answering(Err(contract::Failure::Error(refused.into()))), &pulse, None)
            .err().unwrap(), refused);
        // A reply of another operator model never reaches the window.
        assert!(exchange(Answering(Ok(br#"{"schema":28}"#.to_vec())), &pulse, None).err().unwrap()
            .contains(&format!("operator model {} required", crate::model::OPERATOR_SCHEMA)));
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
