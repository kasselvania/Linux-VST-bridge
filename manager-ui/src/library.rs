//! egui drawing of the Plug-ins screens from the screen model. The search,
//! filters and opened plug-ins live in the window's state; decisions and
//! words live in the screen model.
use crate::model::{Action, Product, Snapshot};
use crate::screen_model::plugins::{self, LoadingList, PluginCard, PluginList, RoleFilter, StatusFilter};
use crate::screen_model::setup::SetupControls;
use crate::screen_model::fact_line;
use crate::widgets::{self, warning_color};
use eframe::egui;

/// The Plug-ins page's search, filters and opened plug-ins, kept in the
/// window's state.
pub use crate::screen_model::state::PluginsView;

/// Current product identity remains visible while the separately requested
/// compatibility/history readback loads. It carries no action authority.
/// `built`: the list as the window's screen built it, if it did.
pub fn show_current(ui: &mut egui::Ui, state: &mut PluginsView, current: &Snapshot, built: Option<LoadingList>,
    fresh: bool, details_attempted: bool, retry_details: &mut bool) {
    if state.scroll_to_focus {
        ui.scroll_to_cursor(Some(egui::Align::Min));
        state.scroll_to_focus = false;
    }
    ui.heading(plugins::HEADING);
    let mut view = built.unwrap_or_else(|| LoadingList::build(current, &state.query(), fresh, details_attempted));
    if let Some(title) = view.focus_title {
        ui.strong(title);
        if ui.add_sized([180.0, 44.0], egui::Button::new(plugins::SHOW_ALL)).clicked() {
            state.focus = None;
            view = LoadingList::build(current, &state.query(), fresh, details_attempted);
        }
    }
    widgets::text(ui, &view.notice);
    if let Some(retry) = view.retry {
        if ui.add_sized([240.0, 44.0], egui::Button::new(retry)).clicked() {
            *retry_details = true;
        }
    }
    let searched = state.search.clone();
    ui.add(egui::TextEdit::singleline(&mut state.search)
        .hint_text(plugins::SEARCH_HINT)
        .desired_width(f32::INFINITY));
    if !state.search.is_empty() && ui.button(plugins::CLEAR_SEARCH).clicked() {
        state.search.clear();
    }
    if state.search != searched {
        view = LoadingList::build(current, &state.query(), fresh, details_attempted);
    }
    ui.small(&view.count);
    if let Some(empty) = view.empty {
        ui.label(empty);
    }
    let mut selected = None;
    for row in &view.rows {
        ui.push_id((&row.key.environment, &row.key.class_id, &row.key.module_sha256), |ui| {
            egui::Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
                ui.label(egui::RichText::new(&row.name).size(19.0).strong());
                ui.small(fact_line(&row.facts));
                if let Some(status) = row.status { ui.label(status); }
                for note in &row.notes { widgets::text(ui, note); }
                if let Some(manage) = row.manage {
                    if ui.add_sized([240.0, 44.0], egui::Button::new(manage)).clicked() {
                        selected = Some(row.key.clone());
                    }
                }
            });
        });
    }
    if let Some(key) = selected { state.focus_product(key); }
}

/// The list or the opened plug-in. `built`: the list as the window's screen
/// built it, with whether one plug-in was open; a search or filter changed
/// here rebuilds it, so the change shows in this frame.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut PluginsView,
    snapshot: &Snapshot,
    built: Option<(PluginList, bool)>,
    pending: bool,
    chosen: &mut Option<Action>,
    details: impl Fn(&mut egui::Ui, &Product),
) {
    if state.scroll_to_focus {
        ui.scroll_to_cursor(Some(egui::Align::Min));
        state.scroll_to_focus = false;
    }
    let before = state.clone();
    let single_product = built.as_ref().map_or_else(|| state.focused_product().is_some(), |(_, single)| *single);
    ui.horizontal_wrapped(|ui| {
        ui.heading(plugins::HEADING);
        if state.focus.is_some() && ui.add_sized([180.0, 44.0],
            egui::Button::new(plugins::SHOW_ALL)).clicked() {state.focus = None;}
    });
    if !single_product {
        ui.add(egui::TextEdit::singleline(&mut state.search)
            .hint_text(plugins::SEARCH_HINT)
            .desired_width(f32::INFINITY));
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("library-role")
                .selected_text(state.role.label()).show_ui(ui, |ui| {
                for choice in RoleFilter::CHOICES {
                    ui.selectable_value(&mut state.role, choice, choice.label());
                }
            });
            egui::ComboBox::from_id_salt("library-status")
                .selected_text(state.status.label()).show_ui(ui, |ui| {
                for choice in StatusFilter::CHOICES {
                    ui.selectable_value(&mut state.status, choice, choice.label());
                }
            });
            if state.query().filtered() && ui.button(plugins::CLEAR_FILTERS).clicked() {*state = PluginsView::default();}
        });
    }
    let list = match built {
        Some((list, _)) if *state == before => list,
        _ => PluginList::build(snapshot, &state.query(), single_product, plugins::Context {
            busy: snapshot.system.inactive_reason(), pending, expanded: state.expanded }),
    };
    for line in &list.summary { ui.small(line); }
    if let Some((title, hint)) = list.empty {
        ui.add_space(16.0);
        ui.strong(title);
        ui.label(hint);
    }
    for group in &list.groups {
        if let Some(heading) = &group.heading {
            ui.add_space(12.0);
            ui.strong(heading);
        }
        for card in &group.cards {
            plugin_card(ui, card, chosen, |ui| {
                if let Some(record) = card.record { details(ui, &snapshot.products[record]); }
            });
        }
    }
}

/// One plug-in's card. Same class can have several installed builds or environments.
fn plugin_card(ui: &mut egui::Ui, card: &PluginCard, chosen: &mut Option<Action>,
    records: impl FnOnce(&mut egui::Ui)) {
    ui.push_id((&card.key.environment, &card.key.class_id, &card.key.module_sha256), |ui| {
        egui::Frame::group(ui.style())
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(&card.name).size(19.0).strong());
                    let color = if card.status.class.needs_attention() {
                        warning_color(ui)
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.label(egui::RichText::new(card.status.words).color(color));
                });
                ui.small(fact_line(&card.facts));
                for note in &card.notes { widgets::text(ui, note); }
                if let Some(lead) = &card.lead {
                    if let Some(preface) = &lead.preface { ui.label(preface); }
                    widgets::lead_offer(ui, &lead.offer, chosen);
                }
                widgets::offer_row(ui, &card.offers, chosen);
                for uninstall in &card.uninstallers {
                    widgets::offer_row(ui, &uninstall.row, chosen);
                    ui.small(&uninstall.explanation);
                }
                for section in [&card.settings, &card.help, &card.availability,
                    &card.other_setups, &card.previous_setups].into_iter().flatten() {
                    widgets::disclosure(ui, section, chosen);
                }
                if let Some(shared) = &card.shared_setup {
                    egui::CollapsingHeader::new(shared.title)
                        .default_open(shared.start_open)
                        .show(ui, |ui| setup_controls(ui, &shared.controls, chosen));
                }
                widgets::disclosure_with(ui, &card.technical, chosen, records);
            });
    });
}

/// An existing setup's own controls, from the screen model. Shared by the
/// plug-in card and Setup's "Manage an existing setup".
pub(crate) fn setup_controls(ui: &mut egui::Ui, controls: &SetupControls, chosen: &mut Option<Action>) {
    for help in &controls.help { ui.label(help); }
    ui.strong(controls.programs_title);
    ui.small(controls.programs_hint);
    widgets::offer_row(ui, &controls.programs, chosen);
    widgets::offer_row(ui, &controls.running, chosen);
    ui.label(&controls.consequence);
    widgets::text(ui, &controls.affected);
    widgets::disclosure(ui, &controls.identity, chosen);
    if let Some(runtime) = &controls.runtime {
        egui::CollapsingHeader::new(runtime.title).show(ui, |ui| {
            ui.label(&runtime.current);
            ui.label(&runtime.consequence);
            ui.strong(runtime.affected_title);
            for owner in &runtime.affected { ui.label(owner); }
            let key = egui::Id::new(("environment_runtime", &controls.environment));
            let mut selected = ui.data_mut(|data| data.get_temp::<String>(key)).unwrap_or_default();
            egui::ComboBox::from_id_salt(("environment_runtime_choice", &controls.environment))
                .selected_text(runtime.choices.iter().find(|choice| choice.key == selected)
                    .map(|choice| choice.label.as_str()).unwrap_or(runtime.placeholder))
                .show_ui(ui, |ui| for choice in &runtime.choices {
                    ui.selectable_value(&mut selected, choice.key.clone(), &choice.label);
                });
            ui.data_mut(|data| data.insert_temp(key, selected.clone()));
            if let Some(choice) = runtime.choices.iter().find(|choice| choice.key == selected) {
                widgets::offer_row(ui, &choice.content, chosen);
            }
            if let Some(none) = runtime.none { ui.small(none); }
        });
    }
    if let Some(last) = &controls.last_installer {
        ui.label(&last.state);
        if let Some(installation) = &last.installation { ui.label(installation); }
        if let Some(warning) = last.warning { ui.colored_label(warning_color(ui), warning); }
        widgets::disclosure(ui, &last.details, chosen);
    }
    widgets::offer_row(ui, &controls.continuation, chosen);
    let installers = &controls.installers;
    let key = egui::Id::new(("companion_installer", &controls.environment));
    let mut selected = ui.data_mut(|data| data.get_temp::<String>(key))
        .unwrap_or_else(|| installers.preselected.clone().unwrap_or_default());
    egui::ComboBox::from_id_salt(("companion_choice", &controls.environment))
        .selected_text(installers.choices.iter().find(|choice| choice.key == selected)
            .map(|choice| choice.label.as_str()).unwrap_or(installers.placeholder))
        .show_ui(ui, |ui| {
            for choice in &installers.choices {
                ui.selectable_value(&mut selected, choice.key.clone(), &choice.label);
            }
        });
    ui.data_mut(|data| data.insert_temp(key, selected.clone()));
    if let Some(choice) = installers.choices.iter().find(|choice| choice.key == selected) {
        let (run, advanced) = &choice.content;
        widgets::offer_row(ui, run, chosen);
        if let Some(advanced) = advanced { widgets::disclosure(ui, advanced, chosen); }
    }
    if let Some(none) = installers.none { ui.small(none); }
}

#[cfg(test)]
mod tests {
    use crate::screen_model::ProductKey;
    use super::*;
    use crate::model::{AvailableAction, CompatibilityWorkflow, CompatibilityPhase};
    use crate::presentation;
    use crate::screen_model::offers;
    use crate::screen_model::offers::{refusal as primary_refusal, setup_choice_label, visible_refusals};
    use crate::screen_model::product::{self, status};
    use crate::screen_model::{settings, Status, StatusClass};
    fn snapshot() -> Snapshot {
        serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap()
    }
    #[test]
    fn shared_settings_keep_requested_probe_support_and_audio_facts_separate() {
        fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(items) => for item in items { texts(item, out); },
                _ => {}
            }
        }
        let mut product = snapshot().products.remove(0);
        product.details = serde_json::json!({"configuration": {
            "candidate":"candidate-exact", "publication":"another_configuration",
            "launch":{"runtime":"selected-runtime-r7", "environment":"existing-environment",
                "environment_revision":4,"module":"module-exact","class_id":"unfamiliar-class",
                "host":"host-exact","native":"native-exact"},
            "choices":[{"name":"Graphics", "value":"Wine D3D11 fallback", "source":"Explicit local choice",
                "scope":"New hosts for this class", "change":"Apply through trial publication"},
                {"name":"Accessibility", "value":"Windows default", "source":"Explicit local choice",
                "scope":"New hosts for this class", "change":"Apply through trial publication"}],
            "buffering":{"added_frames":512,"source":"Explicit class preference"},
            "qualification":"Not yet qualified",
            "assessment":{"editor":{"status":"opened"}, "runtime_probes":[
                {"api":"d3d11_default", "status":"passed", "renderer":"Probe renderer only", "implementation":"wine_builtin"}]}
        }, "graphics":{"requested":"Legacy duplicate"}});
        product.actions = vec![
            AvailableAction {label:"Prepare compatibility trial".into(), disabled_reason:None,
                action:Action::CandidateSettingsPrepare {candidate:"candidate-exact".into(),
                    settings:crate::model::LocalSettings::default(),expected_current:None}},
            AvailableAction {label:"Legacy graphics trial".into(),disabled_reason:None,
                action:Action::CandidateGraphicsPrepare {candidate:"candidate-exact".into(),
                    backend:None,expected_current:None}},
            AvailableAction {label:"Assess selected settings".into(),disabled_reason:Some("Assessment unavailable while this class is active".into()),
                action:Action::CandidateGraphicsAssess {candidate:"candidate-exact".into()}},
        ];
        for width in [960.0, 560.0] {
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                let mut current = snapshot();
                current.products = vec![product.clone()];
                let mut library = PluginsView { expanded: true, ..Default::default() };
                show(ui, &mut library, &current, None, false, &mut None, |_,_|{});
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape,&mut labels); }
            output.textures_delta.clear();
            for expected in ["Settings", "Graphics: Wine D3D11 fallback", "Accessibility: Windows default",
                "Source: Explicit local choice", "Selected runtime: selected-runtime-r7",
                "Selected environment: existing-environment", "Environment revision: 4", "Not yet qualified",
                "512 added bridge frames", "The DAW sets the live sample rate", "Prepared only.",
                "Effective editor renderer: not established", "Independent runtime probes", "Probe renderer only",
                "Direct3D 11 on this launch: Wine's built-in", "Wine's stand-in adapter name",
                "Assessment unavailable while this class is active"] {
                assert!(labels.iter().any(|line| line.contains(expected)),
                    "{expected} missing at {width}: {labels:?}");
            }
            for forbidden in ["Legacy graphics trial", "Legacy duplicate", "48 kHz", "1024 frames"] {
                assert!(!labels.iter().any(|line| line.contains(forbidden)), "Unexpected {forbidden}");
            }
            let mut same_callback = product.clone();
            same_callback.details["configuration"]["buffering"] = serde_json::json!({
                "added_frames":0,"remembered_frames":512,"delivery_mode":"same_callback",
                "source":"Explicit class preference"});
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                widgets::disclosure(ui,&settings::section(&same_callback,None,false,true).unwrap(),&mut None);
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape,&mut labels); }
            output.textures_delta.clear();
            assert!(labels.iter().any(|line|line == "0 added bridge frames"));
            assert!(labels.iter().any(|line|line.contains("retains 512 frames for restoration")));
            assert!(!labels.iter().any(|line|line.contains("512 added bridge frames")));
        }
    }

    #[test]
    fn preparing_local_settings_remains_clickable_with_unrelated_activity() {
        fn trial_position(shape: &egui::epaint::Shape) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == "Prepare runtime defaults · Windows accessibility defaults" =>
                    Some(text.pos + text.galley.size() * 0.5),
                egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(trial_position),
                _ => None,
            }
        }
        let mut product = snapshot().products.remove(0);
        product.details = serde_json::json!({});
        let action = Action::CandidateSettingsPrepare {candidate:"selected-candidate".into(),
            settings:crate::model::LocalSettings {graphics:None,
                accessibility:crate::model::AccessibilityChoice::WindowsDefault},
            expected_current:Some(crate::model::PublicationIdentity {id:"selected-publication".into(),sha256:"exact-digest".into()})};
        product.actions = vec![AvailableAction {label:"Prepare settings trial".into(),action:action.clone(),disabled_reason:None}];
        for disabled in [false, true] {
            product.actions[0].disabled_reason = disabled.then(|| "Refresh changed configuration".into());
            let mut snapshot = snapshot();
            snapshot.products = vec![product.clone()];
            snapshot.system.dsp = 1;
            snapshot.environments.clear();snapshot.vendor_applications.clear();
            assert!(snapshot.system.inactive_reason().is_some());
            let mut library = PluginsView {expanded:true,..Default::default()};
            let ctx = egui::Context::default();
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            for pressed in [None, Some(true), Some(false)] {
                let events = pressed.map(|pressed| vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output = ctx.run_ui(egui::RawInput {events,..Default::default()}, |ui| {
                    show(ui, &mut library, &snapshot, None,false,&mut chosen,|_,_|{});
                });
                if pressed.is_none() {
                    point = output.shapes.iter().find_map(|shape|trial_position(&shape.shape)).unwrap();
                }
                output.textures_delta.clear();
            }
            assert_eq!(chosen, (!disabled).then(|| action.clone()));
        }
    }
    #[test]
    fn ui2_guided_product_keeps_posture_action_and_refusal_visible_at_both_widths() {
        fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(items) => for item in items { texts(item, out); },
                _ => {}
            }
        }
        let mut snapshot = snapshot();
        snapshot.products.truncate(1);
        let product = &mut snapshot.products[0];
        product.disposition = "installed_unqualified".into();
        product.availability=Some(crate::model::NativeAvailability {available:false,reason:None});
        product.compatibility = Some(CompatibilityWorkflow {
            phase: CompatibilityPhase::NotChecked,
            summary: "Installed · compatibility not checked".into(),
            established: vec![], remaining: vec![], current_inspection: None,
            current_candidate: None,
            primary: Some(AvailableAction {
                label: "Check compatibility".into(),
                action: Action::CompatibilityCheck {
                    selection: "aa".repeat(32), audio_layout: None,
                    recipe: "bb".repeat(32), predecessor: None,
                },
                disabled_reason: Some("Close the plug-in or DAW before checking".into()),
            }), alternatives: vec![],
        });
        for width in [960.0, 560.0] {
            let ctx = egui::Context::default();
            let mut library = PluginsView::default();
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                show(ui, &mut library, &snapshot, None, false, &mut chosen, |_, _| {});
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
            output.textures_delta.clear();
            for expected in ["Installed · not available in your DAW yet", "Prepare for your DAW",
                "Close the plug-in or DAW before checking"] {
                assert!(labels.iter().any(|line| line.contains(expected)),
                    "{expected} missing at {width}: {labels:?}");
            }
            assert!(chosen.is_none());
        }
    }

    #[test]
    fn ui2_problem_form_remains_openable_while_finish_waits_for_retirement() {
        let mut snapshot = snapshot();
        snapshot.products.truncate(1);
        let product = &mut snapshot.products[0];
        let candidate = "aa".repeat(32);
        let expected_current = crate::model::PublicationIdentity {
            id: "bb".repeat(16), sha256: "cc".repeat(32),
        };
        let report = AvailableAction {
            label: "Record test result".into(),
            action: Action::CompatibilityResult {
                candidate: candidate.clone(), expected_current,
                result: crate::model::TestResultKind::Worked, passed: vec![],
                failed_area: None, note: String::new(),
            }, disabled_reason: None,
        };
        assert_eq!(primary_refusal(&report, Some("Cleanup unconfirmed")), None);
        let mut target_busy = report.clone();
        target_busy.disabled_reason = Some("This plug-in has not retired".into());
        assert_eq!(primary_refusal(&target_busy, Some("Cleanup unconfirmed")), None);
        product.compatibility = Some(CompatibilityWorkflow {
            phase: CompatibilityPhase::AwaitingRetirement,
            summary: "Problem recorded. Close the DAW, then finish this result.".into(),
            established: vec![], remaining: vec![], current_inspection: None,
            current_candidate: Some(candidate),
            primary: Some(AvailableAction {
                label: "Finish recording test result".into(),
                action: Action::CompatibilityFinishResult { operation: "dd".repeat(16) },
                disabled_reason: Some("Cleanup unconfirmed".into()),
            }), alternatives: vec![],
        });
        assert_eq!(status(product).words, "Available in your DAW");
        // Finishing a recovery is the plug-in's lead step, not an optional test result.
        assert!(PluginCard::build(&snapshot, &snapshot.products[0], plugins::Context {
            busy: Some("Cleanup unconfirmed"), pending: false, expanded: false }).lead.is_some());
        let product = &snapshot.products[0];
        assert_eq!(primary_refusal(product.compatibility.as_ref().unwrap().primary.as_ref().unwrap(),
            Some("Cleanup unconfirmed")), Some("Cleanup unconfirmed"));
        for width in [560.0,960.0] {
            let ctx=egui::Context::default();
            let mut library=PluginsView::default();
            let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
                ui.set_max_width(width-32.0);
                show(ui, &mut library, &snapshot, None,false,&mut None,|_,_|{});
            });
            fn labels(shape:&egui::epaint::Shape,out:&mut Vec<String>) {
                match shape {
                    egui::epaint::Shape::Text(text)=>out.push(text.galley.text().into()),
                    egui::epaint::Shape::Vec(items)=>for item in items {labels(item,out);},
                    _=>{},
                }
            }
            let mut text=vec![];for shape in &output.shapes {labels(&shape.shape,&mut text);}
            assert!(text.iter().any(|line|line=="Available in your DAW"));
            assert!(text.iter().any(|line|line=="Problem recorded. Close the DAW, then finish this result."));
            assert!(text.iter().any(|line|line=="Finish plug-in recovery"));
            assert!(text.iter().any(|line|line=="Cleanup unconfirmed"));
            assert!(!text.iter().any(|line|line==product::daw_guidance(None)));
            assert!(!library.expanded);
            output.textures_delta.clear();
        }
    }

    #[test]
    fn existing_guided_product_keeps_updates_and_previous_setups_discoverable() {
        fn texts(shape:&egui::epaint::Shape,out:&mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text)=>out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(items)=>for item in items {texts(item,out);},
                _=>{}
            }
        }
        let mut snapshot=snapshot();snapshot.products.truncate(1);
        snapshot.environments.clear();snapshot.vendor_applications.clear();
        let p=&mut snapshot.products[0];p.active_revision=Some(7);
        p.compatibility=Some(CompatibilityWorkflow {
            phase:CompatibilityPhase::OrdinarySupported,summary:"Published".into(),
            established:vec![],remaining:vec![],current_inspection:None,current_candidate:None,
            primary:None,alternatives:vec![],
        });
        p.actions=vec![
            AvailableAction{label:"Check compatibility again".into(),
                action:Action::PluginReinspect{selection:"aa".repeat(32),audio_layout:None},
                disabled_reason:None},
            AvailableAction{label:"Prepare a test bridge update".into(),
                action:Action::PluginPrepare{selection:"aa".repeat(32),inspection:"bb".repeat(32),
                    recipe:"cc".repeat(32),predecessor:None},
                disabled_reason:Some("Installed module changed; rescan first".into())},
            AvailableAction{label:"Roll back to revision 6".into(),
                action:Action::OrdinaryRollback{class_id:p.class_id.clone(),publication:"dd".repeat(16)},
                disabled_reason:None},
        ];
        for (width,expanded) in [(960.0,false),(560.0,false),(960.0,true),(560.0,true)] {
            let ctx=egui::Context::default();let mut library=PluginsView {expanded,..Default::default()};let mut chosen=None;
            let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
                ui.set_max_width(width-32.0);
                show(ui, &mut library, &snapshot, None,false,&mut chosen,|_,_|{});
            });
            let mut labels=vec![];for clipped in &output.shapes {texts(&clipped.shape,&mut labels);}
            output.textures_delta.clear();
            for expected in ["Other setup options (2)","Previous setups (1)"] {
                assert!(labels.iter().any(|s|s.contains(expected)),"{expected} absent at {width}");
            }
            for expected in ["Check this plug-in again","Prepare this setup",
                "Installed module changed; rescan first","Restore previous setup · dddddddddddd"] {
                assert_eq!(labels.iter().any(|s|s==expected),expanded,"{expected} at {width}");
            }
            assert_eq!(library.expanded,expanded);assert!(chosen.is_none());
        }
    }

    #[test]
    fn removed_plug_ins_read_as_removed_and_lead_with_add_back() {
        use crate::model::{NativeAvailability, PublicationIdentity};
        fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(items) => for item in items { texts(item, out); },
                _ => {}
            }
        }
        let render = |snapshot: &Snapshot| {
            let ctx = egui::Context::default();
            let mut library = PluginsView::default();
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                show(ui, &mut library, snapshot, None, false, &mut chosen, |_,_| {});
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
            output.textures_delta.clear();
            labels
        };
        let mut snapshot = snapshot();
        snapshot.products.truncate(1);
        snapshot.environments.clear();
        snapshot.vendor_applications.clear();
        let identity = PublicationIdentity {id:"ac".repeat(16),sha256:"bd".repeat(32)};
        let class = snapshot.products[0].class_id.clone();
        {
            let product = &mut snapshot.products[0];
            product.compatibility = None;
            product.disposition = "ready".into();
            product.availability = Some(NativeAvailability {available:true,reason:None});
            product.actions = vec![AvailableAction {label:"Remove from my DAW".into(),
                action:Action::PluginRemoveFromDaw {class_id:class.clone(),expected_current:identity.clone()},
                disabled_reason:None}];
        }
        let labels = render(&snapshot);
        assert!(labels.iter().any(|l| l == "Remove from my DAW"), "an everyday control: {labels:?}");
        let removed = "Removed from your DAW. Projects that use it open with it missing until you add it back.";
        {
            let product = &mut snapshot.products[0];
            product.disposition = "removed_from_daw".into();
            product.availability = Some(NativeAvailability {available:false,reason:Some(removed.into())});
            product.actions = vec![AvailableAction {label:"Add back to my DAW".into(),
                action:Action::PluginAddBackToDaw {class_id:class,expected_removed:identity},
                disabled_reason:None}];
        }
        assert_eq!(status(&snapshot.products[0]), Status {words:"Removed from your DAW", class:StatusClass::NeedsYou});
        let labels = render(&snapshot);
        assert!(labels.iter().any(|l| l == "Removed from your DAW"));
        assert!(labels.iter().any(|l| l == removed));
        assert!(labels.iter().any(|l| l == "Add back to my DAW"));
        assert!(!labels.iter().any(|l| l == "DAW connection needs attention"));
    }

    #[test]
    fn selected_keep_restore_and_prepared_trials_render_together() {
        use crate::model::{PublicationIdentity, TestResultKind};
        fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(items) => for item in items { texts(item, out); },
                _ => {}
            }
        }
        let mut snapshot = snapshot();
        snapshot.products.truncate(1);
        snapshot.environments.clear();
        snapshot.vendor_applications.clear();
        let product = &mut snapshot.products[0];
        let current = PublicationIdentity {id:"ac".repeat(16),sha256:"bd".repeat(32)};
        let keep = AvailableAction {label:"Keep these settings and record the result".into(),
            action:Action::CompatibilityResult {candidate:"ab".repeat(32),expected_current:current.clone(),
                result:TestResultKind::Worked,passed:vec![],failed_area:None,note:String::new()},
            disabled_reason:None};
        let ready = AvailableAction {label:"Try Wine D3D11 graphics, Windows accessibility disabled · setup 25".into(),
            action:Action::CompatibilityPublishTest {candidate:"cd".repeat(32),expected_current:Some(current.clone())},
            disabled_reason:None};
        let stale = AvailableAction {label:"Try runtime graphics defaults, Windows accessibility defaults · setup 24".into(),
            action:Action::CompatibilityPublishTest {candidate:"ef".repeat(32),expected_current:Some(current.clone())},
            disabled_reason:Some("The previous selection changed. Prepare a trial from the current configuration.".into())};
        let restore = AvailableAction {label:"Restore the settings from before this trial".into(),
            action:Action::ExperimentalDisable {candidate:"ab".repeat(32)},disabled_reason:None};
        let expert = AvailableAction {label:"Record an operator observation".into(),
            action:Action::CandidateObserve {candidate:"ab".repeat(32),area:"editor".into(),
                status:"not_tested".into(),note:String::new()},disabled_reason:None};
        product.compatibility = Some(CompatibilityWorkflow {
            phase:CompatibilityPhase::AvailableForTest,summary:"Selected trial remains available".into(),
            established:vec![],remaining:vec![],current_inspection:None,current_candidate:Some("ab".repeat(32)),
            primary:Some(keep.clone()),alternatives:vec![ready.clone(),stale.clone()],
        });
        product.actions = vec![restore.clone(), AvailableAction {label:ready.label.clone(),
            action:Action::ExperimentalReplace {candidate:"cd".repeat(32),expected_current:current.clone()},
            disabled_reason:None}, AvailableAction {label:stale.label.clone(),
            action:Action::ExperimentalReplace {candidate:"ef".repeat(32),expected_current:current},
            disabled_reason:stale.disabled_reason.clone()}, expert.clone()];
        for (width, expanded) in [(960.0,false),(560.0,false),(960.0,true),(560.0,true)] {
            let ctx = egui::Context::default();
            ctx.global_style_mut(|style| style.animation_time = 0.0);
            let mut library = PluginsView {expanded,..Default::default()};
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                show(ui, &mut library, &snapshot, None, false, &mut chosen, |_,_| {});
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
            output.textures_delta.clear();
            for offer in [&keep,&restore] {
                assert_eq!(labels.iter().filter(|label| **label == offers::action_label(offer)).count(), usize::from(expanded),
                    "optional controls are disclosed at {width}, expanded={expanded}");
            }
            for offer in [&ready,&stale] {
                assert_eq!(labels.iter().filter(|label|**label==setup_choice_label(&snapshot.products[0],offer)).count(),usize::from(expanded));
            }
            assert_eq!(labels.iter().any(|label|Some(label)==stale.disabled_reason.as_ref()),expanded);
            assert_eq!(labels.iter().filter(|label| **label == expert.label).count(), usize::from(expanded),
                "expert section must actually be open only when requested");
            assert_eq!(library.expanded, expanded);
            assert!(chosen.is_none());
        }
    }
    #[test]
    fn previous_setups_expand_to_exact_restore_controls_and_preserve_pending_refusal() {
        fn centers(shape:&egui::epaint::Shape,out:&mut Vec<(String,egui::Pos2)>) {
            match shape {
                egui::epaint::Shape::Text(text)=>out.push((text.galley.text().into(),text.pos+text.galley.size()*0.5)),
                egui::epaint::Shape::Vec(items)=>for item in items {centers(item,out);},
                _=>{},
            }
        }
        for pending in [false,true] {
            let mut snapshot=snapshot();snapshot.products.truncate(1);
            snapshot.environments.clear();snapshot.vendor_applications.clear();
            let product=&mut snapshot.products[0];product.disposition="ready".into();
            product.compatibility=None;
            product.actions=vec![AvailableAction {label:"Check current setup".into(),
                action:Action::PluginReinspect {selection:"current-selection".into(),audio_layout:None},disabled_reason:None}];
            for index in 0..12 {
                product.actions.push(AvailableAction {label:format!("Restore previous setup {index}"),
                    action:Action::OrdinaryRollback {class_id:product.class_id.clone(),publication:format!("exact-publication-{index}")},
                    disabled_reason:(index==0).then(||"Previous files are unavailable".into())});
            }
            product.history = (0..12).map(|index| crate::model::History {revision:1,claim:"test".into(),
                publication:format!("exact-publication-{index}"),active:false,rollback_allowed:true,
                description:format!("Version 1 · settings choice {index}"),rollback_unavailable:None}).collect();
            let expected=product.actions[2].action.clone();
            let ctx=egui::Context::default();presentation::configure_style(&ctx);
            ctx.global_style_mut(|style|style.animation_time=0.0);
            let mut library=PluginsView::default();let mut chosen=None;let mut point=egui::Pos2::ZERO;
            for frame in 0..9 {
                let click=matches!(frame,3|4|7|8);
                let events=if click {vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                    pos:point,button:egui::PointerButton::Primary,pressed:matches!(frame,3|7),modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,egui::vec2(560.0,2000.0))),..Default::default()},|ui| {
                    show(ui, &mut library, &snapshot, None,pending,&mut chosen,|_,_|{});
                });
                let mut text=vec![];for shape in &output.shapes {centers(&shape.shape,&mut text);}
                if frame==2 {
                    assert!(!text.iter().any(|(label,_)|label.starts_with("Restore previous setup")));
                    point=text.iter().find(|(label,_)|label=="Previous setups (12)").unwrap().1;
                    assert!(!text.iter().any(|(label,_)|label=="Check this plug-in again"));
                    assert!(text.iter().find(|(label,_)|label==product::daw_guidance(None)).unwrap().1.y<700.0);
                }
                if frame==6 {
                    assert!(text.iter().any(|(label,_)|label=="Previous files are unavailable"));
                    point=text.iter().find(|(label,_)|label=="Restore previous setup 2").unwrap().1;
                }
                output.textures_delta.clear();
            }
            assert_eq!(chosen,(!pending).then_some(expected));
        }
    }
    #[test]
    fn many_alternate_setups_keep_primary_recovery_and_vendor_controls_visible() {
        fn centers(shape:&egui::epaint::Shape,out:&mut Vec<(String,egui::Pos2)>) {
            match shape {
                egui::epaint::Shape::Text(text)=>out.push((text.galley.text().into(),text.pos+text.galley.size()*0.5)),
                egui::epaint::Shape::Vec(items)=>for item in items {centers(item,out);},
                _=>{},
            }
        }
        for primary_is_option in [false,true] {for pending in [false,true] {
            let mut snapshot=snapshot();snapshot.products.truncate(1);
            snapshot.environments.clear();snapshot.vendor_applications.clear();
            let product=&mut snapshot.products[0];
            let publication=crate::model::PublicationIdentity {id:"current-publication".into(),sha256:"current-digest".into()};
            let primary=if primary_is_option {Action::CompatibilityCheck {selection:"current-selection".into(),
                audio_layout:None,recipe:"current-recipe".into(),predecessor:None}}
                else {Action::CompatibilityResult {candidate:"current-candidate".into(),expected_current:publication.clone(),
                    result:crate::model::TestResultKind::Worked,passed:vec![],failed_area:None,note:String::new()}};
            product.compatibility=Some(CompatibilityWorkflow {phase:if primary_is_option { CompatibilityPhase::NotChecked }
                else { CompatibilityPhase::TestResultIncomplete },
                summary:"The selected setup needs attention.".into(),established:vec![],remaining:vec![],
                current_inspection:None,current_candidate:None,
                primary:Some(AvailableAction {label:"Current next step".into(),action:primary,disabled_reason:None}),alternatives:vec![]});
            product.details=serde_json::json!({"configuration":{"buffering":{"added_frames":512}}});
            product.actions=vec![AvailableAction {label:"Check compatibility".into(),action:Action::PluginReinspect {
                selection:"current-selection".into(),audio_layout:None},disabled_reason:None},
                AvailableAction {label:"Open vendor application".into(),action:Action::VendorApplicationOpen {
                    application:"exact-vendor-application".into()},disabled_reason:None},
                AvailableAction {label:"Return to previous working setup".into(),action:Action::ExperimentalDisable {
                    candidate:"current-candidate".into()},disabled_reason:None}];
            for index in 0..22 {product.actions.push(AvailableAction {label:format!("Apply alternate setup {index}"),
                action:Action::ExperimentalReplace {candidate:format!("exact-candidate-{index}"),expected_current:publication.clone()},
                disabled_reason:(index==0).then(||"This previous build does not match the current application".into())});}
            for index in 0..3 {product.actions.push(AvailableAction {label:format!("Prepare profile trial {index}"),
                action:Action::CandidateSettingsPrepare {candidate:format!("profile-candidate-{index}"),
                    settings:crate::model::LocalSettings::default(),expected_current:Some(publication.clone())},disabled_reason:None});}
            for index in 0..15 {product.actions.push(AvailableAction {label:format!("Restore setup {index}"),
                action:Action::OrdinaryRollback {class_id:product.class_id.clone(),publication:format!("previous-publication-{index}")},disabled_reason:None});}
            product.details["preparation"] = serde_json::json!({"candidates":(0..22).map(|index|
                serde_json::json!({"id":format!("exact-candidate-{index}"),"lineage":{"ordinal":index+1}})).collect::<Vec<_>>()});
            let expected=product.actions[4].action.clone();
            let mut library=PluginsView::default();library.focus_product(ProductKey::from(&*product));
            let ctx=egui::Context::default();presentation::configure_style(&ctx);
            ctx.global_style_mut(|style|style.animation_time=0.0);
            let mut chosen=None;let mut point=egui::Pos2::ZERO;
            for frame in 0..9 {
                let events=if matches!(frame,3|4|7|8) {vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                    pos:point,button:egui::PointerButton::Primary,pressed:matches!(frame,3|7),modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,egui::vec2(560.0,2400.0))),..Default::default()},|ui|show(ui, &mut library, &snapshot, None,pending,&mut chosen,|_,_|{}));
                let mut text=vec![];for shape in &output.shapes {centers(&shape.shape,&mut text);}
                if frame==2 {
                    for label in ["Help and testing", "Open vendor application","DAW availability","Previous setups (15)"] {
                        assert!(text.iter().any(|(shown,_)|shown==label),"{label} disappeared");
                    }
                    assert!(!text.iter().any(|(label,_)|label.starts_with("Apply alternate setup")||label.starts_with("Prepare profile trial")||label.starts_with("Restore setup")));
                    point=text.iter().find(|(label,_)|label==if primary_is_option {
                        "Other setup options (24)" } else { "Other setup options (23)" }).unwrap().1;
                }
                if frame==6 {
                    let refusal=text.iter().find(|(label,_)|label=="This previous build does not match the current application").unwrap().1;
                    point=text.iter().find(|(label,_)|label=="Use this prepared setup · setup 2").unwrap().1;
                    assert!(refusal.y<point.y,"The refusal belongs beside its own offer");
                }
                output.textures_delta.clear();
            }
            assert_eq!(chosen,(!pending).then_some(expected));
        }}
    }
    #[test]
    fn current_product_selection_uses_exact_identity() {
        let snapshot = snapshot();
        let exact = ProductKey::from(&snapshot.products[0]);
        let other = ProductKey::from(&snapshot.products[1]);
        let mut library = PluginsView::default();
        assert_eq!(library.focused_product(), None);
        library.focus_products(vec![exact.clone(), other]);
        assert_eq!(library.focused_product(), None);
        library.focus_product(exact.clone());
        assert_eq!(library.focused_product(), Some(&exact));
        assert_eq!(plugins::matching(&snapshot, &library.query()).len(), 1);
    }

    #[test]
    fn multi_class_setup_route_offers_each_exact_product_detail() {
        fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(items) => for item in items { texts(item, out); },
                _ => {}
            }
        }
        let snapshot = snapshot();
        let first = ProductKey::from(&snapshot.products[0]);
        let second = ProductKey::from(&snapshot.products[1]);
        for fresh in [true, false] {
            let mut library = PluginsView::default();
            library.focus_products(vec![first.clone(), second.clone()]);
            assert!(library.focused_product().is_none());
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                show_current(ui, &mut library, &snapshot, None, fresh, false, &mut false);
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
            output.textures_delta.clear();
            assert_eq!(labels.iter().filter(|line| line.as_str() == "Manage plug-in").count(), 2,
                "read-only detail navigation remains available, fresh={fresh}");
            library.focus_product(second.clone());
            assert_eq!(library.focused_product(), Some(&second));
            assert_eq!(plugins::matching(&snapshot, &library.query()).len(), 1);
        }
    }

    #[test]
    fn available_result_is_optional_but_exact_report_remains_openable_with_activity() {
        fn centers(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Pos2)>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push((text.galley.text().into(), text.pos + text.galley.size() * 0.5)),
                egui::epaint::Shape::Vec(items) => for item in items { centers(item, out); },
                _ => {},
            }
        }
        for pending in [false, true] {
            let mut snapshot = snapshot();
            snapshot.products.truncate(1);
            snapshot.environments.clear(); snapshot.vendor_applications.clear();
            snapshot.system.dsp = 1;
            let product = &mut snapshot.products[0];
            let report = Action::CompatibilityResult { candidate: "exact-selected-setup".into(),
                expected_current: crate::model::PublicationIdentity { id: "exact-selection".into(), sha256: "exact-digest".into() },
                result: crate::model::TestResultKind::Worked, passed: vec![], failed_area: None, note: String::new() };
            product.actions.clear(); product.details = serde_json::json!({});
            product.compatibility = Some(CompatibilityWorkflow { phase: CompatibilityPhase::AvailableForTest,
                summary: "Selected setup; audio check retained".into(), established: vec!["Audio passed".into()],
                remaining: vec!["Saved-project recall".into()], current_inspection: None, current_candidate: None,
                primary: Some(AvailableAction { label: "Old primary label".into(), action: report.clone(),
                    disabled_reason: Some("Successful result needs clean retirement".into()) }), alternatives: vec![] });
            let mut library = PluginsView::default(); library.focus_product(ProductKey::from(&*product));
            let ctx = egui::Context::default(); presentation::configure_style(&ctx);
            ctx.global_style_mut(|style| style.animation_time = 0.0);
            let mut chosen = None; let mut point = egui::Pos2::ZERO;
            for frame in 0..9 {
                let events = if matches!(frame, 3|4|7|8) { vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton { pos:point, button:egui::PointerButton::Primary,
                        pressed:matches!(frame,3|7), modifiers:egui::Modifiers::NONE }] } else { vec![] };
                let mut output = ctx.run_ui(egui::RawInput { events, screen_rect:Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO, egui::vec2(560.0, 1200.0))), ..Default::default() },
                    |ui| show(ui, &mut library, &snapshot, None, pending, &mut chosen, |_,_|{}));
                let mut text = vec![]; for shape in &output.shapes { centers(&shape.shape, &mut text); }
                output.textures_delta.clear();
                if frame == 2 {
                    assert!(!text.iter().any(|(label,_)| label == "Record a test result or problem" || label == "Observed: Audio passed"));
                    assert!(text.iter().any(|(label,_)| label.starts_with("In your native Linux DAW")));
                    point = text.iter().find(|(label,_)| label == "Help and testing").unwrap().1;
                }
                if frame == 6 {
                    assert!(text.iter().any(|(label,_)| label == "Observed: Audio passed"));
                    assert!(text.iter().any(|(label,_)| label == "Not yet checked: Saved-project recall"));
                    point = text.iter().find(|(label,_)| label == "Record a test result or problem").unwrap().1;
                }
            }
            assert_eq!(chosen, (!pending).then_some(report));
        }
    }

    #[test]
    fn current_published_product_exposes_discovery_without_expanding_details() {
        fn position(shape: &egui::epaint::Shape) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(text)
                    if text.galley.text() == "Find installed plug-ins" =>
                    Some(text.pos + text.galley.size() * 0.5),
                egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(position),
                _ => None,
            }
        }
        let mut s = snapshot();
        s.products = vec![s.products[1].clone()];
        let product = &mut s.products[0];
        product.active_revision = Some(7);
        product.actions.clear();
        product.compatibility = Some(CompatibilityWorkflow {
            phase: CompatibilityPhase::OrdinarySupported, summary: "Published".into(),
            established: vec![], remaining: vec![], current_inspection: None,
            current_candidate: None, primary: None, alternatives: vec![],
        });
        let action = Action::EnvironmentRescan { environment: product.environment.clone() };
        s.environments.retain(|environment| environment.id == product.environment);
        s.environments[0].last_scan = serde_json::json!({"module_count":3,"stale":false});
        s.environments[0].actions = vec![AvailableAction {
            label: "Find installed plug-ins".into(), action: action.clone(), disabled_reason: None,
        }];
        assert_eq!(s.vendor_applications[0].state,"closed");
        for width in [960.0,560.0] {
            for (refusal,dsp,clickable) in [
                (None,0,true), (Some("Vendor application is active"),0,false), (None,1,false),
            ] {
                s.environments[0].actions[0].disabled_reason = refusal.map(str::to_owned);
                s.system.dsp = dsp;
                let ctx = egui::Context::default();
                let mut library = PluginsView::default();
                let mut point = egui::Pos2::ZERO;
                let mut chosen = None;
                for pressed in [None,Some(true),Some(false)] {
                    let events = pressed.map(|pressed| vec![egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                            pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                    let mut output = ctx.run_ui(egui::RawInput {events,..Default::default()},|ui| {
                        ui.set_max_width(width-32.0);
                        show(ui, &mut library, &s, None,false,&mut chosen,|_,_|{});
                    });
                    if pressed.is_none() {
                        point = output.shapes.iter().find_map(|shape|position(&shape.shape))
                            .expect("discovery must be visible with details collapsed");
                    }
                    output.textures_delta.clear();
                }
                assert!(!library.expanded);
                assert_eq!(chosen,clickable.then(||action.clone()));
            }
        }
    }

    #[test]
    fn focused_product_is_selected_by_exact_identity_even_after_search() {
        let mut s = snapshot();
        let mut library = PluginsView {
            search: "other".into(),
            ..Default::default()
        };
        let key = ProductKey::from(&s.products[2]);
        library.focus_product(key);
        assert_eq!(plugins::matching(&s, &library.query()).len(), 1);
        assert_eq!(plugins::matching(&s, &library.query())[0].name, "Serum 2 FX");
        assert!(library.scroll_to_focus);
        let mut quarantined = s.products[0].clone();
        quarantined.name = "Quarantined module".into();
        quarantined.class_id.clear();
        s.products.push(quarantined.clone());
        library.focus_product(ProductKey::from(&quarantined));
        assert_eq!(plugins::matching(&s, &library.query()).len(), 1);
        assert_eq!(plugins::matching(&s, &library.query())[0].name, "Quarantined module");
    }

    #[test]
    fn class_controls_use_exact_refusals_while_shared_controls_require_global_inactivity() {
        fn position(shape: &egui::epaint::Shape) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == "Selected action" =>
                    Some(text.pos + text.galley.size() * 0.5),
                egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(position),
                _ => None,
            }
        }
        for (action, refusal, clickable) in [
            (Action::OrdinaryRollback {class_id:"class".into(),publication:"previous".into()},None,true),
            (Action::BufferingSet {class_id:"class".into(),added_frames:512},Some("This class is active"),false),
            (Action::DeliverySet {class_id:"class".into(),mode:crate::model::DeliveryMode::SameCallback},Some("This class is active"),false),
            (Action::EnvironmentRescan {environment:"environment".into()},None,true),
        ] {
            let offer = AvailableAction {label:"Selected action".into(),action:action.clone(),disabled_reason:refusal.map(str::to_owned)};
            let busy = Some("Another class is active");
            assert_eq!(primary_refusal(&offer,busy).is_none(),clickable);
            assert!(!visible_refusals(std::slice::from_ref(&offer),busy,false)
                .iter().any(|reason| reason == "Another class is active"));
            let ctx = egui::Context::default();
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            for pressed in [None,Some(true),Some(false)] {
                let events = pressed.map(|pressed| vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output = ctx.run_ui(egui::RawInput {events,..Default::default()},|ui| {
                    crate::widgets::offer_row(ui,
                        &offers::row(std::slice::from_ref(&offer), offers::Importance::Secondary, busy, false),
                        &mut chosen);
                });
                if pressed.is_none() {
                    point = output.shapes.iter().find_map(|shape| position(&shape.shape)).unwrap();
                }
                output.textures_delta.clear();
            }
            assert_eq!(chosen,clickable.then_some(action));
        }
    }
}
