//! The typed exchange between the manager service and its clients.
//!
//! The window, the `operator` command line and the Native Access browser
//! return each send one [`Request`] and get back one [`Reply`] or a
//! [`Failure`]. Everything both ends must agree on lives here: a request's
//! command form, the reply envelopes and schema checks, the size and time
//! bounds, and how a refusal reaches the window. A [`Transport`] only carries
//! the bytes; today that is the installed manager run once per request
//! ([`Subprocess`]). The operator wire model (`operator_model`) is unchanged.
use crate::operator_model as ui;
use std::{
    ffi::OsString,
    fs::File,
    io::{self, Read as _, Write as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// An action's request body.
pub const REQUEST_LIMIT: usize = 16384;
/// One reply.
pub const REPLY_LIMIT: u64 = 8 * 1024 * 1024;
/// A refusal as the answering side wrote it.
pub const REFUSAL_LIMIT: u64 = 4096;
/// The part of a refusal shown to the musician.
const REFUSAL_SHOWN: usize = 512;
/// Every exchange except an installer import, which copies the chosen file.
pub const DEADLINE: Duration = Duration::from_secs(45);
pub const IMPORT_DEADLINE: Duration = Duration::from_secs(300);

/// The refusal a previous frontend receives after an update or restore
/// selected another one. Its window pauses instead of acting.
pub const FRONTEND_SUPERSEDED: &str = "operator_frontend_superseded: Bridge updated. This previous manager window is paused. Open the current manager to continue; unfinished inputs and accepted operations are retained";
const SUPERSEDED_CODE: &str = "operator_frontend_superseded: ";
/// The only failure a browser return reports. Which check refused it stays
/// private: the return carries a sign-in secret.
pub const BROWSER_RETURN_UNDELIVERED: &str = "Native Access login return could not be delivered. Open Native Access through the manager and start a fresh sign-in.";

/// Exact identity of one installed plug-in build in one setup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductKey {
    pub environment: String,
    pub module_sha256: String,
    pub class_id: String,
}
impl From<&ui::Product> for ProductKey {
    fn from(product: &ui::Product) -> Self {
        Self { environment: product.environment.clone(),
            module_sha256: product.module_sha256.clone(), class_id: product.class_id.clone() }
    }
}

/// A status read. Reads change nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    /// Current records and offers with a fresh platform assessment.
    Overview,
    /// Current records and offers only.
    CurrentOverview,
    /// One plug-in page.
    Product(ProductKey),
    /// The deep Diagnostics snapshot.
    Snapshot,
    Activity,
    /// The cheap change signal an idle window polls.
    Pulse,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Request {
    Read(Read),
    /// An offered action; its receipt says whether it was accepted.
    Action(ui::Request),
    /// Import the installer given as this exchange's input, under its file name.
    ImportInstaller { name: String },
    /// A Native Access browser return, for the live sign-in that awaits it.
    BrowserReturn(String),
}
// A browser return carries a sign-in secret: it is never written out.
impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(read) => f.debug_tuple("Read").field(read).finish(),
            Self::Action(request) => f.debug_tuple("Action").field(request).finish(),
            Self::ImportInstaller { name } => f.debug_struct("ImportInstaller").field("name", name).finish(),
            Self::BrowserReturn(_) => f.write_str("BrowserReturn(..)"),
        }
    }
}

pub enum Reply {
    Overview(Box<ui::InteractiveOverview>),
    CurrentOverview(Box<ui::CurrentOverview>),
    Product(Box<ui::CurrentProductDetail>),
    Snapshot(Box<ui::Snapshot>),
    Activity(ui::Activity),
    Pulse(ui::Pulse),
    Receipt(ui::Receipt),
    Imported(ui::InstallerImportResult),
    /// The browser return reached its sign-in.
    Delivered,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// An update or restore selected another frontend; this one is paused.
    FrontendSuperseded,
    /// The request was not answered. The text, for the musician, says why.
    Error(String),
}

impl Request {
    /// The client exchange a manager command line names, or `None` for any
    /// other command. Only an action reads `input`: its bounded request body.
    pub fn from_command(args: &[String], input: impl io::Read) -> Option<crate::Result<Self>> {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let request = match args[..] {
            ["operator", "overview"] => Self::Read(Read::Overview),
            ["operator", "current-overview"] => Self::Read(Read::CurrentOverview),
            ["operator", "product", environment, module, class] => Self::Read(Read::Product(ProductKey {
                environment: environment.into(), module_sha256: module.into(), class_id: class.into() })),
            ["operator", "snapshot"] => Self::Read(Read::Snapshot),
            ["operator", "activity"] => Self::Read(Read::Activity),
            ["operator", "pulse"] => Self::Read(Read::Pulse),
            ["operator", "request"] => return Some(read_action(input).map(Self::Action)),
            ["import-installer", name] => Self::ImportInstaller { name: name.into() },
            ["native-access-callback", url] => Self::BrowserReturn(url.into()),
            _ => return None,
        };
        Some(Ok(request))
    }
    /// The manager command line that answers this request.
    pub fn command(&self) -> Vec<String> {
        let operator = |args: &[&str]| std::iter::once("operator").chain(args.iter().copied())
            .map(String::from).collect();
        match self {
            Self::Read(Read::Overview) => operator(&["overview"]),
            Self::Read(Read::CurrentOverview) => operator(&["current-overview"]),
            Self::Read(Read::Product(key)) => operator(&["product", &key.environment,
                &key.module_sha256, &key.class_id]),
            Self::Read(Read::Snapshot) => operator(&["snapshot"]),
            Self::Read(Read::Activity) => operator(&["activity"]),
            Self::Read(Read::Pulse) => operator(&["pulse"]),
            Self::Action(_) => operator(&["request"]),
            Self::ImportInstaller { name } => vec!["import-installer".into(), name.clone()],
            Self::BrowserReturn(url) => vec!["native-access-callback".into(), url.clone()],
        }
    }
    /// What the command reads on its input, besides an imported installer.
    fn body(&self) -> serde_json::Result<Option<Vec<u8>>> {
        match self {
            Self::Action(request) => serde_json::to_vec(request).map(Some),
            _ => Ok(None),
        }
    }
    fn deadline(&self) -> Duration {
        if matches!(self, Self::ImportInstaller { .. }) { IMPORT_DEADLINE } else { DEADLINE }
    }
}
fn read_action(input: impl io::Read) -> crate::Result<ui::Request> {
    let mut bytes = Vec::new();
    input.take(REQUEST_LIMIT as u64 + 1).read_to_end(&mut bytes)?;
    crate::require(bytes.len() <= REQUEST_LIMIT, "operator_request_bound")?;
    Ok(serde_json::from_slice(&bytes)?)
}

impl Reply {
    /// The reply as the answering side writes it: one JSON document, or
    /// nothing for a delivered browser return.
    pub fn encode(&self) -> serde_json::Result<Option<String>> {
        match self {
            Self::Overview(value) => serde_json::to_string(value),
            Self::CurrentOverview(value) => serde_json::to_string(value),
            Self::Product(value) => serde_json::to_string(value),
            Self::Snapshot(value) => serde_json::to_string(value),
            Self::Activity(value) => serde_json::to_string(value),
            Self::Pulse(value) => serde_json::to_string(value),
            Self::Receipt(value) => serde_json::to_string(value),
            Self::Imported(value) => serde_json::to_string(value),
            Self::Delivered => return Ok(None),
        }.map(Some)
    }
}

fn paired_model_required() -> String {
    format!("Update the frontend and manager together: operator model {} required", ui::OPERATOR_SCHEMA)
}
/// Check and type the reply to `request`. A manager of another operator
/// model, or a reply that does not match what was asked, is refused here.
pub fn decode(request: &Request, data: &[u8]) -> Result<Reply, String> {
    let read = match request {
        Request::Read(read) => read,
        Request::ImportInstaller { .. } => return decode_import(data),
        Request::BrowserReturn(_) => return Ok(Reply::Delivered),
        Request::Action(_) => {
            let envelope: serde_json::Value =
                serde_json::from_slice(data).map_err(|_| "Invalid manager response")?;
            if envelope["schema"] != ui::OPERATOR_SCHEMA { return Err(paired_model_required()); }
            return serde_json::from_slice(data).map(Reply::Receipt).map_err(|_| incompatible());
        }
    };
    let envelope: serde_json::Value =
        serde_json::from_slice(data).map_err(|_| "Invalid manager response")?;
    // Paired overview and product readbacks carry their own envelope.
    if matches!(read, Read::Overview | Read::CurrentOverview | Read::Product(_))
        && (envelope["schema"] != 1 || envelope["operator_schema"] != ui::OPERATOR_SCHEMA) {
        return Err(paired_model_required());
    }
    match read {
        Read::CurrentOverview => {
            let overview: ui::CurrentOverview = serde_json::from_slice(data)
                .map_err(|_| "Incompatible manager current overview; update the frontend and manager together")?;
            if overview.scope != "current_only" || overview.current.schema != ui::OPERATOR_SCHEMA {
                return Err("Inconsistent manager current overview; refresh before acting".into());
            }
            Ok(Reply::CurrentOverview(Box::new(overview)))
        }
        Read::Overview => {
            let overview: ui::InteractiveOverview = serde_json::from_slice(data)
                .map_err(|_| "Incompatible manager overview; update the frontend and manager together")?;
            if overview.scope != "current_only" || overview.current.schema != ui::OPERATOR_SCHEMA
                || overview.readiness.schema != 1
                || overview.current.state_token != overview.readiness.state_token
                || overview.current.system != overview.readiness.system {
                return Err("Inconsistent manager overview; refresh before acting".into());
            }
            Ok(Reply::Overview(Box::new(overview)))
        }
        Read::Product(key) => {
            let detail: ui::CurrentProductDetail = serde_json::from_slice(data)
                .map_err(|_| "Incompatible manager product readback; update the frontend and manager together")?;
            if detail.state_token.is_empty() || detail.current_generation.is_empty()
                || ProductKey::from(&detail.product) != *key
                || detail.environments.iter().any(|environment| environment.id != key.environment)
                || detail.vendor_applications.iter().any(|application|
                    application.details["environment"].as_str() != Some(key.environment.as_str())) {
                return Err("Inconsistent manager product readback; refresh before acting".into());
            }
            Ok(Reply::Product(Box::new(detail)))
        }
        Read::Snapshot | Read::Activity | Read::Pulse => {
            if envelope["schema"] != ui::OPERATOR_SCHEMA { return Err(paired_model_required()); }
            match read {
                Read::Snapshot => serde_json::from_slice(data).map(|s| Reply::Snapshot(Box::new(s))),
                Read::Activity => serde_json::from_slice(data).map(Reply::Activity),
                _ => serde_json::from_slice(data).map(Reply::Pulse),
            }.map_err(|_| incompatible())
        }
    }
}
fn incompatible() -> String {
    "Incompatible manager response; update the frontend and manager together".into()
}
fn decode_import(data: &[u8]) -> Result<Reply, String> {
    let envelope: serde_json::Value =
        serde_json::from_slice(data).map_err(|_| "Invalid import receipt")?;
    if envelope["schema"] != 2 {
        return Err("Update the frontend and manager together: installer import result 2 required".into());
    }
    let value: ui::InstallerImportResult = serde_json::from_value(envelope)
        .map_err(|_| "Incompatible installer import result")?;
    if value.installer_sha256.len() != 64
        || !value.installer_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value.display_label.is_empty() || value.display_label.len() > 96
        || value.display_label.chars().any(char::is_control)
        || !matches!(value.format.as_str(), "pe_executable" | "msi_compound") {
        return Err("Installer import refused".into());
    }
    Ok(Reply::Imported(value))
}

/// How the answering side's refusal, as it wrote it, reaches the client: a
/// paused previous frontend is told so; anything else is shown, bounded.
pub fn refusal(written: &[u8]) -> Failure {
    let error = String::from_utf8_lossy(written);
    let refusal = error.trim().strip_prefix("Error: ").unwrap_or(error.trim()).trim_matches('"');
    if refusal.starts_with(SUPERSEDED_CODE) {
        return Failure::FrontendSuperseded;
    }
    Failure::Error(error.chars().take(REFUSAL_SHOWN).collect())
}

/// Carries one request to the side that answers it and returns that side's
/// reply bytes, unchecked, or its refusal.
pub trait Transport {
    fn exchange(&self, request: &Request, input: Option<File>) -> Result<Vec<u8>, Failure>;
}

/// A manager client: one transport, every reply checked by [`decode`].
pub struct Client<T> {
    transport: T,
}
impl<T: Transport> Client<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
    /// `input` is the installer an import copies; other requests take none.
    pub fn call(&self, request: &Request, input: Option<File>) -> Result<Reply, Failure> {
        let data = self.transport.exchange(request, input)?;
        decode(request, &data).map_err(Failure::Error)
    }
}

/// The installed manager command, run once per request. Its exit status and
/// error output carry a refusal.
pub struct Subprocess {
    program: PathBuf,
    environment: Option<Vec<(OsString, OsString)>>,
}
impl Subprocess {
    /// The stable command link in `home`.
    pub fn installed(home: &Path) -> Self {
        Self::new(home.join(".local/bin/linux-vst-bridge"))
    }
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self { program: program.into(), environment: None }
    }
    /// Run the command with exactly `environment` instead of this process's.
    pub fn with_environment<K: Into<OsString>, V: Into<OsString>>(mut self,
        environment: impl IntoIterator<Item = (K, V)>) -> Self {
        self.environment = Some(environment.into_iter().map(|(k, v)| (k.into(), v.into())).collect());
        self
    }
}
impl Transport for Subprocess {
    fn exchange(&self, request: &Request, input: Option<File>) -> Result<Vec<u8>, Failure> {
        let error = |text: &str| Failure::Error(text.into());
        let body = request.body().map_err(|_| error("Request encoding failed"))?;
        let mut command = Command::new(&self.program);
        command.args(request.command());
        if let Some(environment) = &self.environment {
            command.env_clear().envs(environment.iter().map(|(k, v)| (k, v)));
        }
        let mut child = command
            .stdin(input.map(Stdio::from).unwrap_or_else(Stdio::piped))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| error("Installed manager is unavailable"))?;
        if let Some(body) = body {
            child.stdin.as_mut().ok_or_else(|| error("Manager input unavailable"))?
                .write_all(&body).map_err(|_| error("Request delivery failed"))?;
        }
        drop(child.stdin.take());
        let collect = |mut pipe: Box<dyn io::Read + Send>, limit: u64| {
            std::thread::spawn(move || {
                let mut data = Vec::new();
                let result = pipe.by_ref().take(limit + 1).read_to_end(&mut data);
                let overflow = data.len() > limit as usize;
                let _ = io::copy(&mut pipe, &mut io::sink());
                (data, overflow, result.is_ok())
            })
        };
        let out = collect(Box::new(child.stdout.take().ok_or_else(|| error("Manager output unavailable"))?),
            REPLY_LIMIT);
        let err = collect(Box::new(child.stderr.take().ok_or_else(|| error("Manager error unavailable"))?),
            REFUSAL_LIMIT);
        let deadline = Instant::now() + request.deadline();
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|_| error("Manager wait failed"))? {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error("Manager readback timed out. Refresh to inspect any accepted operation."));
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let (data, overflow, read) = out.join().map_err(|_| error("Manager reader failed"))?;
        let (written, _, _) = err.join().map_err(|_| error("Manager error reader failed"))?;
        if !status.success() {
            return Err(refusal(&written));
        }
        if overflow || !read {
            return Err(error("Manager response exceeded its bound"));
        }
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> ui::Snapshot {
        serde_json::from_str(include_str!("../../../manager-ui/examples/library-preview.json")).unwrap()
    }
    fn product(key: &ProductKey) -> Request {
        Request::Read(Read::Product(key.clone()))
    }
    fn action() -> Request {
        Request::Action(ui::Request { schema: ui::OPERATOR_SCHEMA, state_token: "exact-state".into(),
            action: ui::Action::DependencyPrepare {} })
    }
    fn bytes(value: &serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }

    #[test]
    fn command_form_round_trips_every_client_request() {
        let key = ProductKey { environment: "environment".into(), module_sha256: "ab".repeat(32),
            class_id: "cd".repeat(16) };
        let requests = [Request::Read(Read::Overview), Request::Read(Read::CurrentOverview), product(&key),
            Request::Read(Read::Snapshot), Request::Read(Read::Activity), Request::Read(Read::Pulse),
            action(), Request::ImportInstaller { name: "Setup.exe".into() },
            Request::BrowserReturn("native-access://auth?code=secret".into())];
        for request in requests {
            let body = request.body().unwrap().unwrap_or_default();
            let parsed = Request::from_command(&request.command(), body.as_slice()).unwrap().unwrap();
            assert_eq!(parsed, request);
        }
        assert!(!format!("{:?}", Request::BrowserReturn("native-access://auth?code=secret".into()))
            .contains("secret"), "a browser return is never written out");
        let others: [&[&str]; 6] = [&["operator", "worker", "id"], &["operator"], &["serve"],
            &["import-installer"], &["native-access-callback"], &["operator", "product", "environment"]];
        for other in others {
            let args: Vec<String> = other.iter().map(|s| s.to_string()).collect();
            assert!(Request::from_command(&args, &b""[..]).is_none(), "{other:?}");
        }
        let args = |verb: &str| vec!["operator".to_string(), verb.to_string()];
        let oversized = vec![b' '; REQUEST_LIMIT + 1];
        assert_eq!(Request::from_command(&args("request"), oversized.as_slice()).unwrap()
            .unwrap_err().to_string(), "operator_request_bound");
        assert!(Request::from_command(&args("request"), &b"{}"[..]).unwrap().is_err());
    }

    #[test]
    fn current_overview_is_paired_record_readback_without_readiness() {
        let read = |read: Read, value: &serde_json::Value| decode(&Request::Read(read), &bytes(value));
        let mut value = serde_json::to_value(ui::CurrentOverview {
            schema:1,operator_schema:ui::OPERATOR_SCHEMA,
            scope:"current_only".into(),current_generation:"exact-generation".into(),
            current:snapshot(),
        }).unwrap();
        assert!(matches!(read(Read::CurrentOverview, &value), Ok(Reply::CurrentOverview(_))));
        assert!(read(Read::Overview, &value).is_err());
        value["readiness"] = serde_json::json!({"observed_at":1});
        assert!(read(Read::CurrentOverview, &value).is_err());
        value.as_object_mut().unwrap().remove("readiness");
        value["operator_schema"] = (ui::OPERATOR_SCHEMA - 1).into();
        assert!(read(Read::CurrentOverview, &value).is_err());
        value["operator_schema"] = ui::OPERATOR_SCHEMA.into();
        value["scope"] = "history".into();
        assert!(read(Read::CurrentOverview, &value).is_err());
    }
    #[test]
    fn selected_product_readback_requires_exact_paired_identity() {
        let snapshot = snapshot();
        let product_row = snapshot.products[0].clone();
        let key = ProductKey::from(&product_row);
        let mut value = serde_json::to_value(ui::CurrentProductDetail {
            schema: 1, operator_schema: ui::OPERATOR_SCHEMA,
            state_token: snapshot.state_token, current_generation: "exact-generation".into(),
            system: snapshot.system,
            product: product_row, environments: vec![], vendor_applications: vec![],
        }).unwrap();
        assert!(matches!(decode(&product(&key), &bytes(&value)), Ok(Reply::Product(_))));
        value["product"]["class_id"] = "different".into();
        assert!(decode(&product(&key), &bytes(&value)).is_err());
        value["operator_schema"] = 11.into();
        assert!(decode(&product(&key), &bytes(&value)).is_err());
    }
    #[test]
    fn current_frontend_accepts_paired_and_refuses_old_manager() {
        let mut receipt = serde_json::json!({
            "schema": ui::OPERATOR_SCHEMA, "accepted": true, "operation": "exact-operation", "refusal": null
        });
        match decode(&action(), &bytes(&receipt)).unwrap() {
            Reply::Receipt(value) => {
                assert_eq!(value.schema, ui::OPERATOR_SCHEMA);
                assert!(value.accepted);
                assert_eq!(value.operation.as_deref(), Some("exact-operation"));
            }
            _ => panic!("expected dependency request receipt"),
        }
        let system = serde_json::json!({
            "service": "active", "keepers": 2, "dsp": 0, "maintenance": 0,
            "ceiling": 512, "pending_transactions": 0, "stale_transports": 0,
            "cleanup_unconfirmed": false
        });
        let activity = serde_json::json!({
            "schema": ui::OPERATOR_SCHEMA, "system": system, "capture": null, "operation": null
        });
        assert!(matches!(decode(&Request::Read(Read::Activity), &bytes(&activity)), Ok(Reply::Activity(_))));
        let pulse = serde_json::json!({"schema":ui::OPERATOR_SCHEMA,
            "service_state":"active","dsp":0,"keepers":2,"maintenance":0,
            "pending_transactions":0,"cleanup_unconfirmed":false,"current_generation":"exact-generation",
            "operation":null,"operation_live":false});
        assert!(matches!(decode(&Request::Read(Read::Pulse), &bytes(&pulse)), Ok(Reply::Pulse(_))));
        let snapshot = serde_json::json!({
            "schema": ui::OPERATOR_SCHEMA, "state_token": "exact-state", "system": system,
            "capture": null, "operation": null, "onboarding": [], "environments": [],
            "vendor_applications": [], "products": [], "active_sessions": [],
            "recent_incidents": [], "actions": []
        });
        let Reply::Snapshot(decoded) = decode(&Request::Read(Read::Snapshot), &bytes(&snapshot)).unwrap()
            else { panic!("expected snapshot") };
        assert!(decoded.managed_runtime_records.is_empty());
        let mut with_runtime = snapshot.clone();
        with_runtime["managed_runtime_records"] = serde_json::json!([{
            "id":"retained-runtime","record_path":"/managed/runners/retained/runtime.json",
            "runner":null,"failure":"managed_runtime_binding"}]);
        let Reply::Snapshot(decoded) = decode(&Request::Read(Read::Snapshot), &bytes(&with_runtime)).unwrap()
            else { panic!("expected snapshot") };
        assert_eq!(decoded.managed_runtime_records[0]["failure"], "managed_runtime_binding");
        with_runtime["schema"] = 20.into();
        assert!(decode(&Request::Read(Read::Snapshot), &bytes(&with_runtime)).err().unwrap()
            .contains(&format!("operator model {} required", ui::OPERATOR_SCHEMA)));
        for schema in [
            serde_json::json!(6), serde_json::json!(7), serde_json::json!(8), serde_json::json!(9),
            serde_json::json!(10), serde_json::json!(11), serde_json::json!(12), serde_json::json!(14),
            serde_json::json!(20), serde_json::json!("9"), serde_json::Value::Null,
        ] {
            receipt["schema"] = schema;
            for request in [action(), Request::Read(Read::Snapshot), Request::Read(Read::Activity),
                Request::Read(Read::Pulse)] {
                assert!(decode(&request, &bytes(&receipt)).err().unwrap()
                    .contains(&format!("operator model {} required", ui::OPERATOR_SCHEMA)));
            }
        }
        receipt["schema"] = serde_json::json!(ui::OPERATOR_SCHEMA);
        receipt["arbitrary"] = serde_json::json!(true);
        assert!(decode(&action(), &bytes(&receipt)).is_err());
    }
    #[test]
    fn paired_import_result_preserves_identity_and_refuses_old_manager() {
        let import = Request::ImportInstaller { name: "Lunacy Audio.exe".into() };
        let exact = serde_json::json!({"schema":2,"installer_sha256":"ab".repeat(32),
            "byte_size":2048,"format":"pe_executable","newly_imported":false,
            "display_label":"Lunacy Audio","created_at":42});
        match decode(&import, &bytes(&exact)).unwrap() {
            Reply::Imported(value) => {
                assert!(!value.newly_imported);
                assert_eq!(value.installer_sha256, "ab".repeat(32));
            }
            _ => panic!("expected typed import"),
        }
        let mut old = exact;
        old["schema"] = 1.into();
        assert!(decode(&import, &bytes(&old)).err().unwrap()
            .contains("Update the frontend and manager together"));
    }
    #[test]
    fn superseded_frontend_refusal_pauses_and_others_are_shown_bounded() {
        for written in [format!("Error: {FRONTEND_SUPERSEDED:?}\n"), format!("{FRONTEND_SUPERSEDED}\n")] {
            assert_eq!(refusal(written.as_bytes()), Failure::FrontendSuperseded);
        }
        let long = format!("Error: \"{}\"\n", "x".repeat(2000));
        let Failure::Error(shown) = refusal(long.as_bytes()) else { panic!("expected an error") };
        assert_eq!(shown.chars().count(), REFUSAL_SHOWN);
        assert!(shown.starts_with("Error: \"xxx"));
        assert_eq!(refusal(b"Error: \"operator_state_changed_refresh\"\n"),
            Failure::Error("Error: \"operator_state_changed_refresh\"\n".into()));
    }
    #[test]
    fn subprocess_carries_exit_status_bounds_and_input() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("lvb-contract-{}", crate::random_id().unwrap()));
        std::fs::create_dir_all(&root).unwrap();
        let program = root.join("manager");
        // Echo the request body back as the reply; refuse when told to.
        std::fs::write(&program, "#!/bin/sh\ncase \"$2\" in\n  request) cat ;;\n  pulse) echo 'Error: \"operator_state_changed_refresh\"' >&2; exit 1 ;;\n  snapshot) head -c 8388609 /dev/zero ;;\nesac\n").unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let transport = Subprocess::new(&program).with_environment([("PATH", "/usr/bin:/bin")]);
        let request = action();
        assert_eq!(transport.exchange(&request, None).unwrap(), request.body().unwrap().unwrap());
        assert_eq!(transport.exchange(&Request::Read(Read::Pulse), None).unwrap_err(),
            Failure::Error("Error: \"operator_state_changed_refresh\"\n".into()));
        assert_eq!(transport.exchange(&Request::Read(Read::Snapshot), None).unwrap_err(),
            Failure::Error("Manager response exceeded its bound".into()));
        assert_eq!(Subprocess::new(root.join("absent")).exchange(&request, None).unwrap_err(),
            Failure::Error("Installed manager is unavailable".into()));
        // The client checks what the transport carried: a receipt is not an action body.
        assert!(matches!(Client::new(transport).call(&request, None), Err(Failure::Error(_))));
        std::fs::remove_dir_all(root).unwrap();
    }
}
