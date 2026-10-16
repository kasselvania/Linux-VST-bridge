//! Setup's beta updates: paste the beta link once, check the signed update
//! feed, download and stage a newer package, then hand over to that package's
//! own Setup, which runs the existing Update Bridge step with all its checks.
//! The beta key is never shown; only the site name is.
use eframe::egui;
use linux_vst_bridge::update_feed::{self as feed, BetaLink, Check, Offer, UpdateError};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const DAILY: Duration = Duration::from_secs(24 * 60 * 60);
/// How long an unknown DAW state is waited on before Setup's own check decides.
const DAW_UNKNOWN_WAIT: Duration = Duration::from_secs(20);

enum Event {
    Checked(Result<Check, UpdateError>),
    Progress(u64, u64),
    Staged(Result<PathBuf, UpdateError>),
}

#[derive(Clone, Debug, PartialEq)]
enum Status {
    Idle,
    Checking,
    /// The last check failed; the text is one plain line.
    Failed(String),
    UpToDate(String),
    NoPackage(String),
    Manual(String),
    Available(Offer),
    Downloading { offer: Offer, done: u64, total: u64 },
    /// Staged; waiting for the DAW to release bridged plug-ins.
    WaitingForDaw { offer: Offer, frontend: PathBuf, since: Instant },
    /// Setup for the new version was opened.
    HandedOver,
}

pub struct Updates {
    home: Option<PathBuf>,
    link: Option<BetaLink>,
    draft: String,
    editing: bool,
    link_message: Option<String>,
    status: Status,
    receiver: Option<mpsc::Receiver<Event>>,
    next_check: Option<Instant>,
    show_notes: bool,
    /// Why the last download stopped; the offer stays so it can be retried.
    notice: Option<String>,
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
            status: Status::Idle, receiver: None, next_check, show_notes: false, notice: None }
    }
    /// Previews and tests: no link, no checks, no files.
    #[allow(dead_code)]
    pub fn disabled() -> Self {
        Self { home: None, link: None, draft: String::new(), editing: false, link_message: None,
            status: Status::Idle, receiver: None, next_check: None, show_notes: false, notice: None }
    }
    fn busy(&self) -> bool {
        matches!(self.status, Status::Checking | Status::Downloading { .. }
            | Status::WaitingForDaw { .. } | Status::HandedOver)
    }
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
    /// checks, and hands over once the DAW is closed. Returns true when this
    /// window should close because the new version's Setup has opened.
    pub fn poll(&mut self, ctx: &egui::Context, plugins_in_use: Option<bool>) -> bool {
        while let Some(event) = self.receiver.as_ref().and_then(|r| r.try_recv().ok()) {
            match event {
                Event::Checked(result) => {
                    self.receiver = None;
                    self.next_check = Some(Instant::now() + DAILY);
                    self.status = match result {
                        Ok(Check::Available(offer)) => Status::Available(offer),
                        Ok(Check::UpToDate { latest }) => Status::UpToDate(latest),
                        Ok(Check::NoPackage { latest, .. }) => Status::NoPackage(latest),
                        Ok(Check::ManualUpdate { latest }) => Status::Manual(latest),
                        Err(error) => Status::Failed(error.to_string()),
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
                            self.notice = Some(error.to_string());
                            Status::Available(offer.clone())
                        }
                    };
                }
            }
        }
        if !self.busy() && self.link.is_some()
            && self.next_check.is_some_and(|due| Instant::now() >= due) {
            self.start_check(ctx);
        }
        let mut close = false;
        if let Status::WaitingForDaw { frontend, since, .. } = &self.status {
            let ready = match plugins_in_use {
                Some(in_use) => !in_use,
                None => since.elapsed() >= DAW_UNKNOWN_WAIT,
            };
            if ready {
                match hand_over(frontend) {
                    Ok(()) => { self.status = Status::HandedOver; close = true; }
                    Err(error) => self.status = Status::Failed(error),
                }
            }
        }
        if self.busy() && !close { ctx.request_repaint_after(Duration::from_millis(250)); }
        if let Some(due) = self.next_check.filter(|_| !self.busy()) {
            ctx.request_repaint_after(due.saturating_duration_since(Instant::now()).max(Duration::from_secs(1)));
        }
        close
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
                ui.label("Paste the beta link you were sent once. The manager then checks for new test builds by itself.");
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
            } else if let Some(site) = self.link.as_ref().map(|link| link.site().to_owned()) {
                let busy = self.busy();
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("Checking {site} for new builds."));
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
            match &self.status {
                Status::Idle => {}
                Status::Checking => { ui.horizontal(|ui| { ui.spinner(); ui.label("Checking for updates…"); }); }
                Status::Failed(text) => { ui.label(text); }
                Status::UpToDate(latest) => { ui.label(format!("You have the latest version ({}).", version(latest))); }
                Status::NoPackage(latest) => {
                    ui.label(format!("{} is published, but not for this system. Tell the maintainer which Linux you use.", version(latest)));
                }
                Status::Manual(latest) => {
                    ui.label(format!("{} can't be applied from this version inside the app. Download its installer from your beta link and run it once; later updates then work from here.", version(latest)));
                }
                Status::Available(offer) => {
                    ui.label(egui::RichText::new(format!("Update available: {}", version(&offer.version))).strong());
                    if !offer.notes.trim().is_empty() {
                        egui::CollapsingHeader::new("What's new").default_open(self.show_notes)
                            .show(ui, |ui| { ui.label(plain_notes(&offer.notes)); });
                    }
                    if let Some(notice) = &self.notice { ui.colored_label(warning(ui), notice); }
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
                    ui.colored_label(warning(ui), "Your DAW is using bridged plug-ins. Save your project and close your DAW; the update continues by itself when it's closed.");
                }
                Status::HandedOver => { ui.label("Opening Setup to finish the update…"); }
            }
            if !self.busy() && ui.add_enabled(self.link.is_some(),
                egui::Button::new("Check for updates").min_size(egui::vec2(0.0, 44.0))).clicked() {
                self.start_check(&ctx);
            }
            if let Some(offer) = download { self.start_download(&ctx, offer); }
        });
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
        .map_err(|_| UpdateError::Local("The current version could not be read. Use Check again in Setup.".into()))?
        .unwrap_or_else(|| "0".into());
    Ok((trust, current))
}

/// Open the staged version's Setup and ask it to continue with Update Bridge.
/// Setup gets its own user scope, as a launch from Applications would, so
/// this window closing (or the unit it runs in stopping) cannot end the
/// update Setup starts. Without systemd-run it is started directly. A Setup
/// that predates the option refuses it at once; it is then opened plainly.
fn hand_over(frontend: &Path) -> Result<(), String> {
    let unavailable = || "Setup for the new version could not open. Open Linux VST Bridge Setup and Updates from Applications and choose Update Bridge.".to_owned();
    if !frontend.is_file() { return Err(unavailable()); }
    let quiet = |command: &mut Command| { command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()); };
    let scoped = |extra: &[&str]| {
        let mut command = Command::new("systemd-run");
        command.args(["--user", "--scope", "--collect", "--quiet", "--"]).arg(frontend).args(extra);
        quiet(&mut command);
        command.spawn()
    };
    let direct = |extra: &[&str]| {
        let mut command = Command::new(frontend);
        command.args(extra);
        quiet(&mut command);
        command.spawn()
    };
    let mut child = scoped(&["--continue-update"]).or_else(|_| direct(&["--continue-update"]))
        .map_err(|_| unavailable())?;
    std::thread::sleep(Duration::from_millis(800));
    match child.try_wait() {
        Ok(Some(status)) if status.code() == Some(64) => {
            scoped(&[]).or_else(|_| direct(&[])).map_err(|_| unavailable())?;
        }
        // systemd-run could not create the scope.
        Ok(Some(status)) if !status.success() => {
            direct(&["--continue-update"]).map_err(|_| unavailable())?;
        }
        _ => {}
    }
    Ok(())
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
    fn waiting_update_hands_over_only_when_the_daw_is_closed() {
        let ctx = egui::Context::default();
        let mut updates = Updates::disabled();
        // A frontend that does not exist cannot open; the failure is shown plainly.
        updates.status = Status::WaitingForDaw { offer: offer(), frontend: "/nonexistent/setup".into(), since: Instant::now() };
        assert!(!updates.poll(&ctx, Some(true)));
        assert!(matches!(updates.status, Status::WaitingForDaw { .. }));
        assert!(updates.headline().unwrap().starts_with("Close your DAW"));
        assert!(!updates.poll(&ctx, None), "an unknown DAW state waits first");
        assert!(matches!(updates.status, Status::WaitingForDaw { .. }));
        assert!(!updates.poll(&ctx, Some(false)));
        assert!(matches!(&updates.status, Status::Failed(text) if text.contains("Update Bridge")));
    }

    #[test]
    fn results_become_plain_lines_and_schedule_the_daily_check() {
        let ctx = egui::Context::default();
        let mut updates = Updates::disabled();
        for (result, expected) in [
            (Err(UpdateError::Offline), Status::Failed("Couldn't check for updates right now.".into())),
            (Err(UpdateError::LinkRejected), Status::Failed("Your beta link no longer works; paste the new link.".into())),
            (Ok(Check::UpToDate { latest: "0.12.0test146".into() }), Status::UpToDate("0.12.0test146".into())),
            (Ok(Check::Available(offer())), Status::Available(offer())),
        ] {
            let (sender, receiver) = mpsc::channel();
            updates.receiver = Some(receiver);
            updates.status = Status::Checking;
            sender.send(Event::Checked(result)).unwrap();
            assert!(!updates.poll(&ctx, Some(false)));
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
        assert!(!updates.poll(&ctx, Some(false)));
        assert_eq!(updates.status, Status::Available(offer()));
        assert!(updates.notice.as_deref().unwrap().contains("continue where it left off"));
        let labels = rendered(&mut updates_with_link(updates));
        assert!(labels.iter().any(|l| l.contains("continue where it left off")));
        assert!(labels.iter().any(|l| l == "Download and update"));
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
    fn setup_shows_the_site_and_the_offer_but_never_the_key() {
        let mut updates = Updates::disabled();
        let labels = rendered(&mut updates);
        assert!(labels.iter().any(|l| l == "Save beta link"), "{labels:?}");
        updates.link = Some(BetaLink::parse("https://beta.example.ts.net:8443/private-beta/?pw=s3cret").unwrap());
        updates.status = Status::Available(offer());
        let labels = rendered(&mut updates);
        assert!(labels.iter().any(|l| l == "Checking beta.example.ts.net for new builds."), "{labels:?}");
        assert!(labels.iter().any(|l| l == "Update available: Test148"));
        assert!(labels.iter().any(|l| l == "Download and update"));
        assert!(labels.iter().any(|l| l == "What's new"));
        assert!(labels.iter().all(|l| !l.contains("s3cret") && !l.contains("pw=")));
        updates.status = Status::Failed(UpdateError::Offline.to_string());
        let labels = rendered(&mut updates);
        assert!(labels.iter().any(|l| l == "Couldn't check for updates right now."));
        assert!(labels.iter().any(|l| l == "Check for updates"));
        assert_eq!(plain_notes("Setup → Beta updates\r\n"), "Setup -> Beta updates\n");
    }
}
