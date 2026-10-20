//! What is true about one plug-in, in a musician's words. Several screens
//! read these: the plug-in list and detail, Home, Setup and Activity.
use super::{Status, StatusClass};
use crate::model::{Action, CompatibilityPhase, DawChoice, InstallationJourney, InstallationPhase,
    InstallationPluginStage, Product, Snapshot, System};

/// Current DAW availability. It follows the native connection, independently
/// of compatibility checks or reports.
pub fn status(product: &Product) -> Status {
    let status = |words, class| Status { words, class };
    if product.availability.is_none() {
        return status("DAW connection status unavailable", StatusClass::Unknown);
    }
    if product.availability.as_ref().is_some_and(|availability| availability.available) {
        return status("Available in your DAW", StatusClass::Ready);
    }
    // Removed on purpose: not a fault, and not yet-to-be-installed either.
    if product.disposition == "removed_from_daw" {
        return status("Removed from your DAW", StatusClass::NeedsYou);
    }
    if product.disposition == "uninstalled" {
        return status("Uninstalled", StatusClass::NeedsYou);
    }
    if product.availability.as_ref().is_some_and(|availability|
        !availability.available && availability.reason.is_some()) {
        return status("DAW connection needs attention", StatusClass::Problem);
    }
    match product.disposition.as_str() {
        "ready" | "experimental" => status("DAW connection unavailable", StatusClass::Problem),
        "another_configuration" => status("Another setup is available in your DAW", StatusClass::Problem),
        "prepared" => status("Prepared · not available in your DAW yet", StatusClass::NeedsYou),
        "installed_unqualified" => status("Installed · not available in your DAW yet", StatusClass::NeedsYou),
        "quarantined" => status("Scan needs attention", StatusClass::Problem),
        "needs_attention" => status("Needs attention", StatusClass::Problem),
        _ => status("Status unavailable", StatusClass::Unknown),
    }
}

pub fn role(product: &Product) -> &'static str {
    match product.role.as_str() {
        "instrument" => "Instrument",
        "effect" => "Effect",
        _ => "Plug-in",
    }
}

pub fn vendor(product: &Product) -> &str {
    if product.vendor.trim().is_empty() {
        "Unknown vendor"
    } else {
        product.vendor.trim()
    }
}

/// A recorded problem is being recovered; the plug-in waits for it.
pub fn recovery_pending(product: &Product) -> bool {
    product.compatibility.as_ref().is_some_and(|workflow| matches!(workflow.phase,
        CompatibilityPhase::AwaitingRetirement | CompatibilityPhase::PublicationNeedsAttention))
}

pub fn daw_name(daw: DawChoice) -> &'static str {
    match daw {
        DawChoice::Bitwig => "Bitwig",
        DawChoice::Reaper => "REAPER",
        DawChoice::Other => "Another Linux VST3 DAW",
    }
}

pub fn daw_guidance(daw: Option<DawChoice>) -> &'static str {
    match daw {
        Some(DawChoice::Bitwig) => "In Bitwig, add this plug-in from the browser to a track. Rescan VST3s if it is missing.",
        Some(DawChoice::Reaper) => "In REAPER, add this plug-in to a track's FX. Rescan VST3s if it is missing.",
        _ => "In your native Linux DAW, add this plug-in to a track. Rescan VST3s if it is missing.",
    }
}

/// Guidance follows an explicitly chosen journey for this exact installed class.
/// Several DAWs can share the same native VST3 publication.
pub fn product_daw(snapshot: &Snapshot, product: &Product) -> Option<DawChoice> {
    let mut choices = snapshot.installer_setups.iter().filter(|setup|
        setup.discovered.iter().any(|found| found.environment == product.environment
            && found.module_sha256 == product.module_sha256 && found.class_id == product.class_id))
        .filter_map(|setup| setup.journey.as_ref().map(|journey| journey.daw));
    let first = choices.next()?;
    choices.all(|choice| choice == first).then_some(first)
}

pub fn vendor_install_consequence(snapshot: &Snapshot, action: &Action) -> Option<String> {
    let Action::VendorSoftwareInstall { application, .. } = action else { return None; };
    let app = snapshot.vendor_applications.iter().find(|app| app.actions.iter().any(|offer|
        matches!(&offer.action, Action::VendorSoftwareInstall { application: offered, .. }
            if offered == application)))?;
    let environment = app.details["environment"].as_str()?;
    let affected: Vec<_> = snapshot.products.iter()
        .filter(|product| product.environment == environment && product.details["current"] != false)
        .map(|product| product.name.as_str()).collect();
    (!affected.is_empty()).then(|| format!("{} will be unavailable during this work and checked again after you finish here.",
        affected.join(", ")))
}

pub fn installation_progress(snapshot: &Snapshot, product: &Product) -> Option<&'static str> {
    let operation = snapshot.operation.as_ref()?;
    let revalidation = &product.details["automatic_revalidation"];
    let rechecking = revalidation["operation"] == operation["operation"]
        && operation["operation"].as_str().is_some_and(|id| !id.is_empty())
        && revalidation["predecessor"]["id"].as_str().is_some_and(|id| !id.is_empty())
        && revalidation["predecessor"]["sha256"].as_str().is_some_and(|sha|
            sha.len() == 64 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if product.availability.as_ref().is_some_and(|availability| availability.available)
        || (!rechecking && (!matches!(product.disposition.as_str(), "installed_unqualified" | "prepared")
            || !product.details["refusal"].is_null()))
        || product.disposition == "quarantined" || product.details["current"] == false
        || product.compatibility.as_ref().is_some_and(|workflow|
            workflow.phase == CompatibilityPhase::AwaitingRetirement) {
        return None;
    }
    if !matches!(operation["state"].as_str(), Some("waiting" | "running")) { return None; }
    let journey: InstallationJourney = serde_json::from_value(operation["journey"].clone()).ok()?;
    if journey.phase != InstallationPhase::PreparingPlugins || journey.failure.is_some() { return None; }
    let environment = operation["onboarding"].as_str()?;
    let mut setups = snapshot.installer_setups.iter().filter(|setup|
        setup.environment.as_deref() == Some(environment) && setup.journey.is_some());
    let setup = setups.next()?;
    if setups.next().is_some() { return None; }
    let mut discovered = setup.discovered.iter().filter(|found| found.class_id == product.class_id);
    let found = discovered.next()?;
    if discovered.next().is_some() || found.environment != product.environment
        || found.environment != environment || found.module_sha256 != product.module_sha256 {
        return None;
    }
    let mut plugins = journey.plugins.iter().filter(|plugin| plugin.class_id == product.class_id);
    let plugin = plugins.next()?;
    if plugins.next().is_some() || plugin.failure.is_some() { return None; }
    match plugin.stage {
        InstallationPluginStage::Pending => Some("Waiting to connect to your DAW"),
        InstallationPluginStage::Inspecting => Some("Checking this plug-in"),
        InstallationPluginStage::Preparing => Some("Preparing its DAW connection"),
        InstallationPluginStage::Publishing => Some("Connecting to your DAW"),
        _ => None,
    }
}

pub fn terminal_text(terminal: &str) -> &'static str {
    match terminal {
        "editor_controller_failed" => "Editor or controller failed",
        "windows_host_exited" => "Windows plug-in host exited unexpectedly",
        "transport_failed" => "Bridge connection failed",
        _ => "Session failed; details are available",
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivityCertainty {
    Confirmed,
    CleanupUncertain,
    #[default]
    Unavailable,
}

pub fn activity_certainty(system: &System) -> ActivityCertainty {
    // The manager conservatively sets cleanup_unconfirmed when capacity
    // readback is absent. That does not establish the current cleanup state.
    if !system.capacity_available() {
        ActivityCertainty::Unavailable
    } else if system.cleanup_unconfirmed {
        ActivityCertainty::CleanupUncertain
    } else {
        ActivityCertainty::Confirmed
    }
}

impl ActivityCertainty {
    pub fn cleanup_label(self) -> &'static str {
        match self {
            Self::Confirmed => "Cleanup confirmed",
            Self::CleanupUncertain => "Cleanup unconfirmed",
            Self::Unavailable => "Cleanup unknown",
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct ProductActivity {
    pub certainty: ActivityCertainty,
    pub active: usize,
    pub failed_live: Vec<&'static str>,
    pub recent_failure: Option<&'static str>,
    pub recent_cleanup_unconfirmed: bool,
    /// The manager's session record names a class, not an installed build.
    pub shared_class: bool,
}

impl ProductActivity {
    pub fn detail_label(&self) -> String {
        match self.certainty {
            ActivityCertainty::Confirmed if self.shared_class => format!(
                "{} active in this plug-in class · exact build unavailable",
                self.active
            ),
            ActivityCertainty::Confirmed => format!("{} active instance(s)", self.active),
            ActivityCertainty::CleanupUncertain => {
                "Cleanup unresolved · current instances not confirmed".into()
            }
            ActivityCertainty::Unavailable => "Current activity unavailable".into(),
        }
    }
}

pub fn product_activity(snapshot: &Snapshot, product: &Product) -> ProductActivity {
    let mut result = ProductActivity {
        certainty: activity_certainty(&snapshot.system),
        ..ProductActivity::default()
    };
    if product.class_id.is_empty() {
        return result;
    }
    result.shared_class = snapshot
        .products
        .iter()
        .filter(|other| other.class_id == product.class_id)
        .count()
        > 1;
    for row in snapshot
        .active_sessions
        .iter()
        .filter(|row| row["class_id"].as_str() == Some(product.class_id.as_str()))
    {
        if row["recent"] == true {
            if let Some(terminal) = row["terminal"].as_str() {
                result.recent_failure = Some(terminal_text(terminal));
                result.recent_cleanup_unconfirmed |=
                    row["cleanup_confirmed"] != true || row["transport_retired"] != true;
            }
        } else if result.certainty == ActivityCertainty::Confirmed {
            if let Some(terminal) = row["terminal"].as_str() {
                result.failed_live.push(terminal_text(terminal));
            } else if row["state"] == "active" {
                result.active += 1;
            }
        }
    }
    result
}

/// The manager's plain explanation of a retained failure, when it has one.
pub fn problem_text(problem: &serde_json::Value) -> Option<String> {
    let what = problem["what_happened"].as_str().filter(|text| !text.trim().is_empty())?;
    Some([Some(what), problem["likely_cause"].as_str(), problem["what_to_try"].as_str()]
        .into_iter()
        .flatten()
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" "))
}

/// The manager's explanation of this plug-in's last refused DAW load, with
/// how long ago it was refused. The manager withdraws it after a later load.
pub fn last_load_text(product: &Product) -> Option<String> {
    let last = &product.details["last_load"];
    let text = problem_text(&last["problem"])?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs());
    let what = if last["outcome"] == "activation_refused" { "Audio start refused" } else { "Load refused" };
    Some(match last["observed_at"].as_u64().zip(now).map(|(at, now)| refused_ago(at, now)) {
        Some(when) => format!("{what} {when}. {text}"),
        None => text,
    })
}

fn refused_ago(at: u64, now: u64) -> String {
    match now.saturating_sub(at) {
        0..=59 => "less than a minute ago".into(),
        seconds @ 60..=7199 => format!("{} min ago", seconds / 60),
        seconds @ 7200..=172_799 => format!("{} h ago", seconds / 3600),
        seconds => format!("{} days ago", seconds / 86_400),
    }
}

pub fn product_issue(product: &Product) -> Option<String> {
    // An explanation from the exact retained record wins over generic text.
    if let Some(problem) = problem_text(&product.details["problem"]) {
        return Some(problem);
    }
    if let Some(reason) = product.availability.as_ref()
        .filter(|availability| !availability.available)
        .and_then(|availability| availability.reason.as_ref()) {
        return Some(reason.clone());
    }
    if let Some(workflow) = &product.compatibility {
        if matches!(product.disposition.as_str(), "needs_attention" | "another_configuration") {
            return Some(workflow.summary.clone());
        }
    }
    let preparation = &product.details["preparation"];
    let management = [
        product.details["refusal"].as_str(),
        product.details["preparation_failure"].as_str(),
        preparation["operation"]["reason"].as_str(),
        preparation["recovery"].as_str(),
    ]
    .into_iter()
    .flatten()
    .find(|value| !value.trim().is_empty());
    management
        .map(|value| value.replace('_', " "))
        .or_else(|| {
            product.details["inspection_error"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
        })
        .or_else(|| {
            product.details["inspection_hint"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.replace('_', " "))
        })
        .or_else(|| {
            product
                .limitations
                .first()
                .map(|value| value.replace('_', " "))
        })
}

pub fn check_area_label(area: &str) -> &str {
    match area {
        "DawLoad" => "DAW loading",
        "Midi" => "MIDI or note input",
        "Audio" => "Audio or musical processing",
        "Editor" => "Editor display and interaction",
        "Parameters" => "Plug-in controls",
        "Automation" => "Automation",
        "StateRecall" => "Saving and reopening",
        "ProcessingRestart" => "Restarting processing",
        "Retirement" => "Closing the editor and DAW",
        _ => area,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CompatibilityWorkflow, NativeAvailability};

    fn snapshot() -> Snapshot {
        serde_json::from_str(include_str!("../../examples/library-preview.json")).unwrap()
    }

    #[test]
    fn current_native_availability_is_independent_of_compatibility_reports() {
        let mut product = snapshot().products.remove(0);
        product.availability=None;
        assert_eq!(status(&product), Status { words: "DAW connection status unavailable", class: StatusClass::Unknown });
        for phase in [CompatibilityPhase::NotChecked, CompatibilityPhase::CheckFailed,
            CompatibilityPhase::AvailableForTest, CompatibilityPhase::TestResultIncomplete,
            CompatibilityPhase::NeedsWork, CompatibilityPhase::OrdinarySupported] {
            product.compatibility = Some(CompatibilityWorkflow {
                phase, summary:"Optional compatibility record".into(), established:vec![],
                remaining:vec![], current_inspection:None, current_candidate:None,
                primary:None, alternatives:vec![],
            });
            product.availability = Some(NativeAvailability { available:true, reason:None });
            assert_eq!(status(&product), Status { words: "Available in your DAW", class: StatusClass::Ready });
            product.availability = Some(NativeAvailability {
                available:false, reason:Some("The selected native connection is missing.".into()),
            });
            assert_eq!(status(&product), Status { words: "DAW connection needs attention", class: StatusClass::Problem });
            assert_eq!(product_issue(&product).as_deref(),
                Some("The selected native connection is missing."));
        }
    }

    #[test]
    fn installation_and_preparation_do_not_imply_publication() {
        let mut p = snapshot().products.remove(0);
        for state in ["prepared", "installed_unqualified"] {
            p.disposition = state.into();
            p.availability=Some(NativeAvailability {available:false,reason:None});
            assert_eq!(status(&p).class, StatusClass::NeedsYou);
        }
        p.disposition = "experimental".into();
        p.availability=Some(NativeAvailability {available:true,reason:None});
        assert_eq!(status(&p).words, "Available in your DAW");
        p.disposition = "another_configuration".into();
        p.availability=Some(NativeAvailability {available:false,reason:None});
        assert!(status(&p).class.needs_attention());
    }

    #[test]
    fn every_status_has_one_visual_class() {
        let mut p = snapshot().products.remove(0);
        p.availability = Some(NativeAvailability {available:false,reason:None});
        for (disposition, words, class) in [
            ("removed_from_daw", "Removed from your DAW", StatusClass::NeedsYou),
            ("uninstalled", "Uninstalled", StatusClass::NeedsYou),
            ("ready", "DAW connection unavailable", StatusClass::Problem),
            ("experimental", "DAW connection unavailable", StatusClass::Problem),
            ("another_configuration", "Another setup is available in your DAW", StatusClass::Problem),
            ("prepared", "Prepared · not available in your DAW yet", StatusClass::NeedsYou),
            ("installed_unqualified", "Installed · not available in your DAW yet", StatusClass::NeedsYou),
            ("quarantined", "Scan needs attention", StatusClass::Problem),
            ("needs_attention", "Needs attention", StatusClass::Problem),
            ("future_state", "Status unavailable", StatusClass::Unknown),
        ] {
            p.disposition = disposition.into();
            assert_eq!(status(&p), Status { words, class }, "{disposition}");
        }
    }

    #[test]
    fn quarantined_module_shows_retained_scanner_error_before_generic_reason() {
        let mut p = snapshot().products.remove(0);
        p.disposition = "quarantined".into();
        p.details["inspection_error"] =
            serde_json::json!("TimeoutError: Windows call deadline: load_library");
        p.limitations = vec!["inventory_factory_absent_or_duplicate".into()];
        assert_eq!(
            product_issue(&p).as_deref(),
            Some("TimeoutError: Windows call deadline: load_library")
        );
    }

    #[test]
    fn retained_explanation_wins_over_generic_availability_text() {
        let mut p = snapshot().products.remove(0);
        p.disposition = "installed_unqualified".into();
        p.availability = Some(NativeAvailability {available:false,
            reason:Some("This installed plug-in has no current native DAW connection.".into())});
        p.details["inspection_error"] = serde_json::json!("Windows SDK host failed: 90");
        assert_eq!(product_issue(&p).as_deref(),
            Some("This installed plug-in has no current native DAW connection."));
        p.details["problem"] = serde_json::json!({"code":"sdk_component_start",
            "what_happened":"The plug-in loaded, but it refused to start its audio engine.",
            "likely_cause":"It is usually missing something it needs at start-up.",
            "what_to_try":"Use the vendor's installer or app to repair or activate the plug-in, then run the check again."});
        assert_eq!(product_issue(&p).as_deref(),
            Some("The plug-in loaded, but it refused to start its audio engine. It is usually missing something it needs at start-up. Use the vendor's installer or app to repair or activate the plug-in, then run the check again."));
        p.details["problem"] = serde_json::json!({"code":"x","what_happened":" "});
        assert_eq!(product_issue(&p).as_deref(),
            Some("This installed plug-in has no current native DAW connection."));
    }

    #[test]
    fn refused_load_says_how_long_ago() {
        assert_eq!(refused_ago(1_000, 1_030), "less than a minute ago");
        assert_eq!(refused_ago(1_000, 1_000 + 180), "3 min ago");
        assert_eq!(refused_ago(1_000, 1_000 + 3 * 3600), "3 h ago");
        assert_eq!(refused_ago(1_000, 1_000 + 3 * 86_400), "3 days ago");
        assert_eq!(refused_ago(2_000, 1_000), "less than a minute ago");
    }
}
