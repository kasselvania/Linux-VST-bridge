//! Setup's beta updates: paste the beta link once, check the signed update
//! feed, download and stage a newer package, then hand over to that package's
//! own Setup, which runs the existing Update Bridge step with all its checks.
//! The beta link and its key are never shown.
use linux_vst_bridge::update_feed::{self as feed, BetaLink, Check, Offer, UpdateError};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

const DAILY: Duration = Duration::from_secs(24 * 60 * 60);
/// A Setup still running this long after it was opened has taken over.
const HANDOVER_SETTLE: Duration = Duration::from_millis(1500);
/// Without any reading after staging for this long, "Continue anyway" is
/// offered; Setup's own gate still refuses while plug-ins are in use.
const READING_GRACE: Duration = Duration::from_secs(30);
const SETUP_DID_NOT_OPEN: &str = "Setup for the new version could not open. Open Linux VST Bridge Setup and Updates from your applications menu and choose Update Bridge.";

type Admission = Arc<dyn Fn() -> Result<bool, String> + Send + Sync>;

enum Event {
    Ready(Result<(Admission, Option<BetaLink>, Option<String>), String>),
    Checked(Result<Check, UpdateError>),
    Progress(u64, u64),
    Staged(Result<PathBuf, UpdateError>),
}

enum Job {
    Check { home: PathBuf, link: BetaLink, admission: Admission },
    Download { home: PathBuf, link: BetaLink, offer: Offer, admission: Admission },
}
fn worker(home: Option<PathBuf>) -> (mpsc::SyncSender<Job>, mpsc::Receiver<Event>) {
    let (jobs, incoming) = mpsc::sync_channel(1);
    let (outgoing, events) = mpsc::sync_channel(128);
    std::thread::Builder::new().name("bridge-updates-io".into()).spawn(move || {
        let ready = (|| {
            let home = home.ok_or("Home directory unavailable")?;
            let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
            let client = crate::Client::managed(home.clone(), executable)?;
            let admission: Admission = Arc::new(move || client.current_frontend());
            admitted(&admission)?;
            let (link, message) = match feed::load_link(&home) {
                Ok(link) => (link, None),
                Err(_) => (None, Some("Your saved beta link could not be read. Paste it again.".into())),
            };
            Ok((admission, link, message))
        })();
        if outgoing.send(Event::Ready(ready)).is_err() { return; }
        while let Ok(job) = incoming.recv() {
            let event = match job {
                Job::Check { home, link, admission } => Event::Checked(admitted(&admission).map_err(UpdateError::Local).and_then(|()| context()).and_then(|(trust, current)| {
                    let systems = feed::system_names(&linux_vst_bridge::read_os_release());
                    feed::check(&home, &link, &trust, &current, &systems)
                })),
                Job::Download { home, link, offer, admission } => {
                    let mut last = None;
                    Event::Staged(context().and_then(|(trust, current)| feed::download_and_stage_checked(
                        &home, &link, &trust, &current, &offer, &|| admitted(&admission).map_err(UpdateError::LocalDownload), &mut |done, total| {
                            let percent = done.saturating_mul(100) / total.max(1);
                            if last != Some(percent) {
                                last = Some(percent);
                                let _ = outgoing.try_send(Event::Progress(done, total));
                            }
                        })))
                }
            };
            if outgoing.send(event).is_err() { break; }
        }
    }).expect("start the update I/O worker");
    (jobs, events)
}

fn admitted(check: &Admission) -> Result<(), String> {
    match check()? {
        true => Ok(()),
        false => Err("Bridge updated. Open the current manager to update.".into()),
    }
}

use lvb_view_model::updates::{self as panel, Panel, Phase, Step, UpdateNotice as Notice};
/// The updater's refusal as a notice: its sentence, with the exact reason kept
/// for technical details.
fn error_notice(error: UpdateError) -> Notice {
    Notice { technical: error.technical().map(str::to_owned), text: error.to_string() }
}

#[derive(Clone, Debug, PartialEq)]
enum Status {
    Idle,
    Checking,
    /// The last check failed.
    Failed(Notice),
    /// The installed version; nothing newer is published.
    UpToDate(String),
    NoPackage(String),
    /// None: a feed only a newer manager can read.
    Manual(Option<String>),
    Available(Offer),
    Downloading { offer: Offer, done: u64, total: u64 },
    /// Staged; waiting for a reading, taken after staging, that no DAW is
    /// using bridged plug-ins. `observed`: any reading after staging arrived.
    WaitingForDaw { offer: Offer, frontend: PathBuf, since: Instant, observed: bool },
    /// Setup for the new version is opening.
    HandingOver(Offer),
    /// Setup for the new version has taken over.
    HandedOver,
}

/// What to do with a Setup that was just opened.
#[derive(Debug, PartialEq, Eq)]
enum HandoverStep { Wait, Close, RetryPlain, Fail }
/// `exit`: None while Setup runs, else its exit code (None if killed).
fn handover_step(exit: Option<Option<i32>>, elapsed: Duration, retried: bool) -> HandoverStep {
    match exit {
        None if elapsed >= HANDOVER_SETTLE => HandoverStep::Close,
        None => HandoverStep::Wait,
        // Setup finished on its own, e.g. it found this version already selected.
        Some(Some(0)) => HandoverStep::Close,
        // A Setup that predates --continue-update refuses it at once.
        Some(Some(64)) if !retried => HandoverStep::RetryPlain,
        // systemd-run could not make the scope, or Setup failed by itself:
        // nothing is relaunched; this window stays open with a plain line.
        Some(_) => HandoverStep::Fail,
    }
}
struct Handover { child: Child, started: Instant, frontend: PathBuf, retried: bool }

pub struct Updates {
    admission: Option<Admission>,
    initializing: bool,
    admission_error: Option<String>,
    superseded: bool,
    home: Option<PathBuf>,
    link: Option<BetaLink>,
    draft: String,
    editing: bool,
    link_message: Option<String>,
    status: Status,
    receiver: Option<mpsc::Receiver<Event>>,
    jobs: Option<mpsc::SyncSender<Job>>,
    next_check: Option<Instant>,
    /// Why the last download or hand-over stopped; the offer stays.
    notice: Option<Notice>,
    handover: Option<Handover>,
}

impl Default for Updates { fn default() -> Self { Self::new() } }

impl Updates {
    pub fn new() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let (jobs, receiver) = worker(home.clone());
        Self { home, link: None, draft: String::new(), editing: false, link_message: None,
            status: Status::Idle, receiver: Some(receiver), jobs: Some(jobs), next_check: None, notice: None, handover: None,
            admission: None, initializing: true, admission_error: None, superseded: false }
    }
    /// Previews and tests: no link, no checks, no files.
    #[allow(dead_code)]
    pub fn disabled() -> Self {
        Self { admission: None, initializing: false, admission_error: None, superseded: false, home: None, link: None, draft: String::new(), editing: false, link_message: None,
            status: Status::Idle, receiver: None, jobs: None, next_check: None, notice: None, handover: None }
    }
    /// One state of Setup's update panel from the view model's samples, as
    /// the updater holds it. No saved link, no check, no download, no files.
    #[cfg(any(test, feature = "samples"))]
    pub fn sample(sample: &lvb_view_model::samples::UpdateSample) -> Self {
        let link = sample.linked.then(|| BetaLink::parse("https://beta.example.ts.net:8443/private-beta/?pw=s3cret").ok())
            .flatten();
        let notes = "Setup → Beta updates\r\nFaster plug-in list";
        let offer = |version: &str, notes: &str| Offer { version: version.into(), notes: notes.into(),
            system: "cachyos".into(), package: feed::FeedPackage { file: "a.install".into(), size: 10_000_000,
                sha256: "a".repeat(64) } };
        let soon = Instant::now() + Duration::from_secs(3600);
        let long_ago = Instant::now().checked_sub(READING_GRACE + Duration::from_secs(1)).unwrap_or_else(Instant::now);
        let status = match &sample.phase {
            Phase::Idle => Status::Idle,
            Phase::Checking => Status::Checking,
            Phase::Failed(notice) => Status::Failed(notice.clone()),
            Phase::UpToDate(installed) => Status::UpToDate(installed.clone()),
            Phase::NoPackage(latest) => Status::NoPackage(latest.clone()),
            Phase::Manual(latest) => Status::Manual(latest.clone()),
            Phase::Available { version, notes } => Status::Available(offer(version, notes)),
            Phase::Downloading { version, done, total } =>
                Status::Downloading { offer: offer(version, notes), done: *done, total: *total },
            Phase::Waiting { version, observed, long } => Status::WaitingForDaw { offer: offer(version, notes),
                frontend: "/nonexistent/setup".into(), since: if *long { long_ago } else { soon }, observed: *observed },
            Phase::HandingOver => Status::HandingOver(offer("0.12.0test148", notes)),
            Phase::HandedOver => Status::HandedOver,
        };
        Self { link, draft: sample.draft.clone(), editing: sample.editing, link_message: sample.link_message.clone(),
            status, notice: sample.notice.clone(), ..Self::disabled() }
    }
    /// Work is running that a new check or link must not interrupt.
    fn busy(&self) -> bool {
        matches!(self.status, Status::Checking | Status::Downloading { .. }
            | Status::HandingOver(_) | Status::HandedOver)
    }
    fn waiting(&self) -> bool { matches!(self.status, Status::WaitingForDaw { .. }) }
    /// An update is available or under way; Home shows one line for it.
    pub fn headline(&self) -> Option<String> {
        panel::headline(&self.phase())
    }
    /// Where the updater is, for the panel.
    fn phase(&self) -> Phase {
        match &self.status {
            Status::Idle => Phase::Idle,
            Status::Checking => Phase::Checking,
            Status::Failed(notice) => Phase::Failed(notice.clone()),
            Status::UpToDate(installed) => Phase::UpToDate(installed.clone()),
            Status::NoPackage(latest) => Phase::NoPackage(latest.clone()),
            Status::Manual(latest) => Phase::Manual(latest.clone()),
            Status::Available(offer) => Phase::Available { version: offer.version.clone(), notes: offer.notes.clone() },
            Status::Downloading { offer, done, total } =>
                Phase::Downloading { version: offer.version.clone(), done: *done, total: *total },
            Status::WaitingForDaw { offer, since, observed, .. } => Phase::Waiting { version: offer.version.clone(),
                observed: *observed, long: since.elapsed() >= READING_GRACE },
            Status::HandingOver(_) => Phase::HandingOver,
            Status::HandedOver => Phase::HandedOver,
        }
    }
    pub fn panel(&self) -> Panel {
        Panel::build(self.link.is_some(), self.editing, self.busy() || !self.available(), self.admission_error.as_deref().or(self.link_message.as_deref()).or(self.initializing.then_some("Checking installed updater…")),
            &self.phase(), self.notice.as_ref())
    }

    /// Shared admission, also used by the retained egui adapter. Capturing the
    /// immutable identity and reading the saved link happen on the I/O worker.
    pub fn available(&self) -> bool { !self.initializing && !self.superseded && self.admission_error.is_none() }
    fn admit(&mut self) -> bool {
        if self.initializing || self.superseded { return false; }
        let Some(check) = &self.admission else { return self.home.is_none(); };
        match check() {
            Ok(true) => { self.admission_error = None; true },
            Ok(false) => {
                self.superseded = true;
                self.admission_error = Some("Bridge updated. Open the current manager to update.".into());
                false
            },
            Err(error) => { self.admission_error = Some(error); false }
        }
    }

    /// Runs every frame: collects results, starts the start-up and daily
    /// checks, and hands over once a fresh reading shows no DAW is using
    /// bridged plug-ins. `plugins_in_use` is that reading and when it was
    /// taken. Returns true when this window should close because the new
    /// version's Setup has taken over.
    pub fn poll(&mut self, plugins_in_use: Option<(bool, Instant)>) -> bool {
        loop {
            let event = match self.receiver.as_ref().map(|receiver| receiver.try_recv()) {
                Some(Ok(event)) => event,
                Some(Err(mpsc::TryRecvError::Disconnected)) => {
                    self.receiver = None; self.jobs = None;
                    if self.initializing {
                        self.initializing = false;
                        self.admission_error = Some("The update worker stopped. Reopen the manager to continue.".into());
                    }
                    if matches!(self.status, Status::Checking | Status::Downloading { .. }) {
                        self.status = Status::Failed("The update worker stopped. Reopen the manager to continue.".into());
                    }
                    break;
                }
                _ => break,
            };
            match event {
                Event::Ready(result) => {
                    self.initializing = false;
                    match result {
                        Ok((admission, link, message)) => {
                            self.admission = Some(admission); self.link = link; self.link_message = message;
                            self.next_check = self.link.is_some().then(Instant::now);
                        },
                        Err(error) => self.admission_error = Some(error),
                    }
                }
                Event::Checked(result) => {

                    self.next_check = Some(Instant::now() + DAILY);
                    self.status = match result {
                        Ok(Check::Available(offer)) => Status::Available(offer),
                        Ok(Check::UpToDate { installed }) => Status::UpToDate(installed),
                        Ok(Check::NoPackage { latest, .. }) => Status::NoPackage(latest),
                        Ok(Check::ManualUpdate { latest }) => Status::Manual(latest),
                        Err(error) => Status::Failed(error_notice(error)),
                    };
                }
                Event::Progress(done, total) => {
                    if let Status::Downloading { done: d, total: t, .. } = &mut self.status {
                        *d = done; *t = total;
                    }
                }
                Event::Staged(result) => {

                    let Status::Downloading { offer, .. } = &self.status else { continue };
                    self.status = match result {
                        Ok(frontend) => Status::WaitingForDaw { offer: offer.clone(), frontend,
                            since: Instant::now(), observed: false },
                        Err(error) => {
                            self.notice = Some(error_notice(error));
                            Status::Available(offer.clone())
                        }
                    };
                }
            }
        }
        if !self.admit() { return false; }
        if !self.busy() && !self.waiting() && self.link.is_some()
            && self.next_check.is_some_and(|due| Instant::now() >= due) {
            self.start_check();
        }
        if let Status::WaitingForDaw { offer, frontend, since, observed } = &mut self.status {
            if let Some((in_use, _)) = plugins_in_use.filter(|(_, at)| *at > *since) {
                *observed = true;
                if !in_use {
                    let (offer, frontend) = (offer.clone(), frontend.clone());
                    self.start_handover(offer, frontend);
                }
            }
        }
        let close = self.poll_handover();
        close
    }

    fn start_handover(&mut self, offer: Offer, frontend: PathBuf) {
        if !self.admit() { return; }
        match open_setup(&frontend, true) {
            Ok(child) => {
                self.handover = Some(Handover { child, started: Instant::now(), frontend, retried: false });
                self.status = Status::HandingOver(offer);
            }
            Err(()) => {
                self.notice = Some(SETUP_DID_NOT_OPEN.into());
                self.status = Status::Available(offer);
            }
        }
    }

    /// Never waits on the UI thread: Setup's early exit is read once a frame.
    fn poll_handover(&mut self) -> bool {
        let Some(handover) = self.handover.as_mut() else { return false };
        let exit = match handover.child.try_wait() {
            Ok(None) => None,
            Ok(Some(status)) => Some(status.code()),
            Err(_) => Some(None),
        };
        match handover_step(exit, handover.started.elapsed(), handover.retried) {
            HandoverStep::Wait => false,
            HandoverStep::Close => {
                self.handover = None;
                self.status = Status::HandedOver;
                true
            }
            HandoverStep::RetryPlain => {
                // poll() renewed admission after any earlier client or dialog work.
                match open_setup(&handover.frontend, false) {
                    Ok(child) => {
                        handover.child = child;
                        handover.started = Instant::now();
                        handover.retried = true;
                    }
                    Err(()) => self.fail_handover(),
                }
                false
            }
            HandoverStep::Fail => { self.fail_handover(); false }
        }
    }
    fn fail_handover(&mut self) {
        self.handover = None;
        if let Status::HandingOver(offer) = &self.status {
            self.status = Status::Available(offer.clone());
        }
        self.notice = Some(SETUP_DID_NOT_OPEN.into());
    }

    fn start_check(&mut self) {
        let (Some(home), Some(link)) = (self.home.clone(), self.link.clone()) else { return };
        let Some(admission) = self.admission.clone() else { return };
        if self.jobs.as_ref().is_none_or(|jobs| jobs.try_send(Job::Check { home, link, admission }).is_err()) {
            self.status = Status::Failed("The update worker is unavailable. Reopen the manager.".into());
            return;
        }
        self.status = Status::Checking;
        self.next_check = None;
        self.notice = None;
    }

    fn start_download(&mut self, offer: Offer) {
        let (Some(home), Some(link)) = (self.home.clone(), self.link.clone()) else { return };
        let Some(admission) = self.admission.clone() else { return };
        if self.jobs.as_ref().is_none_or(|jobs| jobs.try_send(Job::Download { home, link, offer: offer.clone(), admission }).is_err()) {
            self.notice = Some("The update worker is unavailable. Reopen the manager.".into());
            return;
        }
        self.status = Status::Downloading { offer: offer.clone(), done: 0, total: offer.package.size };
        self.notice = None;
    }

    pub fn step(&mut self, step: Step, draft: Option<String>) {
        if !self.admit() || self.busy() { return; }
        match step {
            Step::ChangeLink if self.link.is_some() => { self.editing = true; self.link_message = None; }
            Step::KeepLink if self.editing => { self.editing = false; self.draft.clear(); self.link_message = None; }
            Step::SaveLink if self.link.is_none() || self.editing => {
                if let Some(draft) = draft { self.draft = draft; }
                if panel::can_save(&self.draft) { self.save_draft(); }
            }
            Step::Check if self.panel().check.is_some_and(|control| control.enabled) => self.start_check(),
            Step::Download => if let Status::Available(offer) = &self.status { self.start_download(offer.clone()); },
            Step::CancelWait => self.cancel_wait(),
            Step::ContinueAnyway => if matches!(&self.status,
                Status::WaitingForDaw { since, observed: false, .. } if since.elapsed() >= READING_GRACE) { self.continue_anyway(); },
            _ => {}
        }
    }

    pub fn next_poll(&self) -> Option<Duration> {
        if self.initializing || (self.available() && (self.busy() || self.waiting())) { Some(Duration::from_millis(250)) }
        else { self.next_check.map(|due| due.saturating_duration_since(Instant::now()).max(Duration::from_secs(1))) }
    }

    fn save_draft(&mut self) {
        match BetaLink::parse(&self.draft) {
            Err(reason) => self.link_message = Some(reason.into()),
            Ok(link) => match self.home.as_deref().ok_or("Home directory unavailable")
                .map_err(|e| e.to_string())
                .and_then(|home| feed::save_link(home, &link).map_err(|_| "The beta link could not be saved.".to_owned())) {
                Ok(()) => {
                    self.link = Some(link);
                    self.draft.clear();
                    self.editing = false;
                    self.link_message = None;
                    self.status = Status::Idle;
                    self.next_check = Some(Instant::now());
                }
                Err(reason) => self.link_message = Some(reason),
            },
        }
    }

    /// No reading arrived after staging: hand over anyway. Setup's own gate
    /// still refuses while bridged plug-ins are in use.
    fn continue_anyway(&mut self) {
        if let Status::WaitingForDaw { offer, frontend, .. } = &self.status {
            let (offer, frontend) = (offer.clone(), frontend.clone());
            self.start_handover(offer, frontend);
        }
    }

    /// Stop waiting for the DAW. The verified download stays, so choosing
    /// Download and update again does not download it again.
    fn cancel_wait(&mut self) {
        if let Status::WaitingForDaw { offer, .. } = &self.status {
            self.status = Status::Available(offer.clone());
            self.notice = Some("The update is downloaded and kept. Choose Download and update when you're ready.".into());
        }
    }
}

/// The compiled release key and the running version.
fn context() -> Result<(linux_vst_bridge::portable_package::Trust, String), UpdateError> {
    let trust = linux_vst_bridge::portable_package::Trust::compiled()
        .map_err(|_| UpdateError::Local("This build has no release key, so it cannot check for updates.".into()))?;
    // This window's own build; the older layout without a record is "0",
    // which only an update by hand can move on from.
    let current = std::env::current_exe().ok().and_then(|path|
        path.parent().and_then(linux_vst_bridge::package_version::generation_version)).unwrap_or_else(|| "0".into());
    Ok((trust, current))
}

/// Open the staged version's Setup, asking it to continue with Update Bridge
/// unless `continue_update` is false. Setup gets its own user scope, as a
/// launch from the applications menu would, so this window closing (or the
/// unit it runs in stopping) cannot end the update Setup starts. Only when
/// systemd-run is missing is Setup started directly.
fn open_setup(frontend: &Path, continue_update: bool) -> Result<Child, ()> {
    if !frontend.is_file() { return Err(()); }
    let flag: &[&str] = if continue_update { &["--continue-update"] } else { &[] };
    let quiet = |command: &mut Command| { command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()); };
    let mut scoped = Command::new("systemd-run");
    scoped.args(["--user", "--scope", "--collect", "--quiet", "--"]).arg(frontend).args(flag);
    quiet(&mut scoped);
    match scoped.spawn() {
        Ok(child) => Ok(child),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut direct = Command::new(frontend);
            direct.args(flag);
            quiet(&mut direct);
            direct.spawn().map_err(|_| ())
        }
        Err(_) => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linux_vst_bridge::update_feed::FeedPackage;
    fn offer() -> Offer {
        Offer { version: "0.12.0test148".into(), notes: "Notes".into(), system: "cachyos".into(),
            package: FeedPackage { file: "a.install".into(), size: 10, sha256: "a".repeat(64) } }
    }
    fn waiting(since: Instant, observed: bool) -> Updates {
        let mut updates = Updates::disabled();
        updates.status = Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(), since, observed };
        updates
    }
    #[test]
    fn waiting_handover_requires_a_reading_taken_after_staging() {
        let staged = Instant::now();
        let mut updates = waiting(staged, false);
        for reading in [Some((true, Instant::now())), None, Some((false, staged - Duration::from_secs(1)))] {
            assert!(!updates.poll(reading));
            assert!(matches!(updates.status, Status::WaitingForDaw { .. }));
        }
        assert!(!updates.poll(Some((false, Instant::now()))));
        assert_eq!(updates.status, Status::Available(offer()));
        assert_eq!(updates.notice.as_ref().unwrap().text, SETUP_DID_NOT_OPEN);
    }
    #[test]
    fn continue_anyway_never_skips_an_actual_in_use_reading_or_the_grace_period() {
        let old = Instant::now() - READING_GRACE - Duration::from_secs(1);
        for (since, observed) in [(Instant::now(), false), (old, true)] {
            let mut updates = waiting(since, observed);
            updates.step(Step::ContinueAnyway, None);
            assert!(matches!(updates.status, Status::WaitingForDaw { .. }));
        }
        let mut updates = waiting(old, false);
        updates.step(Step::ContinueAnyway, None);
        assert_eq!(updates.status, Status::Available(offer()));
        assert_eq!(updates.notice.as_ref().unwrap().text, SETUP_DID_NOT_OPEN);
        let mut updates = waiting(old, false);
        updates.step(Step::CancelWait, None);
        assert_eq!(updates.status, Status::Available(offer()));
        assert!(updates.notice.unwrap().text.contains("downloaded and kept"));
    }
    #[test]
    fn setup_handover_retains_the_exact_existing_exit_rules() {
        let soon = Duration::from_millis(100);
        assert_eq!(handover_step(None, soon, false), HandoverStep::Wait);
        assert_eq!(handover_step(None, HANDOVER_SETTLE, false), HandoverStep::Close);
        assert_eq!(handover_step(Some(Some(0)), soon, false), HandoverStep::Close);
        assert_eq!(handover_step(Some(Some(64)), soon, false), HandoverStep::RetryPlain);
        assert_eq!(handover_step(Some(Some(64)), soon, true), HandoverStep::Fail);
        for code in [Some(1), Some(101), None] {
            assert_eq!(handover_step(Some(code), soon, false), HandoverStep::Fail);
        }
    }
    #[test]
    fn finished_io_keeps_the_persistent_event_channel_and_schedules_the_next_check() {
        let mut updates = Updates::disabled();
        let (sender, receiver) = mpsc::channel();
        updates.receiver = Some(receiver);
        sender.send(Event::Checked(Ok(Check::Available(offer())))).unwrap();
        assert!(!updates.poll(None));
        assert_eq!(updates.status, Status::Available(offer()));
        assert!(updates.next_check.unwrap() > Instant::now());
        updates.status = Status::Downloading { offer: offer(), done: 5, total: 10 };
        sender.send(Event::Staged(Err(UpdateError::DownloadStopped))).unwrap();
        assert!(!updates.poll(None));
        assert_eq!(updates.status, Status::Available(offer()));
        assert!(updates.notice.unwrap().text.contains("continue where it left off"));
    }
    #[test]
    fn disconnected_io_leaves_a_failure_instead_of_endless_working() {
        let mut updates = Updates::disabled();
        let (sender, receiver) = mpsc::channel();
        updates.receiver = Some(receiver); updates.status = Status::Checking;
        drop(sender);
        assert!(!updates.poll(None));
        assert!(matches!(updates.status, Status::Failed(_)));
        assert!(updates.receiver.is_none() && updates.jobs.is_none());
    }
    #[test]
    fn shared_core_rechecks_selection_before_link_save_check_download_and_handover() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let selected = Arc::new(AtomicBool::new(true));
        let current = selected.clone();
        let admission: Admission = Arc::new(move || Ok(current.load(Ordering::SeqCst)));
        let mut updates = Updates::disabled();
        updates.admission = Some(admission.clone());
        assert!(updates.admit());
        selected.store(false, Ordering::SeqCst);
        updates.step(Step::SaveLink, Some("https://beta.example.ts.net:8443/private-beta/?pw=fixture".into()));
        assert!(updates.link.is_none() && updates.draft.is_empty());
        let (jobs, incoming) = mpsc::sync_channel(1);
        updates.jobs = Some(jobs); updates.home = Some("/must-not-touch".into());
        updates.link = Some(BetaLink::parse("https://beta.example.ts.net:8443/private-beta/?pw=fixture").unwrap());
        updates.status = Status::Available(offer());
        updates.step(Step::Check, None); updates.step(Step::Download, None);
        assert!(incoming.try_recv().is_err());
        assert!(!updates.available());
        assert!(updates.panel().message.unwrap().contains("current manager"));
        let mut waiting = waiting(Instant::now() - READING_GRACE, false);
        waiting.admission = Some(admission);
        assert!(!waiting.poll(Some((false, Instant::now()))));
        assert!(waiting.handover.is_none());
        assert!(matches!(waiting.status, Status::WaitingForDaw { .. }));
        assert!(!waiting.available());
    }
    #[test]
    fn all_sample_panels_match_and_never_expose_a_saved_link() {
        for (name, sample) in lvb_view_model::samples::updates() {
            let updates = Updates::sample(&sample);
            assert_eq!(updates.panel(), sample.panel(), "{name}");
            assert_eq!(updates.headline(), panel::headline(&sample.phase), "{name}");
            let text = format!("{:?}", updates.panel());
            assert!(!text.contains("s3cret") && !text.contains("pw=") && !text.contains("ts.net"));
        }
    }
}
