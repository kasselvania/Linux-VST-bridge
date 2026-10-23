//! Native window adapter. Service and dialog work stays on one persistent worker.
#![cfg(not(target_arch = "wasm32"))]
use crate::{presentation::Built, Outcome};
use makepad_product_ui::InputPath;
#[cfg(feature = "samples")]
use crate::preview::{Preview, Selection};
#[cfg(feature = "live")]
use crate::live::Live;

pub enum Controller {
    #[cfg(feature = "samples")]
    Sample(Box<Preview>),
    #[cfg(feature = "live")]
    Live { model: Box<Live>, worker: Worker },
}

impl Controller {
    pub fn new(args: &[String], posture: String, reveal: bool) -> Self {
        #[cfg(feature = "live")]
        if !cfg!(feature = "samples") || args.iter().any(|arg| arg == "--live")
            || std::env::current_exe().ok().is_some_and(|path|
                path.file_name().is_some_and(|name| name == "linux-audio-compatibility-manager"))
        {
            return Self::Live { model: Box::new(Live::new(posture, reveal)), worker: Worker::start(args.iter().any(|arg| arg == "--continue-update")) };
        }
        #[cfg(feature = "samples")]
        {
            let mut model = Preview::new(Selection::from_args(args));
            model.posture = posture;
            model.reveal = reveal;
            model.rebuild();
            if args.iter().any(|arg| arg == "--long-row-fixture") { model.long_row_fixture(); }
            if args.iter().any(|arg| arg == "--support-report-fixture") { model.report_fixture(); }
            Self::Sample(Box::new(model))
        }
        #[cfg(not(feature = "samples"))]
        unreachable!("a frontend feature is required")
    }

    pub fn is_live(&self) -> bool {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(_) => false,
            #[cfg(feature = "live")]
            Self::Live { .. } => true,
        }
    }

    pub fn bootstrap_trace(&self) -> Option<(u64, &'static str)> {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(_) => None,
            #[cfg(feature = "live")]
            Self::Live { model, .. } => model.bootstrap_trace(),
        }
    }

    pub fn built(&self) -> &Built {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(model) => &model.built,
            #[cfg(feature = "live")]
            Self::Live { model, .. } => &model.built,
        }
    }
    pub fn press(&mut self, id: &str, via: InputPath) -> Outcome {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(model) => model.press(id, via),
            #[cfg(feature = "live")]
            Self::Live { model, .. } => model.press(id, via),
        }
    }
    pub fn note(&mut self, line: String) {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(model) => model.note(line),
            #[cfg(feature = "live")]
            Self::Live { model, .. } => model.note(line),
        }
    }
    pub fn new_log(&mut self) -> Vec<String> {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(model) => model.new_log(),
            #[cfg(feature = "live")]
            Self::Live { model, .. } => model.new_log(),
        }
    }
    pub fn field(&mut self, id: &str, text: &str) {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(model) => model.field(id, text),
            #[cfg(feature = "live")]
            Self::Live { model, .. } => model.field(id, text),
        }
    }
    pub fn dismiss_form(&mut self, how: &str) {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(model) => model.dismiss_form(how),
            #[cfg(feature = "live")]
            Self::Live { model, .. } => model.dismiss_form(how),
        }
    }
    pub fn page_first(&self) -> Option<String> {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(model) => model.page_first(),
            #[cfg(feature = "live")]
            Self::Live { model, .. } => model.page_first(),
        }
    }
    pub fn take_clipboard(&mut self) -> Option<String> {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(_) => None,
            #[cfg(feature = "live")]
            Self::Live { model, .. } => model.take_clipboard(),
        }
    }
    pub fn take_close_request(&mut self) -> bool {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(_) => false,
            #[cfg(feature = "live")]
            Self::Live { model, .. } => model.take_close_request(),
        }
    }
    pub fn tick(&mut self) -> bool {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(_) => false,
            #[cfg(feature = "live")]
            Self::Live { model, .. } => {
                let before = model.render_revision();
                model.tick();
                model.render_revision() != before
            },
        }
    }
    pub fn pump(&mut self) -> bool {
        match self {
            #[cfg(feature = "samples")]
            Self::Sample(_) => false,
            #[cfg(feature = "live")]
            Self::Live { model, worker } => {
                let before = model.render_revision();
                if let Some(reply) = worker.receive() {
                    match reply {
                        WorkerReply::Stopped(error) => model.worker_stopped(error),
                        WorkerReply::Bootstrap { panel, close } => {
                            let has_progress = panel.progress.is_some();
                            makepad_product_ui::makepad_widgets::log!("[window] bootstrap ui_received has_progress={} close={}", has_progress, close);
                            model.bootstrap(panel, close);
                            if let Some((revision, phase)) = model.bootstrap_trace() {
                                makepad_product_ui::makepad_widgets::log!("[window] bootstrap ui_applied revision={} phase={}", revision, phase);
                            }
                        },
                        WorkerReply::Client(reply) => model.receive(reply),
                        WorkerReply::ReportPreview(reply) => model.receive_report_preview(reply),
                        WorkerReply::Update { panel, headline, close, available } => model.update(panel, headline, close, available),
                        WorkerReply::UpdateError(error) => model.updater_failed(error),
                        WorkerReply::Superseded => model.pause_superseded(),
                    }
                }
                if let Some(report) = model.take_report_preview() {
                    if let Err(error) = worker.preview_report(report) { model.receive_report_preview(Err(error)); }
                }
                if let Some(query) = model.take_query() {
                    if let Err(error) = worker.send(query) { model.receive(lvb_client::Reply::Error(error)); }
                }
                if let Some((step, draft)) = model.take_update_step() {
                    if let Err(error) = worker.update(step, draft) { model.updater_failed(error); }
                }
                if let Some(step) = model.take_bootstrap_step() {
                    if let Err(error) = worker.bootstrap(step) { model.bootstrap_failed(error); }
                }
                worker.reading(model.updater_reading());
                model.render_revision() != before
            }
        }
    }
}

#[cfg(feature = "live")]
enum WorkerRequest {
    Bootstrap(lvb_client::bootstrap::Step),
    Client(lvb_client::Query),
    ReportPreview(lvb_client::SupportReport),
    Update(lvb_view_model::updates::Step, Option<String>),
    Reading(Option<(bool, f64)>),
}
#[cfg(feature = "live")]
#[allow(dead_code)]
enum WorkerReply {
    Stopped(String),
    Bootstrap { panel: lvb_client::bootstrap::Panel, close: bool },
    Client(lvb_client::Reply),
    ReportPreview(Result<lvb_client::report::Contents, String>),
    Update { panel: lvb_view_model::updates::Panel, headline: Option<String>, close: bool, available: bool },
    UpdateError(String),
    Superseded,
}
// Admission must be observed after any client call: a native chooser can stay
// open while another frontend selects a new generation.
#[cfg(feature = "live")]
fn client_then_admit(
    request: Option<WorkerRequest>,
    call: impl FnOnce(lvb_client::Query) -> lvb_client::Reply,
    selected: impl FnOnce() -> Result<bool, String>,
) -> (Option<WorkerRequest>, Option<lvb_client::Reply>, Result<bool, String>) {
    let (request, reply) = match request {
        Some(WorkerRequest::Client(query)) => (None, Some(call(query))),
        request => (request, None),
    };
    (request, reply, selected())
}

#[cfg(feature = "live")]
pub struct Worker {
    requests: std::sync::mpsc::SyncSender<WorkerRequest>,
    replies: std::sync::mpsc::Receiver<WorkerReply>,
    disconnected: bool,
    reading: Option<(bool, f64)>,
}
#[cfg(feature = "live")]
impl Worker {
    fn preview_report(&self, report: lvb_client::SupportReport) -> Result<(), String> {
        self.requests.try_send(WorkerRequest::ReportPreview(report)).map_err(|_| "The report worker stopped.".into())
    }
    // This adapter is native-only; the shared Linux updater uses std's clock.
    // The portable model retains the original platform-clock observation time.
    #[allow(clippy::disallowed_types, clippy::disallowed_methods)]
    fn start(continue_update: bool) -> Self {
        use std::sync::mpsc::sync_channel;
        let (requests, incoming) = sync_channel(4);
        let (outgoing, replies) = sync_channel(4);
        // Dropping the window closes both channels. No join on the UI thread and
        // no cancellation of an operation already accepted by the service.
        std::thread::Builder::new().name("bridge-client".into()).spawn(move || {
            #[cfg(not(target_os = "linux"))]
            let _ = continue_update;
            #[cfg(target_os = "linux")]
            let mut bootstrap = match lvb_client::bootstrap::entry() {
                Ok(lvb_client::bootstrap::Entry::Ordinary) => None,
                Ok(lvb_client::bootstrap::Entry::Checking) => Some(if continue_update {
                    lvb_client::bootstrap::State::continuing_update()
                } else { lvb_client::bootstrap::State::checking() }),
                Err(error) => Some(lvb_client::bootstrap::State::attention(error)),
            };
            #[cfg(target_os = "linux")]
            if let Some(state) = &bootstrap {
                if outgoing.send(WorkerReply::Bootstrap { panel: state.panel(), close: false }).is_err() { return; }
                makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
            }
            #[cfg(target_os = "linux")]
            let client = (|| {
                let home = std::env::var_os("HOME").ok_or("Home directory unavailable")?;
                let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
                lvb_client::Client::managed(home.into(), executable)
            })();
            #[cfg(target_os = "linux")]
            let mut updates = None;
            #[cfg(target_os = "linux")]
            let mut paused = false;
            #[cfg(target_os = "linux")]
            let mut update_error = None;
            #[cfg(target_os = "linux")]
            let mut reading: Option<(bool, std::time::Instant)> = None;
            #[cfg(target_os = "linux")]
            let mut shown = None;
            loop {
                let request = match incoming.recv_timeout(std::time::Duration::from_millis(250)) {
                    Ok(request) => Some(request),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                };
                #[cfg(target_os = "linux")]
                if let Some(state) = &mut bootstrap {
                    let chosen = if let Some(WorkerRequest::Bootstrap(step)) = request { state.press(step); true } else { false };
                    if let Some(operation) = state.take_operation() {
                        makepad_product_ui::makepad_widgets::log!("[window] bootstrap worker_taken operation={:?}", operation);
                        if outgoing.send(WorkerReply::Bootstrap { panel: state.panel(), close: false }).is_err() { break; }
                        makepad_product_ui::makepad_widgets::log!("[window] bootstrap worker_sent_running operation={:?}", operation);
                        makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                        let close = state.receive(lvb_client::bootstrap::run_and_open(operation));
                        if outgoing.send(WorkerReply::Bootstrap { panel: state.panel(), close }).is_err() { break; }
                        makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                        if close { break; }
                    } else if chosen {
                        if outgoing.send(WorkerReply::Bootstrap { panel: state.panel(), close: false }).is_err() { break; }
                        makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                    }
                    continue;
                }
                let (request, reply, admission) = client_then_admit(request, |query| {
                    #[cfg(target_os = "linux")]
                    { client.as_ref().map_err(Clone::clone)
                        .and_then(|client| client.call(query, &lvb_client::native::RfdSelection))
                        .unwrap_or_else(lvb_client::Reply::Error) }
                    #[cfg(not(target_os = "linux"))]
                    { let _ = query; lvb_client::Reply::Error("The installed Bridge client requires Linux".into()) }
                }, || {
                    #[cfg(target_os = "linux")]
                    { client.as_ref().map_err(Clone::clone).and_then(|client| client.current_frontend()) }
                    #[cfg(not(target_os = "linux"))]
                    { Err("Installed updates require Linux".into()) }
                });
                if let Some(reply) = reply {
                    if outgoing.send(WorkerReply::Client(reply)).is_err() { break; }
                    makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                }
                #[cfg(not(target_os = "linux"))]
                let _ = admission;
                #[cfg(target_os = "linux")]
                let mut update_ack = false;
                #[cfg(target_os = "linux")]
                let update_current = matches!(admission, Ok(true));
                #[cfg(target_os = "linux")]
                if update_current && updates.is_none() { updates = Some(lvb_client::updates::Updates::new()); }
                #[cfg(target_os = "linux")]
                if updates.is_some() && !paused {
                    match admission {
                        Ok(false) => {
                            paused = true;
                            if outgoing.send(WorkerReply::Superseded).is_err() { break; }
                            makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                        }
                        Err(error) if update_error.as_ref() != Some(&error) => {
                            if outgoing.send(WorkerReply::UpdateError(error.clone())).is_err() { break; }
                            update_error = Some(error);
                            makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                        }
                        Ok(true) => update_error = None,
                        _ => {}
                    }
                }
                match request {
                    Some(WorkerRequest::ReportPreview(report)) => {
                        #[cfg(target_os = "linux")]
                        let result = client.as_ref().map_err(Clone::clone)
                            .and_then(|client| client.preview_support_report(&report));
                        #[cfg(not(target_os = "linux"))]
                        let result = { let _ = report; Err("The installed Bridge client requires Linux".into()) };
                        if outgoing.send(WorkerReply::ReportPreview(result)).is_err() { break; }
                        makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                    }
                    Some(WorkerRequest::Bootstrap(step)) => { let _ = step; },
                    Some(WorkerRequest::Client(_)) => unreachable!("client call already completed"),
                    Some(WorkerRequest::Update(step, draft)) => {
                        #[cfg(target_os = "linux")]
                        {
                            if update_current && !paused {
                                if let Some(updates) = &mut updates { updates.step(step, draft); update_ack = true; }
                            } else if outgoing.send(WorkerReply::UpdateError("Open the current installed manager to update.".into())).is_err() { break; }
                        }
                        #[cfg(not(target_os = "linux"))]
                        { let _ = (step, draft); if outgoing.send(WorkerReply::UpdateError("Installed updates require Linux".into())).is_err() { break; } }
                    }
                    Some(WorkerRequest::Reading(value)) => {
                        #[cfg(target_os = "linux")]
                        { reading = value.and_then(|(in_use, observed)| {
                            let age = (makepad_product_ui::makepad_widgets::Cx::monotonic_now() - observed).max(0.0);
                            std::time::Instant::now().checked_sub(std::time::Duration::from_secs_f64(age)).map(|at| (in_use, at))
                        }); }
                        #[cfg(not(target_os = "linux"))]
                        let _ = value;
                    }
                    None => {}
                }
                #[cfg(target_os = "linux")]
                if !paused {
                    // A step may itself touch disk or open Setup. Never carry
                    // its earlier admission into a poll that can hand over.
                    match client.as_ref().map_err(Clone::clone).and_then(|client| client.current_frontend()) {
                        Ok(true) => {},
                        Ok(false) => {
                            paused = true;
                            if outgoing.send(WorkerReply::Superseded).is_err() { break; }
                            makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                            continue;
                        },
                        Err(error) => {
                            if update_error.as_ref() != Some(&error) {
                                if outgoing.send(WorkerReply::UpdateError(error.clone())).is_err() { break; }
                                update_error = Some(error);
                                makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                            }
                            continue;
                        }
                    }
                    let Some(updates) = &mut updates else { continue };
                    let close = updates.poll(reading.filter(|(_, at)| at.elapsed() < std::time::Duration::from_secs(15)));
                    let view = (updates.panel(), updates.headline(), close, updates.available());
                    if update_ack || shown.as_ref() != Some(&view) {
                        if outgoing.send(WorkerReply::Update { panel: view.0.clone(), headline: view.1.clone(), close, available: view.3 }).is_err() { break; }
                        shown = Some(view);
                        makepad_product_ui::makepad_widgets::SignalToUI::set_ui_signal();
                    }
                }
            }
        }).expect("start the Bridge client worker");
        Self { requests, replies, disconnected: false, reading: None }
    }
    fn send(&self, query: lvb_client::Query) -> Result<(), String> {
        self.requests.try_send(WorkerRequest::Client(query)).map_err(|_|
            "The Bridge request could not be queued. Refresh to read its status.".into())
    }
    fn bootstrap(&self, step: lvb_client::bootstrap::Step) -> Result<(), String> {
        self.requests.try_send(WorkerRequest::Bootstrap(step)).map_err(|_| "Setup could not receive this choice. Check again.".into())
    }
    fn update(&self, step: lvb_view_model::updates::Step, draft: Option<String>) -> Result<(), String> {
        self.requests.try_send(WorkerRequest::Update(step, draft)).map_err(|_| "The updater could not receive this choice. Try again.".into())
    }
    fn reading(&mut self, value: Option<(bool, f64)>) {
        if self.reading != value && self.requests.try_send(WorkerRequest::Reading(value)).is_ok() { self.reading = value; }
    }
    fn receive(&mut self) -> Option<WorkerReply> {
        match self.replies.try_recv() {
            Ok(reply) => Some(reply),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) if !self.disconnected => {
                self.disconnected = true;
                Some(WorkerReply::Stopped("The Bridge client stopped. Reopen the window to reconnect.".into()))
            }
            Err(_) => None,
        }
    }
}

#[cfg(all(test, feature = "live"))]
mod tests {
    use super::*;
    #[test]
    fn chooser_return_rechecks_selection_before_any_update_work() {
        use std::sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc};
        let selected = Arc::new(AtomicBool::new(true));
        let selected_on_worker = selected.clone();
        let (opened, is_open) = mpsc::sync_channel(0);
        let (resume, resumed) = mpsc::sync_channel(0);
        let task = std::thread::spawn(move || client_then_admit(
            Some(WorkerRequest::Client(lvb_client::Query::CurrentOverview)),
            |_| {
                opened.send(()).unwrap();
                resumed.recv().unwrap();
                lvb_client::Reply::FrontendSuperseded
            },
            || Ok(selected_on_worker.load(Ordering::SeqCst)),
        ));
        is_open.recv().unwrap();
        selected.store(false, Ordering::SeqCst);
        resume.send(()).unwrap();
        let (pending, reply, admission) = task.join().unwrap();
        assert!(pending.is_none());
        assert!(matches!(reply, Some(lvb_client::Reply::FrontendSuperseded)));
        assert_eq!(admission, Ok(false));
    }
}
