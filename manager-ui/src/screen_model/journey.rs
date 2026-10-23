//! Install progress: one managed installation journey, from getting ready to
//! every installed plug-in's DAW connection. Home, Setup, the apps that
//! install plug-ins and the request bar read these.
use super::{offers, product, Status, StatusClass};
use crate::model::{Action, AvailableAction, InstallationFailure, InstallationFailureStage,
    InstallationJourney, InstallationPhase, InstallationPlugin, InstallationPluginStage, Snapshot};

pub fn phase_label(journey: &InstallationJourney) -> &'static str {
    if journey.phase == InstallationPhase::Available && journey.plugins.is_empty() {
        return "Application session complete";
    }
    match journey.phase {
        InstallationPhase::PreparingEnvironment => "Getting ready to install",
        InstallationPhase::VendorRunning => "Complete the vendor's requests",
        InstallationPhase::Discovering => "Finding installed plug-ins",
        InstallationPhase::PreparingPlugins => "Connecting plug-ins to your DAW",
        InstallationPhase::AwaitingApplication => "Continue in the installed app",
        InstallationPhase::Available => "DAW connection complete",
        InstallationPhase::NeedsAttention => "Installation or DAW connection needs attention",
    }
}

/// A finished journey is installation history. What is in the DAW now comes
/// from each plug-in's current state, so a later removal reads as removed.
pub fn current_plugin_state(journey: &InstallationJourney, plugin: &InstallationPlugin,
    current: Option<(&Snapshot, &str)>) -> Option<Status> {
    if journey.phase != InstallationPhase::Available || plugin.stage != InstallationPluginStage::Available {
        return None;
    }
    let (snapshot, environment) = current?;
    snapshot.products.iter().find(|product| product.environment == environment
        && product.class_id.eq_ignore_ascii_case(&plugin.class_id))
        .map(product::status)
}

/// The journey's headline, saying when a finished one is no longer all in the DAW.
pub fn current_label(journey: &InstallationJourney, current: Option<(&Snapshot, &str)>) -> &'static str {
    if journey.phase == InstallationPhase::Available && !journey.plugins.is_empty() {
        let states: Vec<_> = journey.plugins.iter()
            .filter_map(|plugin| current_plugin_state(journey, plugin, current)).collect();
        let available = states.iter().filter(|state| state.words == "Available in your DAW").count();
        if !states.is_empty() && available == 0 { return "Not in your DAW"; }
        if available < states.len() { return "Partly in your DAW"; }
    }
    phase_label(journey)
}

/// The offer on a card that repeats an unfinished connection step.
pub fn continuation_offer(offers: &[AvailableAction]) -> Option<&AvailableAction> {
    offers.iter().find(|offer| matches!(offer.action,
        Action::EnvironmentRefreshResume { .. } | Action::SoftwareInstallResume { .. }))
}

/// A step that met a busy lock is repeated by the card's own continuation;
/// the sentence names that button as the card shows it. Without one on the
/// card, the manager's sentence ("run the same step again") stays. The busy
/// test and sentence are the manager's own (`operator_lock`).
pub fn failure_text(failure: &InstallationFailure, next: Option<&AvailableAction>) -> String {
    let reason = match next.and_then(|offer| continuation_offer(std::slice::from_ref(offer))) {
        Some(offer) if linux_vst_bridge::operator_lock::is_busy_text(&failure.code) =>
            linux_vst_bridge::operator_lock::busy_sentence_naming(&offers::action_label(offer)),
        _ => failure.reason.clone(),
    };
    let phase = match failure.stage {
        InstallationFailureStage::EnvironmentPreparation => "Installation setup",
        InstallationFailureStage::VendorInstallation => "Vendor installation",
        InstallationFailureStage::Discovery => "Plug-in discovery",
        InstallationFailureStage::Inspection => "Plug-in inspection",
        InstallationFailureStage::Preparation => "Native connection preparation",
        InstallationFailureStage::Publication => "DAW connection",
    };
    format!("{phase}: {reason}")
}

/// Each plug-in of a journey, the DAW it is for and what went wrong.
#[derive(Clone, Debug, PartialEq)]
pub struct Results {
    /// "Your DAW: …"
    pub daw: String,
    /// Said when an app visit added no plug-ins.
    pub empty: Option<&'static str>,
    /// The journey's own failure, with the phase it stopped in.
    pub problem: Option<String>,
    pub plugins: Vec<PluginResult>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PluginResult {
    pub name: String,
    /// Its stage in this journey, or once finished, its current availability.
    pub status: Status,
    pub problem: Option<String>,
    /// What to do about a plug-in whose DAW already has a different setup.
    pub hint: Option<&'static str>,
}

fn stage_status(stage: InstallationPluginStage) -> Status {
    let status = |words, class| Status { words, class };
    match stage {
        InstallationPluginStage::Pending => status("Waiting to connect", StatusClass::Working),
        InstallationPluginStage::Inspecting => status("Checking plug-in interfaces", StatusClass::Working),
        InstallationPluginStage::Preparing => status("Preparing the native connection", StatusClass::Working),
        InstallationPluginStage::Publishing => status("Making the connection available", StatusClass::Working),
        InstallationPluginStage::Available => status("Available in your DAW", StatusClass::Ready),
        InstallationPluginStage::NeedsAttention => status("Connection needs attention", StatusClass::Problem),
        InstallationPluginStage::ReplacementRequired =>
            status("A different setup is already available", StatusClass::NeedsYou),
    }
}

impl Results {
    /// `current` is the snapshot and setup environment that say what is in
    /// the DAW now; `next` is the card's own continuation, if it shows one.
    pub fn build(journey: &InstallationJourney, current: Option<(&Snapshot, &str)>,
        next: Option<&AvailableAction>) -> Self {
        Self {
            daw: format!("Your DAW: {}", product::daw_name(journey.daw)),
            empty: (journey.phase == InstallationPhase::Available && journey.plugins.is_empty())
                .then_some("No plug-ins were added during this visit."),
            problem: journey.failure.as_ref().map(|failure| failure_text(failure, next)),
            plugins: journey.plugins.iter().map(|plugin| PluginResult {
                name: plugin.name.clone(),
                status: current_plugin_state(journey, plugin, current)
                    .unwrap_or_else(|| stage_status(plugin.stage)),
                problem: plugin.failure.as_ref().map(|failure| failure_text(failure, next)),
                hint: (plugin.stage == InstallationPluginStage::ReplacementRequired)
                    .then_some("Open this plug-in to review changing its existing DAW setup."),
            }).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DawChoice;

    fn journey(phase: InstallationPhase, plugins: Vec<InstallationPlugin>) -> InstallationJourney {
        InstallationJourney { daw: DawChoice::Reaper, phase, plugins, failure: None }
    }

    fn plugin(name: &str, class_id: &str, stage: InstallationPluginStage) -> InstallationPlugin {
        InstallationPlugin { selection: "exact".into(), class_id: class_id.into(), name: name.into(),
            stage, failure: None }
    }

    #[test]
    fn a_finished_journey_reads_each_plug_in_as_it_is_now() {
        let mut snapshot: Snapshot = serde_json::from_str(include_str!("../../examples/library-preview.json")).unwrap();
        let environment = snapshot.products[0].environment.clone();
        let finished = journey(InstallationPhase::Available, vec![
            plugin("Kontakt 8", &snapshot.products[0].class_id, InstallationPluginStage::Available)]);
        let current = Some((&snapshot, environment.as_str()));
        assert_eq!(current_label(&finished, current), "DAW connection complete");
        let results = Results::build(&finished, current, None);
        assert_eq!(results.daw, "Your DAW: REAPER");
        assert_eq!(results.plugins[0].status.words, "Available in your DAW");
        snapshot.products[0].disposition = "removed_from_daw".into();
        snapshot.products[0].availability = Some(crate::model::NativeAvailability { available: false, reason: None });
        let current = Some((&snapshot, environment.as_str()));
        assert_eq!(current_label(&finished, current), "Not in your DAW");
        assert_eq!(Results::build(&finished, current, None).plugins[0].status,
            Status { words: "Removed from your DAW", class: StatusClass::NeedsYou });
        assert_eq!(phase_label(&journey(InstallationPhase::Available, vec![])), "Application session complete");
        assert_eq!(Results::build(&journey(InstallationPhase::Available, vec![]), None, None).empty,
            Some("No plug-ins were added during this visit."));
    }

    #[test]
    fn a_busy_step_names_the_continuation_its_card_shows() {
        let failure = InstallationFailure { stage: InstallationFailureStage::Publication,
            reason: "Run the same step again.".into(), code: "operation already running".into(), retryable: true };
        assert_eq!(failure_text(&failure, None), "DAW connection: Run the same step again.");
        let resume = AvailableAction { label: "Continue".into(),
            action: Action::SoftwareInstallResume { onboarding: "exact".into() }, disabled_reason: None };
        assert_eq!(failure_text(&failure, Some(&resume)),
            "DAW connection: The manager was busy finishing another task; choose \u{201c}Continue installation and DAW connection\u{201d}.");
        let mut waiting = journey(InstallationPhase::NeedsAttention, vec![
            plugin("Pads", "pads", InstallationPluginStage::ReplacementRequired),
            plugin("Keys", "keys", InstallationPluginStage::Inspecting)]);
        waiting.failure = Some(failure);
        let results = Results::build(&waiting, None, Some(&resume));
        assert!(results.problem.unwrap().contains("Continue installation and DAW connection"));
        assert_eq!(results.plugins[0].status.class, StatusClass::NeedsYou);
        assert!(results.plugins[0].hint.is_some());
        assert_eq!(results.plugins[1].status, Status { words: "Checking plug-in interfaces", class: StatusClass::Working });
    }
}
