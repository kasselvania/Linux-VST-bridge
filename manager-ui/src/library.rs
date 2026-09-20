//! Presentation over the manager snapshot. No discovery, publication or launch authority.
use crate::model::{Action, AvailableAction, Product, Snapshot};
use eframe::egui;

#[derive(Default)]
pub struct Library {
    pub search: String,
    /// Used by the local preview; production preserves normal collapsed state.
    pub expand_details: bool,
    role: String,
    status: String,
}

fn role(product: &Product) -> &str {
    match product.role.as_str() {
        "instrument" => "Instrument",
        "effect" => "Effect",
        _ => "Plug-in",
    }
}

fn status(product: &Product) -> (&str, &str) {
    match product.disposition.as_str() {
        "ready" => ("Published to Bitwig", "published"),
        "experimental" => ("Published · experimental", "published"),
        "another_configuration" => ("Another configuration published", "attention"),
        "prepared" => ("Prepared · not published", "unpublished"),
        "installed_unqualified" => ("Installed · not published", "unpublished"),
        "quarantined" => ("Scan needs attention", "attention"),
        "needs_attention" => ("Needs attention", "attention"),
        _ => ("Status unavailable", "attention"),
    }
}

fn vendor(product: &Product) -> &str {
    if product.vendor.trim().is_empty() {
        "Unknown vendor"
    } else {
        product.vendor.trim()
    }
}

impl Library {
    fn products<'a>(&self, snapshot: &'a Snapshot) -> Vec<&'a Product> {
        let search = self.search.to_lowercase();
        let mut products: Vec<_> = snapshot
            .products
            .iter()
            .filter(|p| {
                let haystack =
                    format!("{} {} {} {}", p.name, vendor(p), role(p), p.version).to_lowercase();
                search
                    .split_whitespace()
                    .all(|term| haystack.contains(term))
                    && (self.role.is_empty() || self.role == p.role)
                    && (self.status.is_empty() || self.status == status(p).1)
            })
            .collect();
        products.sort_by_cached_key(|p| {
            (
                vendor(p).to_lowercase(),
                p.name.to_lowercase(),
                p.version.clone(),
                p.environment.clone(),
                p.class_id.clone(),
                p.module_sha256.clone(),
            )
        });
        products
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        pending: bool,
        chosen: &mut Option<Action>,
        details: impl Fn(&mut egui::Ui, &Product),
    ) {
        ui.heading("Your plug-ins");
        ui.add_space(2.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search name, vendor, type or version")
                .desired_width(f32::INFINITY),
        );
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("library-role")
                .selected_text(match self.role.as_str() {
                    "instrument" => "Instruments",
                    "effect" => "Effects",
                    _ => "All types",
                })
                .show_ui(ui, |ui| {
                    for (value, label) in [
                        ("", "All types"),
                        ("instrument", "Instruments"),
                        ("effect", "Effects"),
                    ] {
                        ui.selectable_value(&mut self.role, value.into(), label);
                    }
                });
            egui::ComboBox::from_id_salt("library-status")
                .selected_text(match self.status.as_str() {
                    "published" => "Published to Bitwig",
                    "unpublished" => "Not published",
                    "attention" => "Needs attention",
                    _ => "All statuses",
                })
                .show_ui(ui, |ui| {
                    for (value, label) in [
                        ("", "All statuses"),
                        ("published", "Published to Bitwig"),
                        ("unpublished", "Not published"),
                        ("attention", "Needs attention"),
                    ] {
                        ui.selectable_value(&mut self.status, value.into(), label);
                    }
                });
            if (!self.search.is_empty() || !self.role.is_empty() || !self.status.is_empty())
                && ui.button("Clear filters").clicked()
            {
                *self = Self::default();
            }
        });
        let products = self.products(snapshot);
        ui.small(format!(
            "{} of {} plug-ins · grouped by vendor",
            products.len(),
            snapshot.products.len()
        ));
        ui.small("Published plug-ins are registered for Bitwig. Rescan in Bitwig if a plug-in is missing from its browser.");
        if products.is_empty() {
            ui.add_space(16.0);
            ui.strong(if snapshot.products.is_empty() {
                "No plug-ins discovered yet"
            } else {
                "No matching plug-ins"
            });
            ui.label(if snapshot.products.is_empty() { "Open a vendor application or add a Windows installer below, then rescan installed products." } else { "Try another search or clear the filters." });
        }
        let mut previous_vendor = String::new();
        for p in products {
            let group = vendor(p).to_lowercase();
            if group != previous_vendor {
                ui.add_space(12.0);
                ui.strong(vendor(p));
                previous_vendor = group;
            }
            // Same class can have several installed builds or environments.
            ui.push_id((&p.environment, &p.class_id, &p.module_sha256), |ui| {
                egui::Frame::group(ui.style())
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(&p.name).size(19.0).strong());
                            let (label, category) = status(p);
                            let color = if category == "attention" {
                                warning_color(ui)
                            } else {
                                ui.visuals().text_color()
                            };
                            ui.label(egui::RichText::new(label).color(color));
                        });
                        ui.label(format!(
                            "{} · {}",
                            role(p),
                            if p.version.trim().is_empty() {
                                "Version unavailable"
                            } else {
                                &p.version
                            }
                        ));
                        let instances=instance_state(&snapshot.active_sessions,&p.class_id);
                        if instances.active>0 {
                            ui.small(format!("{} active instance(s) of this plug-in class",instances.active));
                        }
                        if instances.editor_failed>0 {
                            ui.colored_label(warning_color(ui),format!("{} instance(s): vendor editor/controller failed; the instance is unavailable",instances.editor_failed));
                        }
                        if instances.host_failed>0 {
                            ui.colored_label(warning_color(ui),format!("{} instance(s): Windows host exited unexpectedly; the instance is unavailable",instances.host_failed));
                        }
                        if instances.transport_failed>0 {
                            ui.colored_label(warning_color(ui),format!("{} instance(s): bridge transport failed; the instance is unavailable",instances.transport_failed));
                        }
                        if let Some(reason) = problem(p) {
                            ui.colored_label(warning_color(ui), reason);
                        }
                        let primary: Vec<_> = p
                            .actions
                            .iter()
                            .filter(|a| primary_action(&a.action))
                            .cloned()
                            .collect();
                        action_buttons(
                            ui,
                            &primary,
                            snapshot.system.inactive_reason(),
                            pending,
                            chosen,
                        );
                        let related = related_actions(p, snapshot);
                        action_buttons(
                            ui,
                            &related,
                            snapshot.system.inactive_reason(),
                            pending,
                            chosen,
                        );
                        egui::CollapsingHeader::new("Details and management")
                            .open(self.expand_details.then_some(true))
                            .show(ui, |ui| {
                                let management: Vec<_> = p
                                    .actions
                                    .iter()
                                    .filter(|a| !primary_action(&a.action))
                                    .cloned()
                                    .collect();
                                action_buttons(
                                    ui,
                                    &management,
                                    snapshot.system.inactive_reason(),
                                    pending,
                                    chosen,
                                );
                                details(ui, p);
                            });
                    });
            });
        }
    }
}

/// Shared with the local preview; controls and defaults are unchanged.
pub fn diagnostics(
    ui: &mut egui::Ui,
    s: &Snapshot,
    pending: bool,
    chosen: &mut Option<Action>,
    expanded: bool,
) {
    let busy = s.system.inactive_reason();
    egui::CollapsingHeader::new("System status and diagnostics").default_open(expanded).show(ui, |ui| {
                if !s.system.capacity_available() { ui.colored_label(egui::Color32::YELLOW,"Service capacity unavailable — DSP, keeper and cleanup status cannot be confirmed"); }
                else { ui.label(format!("Service: {}  ·  Keeper: {}  ·  DSP: {} / {}  ·  Pending: {}  ·  Stale transports: {}",s.system.service,s.system.keepers,s.system.dsp,s.system.ceiling,s.system.pending_transactions,s.system.stale_transports)); }
                if s.system.capacity_available() { ui.label(format!("Installing / scanning: {}",s.system.maintenance)); }
                if s.system.capacity_available() && s.system.cleanup_unconfirmed { ui.colored_label(egui::Color32::YELLOW,"Previous instance cleanup is unconfirmed — retained leases are not proof of a live DSP; new admission is blocked"); }
                ui.label("512 added frames recommended · 256 unqualified");
                ui.label(if s.capture["armed"]==true{"Crash capture: armed for next admitted launch"}else if s.capture["active_retention"].as_u64().unwrap_or(0)>0{"Crash capture: retaining an active instance"}else{"Crash capture: off"});
                if s.capture["armed"]==true { for action in &s.actions { if matches!(action.action,Action::CaptureDisarm{}) { action_buttons(ui,std::slice::from_ref(action),busy,pending,chosen); } } }
                if let Some(op)=&s.operation{egui::CollapsingHeader::new("Last operation receipt").show(ui,|ui|ui.add(egui::Label::new(egui::RichText::new(serde_json::to_string_pretty(op).unwrap_or_default()).monospace()).wrap()));}
                });
}

fn primary_action(action: &Action) -> bool {
    matches!(
        action,
        Action::PluginInspect { .. }
            | Action::PluginReinspect { .. }
            | Action::PluginPrepare { .. }
            | Action::QuarantinedModuleRetry { .. }
            | Action::ExperimentalEnable { .. }
    )
}

fn problem(product: &Product) -> Option<String> {
    let prep = &product.details["preparation"];
    let management = [
        product.details["refusal"].as_str(),
        product.details["preparation_failure"].as_str(),
        prep["operation"]["reason"].as_str(),
        prep["recovery"].as_str(),
    ]
    .into_iter()
    .flatten()
    .find(|s| !s.is_empty())
    .map(|s| s.replace('_', " "));
    management
    .or_else(|| product.details["inspection_error"].as_str()
        .filter(|s|!s.is_empty()).map(str::to_owned))
    .or_else(|| product.details["inspection_hint"].as_str()
        .filter(|s|!s.is_empty()).map(|s|s.replace('_', " ")))
    .or_else(|| {
        (status(product).1 == "attention")
            .then(|| product.limitations.first().map(|s| s.replace('_', " ")))
            .flatten()
    })
}

fn related_actions(product: &Product, snapshot: &Snapshot) -> Vec<AvailableAction> {
    if product.environment.is_empty() {
        return vec![];
    }
    let mut actions = vec![];
    // Association is supplied by the manager's exact environment identity, never vendor/name heuristics.
    for app in &snapshot.vendor_applications {
        if app.details["environment"].as_str() != Some(product.environment.as_str()) {
            continue;
        }
        actions.extend(
            app.actions
                .iter()
                .filter(|a| {
                    matches!(
                        a.action,
                        Action::VendorApplicationOpen { .. }
                            | Action::VendorApplicationFocus { .. }
                            | Action::RendererOpen { .. }
                            | Action::RendererFocus { .. }
                    )
                })
                .cloned(),
        );
    }
    for environment in &snapshot.environments {
        if environment.id == product.environment {
            actions.extend(
                environment
                    .actions
                    .iter()
                    .filter(|a| {
                        matches!(&a.action,
                Action::EnvironmentRescan { environment } if environment == &product.environment)
                    })
                    .cloned(),
            );
        }
    }
    actions
}

fn warning_color(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgb(245, 190, 100)
    } else {
        egui::Color32::from_rgb(140, 77, 0)
    }
}

#[derive(Debug,Default,PartialEq,Eq)]
struct InstanceState {active:usize,editor_failed:usize,host_failed:usize,transport_failed:usize}
fn instance_state(sessions:&[serde_json::Value],class_id:&str)->InstanceState {
    let mut result=InstanceState::default();
    for session in sessions.iter().filter(|row|row["class_id"]==class_id) {
        match session["terminal"].as_str() {
            Some("editor_controller_failed")=>result.editor_failed+=1,
            Some("windows_host_exited")=>result.host_failed+=1,
            Some("transport_failed")=>result.transport_failed+=1,
            _ if session["state"]=="active"=>result.active+=1,
            _=>{}
        }
    }
    result
}

pub fn action_buttons(
    ui: &mut egui::Ui,
    actions: &[AvailableAction],
    busy: Option<&str>,
    pending: bool,
    chosen: &mut Option<Action>,
) {
    if actions.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for a in actions {
            let reason = if a.action.requires_inactive() {
                busy.or(a.disabled_reason.as_deref())
            } else {
                a.disabled_reason.as_deref()
            };
            let response = ui.add_enabled(
                !pending && reason.is_none(),
                egui::Button::new(&a.label).min_size(egui::vec2(0.0, 42.0)),
            );
            if response.clicked() {
                *chosen = Some(a.action.clone());
            }
            if let Some(reason) =
                reason.or(pending.then_some("Waiting for the current request or manager readback"))
            {
                response.on_disabled_hover_text(reason);
            }
        }
    });
    // Keep refusal reasons accessible on touch screens as well as hover.
    let mut reasons = Vec::new();
    for a in actions {
        let reason = if pending {
            Some("Waiting for the current request or manager readback")
        } else if a.action.requires_inactive() {
            busy.or(a.disabled_reason.as_deref())
        } else {
            a.disabled_reason.as_deref()
        };
        if let Some(reason) = reason {
            if !reasons.contains(&reason) {
                reasons.push(reason);
            }
        }
    }
    for reason in reasons {
        ui.small(reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> Snapshot {
        serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap()
    }

    #[test]
    fn search_combines_terms_and_filters_without_losing_unknown_states() {
        let mut snapshot = snapshot();
        snapshot.products[0].disposition = "future_state".into();
        let mut library = Library::default();
        assert_eq!(library.products(&snapshot).len(), 3);
        assert_eq!(library.products(&snapshot)[0].vendor, "Arturia");
        library.search = "  INSTRUMENT   8.13 native ".into();
        assert_eq!(library.products(&snapshot)[0].name, "Kontakt 8");
        library.status = "attention".into();
        assert_eq!(library.products(&snapshot).len(), 1);
        library.role = "effect".into();
        assert!(library.products(&snapshot).is_empty());
    }

    #[test]
    fn installation_and_preparation_do_not_imply_publication() {
        let mut p = snapshot().products.remove(0);
        for state in ["prepared", "installed_unqualified"] {
            p.disposition = state.into();
            assert_eq!(status(&p).1, "unpublished");
        }
        p.disposition = "experimental".into();
        assert_eq!(status(&p).0, "Published · experimental");
        p.disposition = "another_configuration".into();
        assert_eq!(status(&p).1, "attention");
    }

    #[test]
    fn quarantined_module_shows_retained_scanner_error_before_generic_reason() {
        let mut p = snapshot().products.remove(0);
        p.disposition = "quarantined".into();
        p.details["inspection_error"] =
            serde_json::json!("TimeoutError: Windows call deadline: load_library");
        p.limitations = vec!["inventory_factory_absent_or_duplicate".into()];
        assert_eq!(
            problem(&p).as_deref(),
            Some("TimeoutError: Windows call deadline: load_library")
        );
    }

    #[test]
    fn related_actions_use_exact_environment_and_preserve_refusals() {
        let mut s = snapshot();
        s.vendor_applications[1].actions[0].disabled_reason =
            Some("Owned session is running".into());
        let actions = related_actions(&s.products[0], &s);
        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[0].action,
            s.vendor_applications[1].actions[0].action
        );
        assert_eq!(
            actions[0].disabled_reason.as_deref(),
            Some("Owned session is running")
        );
        s.products[0].environment = "different-installation-same-vendor".into();
        assert!(related_actions(&s.products[0], &s).is_empty());
        s.vendor_applications[1].details = serde_json::json!({});
        s.products[0].environment = "fixture-ni".into();
        assert_eq!(related_actions(&s.products[0], &s).len(), 1); // only the exact rescan
    }

    #[test]
    fn basic_instance_failures_are_visible_for_any_published_product() {
        let sessions=vec![
            serde_json::json!({"class_id":"blackhole","state":"failed","terminal":"editor_controller_failed"}),
            serde_json::json!({"class_id":"kontakt","state":"failed","terminal":"windows_host_exited"}),
            serde_json::json!({"class_id":"kontakt","state":"active","terminal":null}),
        ];
        assert_eq!(instance_state(&sessions,"blackhole"),InstanceState{
            editor_failed:1,..Default::default()});
        assert_eq!(instance_state(&sessions,"kontakt"),InstanceState{
            active:1,host_failed:1,..Default::default()});
    }
}
