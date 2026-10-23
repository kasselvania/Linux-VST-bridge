//! The typed exchange between the manager service and its clients.
//!
//! The window, the `operator` command line and the Native Access browser
//! return each send one [`Request`] and get back one [`Reply`] or a
//! [`Failure`]. Everything both ends must agree on lives here: a request's
//! command form, the reply envelopes and schema checks, the size and time
//! bounds, and how a refusal reaches the window. A [`Transport`] only carries
//! the bytes: the running service over its socket ([`Service`]), or the
//! command transport ([`Subprocess`]). [`Installed`] uses only the service;
//! an unavailable service never promotes a client into a local state owner.
use crate::operator_model as ui;
use std::{
    ffi::OsString,
    fs::File,
    io::{self, Read as _, Write as _},
    os::unix::net::UnixStream,
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
/// Ordinary status and action acknowledgement exchanges.
pub const DEADLINE: Duration = Duration::from_secs(45);
/// A connection to the service, which is waited for only while it is busy accepting.
pub const CONNECT_DEADLINE: Duration = Duration::from_secs(2);
pub const IMPORT_DEADLINE: Duration = Duration::from_secs(300);
/// Browser delivery can wait 25 s for readiness, 3 s to write the return and
/// 70 s for the adapter result. Keep one bounded exchange, including connect,
/// instead of abandoning a legitimate delivery at the ordinary 45 s limit.
pub const BROWSER_RETURN_DEADLINE: Duration = Duration::from_secs(100);

/// The refusal a previous frontend receives after an update or restore
/// selected another one. Its window pauses instead of acting.
pub const FRONTEND_SUPERSEDED: &str = "operator_frontend_superseded: Bridge updated. This previous manager window is paused. Open the current manager to continue; unfinished inputs and accepted operations are retained";
const SUPERSEDED_CODE: &str = "operator_frontend_superseded: ";
/// The only failure a browser return reports. Which check refused it stays
/// private: the return carries a sign-in secret.
pub const BROWSER_RETURN_UNDELIVERED: &str = "Native Access login return could not be delivered. Open Native Access through the manager and start a fresh sign-in.";

/// Exact identity of one installed plug-in build in one setup.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// The retained result of this exact receipted operation.
    Operation(String),
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
    Operation(ui::OperationReadback),
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
    /// No service is listening: it is stopped, paused by an operation or restarting.
    Unavailable,
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
            ["operator", "operation", operation] => Self::Read(Read::Operation(operation.into())),
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
            Self::Read(Read::Operation(operation)) => operator(&["operation", operation]),
            Self::Action(_) => operator(&["request"]),
            Self::ImportInstaller { name } => vec!["import-installer".into(), name.clone()],
            Self::BrowserReturn(url) => vec!["native-access-callback".into(), url.clone()],
        }
    }
    /// What the command reads on its input, besides an imported installer.
    pub fn body(&self) -> serde_json::Result<Option<Vec<u8>>> {
        match self {
            Self::Action(request) => serde_json::to_vec(request).map(Some),
            _ => Ok(None),
        }
    }
    /// How long the exchange may take.
    pub fn deadline(&self) -> Duration {
        match self {
            Self::ImportInstaller { .. } => IMPORT_DEADLINE,
            Self::BrowserReturn(_) => BROWSER_RETURN_DEADLINE,
            _ => DEADLINE,
        }
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
            Self::Operation(value) => serde_json::to_string(value),
            Self::Receipt(value) => serde_json::to_string(value),
            Self::Imported(value) => serde_json::to_string(value),
            Self::Delivered => return Ok(None),
        }.map(Some)
    }
    /// The bytes the command line prints for this reply.
    pub fn written(&self) -> serde_json::Result<Vec<u8>> {
        Ok(self.encode()?.map(|document| document + "\n").unwrap_or_default().into_bytes())
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
    // Paired owner readbacks carry their own envelope.
    if matches!(read, Read::Overview | Read::CurrentOverview | Read::Product(_) | Read::Operation(_))
        && (envelope["schema"] != 1 || envelope["operator_schema"] != ui::OPERATOR_SCHEMA) {
        return Err(paired_model_required());
    }
    match read {
        Read::Operation(operation) => {
            let value: ui::OperationReadback = serde_json::from_slice(data)
                .map_err(|_| "Invalid manager operation readback")?;
            if !value.matches(operation) {
                return Err("Manager operation readback does not match the requested operation".into());
            }
            Ok(Reply::Operation(value))
        }
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

/// A refusal as the answering side writes it; the command line's own error
/// output has this form, so every transport carries the same text.
pub fn refusal_text(error: &dyn std::fmt::Debug) -> String {
    format!("Error: {error:?}\n")
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

/// The greeting of a status read on the service socket.
pub const VIEW_GREETING: &[u8; 5] = b"LVV1\n";
/// Commands are sent once. Losing their acknowledgement never permits replay.
pub const COMMAND_GREETING: &[u8; 5] = b"LVA1\n";
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum CommandRequest {
    Action { request: ui::Request },
    ImportInstaller { name: String },
    BrowserReturn { uri: String },
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandEnvelope {
    operator_schema: u32,
    command: CommandRequest,
}
/// Decode one command after its greeting. Imports carry exactly one open
/// descriptor, never a path that the service would reopen.
pub fn read_command_request(peer: &mut UnixStream) -> crate::Result<(Request, Option<File>)> {
    let mut length = [0; 4];
    peer.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    crate::require(length <= REQUEST_LIMIT, "operator_request_bound")?;
    let mut bytes = vec![0; length];
    peer.read_exact(&mut bytes)?;
    let envelope: CommandEnvelope = serde_json::from_slice(&bytes)?;
    crate::require(envelope.operator_schema == ui::OPERATOR_SCHEMA, &paired_model_required())?;
    Ok(match envelope.command {
        CommandRequest::Action { request } => (Request::Action(request), None),
        CommandRequest::ImportInstaller { name } =>
            (Request::ImportInstaller { name }, Some(receive_file(peer)?)),
        CommandRequest::BrowserReturn { uri } => (Request::BrowserReturn(uri), None),
    })
}

fn send_file(peer: &UnixStream, file: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    let mut marker = [b'F'];
    let mut iov = libc::iovec { iov_base: marker.as_mut_ptr().cast(), iov_len: 1 };
    // usize storage gives cmsghdr the alignment it requires on both targets.
    let mut control = [0usize; 8];
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = unsafe { libc::CMSG_SPACE(std::mem::size_of::<libc::c_int>() as _) } as _;
    unsafe {
        let header = libc::CMSG_FIRSTHDR(&message);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<libc::c_int>() as _) as _;
        libc::CMSG_DATA(header).cast::<libc::c_int>().write(file.as_raw_fd());
        if libc::sendmsg(peer.as_raw_fd(), &message, 0) != 1 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}
fn receive_file(peer: &UnixStream) -> io::Result<File> {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    let mut marker = [0];
    let mut iov = libc::iovec { iov_base: marker.as_mut_ptr().cast(), iov_len: 1 };
    let mut control = [0usize; 8];
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = std::mem::size_of_val(&control) as _;
    let mut descriptors = Vec::new();
    unsafe {
        #[cfg(target_os = "linux")]
        let flags = libc::MSG_CMSG_CLOEXEC;
        #[cfg(not(target_os = "linux"))]
        let flags = 0;
        let count = libc::recvmsg(peer.as_raw_fd(), &mut message, flags);
        if count < 0 { return Err(io::Error::last_os_error()); }
        let mut header = libc::CMSG_FIRSTHDR(&message);
        while !header.is_null() {
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                let bytes = ((*header).cmsg_len as usize).saturating_sub(libc::CMSG_LEN(0) as usize);
                for index in 0..bytes / std::mem::size_of::<libc::c_int>() {
                    let fd = libc::CMSG_DATA(header).cast::<libc::c_int>().add(index).read();
                    descriptors.push(OwnedFd::from_raw_fd(fd));
                }
            }
            header = libc::CMSG_NXTHDR(&message, header);
        }
        if count != 1 || marker != [b'F'] || message.msg_flags & libc::MSG_CTRUNC != 0
            || descriptors.len() != 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "installer_descriptor_extent"));
        }
        if libc::fcntl(descriptors[0].as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(File::from(descriptors.pop().expect("one received descriptor")))
}
/// A status read as it crosses the socket: the reading client's operator
/// model and the read's command form.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewRequest {
    operator_schema: u32,
    command: Vec<String>,
}
impl Read {
    /// Every product read is answered by the committing service.
    pub fn served(&self) -> bool {
        true
    }
}
/// The service side of a status read: the read that follows the greeting.
/// A client of another operator model is refused with the message it shows.
pub fn read_view_request(peer: &mut impl io::Read) -> crate::Result<Read> {
    let mut length = [0; 4];
    peer.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    crate::require(length <= REQUEST_LIMIT, "operator_request_bound")?;
    let mut bytes = vec![0; length];
    peer.read_exact(&mut bytes)?;
    let request: ViewRequest = serde_json::from_slice(&bytes)?;
    crate::require(request.operator_schema == ui::OPERATOR_SCHEMA, &paired_model_required())?;
    match Request::from_command(&request.command, io::empty()) {
        Some(Ok(Request::Read(read))) if read.served() => Ok(read),
        _ => Err("operator_view_request".into()),
    }
}
/// The service side of a status read's answer: its reply, or its refusal as
/// [`refusal_text`] wrote it. Each is bounded as the command's output is.
pub fn write_view_reply(peer: &mut impl io::Write, reply: Result<&[u8], &str>) -> io::Result<()> {
    let (status, bytes) = match reply {
        Ok(bytes) if bytes.len() as u64 <= REPLY_LIMIT => (0, bytes),
        Ok(_) => (1, "Manager response exceeded its bound".as_bytes()),
        Err(refusal) => (1, &refusal.as_bytes()[..refusal.len().min(REFUSAL_LIMIT as usize)]),
    };
    let mut frame = Vec::with_capacity(5 + bytes.len());
    frame.push(status);
    frame.extend((bytes.len() as u32).to_le_bytes());
    frame.extend(bytes);
    peer.write_all(&frame)
}

/// The running service, over its socket. It answers the status reads it
/// [serves](Read::served) and nothing else.
pub struct Service {
    socket: PathBuf,
}
impl Service {
    /// The service socket in `home`.
    pub fn installed(home: &Path) -> Self {
        Self::new(home.join(".local/share/linux-vst-bridge/managed/runtime/owner.sock"))
    }
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self { socket: socket.into() }
    }
}
/// One attempt at a status read.
enum Attempt {
    /// The connection closed or reset before the whole reply: the service
    /// stopped or restarted.
    Dropped,
    Failed(Failure),
}
impl Transport for Service {
    fn exchange(&self, request: &Request, input: Option<File>) -> Result<Vec<u8>, Failure> {
        let error = |text: &str| Failure::Error(text.into());
        if !matches!(request, Request::Read(_)) {
            return self.command(request, input);
        }
        if input.is_some() { return Err(error("Unexpected installer descriptor")); }
        if !matches!(request, Request::Read(read) if read.served()) {
            return Err(error("The bridge service does not answer this request"));
        }
        let body = serde_json::to_vec(&ViewRequest { operator_schema: ui::OPERATOR_SCHEMA,
            command: request.command() }).map_err(|_| error("Request encoding failed"))?;
        let mut frame = VIEW_GREETING.to_vec();
        frame.extend((body.len() as u32).to_le_bytes());
        frame.extend(body);
        let deadline = Instant::now() + request.deadline();
        // A service restart can interrupt a read. Retry that read once; an
        // unavailable owner remains unavailable. Commands are never replayed.
        for attempt in 0..2 {
            let mut peer = connect(&self.socket, CONNECT_DEADLINE).map_err(|e| match e.kind() {
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => Failure::Unavailable,
                io::ErrorKind::TimedOut =>
                    error("The bridge service is busy and did not take this status read. Refresh to try again."),
                _ => error("Bridge service is unavailable"),
            })?;
            match read_once(&mut peer, &frame, deadline) {
                Ok(reply) => return Ok(reply),
                Err(Attempt::Failed(failure)) => return Err(failure),
                Err(Attempt::Dropped) if attempt == 0 => continue,
                // Also how a service from before this greeting answers it.
                Err(Attempt::Dropped) => return Err(error(
                    "The bridge service stopped answering this status read. Refresh to try again.")),
            }
        }
        unreachable!("the second attempt returns")
    }
}
impl Service {
    fn command(&self, request: &Request, input: Option<File>) -> Result<Vec<u8>, Failure> {
        let error = |text: &str| Failure::Error(text.into());
        let command = match request {
            Request::Action(request) if input.is_none() => CommandRequest::Action { request: request.clone() },
            Request::ImportInstaller { name } if input.is_some() => CommandRequest::ImportInstaller { name: name.clone() },
            Request::BrowserReturn(uri) if input.is_none() => CommandRequest::BrowserReturn { uri: uri.clone() },
            _ => return Err(error("Manager command input does not match its request")),
        };
        let body = serde_json::to_vec(&CommandEnvelope { operator_schema: ui::OPERATOR_SCHEMA, command })
            .map_err(|_| error("Request encoding failed"))?;
        if body.len() > REQUEST_LIMIT { return Err(error("Manager request exceeded its bound")); }
        let mut frame = COMMAND_GREETING.to_vec();
        frame.extend((body.len() as u32).to_le_bytes());
        frame.extend(body);
        let deadline = Instant::now() + request.deadline();
        let mut peer = connect(&self.socket, CONNECT_DEADLINE).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => Failure::Unavailable,
            _ => error("Bridge service is unavailable; the command was not submitted"),
        })?;
        // Once connected, any transport failure is uncertain. Do not reconnect
        // or start a local writer: the owner may already have accepted it.
        let uncertain = || error("Bridge command acknowledgement was lost. Refresh to inspect the result; the command was not repeated.");
        peer.set_write_timeout(Some(Duration::from_secs(5))).map_err(|_| uncertain())?;
        peer.write_all(&frame).map_err(|_| uncertain())?;
        if let Some(input) = input { send_file(&peer, &input).map_err(|_| uncertain())?; }
        match read_reply(&mut peer, deadline) {
            Ok(reply) => Ok(reply),
            Err(Attempt::Failed(failure)) => Err(failure),
            Err(Attempt::Dropped) => Err(uncertain()),
        }
    }
}
fn read_once(peer: &mut UnixStream, frame: &[u8], deadline: Instant) -> Result<Vec<u8>, Attempt> {
    peer.set_write_timeout(Some(Duration::from_secs(1))).map_err(|_| Attempt::Dropped)?;
    peer.write_all(frame).map_err(|_| Attempt::Dropped)?;
    read_reply(peer, deadline)
}
fn read_reply(peer: &mut UnixStream, deadline: Instant) -> Result<Vec<u8>, Attempt> {
    let lost = |e: io::Error| match e.kind() {
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => Attempt::Failed(Failure::Error(
            "Manager readback timed out. Refresh to inspect any accepted operation.".into())),
        _ => Attempt::Dropped,
    };
    let read = |peer: &mut UnixStream, buffer: &mut [u8]| {
        peer.set_read_timeout(Some(deadline.saturating_duration_since(Instant::now())
            .max(Duration::from_millis(1))))?;
        peer.read_exact(buffer)
    };
    let mut header = [0; 5];
    read(peer, &mut header).map_err(lost)?;
    let length = u32::from_le_bytes(header[1..].try_into().expect("four length bytes")) as u64;
    match header[0] {
        0 if length <= REPLY_LIMIT => {}
        1 if length <= REFUSAL_LIMIT => {}
        0 | 1 => return Err(Attempt::Failed(Failure::Error("Manager response exceeded its bound".into()))),
        _ => return Err(Attempt::Dropped),
    }
    let mut bytes = vec![0; length as usize];
    read(peer, &mut bytes).map_err(lost)?;
    if header[0] == 0 { Ok(bytes) } else { Err(Attempt::Failed(refusal(&bytes))) }
}
/// Connect to `socket`, waiting at most `within` while its listen queue is
/// full; any other refusal is at once.
#[cfg(target_os = "linux")]
fn connect(socket: &Path, within: Duration) -> io::Result<UnixStream> {
    use std::os::{fd::{AsRawFd, FromRawFd}, unix::ffi::OsStrExt};
    let path = socket.as_os_str().as_bytes();
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if path.len() >= address.sun_path.len() { return Err(io::ErrorKind::InvalidInput.into()); }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (slot, byte) in address.sun_path.iter_mut().zip(path) { *slot = *byte as libc::c_char; }
    let length = (std::mem::size_of::<libc::sa_family_t>() + path.len() + 1) as libc::socklen_t;
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK, 0) };
    if fd < 0 { return Err(io::Error::last_os_error()); }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    let deadline = Instant::now() + within;
    while unsafe { libc::connect(stream.as_raw_fd(), (&address as *const libc::sockaddr_un).cast(), length) } != 0 {
        let error = io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EAGAIN) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            Some(libc::EAGAIN) => return Err(io::ErrorKind::TimedOut.into()),
            Some(libc::EINTR) => {}
            _ => return Err(error),
        }
    }
    stream.set_nonblocking(false)?;
    Ok(stream)
}
#[cfg(not(target_os = "linux"))]
fn connect(socket: &Path, _within: Duration) -> io::Result<UnixStream> {
    UnixStream::connect(socket)
}

/// The installed service. An unavailable service never grants a local client
/// permission to become a substitute state owner.
pub struct Installed {
    service: Service,
}
impl Installed {
    pub fn new(service: Service) -> Self {
        Self { service }
    }
    pub fn home(home: &Path) -> Self {
        Self::new(Service::installed(home))
    }
}
impl Transport for Installed {
    fn exchange(&self, request: &Request, input: Option<File>) -> Result<Vec<u8>, Failure> {
        self.service.exchange(request, input)
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
    // The window's preview fixture; it moves with the window crate.
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
    fn browser_return_covers_delivery_waits_without_extending_ordinary_commands() {
        let callback = Request::BrowserReturn("native-access://auth?code=fake".into());
        let delivery_waits = Duration::from_secs(25 + 3 + 70);
        assert!(callback.deadline() >= CONNECT_DEADLINE + delivery_waits);
        assert!(callback.deadline() <= Duration::from_secs(100), "delivery remains bounded");
        assert_eq!(action().deadline(), Duration::from_secs(45));
        assert_eq!(Request::Read(Read::Pulse).deadline(), Duration::from_secs(45));
        assert_eq!(Request::ImportInstaller { name: "Setup.exe".into() }.deadline(),
            Duration::from_secs(300));
    }

    #[test]
    fn command_form_round_trips_every_client_request() {
        let key = ProductKey { environment: "environment".into(), module_sha256: "ab".repeat(32),
            class_id: "cd".repeat(16) };
        let requests = [Request::Read(Read::Overview), Request::Read(Read::CurrentOverview), product(&key),
            Request::Read(Read::Snapshot), Request::Read(Read::Activity), Request::Read(Read::Pulse),
            Request::Read(Read::Operation("ab".repeat(16))),
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
    fn operation_readback_requires_the_requested_identity_and_supported_envelope() {
        let id = "ab".repeat(16);
        let request = Request::Read(Read::Operation(id.clone()));
        let record = serde_json::json!({"schema":1,"operation":id,"state":"completed",
            "result":{"export":"cd".repeat(16),"sha256":"ef".repeat(32)}});
        let value = serde_json::to_value(ui::OperationReadback { schema: 1,
            operator_schema: ui::OPERATOR_SCHEMA, operation: id.clone(), record }).unwrap();
        assert!(matches!(decode(&request, &bytes(&value)), Ok(Reply::Operation(_))));
        let mut older = value.clone(); older["record"]["schema"] = 2.into();
        assert!(matches!(decode(&request, &bytes(&older)), Ok(Reply::Operation(_))));
        for field in ["schema", "operator_schema", "operation", "record"] {
            let mut changed = value.clone(); changed[field] = serde_json::Value::Null;
            assert!(decode(&request, &bytes(&changed)).is_err(), "{field}");
        }
        for (field, changed_value) in [("schema", serde_json::json!(3)),
            ("operation", serde_json::json!("cd".repeat(16))), ("state", serde_json::json!(""))] {
            let mut changed = value.clone(); changed["record"][field] = changed_value;
            assert!(decode(&request, &bytes(&changed)).is_err(), "record.{field}");
        }
        assert!(decode(&Request::Read(Read::Operation("cd".repeat(16))), &bytes(&value)).is_err());
        assert!(decode(&Request::Read(Read::Operation("../latest".into())), &bytes(&value)).is_err());
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
    /// An executable script. A shell writes it, so this test process never
    /// holds it open for writing while another test thread starts a child
    /// (which would make running it fail with "Text file busy").
    fn script(path: &Path, body: &str) {
        let mut writer = Command::new("/bin/sh")
            .args(["-c", "umask 077; cat > \"$1\" && chmod 700 \"$1\"", "sh"]).arg(path)
            .stdin(Stdio::piped()).spawn().unwrap();
        writer.stdin.take().unwrap().write_all(body.as_bytes()).unwrap();
        assert!(writer.wait().unwrap().success());
    }
    #[test]
    fn service_reads_fail_clearly_and_never_create_a_fallback_owner() {
        use std::os::unix::net::UnixListener;
        let root = PathBuf::from("/tmp").join(format!("lvb-service-{}", crate::random_id().unwrap()));
        std::fs::create_dir_all(&root).unwrap();
        let socket = root.join("owner.sock");
        let program = root.join("manager");
        script(&program, "#!/bin/sh\necho \"$2 from the command\"\n");
        let installed = |socket: &Path| Installed::new(Service::new(socket));
        let pulse = Request::Read(Read::Pulse);
        // No service listens: an operation paused it, or it is restarting.
        assert_eq!(Service::new(&socket).exchange(&pulse, None).unwrap_err(), Failure::Unavailable);
        assert_eq!(installed(&socket).exchange(&pulse, None).unwrap_err(), Failure::Unavailable);
        assert_eq!(installed(&socket).exchange(&Request::Read(Read::Overview), None).unwrap_err(), Failure::Unavailable);
        let listener = UnixListener::bind(&socket).unwrap();
        let serve = |connections: usize, answer: fn(&mut UnixStream)| {
            let listener = listener.try_clone().unwrap();
            std::thread::spawn(move || for _ in 0..connections { answer(&mut listener.accept().unwrap().0) })
        };
        // This release's service: one read, one reply, as the command writes it.
        let current = serve(1, |peer| {
            let mut greeting = [0; 5];
            peer.read_exact(&mut greeting).unwrap();
            assert_eq!(&greeting, VIEW_GREETING);
            assert_eq!(read_view_request(peer).unwrap(), Read::Pulse);
            write_view_reply(peer, Ok(&b"held pulse\n"[..])).unwrap();
        });
        assert_eq!(installed(&socket).exchange(&pulse, None).unwrap(), b"held pulse\n");
        current.join().unwrap();
        let exact = serve(1, |peer| {
            let mut greeting = [0; 5]; peer.read_exact(&mut greeting).unwrap();
            assert_eq!(&greeting, VIEW_GREETING);
            let id = "ab".repeat(16);
            assert_eq!(read_view_request(peer).unwrap(), Read::Operation(id.clone()));
            let reply = Reply::Operation(ui::OperationReadback { schema: 1,
                operator_schema: ui::OPERATOR_SCHEMA, operation: id.clone(),
                record: serde_json::json!({"schema":1,"operation":id,"state":"completed"}) });
            write_view_reply(peer, Ok(&reply.written().unwrap())).unwrap();
        });
        assert!(matches!(Client::new(installed(&socket)).call(
            &Request::Read(Read::Operation("ab".repeat(16))), None),
            Ok(Reply::Operation(value)) if value.record["state"] == "completed"));
        exact.join().unwrap();
        // A read the service dropped is asked once more.
        let restarting = listener.try_clone().unwrap();
        let restarted = std::thread::spawn(move || for answered in [false, true] {
            let (mut peer, _) = restarting.accept().unwrap();
            let mut greeting = [0; 5];
            peer.read_exact(&mut greeting).unwrap();
            assert_eq!(read_view_request(&mut peer).unwrap(), Read::Pulse);
            if answered { write_view_reply(&mut peer, Ok(&b"asked again\n"[..])).unwrap(); }
        });
        assert_eq!(installed(&socket).exchange(&pulse, None).unwrap(), b"asked again\n");
        restarted.join().unwrap();
        // An earlier service reads the greeting as a registration and closes,
        // each time. That is reported at once, and not as a version mismatch.
        let earlier = serve(2, |peer| {
            let mut registration = [0; 53];
            peer.read_exact(&mut registration).unwrap();
        });
        let started = Instant::now();
        assert_eq!(installed(&socket).exchange(&pulse, None).unwrap_err(), Failure::Error(
            "The bridge service stopped answering this status read. Refresh to try again.".into()));
        assert!(started.elapsed() < Duration::from_secs(5), "reported at once, never waited out");
        earlier.join().unwrap();
        // A service that stops while answering: once nothing listens, the
        // command answers. Its socket is removed, not only closed: a child
        // another test thread forks meanwhile can hold the listener open
        // until it runs its program.
        let stopping = root.join("stopping.sock");
        let stopping_listener = UnixListener::bind(&stopping).unwrap();
        let stopped = stopping.clone();
        let stop = std::thread::spawn(move || {
            let (mut peer, _) = stopping_listener.accept().unwrap();
            let mut greeting = [0; 5];
            peer.read_exact(&mut greeting).unwrap();
            std::fs::remove_file(&stopped).unwrap();
            drop(stopping_listener);
        });
        assert_eq!(installed(&stopping).exchange(&pulse, None).unwrap_err(), Failure::Unavailable);
        stop.join().unwrap();
        // A service of another operator model refuses with what the window shows.
        let other = serve(1, |peer| {
            let mut greeting = [0; 5];
            peer.read_exact(&mut greeting).unwrap();
            let refused = read_view_request(peer).unwrap_err();
            write_view_reply(peer, Err(&refusal_text(&refused))).unwrap();
        });
        let body = serde_json::to_vec(&ViewRequest { operator_schema: ui::OPERATOR_SCHEMA - 1,
            command: pulse.command() }).unwrap();
        let mut peer = UnixStream::connect(&socket).unwrap();
        peer.write_all(VIEW_GREETING).unwrap();
        peer.write_all(&(body.len() as u32).to_le_bytes()).unwrap();
        peer.write_all(&body).unwrap();
        let mut reply = Vec::new();
        peer.read_to_end(&mut reply).unwrap();
        assert_eq!(reply[0], 1);
        assert_eq!(refusal(&reply[5..]), Failure::Error(format!("Error: \"{}\"\n", paired_model_required())));
        other.join().unwrap();
        // A missing import descriptor is refused before connecting.
        assert!(Service::new(&socket).exchange(&Request::ImportInstaller { name: "test.exe".into() }, None).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn commands_cross_the_owner_socket_once_and_lost_receipts_are_never_replayed() {
        use std::os::unix::net::UnixListener;
        let root = PathBuf::from("/tmp").join(format!("lvb-command-{}", crate::random_id().unwrap()));
        std::fs::create_dir(&root).unwrap();
        let socket = root.join("owner.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let serving = listener.try_clone().unwrap();
        let server = std::thread::spawn(move || {
            let (mut peer, _) = serving.accept().unwrap();
            let mut greeting = [0; 5];
            peer.read_exact(&mut greeting).unwrap();
            assert_eq!(&greeting, COMMAND_GREETING);
            let (request, input) = read_command_request(&mut peer).unwrap();
            assert_eq!(request, action());
            assert!(input.is_none());
            // The owner accepted it, but the connection ends before its receipt.
        });
        let failure = Service::new(&socket).exchange(&action(), None).unwrap_err();
        server.join().unwrap();
        assert!(matches!(failure, Failure::Error(ref text) if text.contains("was not repeated")));
        listener.set_nonblocking(true).unwrap();
        assert_eq!(listener.accept().unwrap_err().kind(), io::ErrorKind::WouldBlock);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn installer_command_transfers_the_selected_open_file_not_its_replaced_path() {
        use std::os::unix::{fs::MetadataExt, net::UnixListener};
        let root = PathBuf::from("/tmp").join(format!("lvb-import-{}", crate::random_id().unwrap()));
        std::fs::create_dir(&root).unwrap();
        let source = root.join("source.exe");
        std::fs::write(&source, b"selected bytes").unwrap();
        let selected = File::open(&source).unwrap();
        let original = selected.metadata().unwrap();
        std::fs::remove_file(&source).unwrap();
        std::fs::write(&source, b"replacement bytes").unwrap();
        let socket = root.join("owner.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = std::thread::spawn(move || {
            let (mut peer, _) = listener.accept().unwrap();
            let mut greeting = [0; 5]; peer.read_exact(&mut greeting).unwrap();
            assert_eq!(&greeting, COMMAND_GREETING);
            let (request, input) = read_command_request(&mut peer).unwrap();
            assert_eq!(request, Request::ImportInstaller { name: "chosen.exe".into() });
            let mut input = input.unwrap();
            let received = input.metadata().unwrap();
            assert_eq!((received.dev(), received.ino()), (original.dev(), original.ino()));
            let mut bytes = Vec::new(); input.read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, b"selected bytes");
            write_view_reply(&mut peer, Ok(b"imported")).unwrap();
        });
        assert_eq!(Service::new(&socket).exchange(&Request::ImportInstaller { name: "chosen.exe".into() }, Some(selected)).unwrap(), b"imported");
        server.join().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn command_envelope_refuses_wrong_schema_and_oversized_body_before_dispatch() {
        for body in [serde_json::to_vec(&CommandEnvelope { operator_schema: ui::OPERATOR_SCHEMA - 1,
            command: CommandRequest::Action { request: match action() { Request::Action(r) => r, _ => unreachable!() } } }).unwrap(),
            vec![b' '; REQUEST_LIMIT + 1]] {
            let (mut client, mut server) = UnixStream::pair().unwrap();
            client.write_all(&(body.len() as u32).to_le_bytes()).unwrap();
            if body.len() <= REQUEST_LIMIT { client.write_all(&body).unwrap(); }
            assert!(read_command_request(&mut server).is_err());
        }
    }
    #[test]
    fn subprocess_carries_exit_status_bounds_and_input() {
        let root = std::env::temp_dir().join(format!("lvb-contract-{}", crate::random_id().unwrap()));
        std::fs::create_dir_all(&root).unwrap();
        let program = root.join("manager");
        // Echo the request body back as the reply; refuse when told to.
        script(&program, "#!/bin/sh\ncase \"$2\" in\n  request) cat ;;\n  pulse) echo 'Error: \"operator_state_changed_refresh\"' >&2; exit 1 ;;\n  snapshot) head -c 8388609 /dev/zero ;;\nesac\n");
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
