//! Diagnostics: the installed version, the service's own status, every
//! manager action, and the retained technical records behind the other pages.
use super::offers::{self, Importance, OfferRow};
use super::{record, version, Control, Detail, Disclosure, Local, Text};
use crate::model::{Action, Activity, Snapshot, System};

/// Each independently read Support section has its own availability.
/// Activity has no generation token and never supplies action authority.
#[derive(Clone, Copy, Debug)]
pub enum Section<'a, T> { NotRequested, Loading, Loaded(T), Failed(&'a str) }

#[derive(Clone, Copy, Debug)]
pub struct SupportRead<'a> {
    pub status: Section<'a, &'a Activity>,
    pub technical: Section<'a, &'a Snapshot>,
}


/// What the installed bridge is and what Setup holds, with a plain copy for
/// bug reports.
#[derive(Clone, Debug, PartialEq)]
pub struct VersionSection {
    pub installed: String,
    pub setup: Option<String>,
    /// How to apply a newer or different build Setup holds.
    pub apply: Option<&'static str>,
    pub runtime: String,
    pub copy: Control,
    pub copied: &'static str,
}

impl VersionSection {
    /// `distribution` is this computer's distribution name, read by the window.
    pub fn build(snapshot: &Snapshot, distribution: Option<&str>) -> Self {
        let setup = version::setup_line(&snapshot.bridge);
        Self {
            installed: version::installed_line(&snapshot.bridge),
            apply: (setup.is_some() && version::setup_title_note(&snapshot.bridge).is_some())
                .then_some("Open Linux VST Bridge Setup and Updates to apply it."),
            setup,
            runtime: format!("Compatibility runtime: {}",
                version::runtime_summary(snapshot).as_deref().unwrap_or("not installed")),
            copy: Control { label: "Copy version details", importance: Importance::Secondary, enabled: true,
                does: Local::Copy(version::version_details(snapshot, distribution)) },
            copied: "Copied. Paste it into your bug report.",
        }
    }
}

/// The Diagnostics page.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostics {
    pub heading: &'static str,
    pub version: VersionSection,
    pub system: Disclosure,
    pub status_control: Option<Control>,
    pub inspection_control: Option<Control>,
    pub inspection_failure: Option<String>,
    pub actions_title: &'static str,
    /// Every manager action, one row each.
    pub actions: Vec<OfferRow>,
    pub records: Disclosure,
    pub technical: Disclosure,
}

/// The service's capacity, cleanup, capture and last operation, open by
/// default on Diagnostics.
pub fn system_status(snapshot: &Snapshot, pending: bool, expanded: bool) -> Disclosure {
    status(&snapshot.system, &snapshot.capture, snapshot.operation.as_ref(), &snapshot.actions, pending, expanded)
}

fn status(system: &System, capture: &serde_json::Value, operation: Option<&serde_json::Value>,
    actions: &[crate::model::AvailableAction], pending: bool, expanded: bool) -> Disclosure {
    let busy = system.inactive_reason();
    let mut content: Vec<Detail> = Vec::new();
    if !system.capacity_available() {
        content.push(Text::alert("Service capacity unavailable — DSP, keeper and cleanup status cannot be confirmed").into());
    } else {
        content.push(Text::body(format!("Service: {}  ·  Keeper: {}  ·  DSP: {} / {}  ·  Pending: {}  ·  Stale transports: {}",
            system.service, system.keepers, system.dsp, system.ceiling, system.pending_transactions,
            system.stale_transports)).into());
    }
    if system.capacity_available() { content.push(Text::body(format!("Installing / scanning: {}", system.maintenance)).into()); }
    if system.capacity_available() && system.cleanup_unconfirmed {
        content.push(Text::alert("Previous instance cleanup is unconfirmed — retained leases are not proof of a live DSP; new admission is blocked").into());
    }
    // A fixed claim the window prints itself, not a service fact; replace it
    // with the service's buffering recommendation (#489, schema 30).
    content.push(Text::body("512 added frames recommended · 256 unqualified").into());
    content.push(Text::body(if capture["armed"] == true { "Crash capture: armed for next admitted launch" }
        else if capture["active_retention"].as_u64().is_some_and(|count| count > 0) { "Crash capture: retaining an active instance" }
        else if capture["armed"] == false && capture["active_retention"].as_u64() == Some(0) { "Crash capture: off" }
        else { "Crash capture: status unavailable" }).into());
    if capture["armed"] == true {
        for action in actions.iter().filter(|action| matches!(action.action, Action::CaptureDisarm {})) {
            content.push(Detail::Offers(offers::row(std::slice::from_ref(action), Importance::Secondary, busy, pending)));
        }
    }
    if let Some(operation) = operation {
        content.push(Detail::Disclosure(Disclosure::new(if operation["summary"] == true { "Last operation summary" } else { "Last operation receipt" }, vec![record(operation)])));
    }
    Disclosure { start_open: expanded, ..Disclosure::new("System status and diagnostics", content) }
}

impl Diagnostics {
    /// Versions and offers are from the current owner view. Status is a
    /// separate observation; full retained records are an explicit inspection.
    pub fn from_current(current: &Snapshot, read: SupportRead<'_>, pending: bool,
        reading: bool, superseded: bool, distribution: Option<&str>) -> Self {
        let control = |label, does| Control { label, does, importance: Importance::Secondary,
            enabled: !reading && !superseded };
        let mut view = Self {
            heading: "Diagnostics", version: VersionSection::build(current, distribution),
            system: Disclosure::new("Service and capture status", vec![]),
            status_control: None, inspection_control: None, inspection_failure: None,
            actions_title: "Manager actions", actions: vec![],
            records: Disclosure::new("Vendor applications and managed setup details", vec![]),
            technical: Disclosure::new("Technical system details", vec![]),
        };
        view.system = match read.status {
            Section::Loaded(activity) => {
                let mut shown = status(&activity.system, &activity.capture, activity.operation.as_ref(), &[], true, true);
                shown.title = "Service and capture status · last read".into();
                shown
            },
            Section::NotRequested => {
                view.status_control = Some(control("Refresh status", Local::Refresh));
                Disclosure { start_open: true, ..Disclosure::new("Service and capture status",
                    vec![Text::body("Service and capture status has not been read.").into()]) }
            }
            Section::Loading => Disclosure { start_open: true,
                ..Disclosure::new("Service and capture status", vec![Text::body("Loading service and capture status…").into()]) },
            Section::Failed(reason) => {
                view.status_control = Some(control("Refresh status", Local::Refresh));
                Disclosure { start_open: true, ..Disclosure::new("Service and capture status",
                    vec![Text::alert(format!("Service and capture status could not be loaded. {reason}")).into()]) }
            }
        };
        // All actions come from current records, never from Activity or an
        // older full Snapshot. In particular, armed does not invent Disarm.
        view.actions = current.actions.iter().map(|offer| offers::row(std::slice::from_ref(offer),
            Importance::Secondary, current.system.inactive_reason(), pending)).collect();
        match read.technical {
            Section::Loaded(snapshot) => {
                let full = Self::build(snapshot, true, distribution);
                view.records = full.records; view.technical = full.technical;
            }
            Section::NotRequested => {
                view.technical.content.push(Text::body("Read retained setup and technical records when you need to inspect them. This can take a little longer.").into());
                view.inspection_control = Some(control("Inspect technical records", Local::RetryDetails));
            }
            Section::Loading => view.technical.content.push(Text::body("Loading technical records…").into()),
            Section::Failed(reason) => {
                view.inspection_failure = Some(reason.into());
                view.technical.content.push(Text::alert(reason).into());
                view.inspection_control = Some(control("Read technical records again", Local::RetryDetails));
            }
        }
        view
    }

    pub fn build(snapshot: &Snapshot, pending: bool, distribution: Option<&str>) -> Self {
        let busy = snapshot.system.inactive_reason();
        let row = |actions: &[crate::model::AvailableAction]| offers::row(actions, Importance::Secondary, busy, pending);
        let mut records: Vec<Detail> = Vec::new();
        for app in &snapshot.vendor_applications {
            records.push(Text::strong(&app.name).into());
            records.push(Detail::Offers(row(&app.actions)));
            for value in [&app.details["dependency_operation"], &app.details] {
                if value["dependency"].is_object() {
                    records.extend(dependency_retirement_lines(value).into_iter().map(|line| Text::detail(line).into()));
                }
            }
            records.push(record(&app.details));
        }
        for environment in &snapshot.environments {
            records.push(Text::strong(&environment.family).into());
            records.push(Detail::Offers(row(&environment.actions)));
            records.push(Text::detail(format!("Environment: {} · revision {} · runner {}",
                environment.id, environment.revision, environment.runner)).into());
            records.push(record(&environment.last_scan));
        }
        let mut technical: Vec<Detail> = Vec::new();
        if let Ok(value) = serde_json::to_value(&snapshot.system) { technical.push(record(&value)); }
        technical.push(record(&snapshot.capture));
        technical.extend(snapshot.managed_runtime_records.iter().map(record));
        Self {
            heading: "Diagnostics",
            version: VersionSection::build(snapshot, distribution),
            system: system_status(snapshot, pending, true),
            status_control: None, inspection_control: None, inspection_failure: None,
            actions_title: "Manager actions",
            actions: snapshot.actions.iter()
                // Disarming is offered next to the armed state above.
                .filter(|offer| !(snapshot.capture["armed"] == true && matches!(offer.action, Action::CaptureDisarm {})))
                .map(|offer| row(std::slice::from_ref(offer))).collect(),
            records: Disclosure::new("Vendor applications and managed setup details", records),
            technical: Disclosure::new("Technical system details", technical),
        }
    }
}

/// What a vendor app's background service and its last stop left behind.
pub fn dependency_retirement_lines(v: &serde_json::Value) -> Vec<String> {
    if v.is_null() {
        return Vec::new();
    }
    let r = if v["dependency_retirement"].is_object() {
        &v["dependency_retirement"]
    } else {
        &v["dependency"]
    };
    let mut lines = vec![
        format!(
            "Dependency service stop: {}",
            if r["service_retirement_confirmed"] == true {
                "confirmed"
            } else {
                "not confirmed"
            }
        ),
        format!(
            "Dependency process cleanup: {}",
            if r["process_cleanup_confirmed"] == true {
                "confirmed"
            } else {
                "not confirmed"
            }
        ),
        format!(
            "Forced cleanup: {}",
            if r["forced_cleanup_used"] == true {
                "used; does not confirm a clean service stop"
            } else {
                "not reported"
            }
        ),
    ];
    lines.push(match r["dependency_cleanup_disposition"].as_str() {
        Some("graceful_service_retirement")=>"Native Access closed with graceful dependency retirement.".into(),
        Some("exact_owned_session_cleanup")=>"Native Access closed with exact-owned compatibility cleanup; graceful service shutdown was not confirmed.".into(),
        Some("reverified_after_unconfirmed_cleanup")=>"The last session did not confirm it closed; the bridge later confirmed nothing from it was still running.".into(),
        _=>"Native Access or dependency cleanup is not confirmed.".into(),
    });
    if let Some(error) = v["error"].as_str() {
        let explanation=match error {
            "dependency_running_not_ready"=>"The service did not establish exact process and listener readiness; Native Access was not opened.",
            "dependency_recovery_registration_missing"=>"The verified daemon exists, but its service registration is missing. Recovery did not reinstall it.",
            "dependency_recovery_payload_changed"|"dependency_generation_changed"=>"The installed daemon differs from the admitted bundle payload. No replacement was attempted.",
            "dependency_foreign_conflict"=>"Another copy of this app's background service is already running from a different Wine setup. It was not used, adopted or stopped. Close the program using it, then try again.",
            "dependency_listener_conflict"=>"Another program is using the connection this app's background service needs (usually another Wine setup's copy of it). It was not used or stopped. Close it, then try again.",
            "dependency_same_prefix_unowned"=>"A conflicting or unowned daemon prevents preparation. It was not adopted or stopped.",
            "dependency_command_timeout"=>"A service or installer command did not provide its required acknowledgment in time.",
            "dependency_installer_nonzero"=>"The installer returned a nonzero result. Installed files and service readiness are separate facts.",
            _=>"The dependency operation did not complete; inspect the retained step and retirement results.",
        };
        lines.push(format!("Dependency failure: {error}. {explanation}"));
    }
    if v["dependency"]["recovery"]["authority"]
        == "qualified_bundle_payload_and_retired_installation"
    {
        lines.push("Recovering the verified installed dependency without reinstalling; the earlier installer failure remains recorded.".into());
        lines.push(
            if v["dependency"]["ready_tested"] == true {
                "Fresh service readiness: verified."
            } else {
                "Fresh service readiness: not established."
            }
            .into(),
        );
    }
    if let Some(stages) = v["dependency"]["stages"].as_array() {
        for stage in stages.iter().filter(|s| s["action"] == "install") {
            let result = &stage["installer_result"];
            lines.push(match result["installer_exit"].as_u64().filter(|n| *n<=u32::MAX as u64) {
                Some(code) if result["authority"]=="exact_windows_child_handle_exit"=>format!("Dependency installer exit: {code}. This alone does not establish readiness."),
                _=>"Dependency installer exit: unavailable.".into(),
            });
            lines.push(match stage["exit"].as_i64() {
                Some(code) => format!("Dependency runner exit: {code}."),
                None => "Dependency runner retirement was not observed when the command returned."
                    .into(),
            });
        }
    }
    let observation = &r["stop_observation"];
    if observation["schema"] == 3 {
        let explanation=match observation["classification"].as_str() {
            Some("NAD2_STOP_CONFIRMED")=>"Exact service, process-generation, and listener retirement was confirmed.",
            Some("NAD2_STOP_SUBMITTED_PROGRESSING")=>"A stop was submitted or already pending and bounded progress was observed; retirement was not confirmed.",
            Some("NAD2_STOP_SUBMITTED_NO_TRANSITION")=>"The stop request was submitted, but no transition toward service retirement was observed before the bound.",
            Some("NAD2_STOP_NOT_SUBMITTED")=>"The stop request was not submitted; the retained control error is authoritative.",
            Some("NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS")=>"SCM reported stopped while the exact process or an owned listener remained.",
            Some("NAD2_STOP_OBSERVATION_UNAVAILABLE")=>"The bounded stop observation was unavailable.",
            _=>"The bounded stop classification was unavailable.",
        };
        lines.push(format!(
            "Bounded stop response: {} {explanation}",
            observation["classification"]
                .as_str()
                .unwrap_or("unavailable")
        ));
        lines.push(format!("SCM state: {} -> {}; controls masks: {} -> {}; checkpoint: {} -> {}; wait hint: {} -> {} ms.",
            observation["initial_state"],observation["final_state"],observation["initial_controls_accepted"],
            observation["final_controls_accepted"],observation["initial_checkpoint"],observation["checkpoint"],
            observation["initial_wait_hint_ms"],observation["wait_hint_ms"]));
        match observation["control_count"].as_u64() {
            Some(0)=>lines.push(match observation["initial_state"].as_u64() {
                Some(1)=>"No ControlService request was issued; the service was already stopped.".into(),
                Some(3)=>"No ControlService request was issued; the service was already stopping.".into(),
                _=>"No ControlService request was issued; the service state could not be evaluated.".into(),
            }),
            Some(1) if observation["control_submitted"]==true=>lines.push(format!("ControlService result: submitted in {} ms; returned state {}, controls mask {}, checkpoint {}, wait hint {} ms.",
                observation["control_elapsed_ms"],observation["control_return"]["state"],
                observation["control_return"]["controls_accepted"],observation["control_return"]["checkpoint"],
                observation["control_return"]["wait_hint_ms"])),
            Some(1)=>lines.push(format!("The ControlService request was not submitted; error {}.",observation["control_error"])),
            _=>{},
        }
        lines.push(format!("Exact process wait: {} ({}); listener mask: {} -> {}; distinct transitions: {} total, {} retained, {} dropped.",
            observation["process_wait_class"],observation["process_wait"],observation["initial_listener_mask"],
            observation["listener_mask"],observation["transition_count_total"],observation["transition_count_retained"],
            observation["transition_count_dropped"]));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::Page;
    use crate::samples::{self, STATES};

    fn rows(details: &[Detail]) -> Vec<&OfferRow> {
        details.iter().flat_map(|detail| match detail {
            Detail::Offers(row) | Detail::Held { row, .. } => vec![row],
            Detail::Disclosure(inner) => rows(&inner.content),
            Detail::Group(inner) => rows(inner),
            _ => vec![],
        }).collect()
    }

    #[test]
    fn current_support_offers_and_versions_do_not_wait_for_other_sections() {
        let mut current = samples::snapshot(Page::Diagnostics, Some("populated"));
        current.capture = serde_json::Value::Null;
        current.actions = vec![crate::model::AvailableAction { label: "Create problem report".into(),
            action: Action::SupportExport {}, disabled_reason: None }];
        for status in [Section::NotRequested, Section::Loading, Section::Failed("status fixture failure")] {
            let view = Diagnostics::from_current(&current, SupportRead { status,
                technical: Section::NotRequested }, false, false, false, None);
            assert_eq!(view.version, VersionSection::build(&current, None));
            let listed: Vec<_> = view.actions.iter().flat_map(|r| &r.offers).collect();
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].intent, Action::SupportExport {});
            assert!(listed[0].availability.enabled());
            assert!(view.records.content.is_empty(), "unread records are not fabricated");
            assert!(!format!("{:?}", view.system).contains("Crash capture: off"));
            assert_eq!(view.inspection_control.unwrap().does, Local::RetryDetails);
        }
    }

    #[test]
    fn separate_activity_observation_does_not_invent_capture_off_or_actions() {
        let mut current = samples::snapshot(Page::Diagnostics, Some("populated"));
        current.actions = vec![crate::model::AvailableAction { label: "Create problem report".into(),
            action: Action::SupportExport {}, disabled_reason: None }];
        for (capture, expected) in [
            (serde_json::Value::Null, "status unavailable"),
            (serde_json::json!({"armed": false}), "status unavailable"),
            (serde_json::json!({"armed": true,"active_retention":0}), "armed for next admitted launch"),
            (serde_json::json!({"armed": false,"active_retention":1}), "retaining an active instance"),
            (serde_json::json!({"armed": false,"active_retention":0}), "Crash capture: off"),
        ] {
            let activity = Activity { schema: crate::model::OPERATOR_SCHEMA, live: crate::model::LiveActivity::unavailable("fixture"), recent: crate::model::Observation::Unavailable("fixture".into()), incidents: crate::model::Observation::Unavailable("fixture".into()), system: current.system.clone(),
                capture, operation: None, };
            let view = Diagnostics::from_current(&current, SupportRead { status: Section::Loaded(&activity),
                technical: Section::Failed("technical fixture failure") }, true, false, false, None);
            assert!(format!("{:?}", view.system).contains(expected));
            assert!(view.system.title.contains("last read"));
            assert!(rows(&view.system.content).is_empty(), "armed does not invent a Disarm offer");
            assert!(view.actions.iter().flat_map(|r| &r.offers).all(|o| !o.availability.enabled()));
            assert!(view.inspection_control.unwrap().enabled, "local inspection is independent of stale action authority");
            assert_eq!(view.inspection_failure.as_deref(), Some("technical fixture failure"));
        }
    }

    #[test]
    fn every_manager_action_is_listed_once_with_its_refusal() {
        for state in STATES {
            let snapshot = samples::snapshot(Page::Diagnostics, Some(state));
            for pending in [false, true] {
                let view = Diagnostics::build(&snapshot, pending, Some("Example Linux"));
                let armed = snapshot.capture["armed"] == true;
                let listed: Vec<_> = view.actions.iter().chain(rows(&view.system.content))
                    .flat_map(|row| row.offers.iter().map(move |offer| (offer, row))).collect();
                assert_eq!(listed.len(), snapshot.actions.len(), "{state}: each manager action appears once");
                for (offer, row) in listed {
                    assert!(snapshot.actions.iter().any(|action| action.action == offer.intent), "{state}");
                    if let Some(reason) = offer.availability.reason() {
                        assert!(row.notes.iter().any(|note| note == reason), "{state}: {} hides {reason}", offer.label);
                    }
                    if pending { assert!(!offer.availability.enabled(), "{state}"); }
                }
                assert!(view.system.start_open, "{state}");
                assert_eq!(armed, view.system.content.iter().any(|detail|
                    matches!(detail, Detail::Text(text) if text.words == "Crash capture: armed for next admitted launch")));
            }
        }
    }

    #[test]
    fn full_diagnostics_show_the_version_dependency_and_receipt_records() {
        let snapshot = samples::snapshot(Page::Diagnostics, Some("diagnostics_full"));
        let view = Diagnostics::build(&snapshot, false, Some("Example Linux"));
        assert_eq!(view.version.installed, "Installed: Linux VST Bridge 0.12.0preview");
        assert_eq!(view.version.setup.as_deref(), Some("Update ready in Setup: 0.12.0preview2"));
        assert!(view.version.apply.is_some());
        assert_eq!(view.version.runtime, "Compatibility runtime: r7");
        assert!(matches!(&view.version.copy.does, Local::Copy(details) if details.contains("Distribution: Example Linux")));
        let words: Vec<_> = view.records.content.iter().filter_map(|detail| match detail {
            Detail::Text(text) => Some(text.words.as_str()), _ => None }).collect();
        assert!(words.iter().any(|line| line.starts_with("Dependency failure: dependency_command_timeout.")));
        assert!(words.iter().any(|line| line.starts_with("Bounded stop response: NAD2_STOP_SUBMITTED_PROGRESSING")));
        assert!(view.system.content.iter().any(|detail|
            matches!(detail, Detail::Disclosure(inner) if inner.title == "Last operation receipt")));
        assert_eq!(view.technical.content.len(), 4, "system, capture and two runtime records");
    }
}
