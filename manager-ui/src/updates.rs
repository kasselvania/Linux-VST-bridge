//! Setup's beta updates: paste the beta link once, check the signed update
//! feed, download and stage a newer package, then hand over to that package's
//! own Setup, which runs the existing Update Bridge step with all its checks.
//! The beta link and its key are never shown.
use eframe::egui;
use linux_vst_bridge::update_feed::{self as feed, BetaLink, Check, Offer, UpdateError};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const DAILY: Duration = Duration::from_secs(24 * 60 * 60);
/// A Setup still running this long after it was opened has taken over.
const HANDOVER_SETTLE: Duration = Duration::from_millis(1500);
const SETUP_DID_NOT_OPEN: &str = "Setup for the new version could not open. Open Linux VST Bridge Setup and Updates from your applications menu and choose Update Bridge.";

enum Event {
    Checked(Result<Check, UpdateError>),
    Progress(u64, u64),
    Staged(Result<PathBuf, UpdateError>),
}

/// A plain line, with the manager's exact refusal kept for technical details.
#[derive(Clone, Debug, PartialEq)]
struct Notice { text: String, technical: Option<String> }
impl From<UpdateError> for Notice {
    fn from(error: UpdateError) -> Self {
        Self { technical: error.technical().map(str::to_owned), text: error.to_string() }
    }
}
impl From<&str> for Notice {
    fn from(text: &str) -> Self { Self { text: text.into(), technical: None } }
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
    /// using bridged plug-ins.
    WaitingForDaw { offer: Offer, frontend: PathBuf, since: Instant },
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
    home: Option<PathBuf>,
    link: Option<BetaLink>,
    draft: String,
    editing: bool,
    link_message: Option<String>,
    status: Status,
    receiver: Option<mpsc::Receiver<Event>>,
    next_check: Option<Instant>,
    /// Why the last download or hand-over stopped; the offer stays.
    notice: Option<Notice>,
    handover: Option<Handover>,
}

fn version(text: &str) -> String { feed::display_version(text) }

impl Updates {
    pub fn new() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let (link, link_message) = match home.as_deref().map(feed::load_link) {
            Some(Ok(link)) => (link, None),
            Some(Err(_)) => (None, Some("Your saved beta link could not be read. Paste it again.".to_owned())),
            None => (None, None),
        };
        let next_check = link.is_some().then(Instant::now);
        Self { home, link, draft: String::new(), editing: false, link_message,
            status: Status::Idle, receiver: None, next_check, notice: None, handover: None }
    }
    /// Previews and tests: no link, no checks, no files.
    #[allow(dead_code)]
    pub fn disabled() -> Self {
        Self { home: None, link: None, draft: String::new(), editing: false, link_message: None,
            status: Status::Idle, receiver: None, next_check: None, notice: None, handover: None }
    }
    /// Work is running that a new check or link must not interrupt.
    fn busy(&self) -> bool {
        matches!(self.status, Status::Checking | Status::Downloading { .. }
            | Status::HandingOver(_) | Status::HandedOver)
    }
    fn waiting(&self) -> bool { matches!(self.status, Status::WaitingForDaw { .. }) }
    /// An update is available or under way; Home shows one line for it.
    pub fn headline(&self) -> Option<String> {
        match &self.status {
            Status::Available(offer) => Some(format!("Update available: {}", version(&offer.version))),
            Status::Downloading { offer, .. } => Some(format!("Downloading the update to {}…", version(&offer.version))),
            Status::WaitingForDaw { offer, .. } => Some(format!("Close your DAW to finish updating to {}.", version(&offer.version))),
            _ => None,
        }
    }

    /// Runs every frame: collects results, starts the start-up and daily
    /// checks, and hands over once a fresh reading shows no DAW is using
    /// bridged plug-ins. `plugins_in_use` is that reading and when it was
    /// taken. Returns true when this window should close because the new
    /// version's Setup has taken over.
    pub fn poll(&mut self, ctx: &egui::Context, plugins_in_use: Option<(bool, Instant)>) -> bool {
        while let Some(event) = self.receiver.as_ref().and_then(|r| r.try_recv().ok()) {
            match event {
                Event::Checked(result) => {
                    self.receiver = None;
                    self.next_check = Some(Instant::now() + DAILY);
                    self.status = match result {
                        Ok(Check::Available(offer)) => Status::Available(offer),
                        Ok(Check::UpToDate { installed }) => Status::UpToDate(installed),
                        Ok(Check::NoPackage { latest, .. }) => Status::NoPackage(latest),
                        Ok(Check::ManualUpdate { latest }) => Status::Manual(latest),
                        Err(error) => Status::Failed(error.into()),
                    };
                }
                Event::Progress(done, total) => {
                    if let Status::Downloading { done: d, total: t, .. } = &mut self.status {
                        *d = done; *t = total;
                    }
                }
                Event::Staged(result) => {
                    self.receiver = None;
                    let Status::Downloading { offer, .. } = &self.status else { continue };
                    self.status = match result {
                        Ok(frontend) => Status::WaitingForDaw { offer: offer.clone(), frontend, since: Instant::now() },
                        Err(error) => {
                            self.notice = Some(error.into());
                            Status::Available(offer.clone())
                        }
                    };
                }
            }
        }
        if !self.busy() && !self.waiting() && self.link.is_some()
            && self.next_check.is_some_and(|due| Instant::now() >= due) {
            self.start_check(ctx);
        }
        if let Status::WaitingForDaw { offer, frontend, since } = &self.status {
            if plugins_in_use.is_some_and(|(in_use, at)| !in_use && at > *since) {
                let (offer, frontend) = (offer.clone(), frontend.clone());
                self.start_handover(offer, frontend);
            }
        }
        let close = self.poll_handover();
        if (self.busy() || self.waiting()) && !close {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        if let Some(due) = self.next_check.filter(|_| !self.busy()) {
            ctx.request_repaint_after(due.saturating_duration_since(Instant::now()).max(Duration::from_secs(1)));
        }
        close
    }

    fn start_handover(&mut self, offer: Offer, frontend: PathBuf) {
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

    fn start_check(&mut self, ctx: &egui::Context) {
        let (Some(home), Some(link)) = (self.home.clone(), self.link.clone()) else { return };
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.status = Status::Checking;
        self.next_check = None;
        self.notice = None;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = context(&home).and_then(|(trust, current)| {
                let systems = feed::system_names(&feed::read_os_release());
                feed::check(&home, &link, &trust, &current, &systems)
            });
            let _ = sender.send(Event::Checked(result));
            ctx.request_repaint();
        });
    }

    fn start_download(&mut self, ctx: &egui::Context, offer: Offer) {
        let (Some(home), Some(link)) = (self.home.clone(), self.link.clone()) else { return };
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.status = Status::Downloading { offer: offer.clone(), done: 0, total: offer.package.size };
        self.notice = None;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut last = None;
            let result = context(&home).and_then(|(trust, current)| {
                feed::download_and_stage(&home, &link, &trust, &current, &offer, &mut |done, total| {
                    // About one progress event per percent.
                    let step = done * 100 / total.max(1);
                    if last != Some(step) {
                        last = Some(step);
                        let _ = sender.send(Event::Progress(done, total));
                        ctx.request_repaint();
                    }
                })
            });
            let _ = sender.send(Event::Staged(result));
            ctx.request_repaint();
        });
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

    /// Setup's "Beta updates" group.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
            ui.label(egui::RichText::new("Beta updates").size(18.0).strong());
            if self.link.is_none() || self.editing {
                ui.label("Paste the beta link from your invitation. You only need to do this once.");
                ui.horizontal_wrapped(|ui| {
                    let field = ui.add(egui::TextEdit::singleline(&mut self.draft)
                        .password(true).hint_text("Paste your beta link").desired_width(360.0));
                    let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.add_enabled(!self.draft.trim().is_empty(),
                        crate::presentation::primary_button("Save beta link", ui)).clicked() || entered {
                        self.save_draft();
                    }
                    if self.link.is_some() && ui.button("Cancel").clicked() {
                        self.editing = false;
                        self.draft.clear();
                        self.link_message = None;
                    }
                });
                ui.small("The link's key is stored privately on this computer and is never shown or sent anywhere else.");
            } else {
                let busy = self.busy();
                ui.horizontal_wrapped(|ui| {
                    ui.label("The manager checks your beta link for new builds.");
                    if !busy && ui.button("Paste a new link").clicked() {
                        self.editing = true;
                        self.link_message = None;
                    }
                });
            }
            if let Some(message) = &self.link_message {
                ui.colored_label(warning(ui), message);
            }
            if self.link.is_none() { return; }
            ui.add_space(4.0);
            let ctx = ui.ctx().clone();
            let mut download = None;
            let mut cancel = false;
            match &self.status {
                Status::Idle => {}
                Status::Checking => { ui.horizontal(|ui| { ui.spinner(); ui.label("Checking for updates…"); }); }
                Status::Failed(notice) => notice_line(ui, notice, false),
                Status::UpToDate(installed) => { ui.label(format!("You have the latest version ({}).", version(installed))); }
                Status::NoPackage(latest) => {
                    ui.label(format!("{} is published, but not for this system. Tell the maintainer which Linux you use.", version(latest)));
                }
                Status::Manual(latest) => {
                    ui.label(match latest {
                        Some(latest) => format!("{} can't be applied from this version inside the app. Download its installer from your beta link and run it once; later updates then work from here.", version(latest)),
                        None => "A newer build is published that this version can't apply inside the app. Download its installer from your beta link and run it once; later updates then work from here.".into(),
                    });
                }
                Status::Available(offer) => {
                    ui.label(egui::RichText::new(format!("Update available: {}", version(&offer.version))).strong());
                    if !offer.notes.trim().is_empty() {
                        egui::CollapsingHeader::new("What's new").default_open(false)
                            .show(ui, |ui| { ui.label(plain_notes(&offer.notes)); });
                    }
                    if let Some(notice) = &self.notice { notice_line(ui, notice, true); }
                    ui.label("Close your DAW before updating. Your plug-ins, projects and settings are kept, and Restore previous setup can bring back this version.");
                    if ui.add(crate::presentation::primary_button("Download and update", ui)).clicked() {
                        download = Some(offer.clone());
                    }
                }
                Status::Downloading { offer, done, total } => {
                    ui.label(format!("Downloading {}…", version(&offer.version)));
                    let fraction = if *total == 0 { 0.0 } else { *done as f32 / *total as f32 };
                    ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).show_percentage()
                        .text(format!("{:.0} of {:.0} MB", *done as f64 / 1e6, *total as f64 / 1e6)));
                    ui.small("The download is checked against the signed update before anything is installed.");
                }
                Status::WaitingForDaw { offer, .. } => {
                    ui.label(egui::RichText::new(format!("{} is downloaded and checked.", version(&offer.version))).strong());
                    ui.colored_label(warning(ui), "The update continues by itself once no DAW is using bridged plug-ins. If your DAW is open, save your project and close it.");
                    if ui.button("Cancel").clicked() { cancel = true; }
                }
                Status::HandingOver(_) => { ui.horizontal(|ui| { ui.spinner(); ui.label("Opening Setup to finish the update…"); }); }
                Status::HandedOver => { ui.label("Setup is finishing the update."); }
            }
            if !self.busy() && ui.add_enabled(self.link.is_some(),
                egui::Button::new("Check for updates").min_size(egui::vec2(0.0, 44.0))).clicked() {
                self.start_check(&ctx);
            }
            if cancel { self.cancel_wait(); }
            if let Some(offer) = download { self.start_download(&ctx, offer); }
        });
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

fn notice_line(ui: &mut egui::Ui, notice: &Notice, warn: bool) {
    if warn { ui.colored_label(warning(ui), &notice.text); } else { ui.label(&notice.text); }
    if let Some(technical) = &notice.technical {
        egui::CollapsingHeader::new("Technical details").show(ui, |ui| { ui.small(technical); });
    }
}

/// The notes as plain text in the manager's font, which has no arrow glyphs.
fn plain_notes(notes: &str) -> String {
    notes.replace('\r', "").replace('→', "->").replace('←', "<-")
}

fn warning(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode { egui::Color32::from_rgb(245, 190, 100) } else { egui::Color32::from_rgb(140, 77, 0) }
}

/// The compiled release key and the running version.
fn context(home: &Path) -> Result<(linux_vst_bridge::portable_package::Trust, String), UpdateError> {
    let trust = linux_vst_bridge::portable_package::Trust::compiled()
        .map_err(|_| UpdateError::Local("This build has no release key, so it cannot check for updates.".into()))?;
    let current = feed::installed_version(home)
        .map_err(|_| UpdateError::Local("The current version could not be read. Choose Check for updates to try again.".into()))?
        .unwrap_or_else(|| "0".into());
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

    #[test]
    fn waiting_update_hands_over_only_on_a_fresh_reading_without_plug_ins() {
        let ctx = egui::Context::default();
        let mut updates = Updates::disabled();
        let staged = Instant::now();
        updates.status = Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(), since: staged };
        assert!(updates.headline().unwrap().starts_with("Close your DAW"));
        // Plug-ins in use, no reading, or a reading from before staging: keep waiting.
        for reading in [Some((true, Instant::now())), None, Some((false, staged - Duration::from_secs(1)))] {
            assert!(!updates.poll(&ctx, reading));
            assert!(matches!(updates.status, Status::WaitingForDaw { .. }), "{reading:?}");
        }
        // A fresh "none in use" reading hands over; a Setup that cannot open
        // leaves this window open with a plain line and the offer kept.
        assert!(!updates.poll(&ctx, Some((false, Instant::now()))));
        assert_eq!(updates.status, Status::Available(offer()));
        assert_eq!(updates.notice.as_ref().unwrap().text, SETUP_DID_NOT_OPEN);
    }

    #[test]
    fn cancel_keeps_the_offer_and_check_stays_reachable_while_waiting() {
        let mut updates = updates_with_link(Updates::disabled());
        updates.status = Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(), since: Instant::now() };
        let labels = rendered(&mut updates);
        for label in ["Cancel", "Check for updates", "Paste a new link"] {
            assert!(labels.iter().any(|l| l == label), "{label}: {labels:?}");
        }
        updates.cancel_wait();
        assert_eq!(updates.status, Status::Available(offer()));
        assert!(updates.notice.as_ref().unwrap().text.contains("downloaded and kept"));
    }

    #[test]
    fn handover_decisions() {
        let soon = Duration::from_millis(100);
        let later = HANDOVER_SETTLE;
        assert_eq!(handover_step(None, soon, false), HandoverStep::Wait);
        assert_eq!(handover_step(None, later, false), HandoverStep::Close);
        assert_eq!(handover_step(Some(Some(0)), soon, false), HandoverStep::Close);
        assert_eq!(handover_step(Some(Some(64)), soon, false), HandoverStep::RetryPlain);
        assert_eq!(handover_step(Some(Some(64)), soon, true), HandoverStep::Fail);
        // systemd-run failing, or Setup failing by itself, is never relaunched.
        for code in [Some(1), Some(101), None] {
            assert_eq!(handover_step(Some(code), soon, false), HandoverStep::Fail);
        }
    }

    #[test]
    fn results_become_plain_lines_and_schedule_the_daily_check() {
        let ctx = egui::Context::default();
        let mut updates = Updates::disabled();
        for (result, expected) in [
            (Err(UpdateError::Offline), Status::Failed(Notice::from("Couldn't check for updates right now."))),
            (Err(UpdateError::LinkRejected), Status::Failed(Notice::from("Your beta link no longer works; paste the new link."))),
            (Ok(Check::UpToDate { installed: "0.12.0test146".into() }), Status::UpToDate("0.12.0test146".into())),
            (Ok(Check::ManualUpdate { latest: None }), Status::Manual(None)),
            (Ok(Check::Available(offer())), Status::Available(offer())),
        ] {
            let (sender, receiver) = mpsc::channel();
            updates.receiver = Some(receiver);
            updates.status = Status::Checking;
            sender.send(Event::Checked(result)).unwrap();
            assert!(!updates.poll(&ctx, Some((false, Instant::now()))));
            assert_eq!(updates.status, expected);
            let due = updates.next_check.unwrap().saturating_duration_since(Instant::now());
            assert!(due > DAILY - Duration::from_secs(5) && due <= DAILY);
        }
        assert_eq!(updates.headline().as_deref(), Some("Update available: Test148"));
        // A stopped download keeps the offer, so Download and update resumes it.
        let (sender, receiver) = mpsc::channel();
        updates.receiver = Some(receiver);
        updates.status = Status::Downloading { offer: offer(), done: 5, total: 10 };
        sender.send(Event::Staged(Err(UpdateError::DownloadStopped))).unwrap();
        assert!(!updates.poll(&ctx, Some((false, Instant::now()))));
        assert_eq!(updates.status, Status::Available(offer()));
        assert!(updates.notice.as_ref().unwrap().text.contains("continue where it left off"));
        let labels = rendered(&mut updates_with_link(updates));
        assert!(labels.iter().any(|l| l.contains("continue where it left off")));
        assert!(labels.iter().any(|l| l == "Download and update"));
        // An install refusal shows plain words; its code only under Technical details.
        let mut updates = updates_with_link(Updates::disabled());
        updates.status = Status::Available(offer());
        updates.notice = Some(UpdateError::Install("user_package_intake_busy".into()).into());
        let labels = rendered(&mut updates);
        assert!(labels.iter().any(|l| l == "Setup is already installing something; try again when it finishes."));
        assert!(labels.iter().any(|l| l == "Technical details"));
        assert!(labels.iter().all(|l| !l.contains("user_package")), "{labels:?}");
    }

    fn updates_with_link(mut updates: Updates) -> Updates {
        updates.link = Some(BetaLink::parse("https://beta.example.ts.net:8443/private-beta/?pw=s3cret").unwrap());
        updates
    }

    fn rendered(updates: &mut Updates) -> Vec<String> {
        fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(items) => for item in items { texts(item, out); },
                _ => {}
            }
        }
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| updates.show(ui));
        let mut labels = vec![];
        for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
        output.textures_delta.clear();
        labels
    }

    #[test]
    fn setup_shows_the_offer_but_never_the_link_or_its_key() {
        let mut updates = Updates::disabled();
        let labels = rendered(&mut updates);
        assert!(labels.iter().any(|l| l == "Save beta link"), "{labels:?}");
        assert!(labels.iter().any(|l| l == "Paste the beta link from your invitation. You only need to do this once."));
        let mut updates = updates_with_link(updates);
        updates.status = Status::Available(offer());
        let labels = rendered(&mut updates);
        assert!(labels.iter().any(|l| l == "The manager checks your beta link for new builds."), "{labels:?}");
        assert!(labels.iter().any(|l| l == "Update available: Test148"));
        assert!(labels.iter().any(|l| l == "Download and update"));
        assert!(labels.iter().any(|l| l == "What's new"));
        assert!(labels.iter().all(|l| !l.contains("s3cret") && !l.contains("pw=") && !l.contains("ts.net")));
        updates.status = Status::Failed(UpdateError::Offline.into());
        let labels = rendered(&mut updates);
        assert!(labels.iter().any(|l| l == "Couldn't check for updates right now."));
        assert!(labels.iter().any(|l| l == "Check for updates"));
        updates.status = Status::UpToDate("0.12.0test146".into());
        assert!(rendered(&mut updates).iter().any(|l| l == "You have the latest version (Test146)."));
        assert_eq!(plain_notes("Setup → Beta updates\r\n"), "Setup -> Beta updates\n");
    }
}
