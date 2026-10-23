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
/// Without any reading after staging for this long, "Continue anyway" is
/// offered; Setup's own gate still refuses while plug-ins are in use.
const READING_GRACE: Duration = Duration::from_secs(30);
const SETUP_DID_NOT_OPEN: &str = "Setup for the new version could not open. Open Linux VST Bridge Setup and Updates from your applications menu and choose Update Bridge.";

enum Event {
    Checked(Result<Check, UpdateError>),
    Progress(u64, u64),
    Staged(Result<PathBuf, UpdateError>),
}

use crate::screen_model::updates::{self as panel, Block, Link, Panel, Phase, Step, UpdateNotice as Notice};
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
    /// Every state Setup's update panel draws, for render checks. Sample
    /// values only: no saved link, no check, no download, no files.
    #[cfg(test)]
    pub fn samples() -> Vec<(&'static str, Self)> {
        let link = || BetaLink::parse("https://beta.example.ts.net:8443/private-beta/?pw=s3cret").ok();
        let offer = || Offer { version: "0.12.0test148".into(), notes: "Setup → Beta updates\r\nFaster plug-in list".into(),
            system: "cachyos".into(), package: feed::FeedPackage { file: "a.install".into(), size: 10_000_000,
                sha256: "a".repeat(64) } };
        let with = |status: Status| Self { link: link(), status, ..Self::disabled() };
        let soon = Instant::now() + Duration::from_secs(3600);
        let long_ago = Instant::now().checked_sub(READING_GRACE + Duration::from_secs(1)).unwrap_or_else(Instant::now);
        vec![
            ("no_link", Self::disabled()),
            ("unreadable_link", Self { link_message: Some("Your saved beta link could not be read. Paste it again.".into()),
                ..Self::disabled() }),
            ("idle", with(Status::Idle)),
            ("editing", Self { editing: true, draft: "https://example".into(),
                link_message: Some("This is not a beta link.".into()), ..with(Status::Idle) }),
            ("checking", with(Status::Checking)),
            ("failed", with(Status::Failed(error_notice(UpdateError::Install("user_package_intake_busy".into()))))),
            ("up_to_date", with(Status::UpToDate("0.12.0test146".into()))),
            ("no_package", with(Status::NoPackage("0.12.0test148".into()))),
            ("manual", with(Status::Manual(Some("0.12.0test148".into())))),
            ("manual_unknown", with(Status::Manual(None))),
            ("available", with(Status::Available(offer()))),
            ("available_notice", Self { notice: Some(error_notice(UpdateError::DownloadStopped)),
                ..with(Status::Available(offer())) }),
            ("downloading", with(Status::Downloading { offer: offer(), done: 4_000_000, total: 10_000_000 })),
            ("waiting", with(Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(),
                since: soon, observed: false })),
            ("waiting_long", with(Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(),
                since: long_ago, observed: false })),
            ("handing_over", with(Status::HandingOver(offer()))),
            ("handed_over", with(Status::HandedOver)),
        ]
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
    fn panel(&self) -> Panel {
        Panel::build(self.link.is_some(), self.editing, self.busy(), self.link_message.as_deref(),
            &self.phase(), self.notice.as_ref())
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
                        Err(error) => Status::Failed(error_notice(error)),
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
        if !self.busy() && !self.waiting() && self.link.is_some()
            && self.next_check.is_some_and(|due| Instant::now() >= due) {
            self.start_check(ctx);
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
            let result = context().and_then(|(trust, current)| {
                let systems = feed::system_names(&linux_vst_bridge::read_os_release());
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
            let result = context().and_then(|(trust, current)| {
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

    /// Setup's "Beta updates" group, drawn from its screen model. A link
    /// change applies at once; the update's own controls apply at the end.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
            let mut view = self.panel();
            ui.label(egui::RichText::new(view.title).size(18.0).strong());
            let mut link_step = None;
            match &view.link {
                Link::Entry { prompt, hint, save, keep, privacy } => {
                    ui.label(*prompt);
                    ui.horizontal_wrapped(|ui| {
                        let field = ui.add(egui::TextEdit::singleline(&mut self.draft)
                            .password(true).hint_text(*hint).desired_width(360.0));
                        let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui.add_enabled(panel::can_save(&self.draft),
                            crate::presentation::primary_button(*save, ui)).clicked() || entered {
                            self.save_draft();
                            link_step = Some(Step::SaveLink);
                        }
                        if let Some(keep) = keep {
                            if ui.button(keep.label).clicked() {
                                self.editing = false;
                                self.draft.clear();
                                self.link_message = None;
                                link_step = Some(keep.does);
                            }
                        }
                    });
                    ui.small(*privacy);
                }
                Link::Saved { line, change } => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(*line);
                        if let Some(change) = change {
                            if ui.button(change.label).clicked() {
                                self.editing = true;
                                self.link_message = None;
                                link_step = Some(change.does);
                            }
                        }
                    });
                }
            }
            if link_step.is_some() { view = self.panel(); }
            if let Some(message) = &view.message {
                ui.colored_label(warning(ui), message);
            }
            let Some(status) = &view.status else { return };
            ui.add_space(4.0);
            let ctx = ui.ctx().clone();
            let mut step = None;
            match status {
                Block::Idle => {}
                Block::Working(words) => { ui.horizontal(|ui| { ui.spinner(); ui.label(*words); }); }
                Block::Line(line) => { ui.label(line); }
                Block::Notice { notice, warn } => notice_line(ui, notice, *warn),
                Block::Available { headline, notes, notice, advice, download } => {
                    ui.label(egui::RichText::new(headline).strong());
                    if let Some((title, notes)) = notes {
                        egui::CollapsingHeader::new(*title).default_open(false)
                            .show(ui, |ui| { ui.label(plain_notes(notes)); });
                    }
                    if let Some(notice) = notice { notice_line(ui, notice, true); }
                    ui.label(*advice);
                    if ui.add(crate::presentation::primary_button(download.label, ui)).clicked() {
                        step = Some(download.does);
                    }
                }
                Block::Downloading { line, fraction, progress, note } => {
                    ui.label(line);
                    ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).show_percentage().text(progress));
                    ui.small(*note);
                }
                Block::Waiting { headline, warning: caution, continue_anyway, cancel, recheck } => {
                    ui.label(egui::RichText::new(headline).strong());
                    ui.colored_label(warning(ui), *caution);
                    ui.horizontal_wrapped(|ui| {
                        if let Some(proceed) = continue_anyway {
                            if ui.button(proceed.label).clicked() { step = Some(proceed.does); }
                        }
                        if ui.button(cancel.label).clicked() { step = Some(cancel.does); }
                    });
                    if let Some(recheck) = recheck { ui.small(*recheck); }
                }
            }
            if let Some(check) = &view.check {
                if ui.add_enabled(check.enabled, egui::Button::new(check.label)
                    .min_size(egui::vec2(0.0, 44.0))).clicked() {
                    self.start_check(&ctx);
                }
            }
            match step {
                Some(Step::CancelWait) => self.cancel_wait(),
                Some(Step::ContinueAnyway) => self.continue_anyway(),
                Some(Step::Download) => if let Status::Available(offer) = &self.status {
                    let offer = offer.clone();
                    self.start_download(&ctx, offer);
                },
                _ => {}
            }
        });
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

/// The notes as plain text in the window's font, which has no arrow glyphs.
fn plain_notes(notes: &str) -> String {
    notes.replace('\r', "").replace('→', "->").replace('←', "<-")
}

fn notice_line(ui: &mut egui::Ui, notice: &Notice, warn: bool) {
    if warn { ui.colored_label(warning(ui), &notice.text); } else { ui.label(&notice.text); }
    if let Some(technical) = &notice.technical {
        egui::CollapsingHeader::new(panel::TECHNICAL).show(ui, |ui| { ui.small(technical); });
    }
}

fn warning(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode { egui::Color32::from_rgb(245, 190, 100) } else { egui::Color32::from_rgb(140, 77, 0) }
}

/// The compiled release key and the running version.
fn context() -> Result<(linux_vst_bridge::portable_package::Trust, String), UpdateError> {
    let trust = linux_vst_bridge::portable_package::Trust::compiled()
        .map_err(|_| UpdateError::Local("This build has no release key, so it cannot check for updates.".into()))?;
    // This window's own build; the older layout without a record is "0",
    // which only an update by hand can move on from.
    let current = crate::presentation::own_version().unwrap_or("0").to_owned();
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
        updates.status = Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(),
            since: staged, observed: false };
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
        updates.status = Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(),
            since: Instant::now(), observed: false };
        let labels = rendered(&mut updates);
        for label in ["Cancel", "Check for updates", "Paste a new link"] {
            assert!(labels.iter().any(|l| l == label), "{label}: {labels:?}");
        }
        assert!(!labels.iter().any(|l| l == "Continue anyway"), "not before the grace period");
        updates.cancel_wait();
        assert_eq!(updates.status, Status::Available(offer()));
        assert!(updates.notice.as_ref().unwrap().text.contains("downloaded and kept"));
    }

    #[test]
    fn continue_anyway_appears_only_when_no_reading_arrives() {
        let ctx = egui::Context::default();
        let long_ago = Instant::now() - READING_GRACE - Duration::from_secs(1);
        let mut updates = updates_with_link(Updates::disabled());
        updates.status = Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(),
            since: long_ago, observed: false };
        let labels = rendered(&mut updates);
        assert!(labels.iter().any(|l| l == "Continue anyway"), "{labels:?}");
        assert!(labels.iter().any(|l| l == "Setup will check again that no plug-ins are in use."));
        // A fresh reading that plug-ins are in use: the DAW is open, so no
        // way around it is offered.
        assert!(!updates.poll(&ctx, Some((true, Instant::now()))));
        assert!(matches!(updates.status, Status::WaitingForDaw { observed: true, .. }));
        assert!(!rendered(&mut updates).iter().any(|l| l == "Continue anyway"));
        // Continue anyway opens Setup; a Setup that cannot open says so.
        updates.status = Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(),
            since: long_ago, observed: false };
        updates.continue_anyway();
        assert_eq!(updates.status, Status::Available(offer()));
        assert_eq!(updates.notice.as_ref().unwrap().text, SETUP_DID_NOT_OPEN);
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
        updates.notice = Some(error_notice(UpdateError::Install("user_package_intake_busy".into())));
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
        updates.status = Status::Failed(error_notice(UpdateError::Offline));
        let labels = rendered(&mut updates);
        assert!(labels.iter().any(|l| l == "Couldn't check for updates right now."));
        assert!(labels.iter().any(|l| l == "Check for updates"));
        updates.status = Status::UpToDate("0.12.0test146".into());
        assert!(rendered(&mut updates).iter().any(|l| l == "You have the latest version (Test146)."));
        assert_eq!(plain_notes("Setup → Beta updates\r\n"), "Setup -> Beta updates\n");
    }
}
