//! Setup's beta updates: saving the beta link, checking the signed feed,
//! downloading, waiting for the DAW to close and handing over to the new
//! version's Setup. The window's updater owns that work; this says where it
//! is and what can be pressed. The link and its key are never shown.
//! Versions are shown as the update feed names them (`display_version`).
use linux_vst_bridge::update_feed::display_version;

/// A plain line, with the manager's exact refusal kept for technical details.
#[derive(Clone, Debug, PartialEq)]
pub struct UpdateNotice {
    pub text: String,
    pub technical: Option<String>,
}

/// The heading over a notice's exact refusal.
pub const TECHNICAL: &str = "Technical details";

/// Where the updater is, as it records it.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Idle,
    Checking,
    Failed(UpdateNotice),
    UpToDate(String),
    NoPackage(String),
    /// None: a feed only a newer manager can read.
    Manual(Option<String>),
    Available { version: String, notes: String },
    Downloading { version: String, done: u64, total: u64 },
    /// `observed`: a reading after staging arrived; `long`: none arrived for
    /// the grace period.
    Waiting { version: String, observed: bool, long: bool },
    HandingOver,
    HandedOver,
}

/// What a control of the panel does; the updater carries it out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    SaveLink,
    KeepLink,
    ChangeLink,
    Check,
    Download,
    ContinueAnyway,
    CancelWait,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateControl {
    pub label: &'static str,
    pub enabled: bool,
    pub does: Step,
}

fn control(label: &'static str, does: Step) -> UpdateControl {
    UpdateControl { label, enabled: true, does }
}

/// The beta link: asked for once, then only offered to change.
#[derive(Clone, Debug, PartialEq)]
pub enum Link {
    /// The link can be saved once `can_save` accepts what is typed.
    Entry { prompt: &'static str, hint: &'static str, save: &'static str, keep: Option<UpdateControl>,
        privacy: &'static str },
    Saved { line: &'static str, change: Option<UpdateControl> },
}

/// The update's state in words, with its controls.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Idle,
    /// Work under way, in its own words.
    Working(&'static str),
    Line(String),
    Notice { notice: UpdateNotice, warn: bool },
    /// `notes`: the release notes as published, under their heading.
    Available { headline: String, notes: Option<(&'static str, String)>, notice: Option<UpdateNotice>,
        advice: &'static str, download: UpdateControl },
    Downloading { line: String, fraction: f32, progress: String, note: &'static str },
    Waiting { headline: String, warning: &'static str, continue_anyway: Option<UpdateControl>,
        cancel: UpdateControl, recheck: Option<&'static str> },
}

/// Setup's "Beta updates" group.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    pub title: &'static str,
    pub link: Link,
    /// Why the link was not saved or could not be read.
    pub message: Option<String>,
    /// None until a link is saved.
    pub status: Option<Block>,
    pub check: Option<UpdateControl>,
}

/// Something is typed to save as the beta link.
pub fn can_save(draft: &str) -> bool {
    !draft.trim().is_empty()
}

/// An update available or under way, as one line for the other pages.
pub fn headline(phase: &Phase) -> Option<String> {
    match phase {
        Phase::Available { version, .. } => Some(format!("Update available: {}", display_version(version))),
        Phase::Downloading { version, .. } => Some(format!("Downloading the update to {}…", display_version(version))),
        Phase::Waiting { version, .. } => Some(format!("Close your DAW to finish updating to {}.", display_version(version))),
        _ => None,
    }
}

impl Panel {
    /// `linked`: a beta link is saved; `editing`: the musician is changing it;
    /// `busy`: a check, download or hand-over runs; `notice`: why the last
    /// download or hand-over stopped.
    pub fn build(linked: bool, editing: bool, busy: bool, link_message: Option<&str>,
        phase: &Phase, notice: Option<&UpdateNotice>) -> Self {
        let version = display_version;
        Self {
            title: "Beta updates",
            link: if !linked || editing {
                Link::Entry {
                    prompt: "Paste the beta link from your invitation. You only need to do this once.",
                    hint: "Paste your beta link",
                    save: "Save beta link",
                    keep: linked.then(|| control("Cancel", Step::KeepLink)),
                    privacy: "The link's key is stored privately on this computer and is never shown or sent anywhere else.",
                }
            } else {
                Link::Saved { line: "The manager checks your beta link for new builds.",
                    change: (!busy).then(|| control("Paste a new link", Step::ChangeLink)) }
            },
            message: link_message.map(str::to_owned),
            status: linked.then(|| match phase {
                Phase::Idle => Block::Idle,
                Phase::Checking => Block::Working("Checking for updates…"),
                Phase::Failed(notice) => Block::Notice { notice: notice.clone(), warn: false },
                Phase::UpToDate(installed) => Block::Line(format!("You have the latest version ({}).", version(installed))),
                Phase::NoPackage(latest) => Block::Line(format!(
                    "{} is published, but not for this system. Tell the maintainer which Linux you use.", version(latest))),
                Phase::Manual(latest) => Block::Line(match latest {
                    Some(latest) => format!("{} can't be applied from this version inside the app. Download its installer from your beta link and run it once; later updates then work from here.", version(latest)),
                    None => "A newer build is published that this version can't apply inside the app. Download its installer from your beta link and run it once; later updates then work from here.".into(),
                }),
                Phase::Available { version: offered, notes } => Block::Available {
                    headline: format!("Update available: {}", version(offered)),
                    notes: (!notes.trim().is_empty()).then(|| ("What's new", notes.clone())),
                    notice: notice.cloned(),
                    advice: "Close your DAW before updating. Your plug-ins, projects and settings are kept, and Restore previous setup can bring back this version.",
                    download: control("Download and update", Step::Download),
                },
                Phase::Downloading { version: offered, done, total } => Block::Downloading {
                    line: format!("Downloading {}…", version(offered)),
                    fraction: if *total == 0 { 0.0 } else { *done as f32 / *total as f32 },
                    progress: format!("{:.0} of {:.0} MB", *done as f64 / 1e6, *total as f64 / 1e6),
                    note: "The download is checked against the signed update before anything is installed.",
                },
                Phase::Waiting { version: offered, observed, long } => Block::Waiting {
                    headline: format!("{} is downloaded and checked.", version(offered)),
                    warning: "The update continues by itself once no DAW is using bridged plug-ins. If your DAW is open, save your project and close it.",
                    continue_anyway: (!observed && *long).then(|| control("Continue anyway", Step::ContinueAnyway)),
                    cancel: control("Cancel", Step::CancelWait),
                    recheck: (!observed && *long).then_some("Setup will check again that no plug-ins are in use."),
                },
                Phase::HandingOver => Block::Working("Opening Setup to finish the update…"),
                Phase::HandedOver => Block::Line("Setup is finishing the update.".into()),
            }),
            check: (linked && !busy).then(|| control("Check for updates", Step::Check)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_link_is_asked_for_once_and_its_key_never_shown() {
        let unlinked = Panel::build(false, false, false, None, &Phase::Idle, None);
        assert!(matches!(unlinked.link, Link::Entry { keep: None, .. }));
        assert!(!can_save("  ") && can_save("https://beta"));
        assert!(unlinked.status.is_none() && unlinked.check.is_none());
        let linked = Panel::build(true, false, false, None, &Phase::Idle, None);
        assert!(matches!(linked.link, Link::Saved { change: Some(_), .. }));
        assert_eq!(linked.status, Some(Block::Idle));
        assert_eq!(linked.check.unwrap().does, Step::Check);
        let checking = Panel::build(true, false, true, None, &Phase::Checking, None);
        assert!(matches!(checking.link, Link::Saved { change: None, .. }) && checking.check.is_none());
    }

    #[test]
    fn waiting_offers_continue_anyway_only_without_any_reading() {
        let waiting = |observed, long| match Panel::build(true, false, false, None,
            &Phase::Waiting { version: "0.12.0test148".into(), observed, long }, None).status {
            Some(Block::Waiting { continue_anyway, .. }) => continue_anyway.is_some(),
            other => panic!("{other:?}"),
        };
        assert!(!waiting(false, false));
        assert!(waiting(false, true));
        assert!(!waiting(true, true), "a reading says the DAW is open: no way around it");
        assert_eq!(headline(&Phase::Waiting { version: "0.12.0test148".into(), observed: false, long: false }).unwrap(),
            "Close your DAW to finish updating to Test148.");
    }
}
