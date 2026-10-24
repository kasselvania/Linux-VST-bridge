//! The forms: the DAW choice, starting an installation, naming an installer,
//! confirming a removal or deletion, and recording a plug-in's test result. Each form checks its
//! exact current offer again before it can be submitted; that lookup, and the
//! shared-resource refusal it applies, duplicate the manager's own admission
//! and go when the service supplies a refusal on each offer (#489).
use super::product::daw_name;
use crate::model::{Action, AvailableAction, DawChoice, ProblemCategory, PublicationIdentity, Snapshot, TestArea,
    TestResultKind};

/// The musician's DAW, chosen before an installation starts. A local draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DawPicker {
    pub placeholder: &'static str,
    pub choices: [(DawChoice, &'static str); 3],
}

pub fn daw_picker() -> DawPicker {
    DawPicker { placeholder: "Choose your DAW",
        choices: [DawChoice::Bitwig, DawChoice::Reaper, DawChoice::Other].map(|choice| (choice, daw_name(choice))) }
}

impl DawPicker {
    pub fn shown(&self, daw: Option<DawChoice>) -> &'static str {
        daw.map(daw_name).unwrap_or(self.placeholder)
    }
}

/// Starting an installation, or finding and connecting a setup's plug-ins.
#[derive(Clone, Debug, PartialEq)]
pub struct InstallForm {
    pub title: &'static str,
    pub question: &'static str,
    pub daw: DawPicker,
    pub explanation: &'static str,
    pub note: &'static str,
    pub submit: &'static str,
    /// Why it cannot start now: the current offer's refusal, or that it changed.
    pub refusal: Option<String>,
    pub cancel: &'static str,
}

impl InstallForm {
    pub fn build(snapshot: Option<&Snapshot>, form: &Action) -> Self {
        let refresh = matches!(form, Action::EnvironmentRefresh { .. });
        Self {
            title: if refresh { "Find and connect" } else { "Install and connect" },
            question: "Which Linux DAW do you use?",
            daw: daw_picker(),
            explanation: if refresh {"The manager finds installed instruments and effects and connects missing plug-ins automatically. Your working plug-ins keep their current setup."}
                else {"Complete requests in the vendor's window. The manager then finds and connects all newly installed VST3 plug-ins automatically."},
            note: "Native VST3 plug-ins may also appear in other compatible Linux DAWs.",
            submit: if refresh { "Find and connect" } else { "Start installation" },
            refusal: match installation_form_offer(snapshot, form) {
                Ok(reason) => reason,
                Err(reason) => Some(reason.into()),
            },
            cancel: "Cancel",
        }
    }
}

pub fn software_install_offer(snapshot: Option<&Snapshot>, installer: &str, policy: Option<crate::model::Powershell>)
    -> Result<Option<String>, &'static str> {
    let snapshot = snapshot.ok_or("Current manager status is unavailable. Refresh before installing.")?;
    snapshot.installer_setups.iter().flat_map(|setup|
        setup.primary.iter().chain(setup.secondary.iter()))
        .find(|offer| offer.action.software_install().is_some_and(|(offered,_,choice)|
            offered==installer && choice==policy))
        .map(|offer| offer.disabled_reason.clone())
        .ok_or("This installation offer changed. Refresh Setup before continuing.")
}

pub fn installation_with_daw(action: &Action, daw: DawChoice) -> Option<Action> {
    if let Some(action)=action.software_install_with_daw(daw) {return Some(action);}
    match action {
        Action::VendorSoftwareInstall { application, .. } =>
            Some(Action::VendorSoftwareInstall { application: application.clone(), daw }),
        Action::EnvironmentRefresh {environment,..}=>
            Some(Action::EnvironmentRefresh {environment:environment.clone(),daw}),
        Action::RendererSoftwareInstall {application,policy,..}=>
            Some(Action::RendererSoftwareInstall {application:application.clone(),policy:*policy,daw}),
        _ => None,
    }
}

pub fn installation_form_offer(snapshot: Option<&Snapshot>, action: &Action)
    -> Result<Option<String>, &'static str> {
    if let Some((installer,_,policy))=action.software_install() {
        return software_install_offer(snapshot,installer,policy);
    }
    match action {
        Action::RendererSoftwareInstall {application,policy,..}=> {
            let snapshot=snapshot.ok_or("Current manager status is unavailable. Refresh before continuing.")?;
            snapshot.vendor_applications.iter().flat_map(|app|&app.actions).find(|offer|
                matches!(&offer.action,Action::RendererSoftwareInstall {application:offered,policy:offered_policy,..}
                    if offered==application && offered_policy==policy))
                .map(|offer|offer.disabled_reason.clone())
                .ok_or("This app's offered rendering choice changed. Refresh Setup before continuing.")
        },
        Action::EnvironmentRefresh {environment,..}=> {
            let snapshot=snapshot.ok_or("Current manager status is unavailable. Refresh before continuing.")?;
            snapshot.installer_setups.iter().flat_map(|setup|setup.primary.iter().chain(setup.secondary.iter()))
                .find(|offer|matches!(&offer.action,Action::EnvironmentRefresh {environment:offered,..} if offered==environment))
                .map(|offer|offer.disabled_reason.clone())
                .ok_or("This setup refresh offer changed. Refresh Setup before continuing.")
        },
        Action::VendorSoftwareInstall { application, .. } => {
            let snapshot = snapshot.ok_or("Current manager status is unavailable. Refresh before installing.")?;
            snapshot.vendor_applications.iter().flat_map(|app| &app.actions)
                .find(|offer| matches!(&offer.action,
                    Action::VendorSoftwareInstall { application: offered, .. } if offered == application))
                .map(|offer| offer.disabled_reason.clone())
                .ok_or("This vendor installation offer changed. Refresh Setup before continuing.")
        }
        _ => Err("This installation offer changed. Refresh Setup before continuing."),
    }
}

/// Where a pressed offer goes before anything is sent: to the manager as it
/// is, or first to the form that completes or confirms it. Every frontend
/// routes through this, so no confirmation or DAW choice can be skipped.
#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Send(Action),
    /// Choose the DAW, then send the installation with it.
    ChooseDaw(Action),
    /// Record what was observed: a test result, or the maintainer's
    /// observation or review.
    Record(Action),
    /// Name the installer.
    Rename(Action),
    /// Review a setup change, removal or deletion before sending it.
    Confirm(Action),
}

/// `daw`: the musician's DAW choice for installations, when made.
pub fn route(action: Action, daw: Option<DawChoice>) -> Route {
    if let Some(installation) = installation_with_daw(&action, daw.unwrap_or(DawChoice::Other)) {
        return if daw.is_some() { Route::Send(installation) } else { Route::ChooseDaw(installation) };
    }
    match action {
        Action::CandidateObserve { .. } | Action::CandidateReview { .. } | Action::CompatibilityResult { .. } =>
            Route::Record(action),
        Action::InstallerRename { .. } => Route::Rename(action),
        _ if needs_confirmation(&action) => Route::Confirm(action),
        _ => Route::Send(action),
    }
}

/// Removing from the DAW or deleting a setup always asks first.
pub fn needs_confirmation(action: &Action) -> bool {
    matches!(action, Action::PresetFiles {command:crate::model::PresetCommand::Restore {..},..} | Action::PluginRemoveFromDaw { .. } | Action::SetupRemoveFromDaw { .. }
        | Action::SetupDelete { .. } | Action::ProductUninstall { .. } | Action::EnvironmentRuntimeSelect { .. } | Action::PluginRestart { .. } | Action::CandidateSettingsTest { .. }
        | Action::EnvironmentInstallerStart { .. } | Action::EnvironmentInstallerStartWithPolicy { .. })
}
/// Setup-level removal offers live in their own section of the setup card.
pub fn removal_offer(action: &Action) -> bool {
    matches!(action, Action::SetupRemoveFromDaw { .. } | Action::SetupDelete { .. })
}
/// The musician's answer to the confirmation. Delete and Uninstall carry it.
pub fn confirmed(action: Action) -> Action {
    match action {
        Action::PresetFiles {product,expected_root,command:crate::model::PresetCommand::Restore {backup,missing_digest,..}} =>
            Action::PresetFiles {product,expected_root,command:crate::model::PresetCommand::Restore {backup,missing_digest,confirmed:true}},
        Action::SetupDelete { installer, environment, forget_installer, .. } =>
            Action::SetupDelete { installer, environment, confirmed: true, forget_installer },
        Action::ProductUninstall { environment, registration, command_sha256, .. } =>
            Action::ProductUninstall { environment, registration, command_sha256, confirmed: true },
        other => other,
    }
}
/// The offer a confirmed request answers: without the musician's own choices.
pub fn offer_identity(action: &Action) -> Action {
    match action.clone() {
        Action::PresetFiles {product,expected_root,command:crate::model::PresetCommand::Restore {backup,missing_digest,..}} =>
            Action::PresetFiles {product,expected_root,command:crate::model::PresetCommand::Restore {backup,missing_digest,confirmed:false}},
        Action::SetupDelete { installer, environment, .. } =>
            Action::SetupDelete { installer, environment, confirmed: false, forget_installer: false },
        Action::ProductUninstall { environment, registration, command_sha256, .. } =>
            Action::ProductUninstall { environment, registration, command_sha256, confirmed: false },
        other => other,
    }
}
/// What a removal or deletion changes, asked before it is sent.
#[derive(Clone, Debug)]
pub struct Confirmation {
    pub title: &'static str,
    pub body: Vec<String>,
    pub confirm: &'static str,
    pub cancel: &'static str,
    /// The exact current offer, or why it can't be confirmed now.
    pub offer: Result<AvailableAction, String>,
    /// The musician's optional choice to also forget the installer file, off
    /// by default, with what it means. Offered when deleting, not when
    /// finishing an earlier deletion.
    pub forget: Option<(&'static str, &'static str)>,
}
pub fn removal_confirmation(snapshot: Option<&Snapshot>, action: &Action) -> Confirmation {
    let finishing = matches!(action, Action::SetupDelete { environment, .. } if snapshot.is_some_and(|s|
        s.deleted_setups.iter().any(|d| &d.environment == environment)));
    Confirmation {
        forget: (matches!(action, Action::SetupDelete { .. }) && !finishing).then_some((
            "Also forget the installer file", "You would need to add the installer again to set this up later.")),
        ..confirmation(snapshot, action)
    }
}
/// A repair file chooser retains the setup revision it was opened for. The
/// ordinary confirmation still uses owner offers; this also rejects a stale
/// local choice instead of applying it to a later revision of that setup.
pub fn confirmation_with_repair(snapshot: Option<&Snapshot>, action: &Action,
    repair: Option<&crate::plugins::repair::InstallerDraft>) -> Confirmation {
    let mut confirmation = removal_confirmation(snapshot,action);
    if let (Some((environment,_,_)),Some(draft)) = (action.environment_installer_start(),repair) {
        if environment == draft.product.environment {
            let offered = snapshot.ok_or("Current setup controls are unavailable.")
                .and_then(|snapshot|crate::plugins::repair::offered(snapshot,draft))
                .and_then(|offer| if &offer.action == action {Ok(offer)} else {Err("The selected installer changed. Choose it again.")});
            confirmation.offer = match offered {
                Ok(offer) => match &offer.disabled_reason {Some(reason)=>Err(reason.clone()),None=>Ok(offer)},
                Err(reason) => Err(reason.into()),
            };
        }
    }
    confirmation
}
fn confirmation(snapshot: Option<&Snapshot>, action: &Action) -> Confirmation {
    const CHANGED: &str = "This choice is no longer offered. Close this and check the current status.";
    // The musician's own answers are not part of the offer.
    let offered = offer_identity(action);
    let current = |offers: Vec<&AvailableAction>| -> Result<AvailableAction, String> {
        let offer = offers.into_iter().find(|offer| offer.action == offered).cloned().ok_or(CHANGED)?;
        match &offer.disabled_reason { Some(reason) => Err(reason.clone()), None => Ok(offer) }
    };
    let Some(snapshot) = snapshot else {
        return Confirmation { title: "Check current status", body: vec![], confirm: "Continue",
            cancel: "Close", offer: Err("Current manager status is unavailable. Refresh before continuing.".into()),
            forget: None };
    };
    match action {
        Action::PresetFiles {command:crate::model::PresetCommand::Restore {..},..} => Confirmation {
            title:"Restore the missing preset files?",
            body:vec!["Copy only the exact missing files offered from the verified backup.".into(),
                "Existing files stay unchanged. Files with different bytes remain visible conflicts.".into()],
            confirm:"Restore missing files",cancel:"Not now",forget:None,
            offer:current(snapshot.products.iter().flat_map(|p|p.actions.iter()).collect()),
        },
        action if action.environment_installer_start().is_some() => {
            let (environment, installer, _) = action.environment_installer_start().unwrap();
            let target = snapshot.environment_installers.iter().find(|row|row.environment == environment);
            let name = snapshot.installer_setups.iter().find(|row|row.installer == installer)
                .map(|row|row.name.as_str()).unwrap_or("the selected Windows installer");
            let mut body = vec![format!("Run {name} in {}?", target.map(|row|row.name.as_str()).unwrap_or("this setup")),
                "Finish the maker's installer in its own window. Choose Repair there if it offers that option.".into(),
                "The installer can change plug-ins, presets, authorisation and shared components in this setup. Stopping it cannot undo those changes.".into()];
            if let Some(target) = target {
                if !target.affected.is_empty() { body.push(format!("Affected plug-ins: {}.",target.affected.join(", "))); }
            }
            body.push("Close plug-ins using this setup before continuing. Other setups can keep running.".into());
            Confirmation {title:"Run installer in this setup?",body,confirm:"Run installer",cancel:"Keep choosing",
                offer:current(snapshot.environment_installers.iter().filter(|row|row.environment == environment)
                    .flat_map(|row|row.choices.iter().chain(&row.actions)).collect()),forget:None}
        },
        Action::CandidateSettingsTest {settings, expected_current, ..} => Confirmation {
            title:"Use these settings and test?",
            body:vec![format!("Display mode: {}. Windows accessibility: {}.",
                if settings.graphics.is_some() {"Wine Direct3D 11"} else {"Runtime defaults"},
                match settings.accessibility {
                    crate::model::AccessibilityChoice::WindowsDefault => "Windows defaults",
                    crate::model::AccessibilityChoice::DisabledForHost => "off for this plug-in",
                    crate::model::AccessibilityChoice::ProfileDefault => "the resolved profile default",
                }),
                if expected_current.is_some() {
                    "Your previous settings are kept so you can restore them. Other setups and Windows installations are unchanged."
                } else {
                    "This makes the plug-in available for a test. You can remove the test from your DAW afterward; its Windows installation stays in place."
                }.into(),
                "After the change, reopen this plug-in in your DAW, check its editor, and report whether it works.".into()],
            confirm:"Use this and test", cancel:"Keep choosing",
            offer:current(snapshot.products.iter().flat_map(|product| &product.actions).collect()), forget:None,
        },
        Action::PluginRestart { snapshot:state, .. } => Confirmation {
            title:"Restart this plug-in instance?",
            body:vec![crate::recovery::LOSS_WARNING.into(),
                format!("The confirmed state to restore is revision {}.", state.revision),
                "Only this instance restarts. Other plug-in instances keep their own state.".into()],
            confirm:"Restart from confirmed state", cancel:"Not now",
            offer:current(snapshot.products.iter().flat_map(|product| &product.actions).collect()), forget:None,
        },
        Action::EnvironmentRuntimeSelect { .. } => {
            let preview = snapshot.presentation.observed().and_then(|p|
                p.change_previews.iter().find(|p| p.action == offered));
            let mut body = vec![];
            if let Some(preview) = preview {
                body.push(preview.consequence.clone());
                for setup in &preview.affected {
                    body.push(format!("Changes: {}{}", setup.name, setup_members(setup)));
                }
                for setup in &preview.unchanged {
                    body.push(format!("Unchanged: {}{}", setup.name, setup_members(setup)));
                }
            }
            let offer = if preview.is_some() {
                current(snapshot.environment_installers.iter().flat_map(|t| &t.runtime)
                    .flat_map(|r| &r.choices).collect())
            } else { Err("The current setup impact is unavailable. Refresh before changing its runtime.".into()) };
            Confirmation { title: "Review this runtime change", body, confirm: "Change this setup's runtime",
                cancel: "Not now", offer, forget: None }
        }
        Action::PluginRemoveFromDaw { class_id, .. } => {
            let product = snapshot.products.iter().find(|p| p.class_id.eq_ignore_ascii_case(class_id)
                && p.actions.iter().any(|offer| offer.action == *action));
            let name = product.map_or("This plug-in", |p| p.name.as_str());
            Confirmation { title: "Remove from your DAW?",
                body: vec![format!("{name} will no longer appear in your DAW's plug-in list."),
                    "Projects that use it will open with it missing until you add it back.".into(),
                    "Its Windows installation and settings stay in its setup, so you can add it back later, unless you delete that setup.".into()],
                confirm: "Remove from my DAW", cancel: "Keep it",
                offer: current(product.into_iter().flat_map(|p| p.actions.iter()).collect()), forget: None }
        }
        Action::SetupRemoveFromDaw { environment } => {
            let names: Vec<_> = snapshot.products.iter().filter(|p| p.environment == *environment
                && p.availability.as_ref().is_some_and(|a| a.available)).map(|p| p.name.as_str()).collect();
            let names = if names.is_empty() { "This setup's plug-ins".into() } else { names.join(", ") };
            Confirmation { title: "Remove from your DAW?",
                body: vec![format!("{names} will no longer appear in your DAW's plug-in list."),
                    "Projects that use them will open with them missing until you add them back.".into(),
                    "Their Windows installation stays in this setup. If you delete this setup afterwards, they can't be added back; you would set it up again.".into()],
                confirm: "Remove from my DAW", cancel: "Keep them",
                offer: current(snapshot.installer_setups.iter().flat_map(|s| s.secondary.iter()).collect()), forget: None }
        }
        Action::SetupDelete { installer, environment, .. } => {
            if let Some(deleted) = snapshot.deleted_setups.iter().find(|d| d.environment == *environment) {
                return Confirmation { title: "Finish deleting this setup?",
                    body: vec![format!("Deleting the Windows setup for {} didn't finish. This removes the rest of its files.", deleted.name),
                        "Your projects are not changed.".into()],
                    confirm: "Finish deleting", cancel: "Not now",
                    offer: current(deleted.actions.iter().collect()), forget: None };
            }
            let setup = snapshot.installer_setups.iter().find(|s| s.installer == *installer);
            let name = setup.map_or_else(|| "this setup".into(), |s| super::setup::display_name(s, snapshot));
            // Every attempt of the setup, not only the current one.
            let attempts: std::collections::BTreeSet<&str> = std::iter::once(environment.as_str())
                .chain(setup.into_iter().flat_map(|s| s.history.iter().filter_map(|h| h.environment.as_deref())))
                .collect();
            let names: Vec<_> = snapshot.products.iter().filter(|p| attempts.contains(p.environment.as_str()))
                .map(|p| p.name.as_str()).collect();
            let (plugins, them) = match names.len() {
                0 => ("its plug-ins".into(), "them"),
                1 => (names[0].to_owned(), "it"),
                _ => (names.join(", "), "them"),
            };
            Confirmation { title: "Delete this setup?",
                body: vec![format!("This deletes the Windows setup for {name}: everything installed in it, and any licences or sign-ins stored there."),
                    "Files you saved from inside its plug-ins or apps and kept in this setup, such as presets or downloaded content, are deleted too.".into(),
                    "If the vendor limits how many computers can be activated, deactivate the licence in the vendor's app first. Deleting the files doesn't give the activation back.".into(),
                    format!("Projects that use {plugins} will open with {them} missing until you set this setup up again. Add back won't be possible."),
                    "The installer you added stays in the manager, so you can set it up again later, unless you choose to forget it below. A record of this setup is kept in its history. Your project files are not changed.".into()],
                confirm: "Delete setup", cancel: "Cancel",
                offer: current(setup.into_iter().flat_map(|s| s.secondary.iter()).collect()), forget: None }
        }
        Action::ProductUninstall { .. } => {
            let uninstaller = snapshot.uninstallers.iter().find(|u| u.action.action == offered);
            let name = uninstaller.map_or("this software", |u| u.name.as_str());
            let mut body = vec![format!("This runs {name}'s own uninstaller in its setup. Follow its window to finish.")];
            if uninstaller.is_some_and(|u| u.coverage != "installed_with_this") {
                body.push("The manager can't tell exactly what this removes: it was installed together with other software, or what it installed wasn't fully recorded. It might be a shared part, such as a runtime or a licence manager, that other software in this setup needs. Uninstall it only if you're sure.".into());
            }
            if let Some(u) = uninstaller.filter(|u| !u.affected.is_empty()) {
                let one = u.affected.len() == 1;
                body.push(format!("First, {} will be removed from your DAW. Projects that use {} will open with {} missing.",
                    u.affected.join(", "), if one { "it" } else { "them" }, if one { "it" } else { "them" }));
            }
            body.push("Afterwards, this setup's plug-ins are checked again one at a time, and any it didn't remove go back into your DAW. This setup stays unavailable until the check finishes, which can take a few minutes for a large setup.".into());
            body.push("The uninstaller may also remove licences or settings it stored there. Your projects are not changed.".into());
            Confirmation { title: "Uninstall?", body, confirm: "Uninstall", cancel: "Cancel",
                offer: current(uninstaller.map(|u| &u.action).into_iter().collect()), forget: None }
        }
        _ => Confirmation { title: "Check current status", body: vec![], confirm: "Continue", cancel: "Close",
            offer: Err(CHANGED.into()), forget: None },
    }
}

fn setup_members(setup: &crate::model::SetupSummary) -> String {
    if setup.product_names.is_empty() { String::new() }
    else { format!(" — {}", setup.product_names.join(", ")) }
}

/// Naming an installer for display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameForm {
    pub title: &'static str,
    pub explanation: &'static str,
    pub identity: String,
    pub save: &'static str,
    pub cancel: &'static str,
}

impl RenameForm {
    /// The name can be saved once `plausible_installer_label` accepts it.
    pub fn build(installer: &str) -> Self {
        Self {
            title: "Name installer",
            explanation: "This name is for display only. It cannot change the installer, environment or plug-ins.",
            identity: format!("Exact installer ID: {}…{}", &installer[..8], &installer[60..]),
            save: "Save name",
            cancel: "Cancel",
        }
    }
}

/// The same bound the manager applies to an installer label: a duplicate of
/// its validation, kept so the form can refuse before sending. Retire it when
/// the service supplies the refusal (#489).
pub fn plausible_installer_label(label: &str) -> bool {
    !label.trim().is_empty()
        && label.trim() == label
        && label.len() <= 96
        && !matches!(label, "." | "..")
        && !label.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')
            || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
}

pub fn test_areas() -> [TestArea; 9] {
    [TestArea::DawLoad, TestArea::Midi, TestArea::Audio, TestArea::Editor,
        TestArea::Parameters, TestArea::Automation, TestArea::StateRecall,
        TestArea::ProcessingRestart, TestArea::Retirement]
}
pub fn test_area_label(area: TestArea) -> &'static str {
    match area {
        TestArea::DawLoad => "Loaded in the DAW", TestArea::Midi => "MIDI or note input",
        TestArea::Audio => "Audio or musical processing", TestArea::Editor => "Editor rendered and was usable",
        TestArea::Parameters => "Controls responded", TestArea::Automation => "Automation responded",
        TestArea::StateRecall => "Save and reopen recalled state",
        TestArea::ProcessingRestart => "Processing restarted cleanly",
        TestArea::Retirement => "Editor and DAW closed cleanly",
    }
}
pub fn problem_category_label(category: ProblemCategory) -> &'static str {
    match category {
        ProblemCategory::FailedToLoad => "Failed to load",
        ProblemCategory::BlankEditor => "Blank or white editor",
        ProblemCategory::EditorFroze => "Editor froze",
        ProblemCategory::EditorDisappeared => "Editor disappeared",
        ProblemCategory::NoAudio => "No audio",
        ProblemCategory::IncorrectAudio => "Incorrect audio",
        ProblemCategory::ControlsUnresponsive => "Controls did not respond",
        ProblemCategory::HostCrashed => "Plug-in or host crashed",
        ProblemCategory::CleanupIncomplete => "Cleanup did not complete",
        ProblemCategory::Other => "Other problem",
    }
}

pub const PROBLEM_CATEGORIES: [ProblemCategory; 10] = [ProblemCategory::FailedToLoad,
    ProblemCategory::BlankEditor, ProblemCategory::EditorFroze, ProblemCategory::EditorDisappeared,
    ProblemCategory::NoAudio, ProblemCategory::IncorrectAudio, ProblemCategory::ControlsUnresponsive,
    ProblemCategory::HostCrashed, ProblemCategory::CleanupIncomplete, ProblemCategory::Other];

/// Recording what the musician observed with one exact setup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultForm {
    pub title: &'static str,
    pub intro: &'static str,
    pub worked: &'static str,
    pub problem: &'static str,
    pub area_question: &'static str,
    pub area_placeholder: &'static str,
    pub checks: &'static str,
    pub note: &'static str,
    /// Said while the form cannot be submitted as filled in.
    pub incomplete: &'static str,
    /// Said when only a problem can be reported now.
    pub problem_only: &'static str,
    pub submit: &'static str,
    pub cancel: &'static str,
}

pub fn result_form() -> ResultForm {
    ResultForm {
        title: "Record plug-in test result",
        intro: "Mark only what you actually observed. The result records only the checks you observed with this setup; it does not establish support for every configuration.",
        worked: "It worked",
        problem: "Report a problem",
        area_question: "Which part had the problem?",
        area_placeholder: "Choose an area",
        checks: "Checks you confirmed",
        note: "Optional detail for a successful test; describe a problem when reporting one",
        incomplete: "Select an observed check, or choose a problem and describe it.",
        problem_only: "You can report a problem now. A successful result needs this plug-in's clean retirement.",
        submit: "Record this test result",
        cancel: "Cancel",
    }
}

/// A result can be recorded once it says what was observed: a check that
/// worked, or a described problem with its area when the category is Other.
/// A duplicate of the manager's own validation of the result, kept so the
/// form can refuse before sending; retire it when the service supplies the
/// refusal (#489).
pub fn result_complete(result: &TestResultKind, passed: &[TestArea], failed_area: Option<TestArea>, note: &str) -> bool {
    match result {
        TestResultKind::Worked => !passed.is_empty(),
        TestResultKind::Problem { category } => !note.trim().is_empty()
            && (*category != ProblemCategory::Other || failed_area.is_some()),
    }
}

/// The maintainer's own observation and review forms for one candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpertForm {
    pub title: &'static str,
    pub observe_intro: &'static str,
    pub areas: [&'static str; 9],
    pub statuses: [&'static str; 5],
    pub observe_submit: &'static str,
    pub accept_intro: &'static str,
    pub needs_work_intro: &'static str,
    pub review_submit: &'static str,
    pub cancel: &'static str,
}

pub fn expert_form() -> ExpertForm {
    ExpertForm {
        title: "Exact candidate observation / review",
        observe_intro: "This records your observation. It does not create a machine measurement or qualification decision.",
        areas: ["daw_load","midi","audio","editor","parameters","automation","state_recall","processing_restart","retirement"],
        statuses: ["passed","failed","not_tested","unavailable","not_applicable"],
        observe_submit: "Record operator observation",
        accept_intro: "Maintainer review: accept this exact local configuration against the retained results. This is not a project-wide support claim. Publication remains a separate action.",
        needs_work_intro: "Record why this exact configuration needs more work. Publication is unchanged.",
        review_submit: "Record explicit review decision",
        cancel: "Cancel",
    }
}

/// Both editable result variants retain their own current offer. Changing a
/// form from Worked to Problem must not borrow the other variant's permission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultOffers {
    pub worked: Result<Option<String>, &'static str>,
    pub problem: Result<Option<String>, &'static str>,
}

/// The exact candidate/publication and result-variant offers, looked up again
/// before submitting. Checks, category and note are the form's editable fields.
pub fn compatibility_result_offer(snapshot: Option<&Snapshot>, candidate: &str,
    expected_current: &PublicationIdentity) -> Result<ResultOffers, &'static str> {
    let snapshot = snapshot.ok_or("Current manager readback unavailable")?;
    let find = |worked| snapshot.products.iter().flat_map(|product| {
            product.actions.iter().chain(product.compatibility.iter().flat_map(|workflow|
                workflow.primary.iter().chain(workflow.alternatives.iter())))
        }).find(|offer| matches!(&offer.action, Action::CompatibilityResult {
            candidate: offered_candidate, expected_current: offered_current, result, ..
        } if offered_candidate == candidate && offered_current == expected_current
            && matches!(result, TestResultKind::Worked) == worked))
        .map(|offer| offer.disabled_reason.clone())
        .ok_or("This exact test-result offer is no longer current. Refresh the plug-in's controls.");
    Ok(ResultOffers {worked: find(true), problem: find(false)})
}

/// Why this result cannot be recorded now, including the owner's refusal for
/// Problem. A successful-result offer never authorizes reporting a problem.
pub fn compatibility_result_refusal<'a>(offer: Option<&'a Result<ResultOffers, &'static str>>,
    result: &TestResultKind) -> Option<&'a str> {
    match offer {
        Some(Err(reason)) => Some(*reason),
        Some(Ok(offers)) => match if matches!(result, TestResultKind::Worked) { &offers.worked } else { &offers.problem } {
            Ok(reason) => reason.as_deref(),
            Err(reason) => Some(*reason),
        },
        None => Some("Current test-result offer unavailable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_test_confirmation_rechecks_exact_choices_baseline_and_owner_refusal() {
        let mut snapshot = crate::samples::base();
        crate::samples::display_fixture(&mut snapshot);
        let action = snapshot.products[0].actions.iter().find(|offer|offer.disabled_reason.is_none()
            && matches!(offer.action,Action::CandidateSettingsTest {..})).unwrap().action.clone();
        assert!(matches!(route(action.clone(),None),Route::Confirm(_)));
        let confirmation = removal_confirmation(Some(&snapshot),&action);
        assert_eq!(confirmation.confirm,"Use this and test");
        assert_eq!(confirmation.offer.unwrap().action,action);
        assert!(confirmation.body.iter().any(|line|line.contains("reopen this plug-in in your DAW")));
        let offer = snapshot.products[0].actions.iter_mut().find(|offer|offer.action == action).unwrap();
        offer.disabled_reason = Some("The affected instance is active".into());
        assert_eq!(removal_confirmation(Some(&snapshot),&action).offer.unwrap_err(),"The affected instance is active");
        let offer = snapshot.products[0].actions.iter_mut().find(|offer|offer.action == action).unwrap();
        offer.disabled_reason = None;
        let Action::CandidateSettingsTest {expected_current:Some(current),..} = &mut offer.action else {panic!()};
        current.sha256 = "ff".repeat(32);
        assert!(removal_confirmation(Some(&snapshot),&action).offer.is_err());
    }

    #[test]
    fn edited_result_rejoins_its_own_variant_and_exact_publication() {
        let mut snapshot = crate::samples::base();
        snapshot.products.truncate(1);
        let product = &mut snapshot.products[0];
        product.compatibility = None;
        let current = PublicationIdentity {id:"ab".repeat(16), sha256:"cd".repeat(32)};
        let action = |result| Action::CompatibilityResult {candidate:"candidate".into(),
            expected_current:current.clone(),result,passed:vec![],failed_area:None,note:String::new()};
        let worked = TestResultKind::Worked;
        let problem = TestResultKind::Problem {category:ProblemCategory::BlankEditor};
        // Put Problem first: Worked must not inherit its availability.
        product.actions = vec![
            AvailableAction {label:"Problem".into(), action:action(TestResultKind::Problem {category:ProblemCategory::Other}), disabled_reason:None},
            AvailableAction {label:"Worked".into(), action:action(worked), disabled_reason:Some("Wait for this instance to close".into())},
        ];
        let offers = compatibility_result_offer(Some(&snapshot), "candidate", &current);
        assert_eq!(compatibility_result_refusal(Some(&offers), &worked), Some("Wait for this instance to close"));
        assert_eq!(compatibility_result_refusal(Some(&offers), &problem), None);
        snapshot.products[0].actions[0].disabled_reason = Some("Finish the previous problem report".into());
        let offers = compatibility_result_offer(Some(&snapshot), "candidate", &current);
        assert_eq!(compatibility_result_refusal(Some(&offers), &problem), Some("Finish the previous problem report"));
        snapshot.products[0].actions.remove(0);
        let offers = compatibility_result_offer(Some(&snapshot), "candidate", &current);
        assert!(compatibility_result_refusal(Some(&offers), &problem).is_some(), "Worked is not a Problem offer");
        snapshot.products[0].actions[0].disabled_reason = None;
        let offers = compatibility_result_offer(Some(&snapshot), "candidate", &current);
        assert_eq!(compatibility_result_refusal(Some(&offers), &worked), None);
        let changed = PublicationIdentity {sha256:"ef".repeat(32), ..current.clone()};
        for offers in [compatibility_result_offer(Some(&snapshot), "candidate", &changed),
            compatibility_result_offer(Some(&snapshot), "another-candidate", &current),
            compatibility_result_offer(None, "candidate", &current)] {
            assert!(compatibility_result_refusal(Some(&offers), &worked).is_some());
            assert!(compatibility_result_refusal(Some(&offers), &problem).is_some());
        }
    }

    #[test]
    fn runtime_change_requires_an_exact_current_impact_and_offer() {
        use crate::model::{ChangePreview, Observation, SetupSummary};
        let mut snapshot = crate::samples::snapshot(crate::window::Page::Setup, Some("existing_setup"));
        let action = snapshot.environment_installers.iter().flat_map(|t| &t.runtime)
            .flat_map(|r| &r.choices).find(|o| o.disabled_reason.is_none()).unwrap().action.clone();
        assert_eq!(route(action.clone(), Some(DawChoice::Reaper)), Route::Confirm(action.clone()));
        assert!(removal_confirmation(Some(&snapshot), &action).offer.is_err());
        let setup = SetupSummary { environment: "exact-setup".into(), name: "Affected setup".into(),
            products: vec![], product_names: vec!["Instrument".into()], runtime: Observation::Observed("kept-runtime".into()) };
        let Observation::Observed(view) = &mut snapshot.presentation else { panic!() };
        view.change_previews.push(ChangePreview { action: action.clone(), affected: vec![setup],
            unchanged: vec![], consequence: "Other setups keep playing".into() });
        let confirmation = removal_confirmation(Some(&snapshot), &action);
        assert!(confirmation.offer.is_ok());
        assert!(confirmation.body.iter().any(|line| line.contains("Affected setup") && line.contains("Instrument")));
        let mut changed = action.clone();
        if let Action::EnvironmentRuntimeSelect { expected_environment, .. } = &mut changed { *expected_environment = "new-revision".into(); }
        assert!(removal_confirmation(Some(&snapshot), &changed).offer.is_err());
        for target in &mut snapshot.environment_installers {
            if let Some(runtime) = &mut target.runtime { runtime.choices.clear(); }
        }
        assert!(removal_confirmation(Some(&snapshot), &action).offer.is_err(), "a preview never grants an offer");
    }

    #[test]
    fn every_removal_asks_first_and_an_installation_asks_for_the_daw_until_chosen() {
        let identity = PublicationIdentity { id: "ab".repeat(16), sha256: "cd".repeat(32) };
        for action in [
            Action::PluginRemoveFromDaw { class_id: "exact".into(), expected_current: identity.clone() },
            Action::SetupRemoveFromDaw { environment: "exact".into() },
            Action::SetupDelete { installer: "ab".repeat(32), environment: "exact".into(), confirmed: false,
                forget_installer: false },
            Action::ProductUninstall { environment: "exact".into(), registration: "exact".into(),
                command_sha256: "ef".repeat(32), confirmed: false },
        ] {
            assert_eq!(route(action.clone(), Some(DawChoice::Reaper)), Route::Confirm(action));
        }
        let install = Action::SoftwareInstall { installer: "ab".repeat(32), daw: DawChoice::Other };
        assert_eq!(route(install.clone(), None), Route::ChooseDaw(install));
        assert_eq!(route(Action::SoftwareInstall { installer: "ab".repeat(32), daw: DawChoice::Other }, Some(DawChoice::Bitwig)),
            Route::Send(Action::SoftwareInstall { installer: "ab".repeat(32), daw: DawChoice::Bitwig }));
        let rename = Action::InstallerRename { installer: "ab".repeat(32), label: String::new() };
        assert_eq!(route(rename.clone(), None), Route::Rename(rename));
        let rescan = Action::EnvironmentRescan { environment: "exact".into() };
        assert_eq!(route(rescan.clone(), None), Route::Send(rescan));
        let result = Action::CompatibilityResult { candidate: "exact".into(), expected_current: identity,
            result: crate::model::TestResultKind::Worked, passed: vec![], failed_area: None, note: String::new() };
        assert_eq!(route(result.clone(), None), Route::Record(result));
    }
}
