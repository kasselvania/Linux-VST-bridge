//! Read-only language and routing over the canonical operator snapshot.
use crate::model::{Action, AvailableAction, CompatibilityPhase, DawChoice, InstallationJourney,
    InstallationPhase, InstallationPluginStage, Product, Snapshot, System};
use eframe::egui;

/// Shared readable, touch-sized presentation for the ordinary app and previews.
pub fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.spacing.interact_size.y = 44.0;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    for (kind, size) in [(egui::TextStyle::Heading, 23.0), (egui::TextStyle::Body, 15.0),
        (egui::TextStyle::Button, 14.0), (egui::TextStyle::Small, 13.0)] {
        style.text_styles.insert(kind, egui::FontId::proportional(size));
    }
    ctx.set_global_style(style);
    let mut visuals = ctx.global_style().visuals.clone();
    if visuals.dark_mode {
        visuals.override_text_color = Some(egui::Color32::from_rgb(224, 227, 232));
        visuals.panel_fill = egui::Color32::from_rgb(24, 27, 32);
        visuals.faint_bg_color = egui::Color32::from_rgb(32, 37, 44);
        visuals.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(71, 79, 91);
    } else {
        visuals.override_text_color = Some(egui::Color32::from_rgb(34, 42, 54));
        visuals.panel_fill = egui::Color32::from_rgb(248, 250, 253);
        visuals.faint_bg_color = egui::Color32::from_rgb(236, 241, 248);
    }
    ctx.set_visuals(visuals);
}

pub fn primary_button(label: impl Into<egui::WidgetText>, ui: &egui::Ui) -> egui::Button<'static> {
    egui::Button::new(label).fill(ui.visuals().selection.bg_fill)
        .min_size(egui::vec2(0.0, 44.0))
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

/// Presentation only: the original offered action and refusal remain authoritative.
pub fn action_label(offer: &AvailableAction) -> String {
    match &offer.action {
        Action::SoftwareInstall { .. } | Action::SoftwareInstallResume { .. }
            if offer.label == "Retry" => "Retry".into(),
        Action::SoftwareInstall { .. } => "Install and connect to your DAW".into(),
        Action::SoftwareInstallResume { .. } => "Continue installation and DAW connection".into(),
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
                crate::model::AccessibilityChoice::ProfileDefault => "default accessibility",
                crate::model::AccessibilityChoice::WindowsDefault => "Windows accessibility defaults",
                crate::model::AccessibilityChoice::DisabledForHost => "Windows accessibility off",
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
            if settings.accessibility == crate::model::AccessibilityChoice::DisabledForHost =>
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
        _ => None,
    }
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductKey {
    pub environment: String,
    pub class_id: String,
    pub module_sha256: String,
}

impl From<&Product> for ProductKey {
    fn from(product: &Product) -> Self {
        Self {
            environment: product.environment.clone(),
            class_id: product.class_id.clone(),
            module_sha256: product.module_sha256.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptKey {
    pub installer: String,
    pub environment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Product(ProductKey),
    Attempt(AttemptKey),
    Session(String),
    Activity,
    Setup,
    Diagnostics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attention {
    pub title: String,
    pub detail: String,
    pub destination: Destination,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Ready,
    InUse,
    Maintenance,
    NeedsAttention,
    Unavailable,
}

impl Health {
    pub fn title(self) -> &'static str {
        match self {
            Self::Ready => "Bridge service running",
            Self::InUse => "Plug-ins running",
            Self::Maintenance => "Installation or scan running",
            Self::NeedsAttention => "Bridge service needs attention",
            Self::Unavailable => "Bridge service status unavailable",
        }
    }
}

pub fn health(system: &System) -> Health {
    if !system.capacity_available() {
        Health::Unavailable
    } else if system.cleanup_unconfirmed
        || system.pending_transactions > 0
        || system.stale_transports > 0
    {
        Health::NeedsAttention
    } else if system.maintenance > 0 {
        Health::Maintenance
    } else if system.dsp > 0 {
        Health::InUse
    } else {
        Health::Ready
    }
}

pub fn owned_service_pause(operation: Option<&serde_json::Value>) -> bool {
    let Some(operation) = operation else { return false; };
    let pause = &operation["owned_service_pause"];
    pause["schema"] == 1
        && pause["automatic_operation"] == operation["operation"]
        && operation["operation"].as_str().is_some_and(|id| !id.is_empty())
        && pause["owner_operation"].as_str().is_some_and(|id| !id.is_empty())
        && matches!(operation["state"].as_str(), Some("waiting" | "running" | "vendor_running"))
        && matches!(operation["journey"]["phase"].as_str(), Some("preparing_environment"
            | "vendor_running" | "discovering" | "preparing_plugins"))
        && operation["journey"]["failure"].is_null()
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

pub fn attentions(snapshot: &Snapshot) -> Vec<Attention> {
    let mut items = Vec::new();
    let mut seen_sessions = std::collections::HashSet::new();
    let system = &snapshot.system;
    if !system.capacity_available() {
        items.push(Attention {
            title: "Bridge readback unavailable".into(),
            detail: "Active plug-ins and safe admission cannot be confirmed. Refresh the manager."
                .into(),
            destination: Destination::Diagnostics,
        });
    }
    if system.capacity_available() && system.cleanup_unconfirmed {
        items.push(Attention {
            title: "Cleanup is unconfirmed".into(),
            detail: "The manager is blocking unsafe new work until ownership is resolved.".into(),
            destination: Destination::Diagnostics,
        });
    }
    if system.pending_transactions > 0 {
        items.push(Attention {
            title: "Publication transaction needs review".into(),
            detail: format!(
                "{} pending transaction(s). Open Setup for the manager's reconcile control.",
                system.pending_transactions
            ),
            destination: Destination::Setup,
        });
    }
    if system.stale_transports > 0 && system.capacity_available() {
        items.push(Attention {
            title: "Old bridge transport remains".into(),
            detail: format!(
                "{} stale transport(s) reported by the manager.",
                system.stale_transports
            ),
            destination: Destination::Diagnostics,
        });
    }
    for (index, row) in snapshot.active_sessions.iter().enumerate() {
        let Some(class_id) = row["class_id"].as_str().filter(|id| !id.is_empty()) else {
            continue;
        };
        let Some(terminal) = row["terminal"].as_str() else {
            continue;
        };
        if snapshot
            .products
            .iter()
            .filter(|product| product.class_id == class_id)
            .count()
            < 2
        {
            continue;
        }
        let session = row["session"].as_str().filter(|id| !id.is_empty());
        if !seen_sessions.insert(session.map_or_else(|| format!("legacy-{index}"), str::to_owned)) {
            continue;
        }
        items.push(Attention {
            title: "Plug-in session needs attention".into(),
            detail: format!("{}. Several installed builds share this class; the exact build is unavailable in the session record.", terminal_text(terminal)),
            destination: session.map_or(Destination::Activity, |id| Destination::Session(id.into())),
        });
    }
    for product in &snapshot.products {
        let activity = product_activity(snapshot, product);
        let problematic = matches!(
            product.disposition.as_str(),
            "needs_attention" | "quarantined" | "another_configuration"
        );
        let detail = if problematic && activity.shared_class {
            Some(
                product_issue(product)
                    .unwrap_or_else(|| "Review this plug-in's manager state.".into()),
            )
        } else if !activity.failed_live.is_empty() {
            (!activity.shared_class).then(|| format!("Live session: {}.", activity.failed_live[0]))
        } else if activity.recent_cleanup_unconfirmed {
            (!activity.shared_class)
                .then(|| "A recent failed session has unconfirmed cleanup.".into())
        } else if problematic {
            Some(
                product_issue(product)
                    .unwrap_or_else(|| "Review this plug-in's manager state.".into()),
            )
        } else {
            activity
                .recent_failure
                .filter(|_| !activity.shared_class)
                .map(|failure| format!("Most recent session: {failure}."))
        };
        if let Some(detail) = detail {
            items.push(Attention {
                title: product.name.clone(),
                detail,
                destination: Destination::Product(ProductKey::from(product)),
            });
        }
    }
    for attempt in &snapshot.onboarding {
        if matches!(
            attempt.state.as_str(),
            "needs_attention" | "cleanup_unconfirmed" | "failed"
        ) {
            items.push(Attention {
                title: "Installation needs attention".into(),
                detail: attempt.required_human_action.clone(),
                destination: Destination::Attempt(AttemptKey {
                    installer: attempt.installer.clone(),
                    environment: attempt.environment.clone(),
                }),
            });
        }
    }
    items
}

pub fn session_product<'a>(snapshot: &'a Snapshot, class_id: &str) -> Option<&'a Product> {
    let matching: Vec<_> = snapshot
        .products
        .iter()
        .filter(|product| !class_id.is_empty() && product.class_id == class_id)
        .collect();
    (matching.len() == 1)
        .then(|| matching.into_iter().next())
        .flatten()
}

pub fn session_display_name<'a>(snapshot: &'a Snapshot, class_id: &str) -> &'a str {
    let mut matches = snapshot
        .products
        .iter()
        .filter(|product| !class_id.is_empty() && product.class_id == class_id);
    match (matches.next(), matches.next()) {
        (Some(product), None) => &product.name,
        (Some(_), Some(_)) => "Shared plug-in class · exact build unavailable",
        _ => "Plug-in class unavailable",
    }
}

pub(crate) fn installer_installation_label(t: &serde_json::Value) -> String {
    if t["durable_installation"] == "partial_installation"
        && t["installation_completeness"] == "unproved"
    {
        "Installation changes observed; completion not verified".into()
    } else {
        format!("Installation observation: {}",
            t["durable_installation"].as_str().unwrap_or("unavailable").replace('_', " "))
    }
}

pub(crate) fn installer_lines(v: &serde_json::Value) -> Vec<String> {
    let t = &v["transaction"];
    if t["schema"] != 1 || t["operation"] != v["operation"] {
        return if v["error"] == "installer_launcher_failed" {
            vec!["Earlier installer result: outer launch route exited nonzero; the failing later stage and installation completeness were not captured.".into()]
        } else {
            vec![]
        };
    }
    let installation = installer_installation_label(t);
    let outcome=match t["outcome"].as_str() {
        Some("in_progress")=>"Installer supervision is ongoing. Exact Focus and Stop controls remain available; cleanup will be checked after retirement.",
        Some("outer_nonzero_stage_unknown")=>"The outer installer route exited nonzero. The failing child or stage is not established.",
        Some("installed_dependency_failed")=>"Application files are installed, but a process performing service/dependency work exited nonzero. Review that stage before reinstalling.",
        Some("child_failed")=>"An owned child exited nonzero. Its role and underlying cause may still be unknown.",
        Some("cancelled")=>"This attempt was cancelled. Earlier failure observations remain retained.",
        Some("cleanup_unconfirmed")=>"Installer cleanup is unconfirmed. Further work is blocked.",
        Some("installed")=>"Application files and installation registration were observed. First launch and dependency health remain unproved.",
        Some("partial_installation") if t["durable_installation"] == "partial_installation"
            && t["installation_completeness"] == "unproved"=>installation.as_str(),
        Some("partial_installation")=>"Partial installation: durable changes exist, but a complete application installation is not established.",
        Some("not_installed")=>"No durable installation was found in the inspected surfaces.",
        _=>"Review the installer stage record; completion does not qualify or publish a plug-in.",
    };
    let mut lines = vec![outcome.into()];
    if let Some(code) = v["startup"]["first_problem"]["code"].as_str() {
        let observation = match code {
            "native_steamclient_load_failed" => "native runtime dependency load failed",
            "native_steamclient_export_unavailable" => "native runtime export unavailable",
            "runtime_assertion_observed" => "runtime assertion observed",
            "prefix_initialization_failed" | "prefix_initialization_timeout" => {
                "environment initialization did not complete"
            }
            _ => "startup problem retained; inspect bounded details",
        };
        lines.push(format!(
            "Earlier startup observation: {observation}. Cancellation and cleanup do not erase it."
        ));
    }
    if let Some(n) = t["outer_launcher_exit"].as_i64() {
        lines.push(format!(
            "Outer launcher exit: {n} (separate from payload and service exits)"
        ));
    }
    if outcome != installation.as_str() {
        lines.push(installation);
    }
    if !t["first_failure"].is_null() {
        let f = &t["first_failure"];
        lines.push(format!("First retained process result: phase {} · role {} · relationship {} · domain {} · status {} · cause unestablished",
            f["phase"].as_str().unwrap_or("unknown"), f["role"].as_str().unwrap_or("unknown"),
            f["relationship"].as_str().unwrap_or("unknown"), f["domain"].as_str().unwrap_or("unknown"), f["status"]));
    }
    let binding = &t["launch_binding"];
    if binding["schema"] == 1 && binding["operation"] == v["operation"] {
        lines.push(if binding["status"] == "bound" {
            "Installer Windows root: exact operation and launch generation observed.".into()
        } else {
            format!(
                "Installer Windows root unavailable: {}. Child attribution may be incomplete.",
                binding["reason"].as_str().unwrap_or("missing observation")
            )
        });
    }
    if t["presence_close"]["schema"] == 1 {
        lines.push(format!("Application presence/close requests observed: {}. Helper success does not prove a match or successful closure; exact match and recheck results remain unavailable.",t["presence_close"]["observation_count"]));
        if t["presence_close"]["operation_classes"]
            .as_array()
            .is_some_and(|rows| {
                rows.iter()
                    .any(|r| r == "presence_query" || r == "close_request")
            })
        {
            lines.push("Observed request mechanism: PowerShell/CIM process query or script process close. The matched object and actual close outcome are unavailable.".into());
        }
    }
    lines.push(
        if v["cleanup_confirmed"] == true {
            "Owned process cleanup confirmed."
        } else {
            "Owned process cleanup not yet confirmed."
        }
        .into(),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap()
    }

    #[test]
    fn health_distinguishes_ready_busy_unavailable_and_cleanup_uncertainty() {
        let mut system = snapshot().system;
        assert_eq!(health(&system), Health::Ready);
        system.dsp = 2;
        assert_eq!(health(&system), Health::InUse);
        system.maintenance = 1;
        assert_eq!(health(&system), Health::Maintenance);
        system.pending_transactions = 1;
        assert_eq!(health(&system), Health::NeedsAttention);
        system.pending_transactions = 0;
        system.cleanup_unconfirmed = true;
        assert_eq!(health(&system), Health::NeedsAttention);
        system.service = "capacity unavailable".into();
        assert_eq!(health(&system), Health::Unavailable);
    }

    #[test]
    fn retained_active_row_never_becomes_a_current_product_claim_without_certain_capacity() {
        let mut snapshot = snapshot();
        snapshot.active_sessions = vec![serde_json::json!({
            "session":"exact-session", "class_id":"fixture-fragments", "state":"active", "recent":false
        })];
        let product = &snapshot.products[1];
        let confirmed = product_activity(&snapshot, product);
        assert_eq!(confirmed.certainty, ActivityCertainty::Confirmed);
        assert_eq!(confirmed.active, 1);
        assert_eq!(confirmed.detail_label(), "1 active instance(s)");
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup confirmed"
        );

        snapshot.system.service = "capacity unavailable".into();
        let unavailable = product_activity(&snapshot, product);
        assert_eq!(unavailable.certainty, ActivityCertainty::Unavailable);
        assert_eq!(unavailable.active, 0);
        assert_eq!(unavailable.detail_label(), "Current activity unavailable");
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup unknown"
        );
        snapshot.system.service = "active".into();
        snapshot.system.cleanup_unconfirmed = true;
        let uncertain = product_activity(&snapshot, product);
        assert_eq!(uncertain.certainty, ActivityCertainty::CleanupUncertain);
        assert_eq!(uncertain.active, 0);
        assert_eq!(
            uncertain.detail_label(),
            "Cleanup unresolved · current instances not confirmed"
        );
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup unconfirmed"
        );
        snapshot.system.service = "capacity unavailable".into();
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup unknown"
        );
        let unavailable_with_conservative_flag = product_activity(&snapshot, product);
        assert_eq!(
            unavailable_with_conservative_flag.certainty,
            ActivityCertainty::Unavailable
        );
        assert!(!attentions(&snapshot)
            .iter()
            .any(|item| item.title == "Cleanup is unconfirmed"));
    }

    #[test]
    fn shared_class_session_stays_class_level_on_both_product_surfaces() {
        let mut snapshot = snapshot();
        let mut second_build = snapshot.products[1].clone();
        second_build.module_sha256 = "different-build".into();
        second_build.environment = "different-environment".into();
        snapshot.products.push(second_build);
        snapshot.active_sessions = vec![serde_json::json!({
            "session":"exact-session", "class_id":"fixture-fragments", "state":"active", "recent":false
        })];
        for product in snapshot
            .products
            .iter()
            .filter(|p| p.class_id == "fixture-fragments")
        {
            let activity = product_activity(&snapshot, product);
            assert!(activity.shared_class);
            assert_eq!(activity.active, 1);
            assert_eq!(
                activity.detail_label(),
                "1 active in this plug-in class · exact build unavailable"
            );
        }
        assert!(session_product(&snapshot, "fixture-fragments").is_none());
        assert_eq!(
            session_display_name(&snapshot, "fixture-fragments"),
            "Shared plug-in class · exact build unavailable"
        );
        assert_eq!(snapshot.active_sessions[0]["session"], "exact-session");
    }

    #[test]
    fn exact_attention_target_and_class_level_session_claims() {
        let mut snapshot = snapshot();
        let p = &snapshot.products[2];
        let key = ProductKey::from(p);
        let item = attentions(&snapshot)
            .into_iter()
            .find(|item| item.title == p.name)
            .unwrap();
        assert_eq!(item.destination, Destination::Product(key));
        assert!(item.detail.contains("Rescan"));
        snapshot.active_sessions = vec![
            serde_json::json!({"class_id":"fixture-fragments","state":"active","recent":false}),
            serde_json::json!({"class_id":"fixture-fragments","state":"failed","recent":true,
                "terminal":"windows_host_exited","cleanup_confirmed":true,"transport_retired":true}),
        ];
        let state = product_activity(&snapshot, &snapshot.products[1]);
        assert_eq!(state.active, 1);
        assert_eq!(
            state.recent_failure,
            Some("Windows plug-in host exited unexpectedly")
        );
        assert!(!state.recent_cleanup_unconfirmed);
        snapshot.products.push(snapshot.products[1].clone());
        assert!(product_activity(&snapshot, &snapshot.products[1]).shared_class);
        assert!(session_product(&snapshot, "fixture-fragments").is_none());
        let class_items: Vec<_> = attentions(&snapshot)
            .into_iter()
            .filter(|item| item.destination == Destination::Activity)
            .collect();
        assert_eq!(class_items.len(), 1);
        assert!(class_items[0].detail.contains("exact build is unavailable"));
        snapshot.active_sessions[1]["session"] = serde_json::json!("exact-session");
        assert!(attentions(&snapshot)
            .iter()
            .any(|item| item.destination == Destination::Session("exact-session".into())));
    }
}
