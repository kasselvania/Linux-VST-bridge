//! Offered actions: their words, consequences and whether they can be pressed now.
//! Labels are presentation only; the offered action and refusal stay the manager's.
use super::{Detail, Disclosure, Text};
use crate::model::{AccessibilityChoice, Action, AvailableAction, Product};

/// Said while an earlier request or the manager's readback is still pending.
pub const WAITING: &str = "Waiting for the current request or manager readback";
/// Said under a Home or Setup card's main step while a request is pending.
pub const WAITING_TO_FINISH: &str = "Waiting for the current request to finish.";
/// Why a held offer cannot be pressed. The screen that holds it says so once.
pub const HELD: &str = "Actions wait for current status";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Importance {
    Primary,
    Secondary,
    Quiet,
}

/// Whether an offer can be pressed now, and the visible reason when not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Availability {
    Enabled,
    /// Another request or readback is in flight; the offer itself stands.
    Waiting,
    /// The screen's status is not current, so its offers are held until it is.
    Held,
    /// The manager, or the shared-resource rule, refuses it for this reason.
    Refused(String),
}

impl Availability {
    /// Held while status is not current. A refusal or pending request still
    /// says its own reason.
    pub fn held(self, held: bool) -> Self {
        if held && self == Self::Enabled { Self::Held } else { self }
    }

    pub fn enabled(&self) -> bool {
        *self == Self::Enabled
    }

    /// Why the offer cannot be pressed now, in the musician's words.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Enabled => None,
            Self::Waiting => Some(WAITING),
            Self::Held => Some(HELD),
            Self::Refused(reason) => Some(reason),
        }
    }
}

/// One manager-offered action as a frontend presents it.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub label: String,
    pub importance: Importance,
    pub availability: Availability,
    pub consequence: Option<&'static str>,
    /// The manager's exact closed action. Pressing the offer emits this
    /// intent unchanged; the window sends it with the current state token.
    pub intent: Action,
}

impl Offer {
    /// This offer, held while its screen's status is not current.
    pub fn held(mut self, held: bool) -> Self {
        self.availability = self.availability.held(held);
        self
    }
}

/// A row of offers, with every refusal and consequence printed once below it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OfferRow {
    /// Said before the row, such as which plug-ins become unavailable.
    pub preface: Option<String>,
    pub offers: Vec<Offer>,
    /// Refusals stay visible for touch users, not only on hover.
    pub notes: Vec<String>,
    pub consequences: Vec<&'static str>,
}

impl OfferRow {
    /// Every offer in the row, held while its screen's status is not current.
    pub fn held(mut self, held: bool) -> Self {
        self.offers = self.offers.into_iter().map(|offer| offer.held(held)).collect();
        self
    }
}

/// One setup a plug-in can switch to or restore, with its exact identity.
#[derive(Clone, Debug, PartialEq)]
pub struct SetupChoice {
    pub offer: Offer,
    pub notes: Vec<String>,
    /// The retained setup's own description, when it has one.
    pub description: Option<String>,
    /// The serialized exact action; it also identifies the choice.
    pub exact_action: String,
    pub identity: Disclosure,
}

/// The manager's own refusal, or the shared-resource refusal for actions that
/// need every bridged instance closed.
///
/// The shared-resource branch duplicates the manager's authority: the window
/// applies `requires_global_inactive` to the service-wide status itself. It
/// moved here unchanged and is retired when the service supplies that refusal
/// on each offer (#489). The same rule filters routine steps in `plugins.rs`.
pub fn refusal<'a>(offer: &'a AvailableAction, busy: Option<&'a str>) -> Option<&'a str> {
    // Window policy, overriding the manager's refusal: opening the result form
    // must remain possible to report a problem while the target is active.
    // Submitting "Worked" still applies the exact offered refusal
    // (`forms::compatibility_result_refusal`). Retired when the service
    // offers separate refusals for opening the form and submitting a result.
    if matches!(offer.action, Action::CompatibilityResult { .. }) { return None; }
    offer.disabled_reason.as_deref().or_else(|| {
        offer.action.requires_global_inactive().then_some(busy).flatten()
    })
}

pub fn availability(offer: &AvailableAction, busy: Option<&str>, pending: bool) -> Availability {
    match refusal(offer, busy) {
        Some(reason) => Availability::Refused(reason.to_owned()),
        None if pending => Availability::Waiting,
        None => Availability::Enabled,
    }
}

pub fn offer(available: &AvailableAction, importance: Importance, busy: Option<&str>, pending: bool) -> Offer {
    Offer {
        label: action_label(available),
        importance,
        availability: availability(available, busy, pending),
        consequence: action_consequence(&available.action),
        intent: available.action.clone(),
    }
}

/// Offers shown together. Visibility supplies no new authority.
pub fn row(actions: &[AvailableAction], importance: Importance, busy: Option<&str>,
    pending: bool) -> OfferRow {
    let mut consequences = Vec::new();
    for offer in actions {
        if let Some(text) = action_consequence(&offer.action) {
            if !consequences.contains(&text) { consequences.push(text); }
        }
    }
    OfferRow {
        preface: None,
        offers: actions.iter().map(|available| offer(available, importance, busy, pending)).collect(),
        notes: visible_refusals(actions, busy, pending),
        consequences,
    }
}

/// Every reason an offer is unavailable, once each: pending work, the shared
/// resource and the manager's own refusal.
pub fn visible_refusals(actions: &[AvailableAction], busy: Option<&str>, pending: bool) -> Vec<String> {
    let mut reasons = Vec::new();
    let mut add = |reason: &str| {
        if !reasons.iter().any(|existing| existing == reason) {
            reasons.push(reason.to_owned());
        }
    };
    for offer in actions {
        if pending {
            add(WAITING);
        }
        if offer.action.requires_global_inactive() {
            if let Some(reason) = busy {
                add(reason);
            }
        }
        if let Some(reason) = &offer.disabled_reason {
            add(reason);
        }
    }
    reasons
}

/// A prepared or retained setup of this plug-in, under its distinct display name.
pub fn setup_choice(product: &Product, available: &AvailableAction, busy: Option<&str>,
    pending: bool) -> SetupChoice {
    let exact_action = serde_json::to_string(&available.action).unwrap_or_default();
    let description = match &available.action {
        Action::OrdinaryRollback { publication, .. } => product.history.iter()
            .find(|record| record.publication == *publication)
            .filter(|record| !record.description.is_empty())
            .map(|record| record.description.clone()),
        _ => None,
    };
    SetupChoice {
        // Context-specific labels already describe the typed action; they are not mapped twice.
        offer: Offer {
            label: setup_choice_label(product, available),
            importance: Importance::Quiet,
            availability: availability(available, busy, pending),
            consequence: action_consequence(&available.action),
            intent: available.action.clone(),
        },
        notes: visible_refusals(std::slice::from_ref(available), busy, pending),
        description,
        identity: Disclosure::new("Exact setup identity", vec![
            Detail::Text(Text::detail(format!("Manager action: {}", available.label))),
            Detail::Text(Text::detail(exact_action.clone())),
        ]),
        exact_action,
    }
}

/// Presentation only: the original offered action and refusal remain authoritative.
pub fn action_label(offer: &AvailableAction) -> String {
    match &offer.action {
        Action::SoftwareInstall { .. } | Action::SoftwareInstallResume { .. }
            if offer.label == "Retry" => "Retry".into(),
        Action::SoftwareInstall { .. } => "Install and connect to your DAW".into(),
        Action::SoftwareInstallResume { .. } => "Continue installation and DAW connection".into(),
        Action::PluginFinishSetup {..} => "Open this plug-in to finish setup".into(),
        Action::PluginSetupFocus {..} => "Show the plug-in window".into(),
        Action::PluginSetupStop {..} => "Stop finishing setup".into(),
        Action::RuntimeInstall {} => offer.label.strip_prefix("Install compatibility runtime")
            .map(|size| format!("Download Windows plug-in support{size}"))
            .unwrap_or_else(|| "Download Windows plug-in support".into()),
        Action::CompatibilityCheck { .. } => "Prepare for your DAW".into(),
        Action::CompatibilityResumeCheck { .. } => "Continue preparing for your DAW".into(),
        Action::PluginInspect { .. } => "Check this plug-in".into(),
        Action::PluginReinspect { .. } => "Check this plug-in again".into(),
        Action::PluginPrepare { .. } => "Prepare this setup".into(),
        Action::CompatibilityPublishTest { expected_current: None, .. }
            | Action::ExperimentalEnable { .. } => "Make available in your DAW".into(),
        Action::CompatibilityPublishTest { expected_current: Some(_), .. }
            | Action::ExperimentalReplace { .. } => "Use this prepared setup".into(),
        Action::ExperimentalDisable { .. } => "Stop using this test setup".into(),
        Action::CandidateWithdraw { .. } => "Remove from your DAW".into(),
        Action::CompatibilityResult { .. } => "Record a test result or problem".into(),
        Action::CompatibilityFinishResult { .. } => offer.label.clone(),
        Action::CandidateGraphicsAssess { .. } => "Check the editor with these settings".into(),
        Action::CandidateSettingsPrepare { settings, .. } => {
            let graphics = if settings.graphics.is_some() { "Wine D3D11" } else { "runtime defaults" };
            let accessibility = match settings.accessibility {
                AccessibilityChoice::ProfileDefault => "default accessibility",
                AccessibilityChoice::WindowsDefault => "Windows accessibility defaults",
                AccessibilityChoice::DisabledForHost => "Windows accessibility off",
            };
            format!("Prepare {graphics} · {accessibility}")
        }
        Action::CandidateGraphicsPrepare { backend, .. } => if backend.is_some() {
            "Prepare Wine D3D11 graphics".into()
        } else { "Prepare runtime graphics defaults".into() },
        _ => offer.label.clone(),
    }
}

pub fn action_consequence(action: &Action) -> Option<&'static str> {
    match action {
        Action::CompatibilityCheck { .. } => Some("Checks the installed plug-in and prepares a native bridge. Making it available is a separate step."),
        Action::PluginInspect { .. } | Action::PluginReinspect { .. } => Some("Checks the installed plug-in. Does not prepare or change the setup selected for your DAW."),
        Action::CandidateSettingsPrepare { settings, .. }
            if settings.accessibility == AccessibilityChoice::DisabledForHost =>
            Some("Prepares next-launch settings with Windows accessibility off for this plug-in's Windows host. Linux/Deck input stays unchanged. Your current DAW setup stays selected until you choose the prepared one."),
        Action::PluginPrepare { .. } | Action::CandidateSettingsPrepare { .. }
            | Action::CandidateGraphicsPrepare { .. } => Some("Prepares another setup. Your current DAW setup stays selected until you choose to use the prepared one."),
        Action::CompatibilityPublishTest { expected_current: None, .. }
            | Action::ExperimentalEnable { .. } => Some("Makes the native VST3 bridge available. Rescan VST3 plug-ins in your DAW if needed, then add it to a track."),
        Action::CompatibilityPublishTest { expected_current: Some(_), .. }
            | Action::ExperimentalReplace { .. } => Some("Changes the bridge setup selected for this plug-in. Previous setups are retained; vendor installation or authorization changes are not undone."),
        Action::ExperimentalDisable { .. } => Some("Restores the retained recovery setup, or removes this plug-in's bridge if no recovery setup exists. Stops if that recovery setup cannot be verified. Vendor installation and authorization are unchanged."),
        Action::CandidateWithdraw { .. } => Some("Removes this plug-in's native bridge from your DAW. Keeps its records and vendor installation; does not undo authorization or vendor changes."),
        Action::OrdinaryRollback { .. } | Action::OrdinaryRestoreRecommended { .. } => Some("Restores the retained bridge setup only. Vendor installation and authorization changes are not undone; use only a setup that still matches the installed plug-in."),
        Action::PluginRemoveFromDaw { .. } => Some("Projects that use it open with it missing until you add it back. Its Windows installation stays in its setup."),
        Action::PluginAddBackToDaw { .. } => Some("Puts it back in your DAW with the setup it had before; if the bridge was updated since, it is checked again first. Rescan VST3 plug-ins in your DAW if it doesn't appear."),
        Action::SetupRemoveFromDaw { .. } => Some("Projects that use them open with them missing until you add them back. If you then delete this setup, they can't be added back."),
        Action::SetupDelete { .. } => Some("Deletes the Windows setup: everything installed in it, files saved inside it such as presets, and any licences or sign-ins stored there. Projects that use its plug-ins open with them missing until you set it up again. Your project files are not changed."),
        Action::ProductUninstall { .. } => Some("Takes its plug-ins out of your DAW, then runs the vendor's own uninstaller in this setup. Follow its window to finish."),
        _ => None,
    }
}

/// Distinct display names for prepared and retained setups of one plug-in.
pub fn setup_choice_label(product: &Product, offer: &AvailableAction) -> String {
    let candidate = match &offer.action {
        Action::ExperimentalReplace { candidate, .. } | Action::CompatibilityPublishTest { candidate, .. }
            | Action::ExperimentalEnable { candidate } => Some(candidate),
        _ => None,
    };
    if let Some(candidate) = candidate {
        let ordinal = product.details["preparation"]["candidates"].as_array()
            .and_then(|rows| rows.iter().find(|row| row["id"].as_str() == Some(candidate.as_str())))
            .and_then(|row| row["lineage"]["ordinal"].as_u64());
        let identity = ordinal.map(|ordinal| ordinal.to_string())
            .unwrap_or_else(|| candidate.chars().take(12).collect());
        return format!("{} · setup {identity}", action_label(offer));
    }
    match &offer.action {
        Action::OrdinaryRollback { publication, .. } => {
            product.history.iter().position(|record| record.publication == *publication)
                .map(|index| format!("Restore previous setup {}", index + 1))
                .unwrap_or_else(|| format!("Restore previous setup · {}", publication.chars().take(12).collect::<String>()))
        }
        Action::OrdinaryRestoreRecommended { .. } => "Restore the recommended retained setup".into(),
        _ => action_label(offer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> crate::model::Snapshot {
        serde_json::from_str(include_str!("../../examples/library-preview.json")).unwrap()
    }

    #[test]
    fn touch_refusals_keep_manager_and_global_reasons_visible() {
        let offers = [AvailableAction {
            label: "Rescan".into(),
            action: Action::EnvironmentRescan {
                environment: "exact".into(),
            },
            disabled_reason: Some("Exact inventory is stale".into()),
        }];
        assert_eq!(
            visible_refusals(&offers, Some("Close active DSP"), false),
            ["Close active DSP", "Exact inventory is stale"]
        );
        assert_eq!(
            visible_refusals(&offers, None, true),
            [
                "Waiting for the current request or manager readback",
                "Exact inventory is stale"
            ]
        );
    }

    #[test]
    fn repeated_history_revisions_and_alternate_setups_have_distinct_display_choices() {
        let mut product = snapshot().products.remove(0);
        product.history = (0..16).map(|index| crate::model::History { revision:1, claim:"retained".into(),
            publication:format!("exact-publication-{index}"), active:false, rollback_allowed:false,
            description:"Version 1 · default graphics".into(), rollback_unavailable:Some("Installed files changed".into()) }).collect();
        product.details = serde_json::json!({"preparation":{"candidates":(0..23).map(|index|
            serde_json::json!({"id":format!("exact-candidate-{index}"),"lineage":{"ordinal":243+index}})).collect::<Vec<_>>()}});
        let labels: Vec<_> = product.history.iter().map(|record| setup_choice_label(&product, &AvailableAction {
            label:"Original exact restore label".into(), action:Action::OrdinaryRollback {class_id:product.class_id.clone(),
                publication:record.publication.clone()}, disabled_reason:record.rollback_unavailable.clone() })).collect();
        assert_eq!(labels.iter().collect::<std::collections::HashSet<_>>().len(), 16);
        let labels: Vec<_> = (0..23).map(|index| setup_choice_label(&product, &AvailableAction {
            label:"Original exact prepared label".into(), action:Action::ExperimentalReplace {candidate:format!("exact-candidate-{index}"),
                expected_current:crate::model::PublicationIdentity {id:"current".into(),sha256:"current-digest".into()}},
            disabled_reason:Some("Installed files changed".into()) })).collect();
        assert_eq!(labels.iter().collect::<std::collections::HashSet<_>>().len(), 23);
        assert!(!labels.iter().any(|label|label.contains("revision")));
    }

    #[test]
    fn availability_keeps_refusal_ahead_of_waiting_and_the_exact_intent() {
        let rescan = AvailableAction { label: "Find installed plug-ins".into(),
            action: Action::EnvironmentRescan { environment: "exact".into() }, disabled_reason: None };
        // A shared-resource action waits for global inactivity; a refusal wins over pending work.
        assert_eq!(availability(&rescan, Some("Close active DSP"), true),
            Availability::Refused("Close active DSP".into()));
        assert_eq!(availability(&rescan, None, true), Availability::Waiting);
        assert_eq!(Availability::Waiting.reason(), Some(WAITING));
        let offered = offer(&rescan, Importance::Secondary, None, false);
        assert!(offered.availability.enabled());
        assert_eq!(offered.intent, rescan.action);
        assert_eq!(offered.label, "Find installed plug-ins");
        // The result form stays openable while the class is busy.
        let report = AvailableAction { label: "Record".into(), action: Action::CompatibilityResult {
            candidate: "exact".into(), expected_current: crate::model::PublicationIdentity {
                id: "id".into(), sha256: "sha".into() }, result: crate::model::TestResultKind::Worked,
            passed: vec![], failed_area: None, note: String::new() },
            disabled_reason: Some("Successful result needs clean retirement".into()) };
        assert_eq!(availability(&report, Some("Busy"), false), Availability::Enabled);
        let row = row(&[report], Importance::Quiet, Some("Busy"), false);
        assert_eq!(row.notes, ["Successful result needs clean retirement"]);
    }
}
