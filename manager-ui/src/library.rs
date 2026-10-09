//! Presentation over the manager snapshot. No discovery, publication or launch authority.
use crate::model::{Action, AvailableAction, CompatibilityPhase, Product, Snapshot};
use crate::presentation::{self, ProductKey};
use eframe::egui;

#[derive(Default)]
pub struct Library {
    pub search: String,
    /// Used by the local preview; production preserves normal collapsed state.
    pub expand_details: bool,
    role: String,
    status: String,
    focus: Option<Vec<ProductKey>>,
    scroll_focus: bool,
}

pub fn role(product: &Product) -> &str {
    match product.role.as_str() {
        "instrument" => "Instrument",
        "effect" => "Effect",
        _ => "Plug-in",
    }
}

pub fn status(product: &Product) -> (&str, &str) {
    if let Some(workflow) = &product.compatibility {
        return match workflow.phase {
            CompatibilityPhase::NotChecked => ("Compatibility not checked", "unpublished"),
            CompatibilityPhase::CheckBlocked => ("Check unavailable", "attention"),
            CompatibilityPhase::Checking => ("Checking compatibility", "unpublished"),
            CompatibilityPhase::CheckFailed => ("Check needs attention", "attention"),
            CompatibilityPhase::ReadyForTest => ("Ready for one test", "unpublished"),
            CompatibilityPhase::AvailableForTest => ("Available experimentally", "published"),
            CompatibilityPhase::AwaitingRetirement => ("Problem recorded · awaiting retirement", "attention"),
            CompatibilityPhase::TestResultIncomplete => ("Finish test result", "attention"),
            CompatibilityPhase::PassedExperimental => ("Passed recorded checks · experimental", "published"),
            CompatibilityPhase::NeedsWork => ("Needs compatibility work", "attention"),
            CompatibilityPhase::OrdinarySupported => ("Linux VST3 published", "published"),
            CompatibilityPhase::PublicationNeedsAttention => ("Publication needs attention", "attention"),
            CompatibilityPhase::HistoricalOnly => ("Historical configuration", "attention"),
        };
    }
    match product.disposition.as_str() {
        "ready" => ("Published", "published"),
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
    pub fn focused_product(&self) -> Option<&ProductKey> {
        self.focus.as_deref().and_then(|keys| (keys.len() == 1).then_some(&keys[0]))
    }

    pub fn focus_product(&mut self, key: ProductKey) {
        self.search.clear();
        self.role.clear();
        self.status.clear();
        self.focus = Some(vec![key]);
        self.scroll_focus = true;
    }
    pub fn focus_products(&mut self, keys: Vec<ProductKey>) {
        self.search.clear();
        self.role.clear();
        self.status.clear();
        self.focus = Some(keys);
        self.scroll_focus = true;
    }

    fn products<'a>(&self, snapshot: &'a Snapshot) -> Vec<&'a Product> {
        if let Some(key) = &self.focus {
            return snapshot
                .products
                .iter()
                .filter(|product| key.contains(&ProductKey::from(*product)))
                .collect();
        }
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

    /// Current product identity remains visible while the separately requested
    /// compatibility/history readback loads. It carries no action authority.
    pub fn show_current(&mut self, ui: &mut egui::Ui, current: &Snapshot,
        fresh: bool, details_attempted: bool, retry_details: &mut bool) {
        if self.scroll_focus {
            ui.scroll_to_cursor(Some(egui::Align::Min));
            self.scroll_focus = false;
        }
        ui.heading("Plug-ins");
        if self.focus.is_some() {
            ui.strong(if self.focused_product().is_some() {
                "Showing the selected plug-in"
            } else {
                "Showing the selected plug-ins"
            });
            if ui.add_sized([180.0, 44.0], egui::Button::new("Show all plug-ins")).clicked() {
                self.focus = None;
            }
        }
        if !fresh {
            ui.colored_label(warning_color(ui),
                "Current product status is unavailable. Check again on Home before acting.");
        } else if details_attempted {
            ui.colored_label(warning_color(ui),
                "Compatibility controls could not be loaded. The product summary remains visible.");
            if ui.add_sized([240.0, 44.0],
                egui::Button::new("Try loading compatibility controls again")).clicked() {
                *retry_details = true;
            }
        } else {
            ui.label(if self.focused_product().is_some() {
                "Loading this plug-in's current compatibility controls…"
            } else {
                "Select a plug-in to see its current compatibility controls."
            });
        }
        ui.add(egui::TextEdit::singleline(&mut self.search)
            .hint_text("Search name, vendor, type or version")
            .desired_width(f32::INFINITY));
        if !self.search.is_empty() && ui.button("Clear search").clicked() {
            self.search.clear();
        }
        let products = self.products(current);
        ui.small(format!("{} of {} current plug-ins", products.len(), current.products.len()));
        if products.is_empty() {
            ui.label(if self.focus.is_some() {
                "The selected plug-in is absent from the current readback. Check again before continuing."
            } else if current.products.is_empty() {
                "No plug-ins discovered yet. Open Setup to add and scan an installer."
            } else {
                "No matching plug-ins. Clear the search to see all current products."
            });
        }
        let mut selected = None;
        for product in products {
            ui.push_id((&product.environment, &product.class_id, &product.module_sha256), |ui| {
                egui::Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
                    ui.label(egui::RichText::new(&product.name).size(19.0).strong());
                    ui.small(format!("{} · {} · {}", vendor(product), role(product),
                        if product.version.is_empty() { "Version unavailable" } else { &product.version }));
                    if fresh { ui.label(status(product).0); }
                    ui.small("Manager-offered compatibility actions appear after current controls load.");
                    // A Setup discovery may focus several exact classes. Each
                    // row must still let the user select one current detail.
                    if fresh && self.focused_product().is_none()
                        && ui.add_sized([240.0, 44.0],
                            egui::Button::new("View compatibility controls")).clicked() {
                        selected = Some(ProductKey::from(product));
                    }
                });
            });
        }
        if let Some(key) = selected { self.focus_product(key); }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        pending: bool,
        chosen: &mut Option<Action>,
        details: impl Fn(&mut egui::Ui, &Product),
    ) {
        if self.scroll_focus {
            ui.scroll_to_cursor(Some(egui::Align::Min));
            self.scroll_focus = false;
        }
        ui.heading("Plug-ins");
        if self.focus.is_some() {
            ui.horizontal_wrapped(|ui| {
                ui.strong(if self.focused_product().is_some() {
                    "Showing the selected plug-in"
                } else {
                    "Showing the selected plug-ins"
                });
                if ui
                    .add_sized([180.0, 42.0], egui::Button::new("Show all plug-ins"))
                    .clicked()
                {
                    self.focus = None;
                }
            });
        }
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
                    "published" => "Published",
                    "unpublished" => "Not published",
                    "attention" => "Needs attention",
                    _ => "All statuses",
                })
                .show_ui(ui, |ui| {
                    for (value, label) in [
                        ("", "All statuses"),
                        ("published", "Published"),
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
            ui.label(if snapshot.products.is_empty() { "Go to Setup to open a vendor application or add a Windows installer, then rescan installed products." } else { "Try another search or clear the filters." });
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
                        let related = related_actions(p, snapshot);
                        let primary = p.compatibility.as_ref().and_then(|workflow| workflow.primary.clone())
                            .or_else(|| p.compatibility.is_none().then(||
                                emphasized_action(p, &related, snapshot.system.inactive_reason())).flatten());
                        let management = ordinary_actions(p, &related, primary.as_ref());
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
                        if let Some(action) = &primary {
                            let reason = primary_refusal(action, snapshot.system.inactive_reason());
                            if let Some(reason) = reason {
                                ui.add_enabled(false, egui::Button::new(&action.label)
                                    .min_size(egui::vec2(240.0, 46.0)));
                                ui.small(reason);
                            } else { emphasized_button(ui, action, pending, chosen); }
                        }
                        if let Some(workflow) = &p.compatibility {
                            action_buttons(ui, &workflow.alternatives,
                                snapshot.system.inactive_reason(), pending, chosen);
                        }
                        action_buttons(ui, &management, snapshot.system.inactive_reason(), pending, chosen);
                        compatibility_settings(ui, p, snapshot.system.inactive_reason(), pending,
                            chosen, self.expand_details);
                        if let Some(target) = snapshot.environment_installers.iter()
                            .find(|target|target.environment == p.environment) {
                            egui::CollapsingHeader::new("Manage shared setup")
                                .default_open(target.actions.iter().any(|offer|
                                    matches!(offer.action,Action::EnvironmentInstallerStop { .. })))
                                .show(ui,|ui|environment_installer_controls(ui,target,pending,chosen));
                        }
                        ui.label(format!(
                            "{} · {}",
                            role(p),
                            if p.version.trim().is_empty() {
                                "Version unavailable"
                            } else {
                                &p.version
                            }
                        ));
                        if let Some(workflow) = &p.compatibility {
                            ui.label(&workflow.summary);
                            for established in &workflow.established {
                                ui.small(format!("✓ {established}"));
                            }
                            if !workflow.remaining.is_empty() {
                                ui.small(format!("Still untested: {}", workflow.remaining.join(", ")));
                            }
                        }
                        let instances = presentation::product_activity(snapshot, p);
                        ui.small(instances.detail_label());
                        for failure in &instances.failed_live {
                            ui.colored_label(
                                warning_color(ui),
                                format!(
                                    "{}live session: {failure}",
                                    if instances.shared_class {
                                        "Class-level "
                                    } else {
                                        ""
                                    }
                                ),
                            );
                        }
                        if let Some(failure) = instances.recent_failure {
                            ui.colored_label(
                                warning_color(ui),
                                format!(
                                    "{}most recent session: {failure}",
                                    if instances.shared_class {
                                        "Class-level "
                                    } else {
                                        ""
                                    }
                                ),
                            );
                            ui.small(if instances.recent_cleanup_unconfirmed {
                                "Cleanup remains unconfirmed."
                            } else {
                                "Cleanup and transport retirement confirmed."
                            });
                        }
                        if let Some(reason) =
                            presentation::product_issue(p).filter(|_| status(p).1 == "attention")
                        {
                            ui.colored_label(warning_color(ui), reason);
                        }
                        egui::CollapsingHeader::new(if p.compatibility.is_some() {
                            "Technical details and expert actions"
                        } else { "Details and manager actions" })
                            .open(self.expand_details.then_some(true))
                            .show(ui, |ui| {
                                let expert: Vec<_> = p
                                    .actions
                                    .iter()
                                    .chain(related.iter())
                                    .filter(|action| {
                                        !matches!(action.action, Action::BufferingSet { .. } | Action::DeliverySet { .. }
                                            | Action::CandidateGraphicsAssess { .. }
                                            | Action::CandidateGraphicsPrepare { .. }
                                            | Action::CandidateSettingsPrepare { .. })
                                            && !management.iter().any(|shown|shown.action==action.action)
                                            && !guided_action_shown(p, primary.as_ref(), &action.action)
                                    })
                                    .cloned()
                                    .collect();
                                action_buttons(
                                    ui,
                                    &expert,
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

/// Display manager-resolved facts and only its offered, typed controls.
fn compatibility_settings(ui: &mut egui::Ui, product: &Product, busy: Option<&str>,
    pending: bool, chosen: &mut Option<Action>, expanded: bool) {
    let configuration = &product.details["configuration"];
    let legacy = &product.details["graphics"];
    let actions = settings_actions(product);
    let buffering: Vec<_> = product.actions.iter().filter(|offer|
        matches!(offer.action, Action::BufferingSet { .. } | Action::DeliverySet { .. })).cloned().collect();
    if configuration.is_null() && legacy.is_null() && actions.is_empty() && buffering.is_empty() {
        return;
    }
    egui::CollapsingHeader::new("Compatibility settings").open(expanded.then_some(true)).show(ui, |ui| {
        let settings = if configuration.is_null() { legacy } else { configuration };
        if !configuration.is_null() {
            ui.strong("Requested choices");
            configuration_choices(ui, configuration);
            ui.strong("Prepared launch");
            for (key, label) in [("runtime", "Selected runtime"), ("environment", "Selected environment"),
                ("environment_revision", "Environment revision")] {
                ui.label(format!("{label}: {}", configuration_value(&configuration["launch"][key])));
            }
            egui::CollapsingHeader::new("Exact launch identity").show(ui, |ui| {
                for (key, label) in [("module", "Module"), ("class_id", "Class"),
                    ("host", "Windows host"), ("native", "Native bridge")] {
                    ui.small(format!("{label}: {}", configuration_value(&configuration["launch"][key])));
                }
                ui.small(format!("Candidate: {}", configuration_value(&configuration["candidate"])));
                ui.small(format!("Publication: {}", configuration_value(&configuration["publication"])));
            });
            ui.strong("Qualification");
            ui.label(configuration["qualification"].as_str().unwrap_or("Qualification unavailable"));
        } else if !legacy.is_null() {
            ui.label(legacy["requested"].as_str().unwrap_or("Requested settings unavailable"));
            ui.small(legacy["reason"].as_str().unwrap_or_default());
        }
        if settings["publication"] == "another_configuration" {
            ui.strong("Prepared only. Your previous configuration is still selected.");
        }
        ui.small("Preparing a trial retains your current selection. Use the offered trial, keep or restore controls to change it.");
        action_buttons(ui, &actions, busy, pending, chosen);
        if !configuration["buffering"].is_null() || !buffering.is_empty() {
            ui.separator();
            ui.strong("Bridge audio delivery");
            let audio = &configuration["buffering"];
            if let Some(frames) = audio["added_frames"].as_u64()
                .or_else(||product.details["added_frames"].as_u64()) {
                ui.label(format!("{frames} added bridge frames"));
                if audio["delivery_mode"] == "same_callback" || product.details["delivery_mode"] == "same_callback" {
                    ui.label("Same-callback delivery · experimental");
                    if let Some(remembered) = audio["remembered_frames"].as_u64().or_else(||product.details["buffered_frames"].as_u64()) {
                        ui.small(format!("Buffered delivery retains {remembered} frames for restoration."));
                    }
                } else { ui.label("Buffered delivery"); }
                ui.small(format!("Source: {}", configuration_value(&configuration["buffering"]["source"])));
            }
            ui.small("This is the bridge's added buffering. The DAW sets the live sample rate and processing block size; they are not measured here.");
            action_buttons(ui, &buffering, busy, pending, chosen);
        }
        if !settings["before"].is_null() {
            egui::CollapsingHeader::new("Before this trial").show(ui, |ui| {
                if !configuration.is_null() {
                    configuration_choices(ui, &settings["before"]);
                } else {
                    ui.label(settings["before"]["requested"].as_str().unwrap_or("Retained previous configuration"));
                }
                graphics_observation(ui, "Previous assessment", &settings["before"]["assessment"]);
            });
        }
        ui.separator();
        graphics_observation(ui, "Configuration assessment", &settings["assessment"]);
        ui.small("Assessment opens the editor in a supervised check. Independent runtime probes do not establish the editor's actual renderer, uninterrupted audio, saved-project recall or a known fix.");
    });
}

fn settings_actions(product: &Product) -> Vec<AvailableAction> {
    let shared = !product.details["configuration"].is_null();
    product.actions.iter().filter(|offer| match offer.action {
        Action::CandidateGraphicsAssess { .. } | Action::CandidateSettingsPrepare { .. } => true,
        Action::CandidateGraphicsPrepare { .. } => !shared,
        _ => false,
    }).cloned().collect()
}

fn configuration_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        _ => "Unavailable".into(),
    }
}

fn configuration_choices(ui: &mut egui::Ui, configuration: &serde_json::Value) {
    let Some(choices) = configuration["choices"].as_array().filter(|choices| !choices.is_empty()) else {
        ui.small("Requested choices unavailable.");
        return;
    };
    for choice in choices {
        ui.label(format!("{}: {}", configuration_value(&choice["name"]), configuration_value(&choice["value"])));
        ui.small(format!("Source: {} · Scope: {}", configuration_value(&choice["source"]), configuration_value(&choice["scope"])));
        ui.small(format!("Application: {}", configuration_value(&choice["change"])));
    }
}

fn graphics_observation(ui: &mut egui::Ui, label: &str, result: &serde_json::Value) {
    ui.strong(label);
    if result.is_null() { ui.small("Not assessed with these settings."); return; }
    ui.label(format!("Editor check: {}", result["editor"]["status"].as_str().unwrap_or("unknown")));
    if matches!(result["editor_retirement"].as_str(), Some("timed_out" | "failed")) {
        ui.colored_label(egui::Color32::YELLOW,
            "The editor did not close. Graphics findings were retained; the editor is not confirmed working.");
    }
    if let Some(reason) = result["recommendation"]["reason"].as_str() { ui.label(reason); }
    if result["applied"]["prepared"].is_string() {
        let setup = result["applied"]["setup"].as_u64()
            .map(|n| format!(" as setup {n}")).unwrap_or_default();
        ui.label(format!("Prepared: Wine's built-in Direct3D 11 is set up for this plug-in's next launch{setup}. Its Try offer below tests it in the DAW."));
    } else if let Some(action) = result["recommendation"]["action"].as_str() { ui.label(format!("Next: {action}")); }
    ui.small("Effective editor renderer: not established by this assessment.");
    if let Some(probes) = result["runtime_probes"].as_array() {
        // What the probe found loaded, whatever graphics setting was requested.
        match probes.iter().find(|probe| probe["api"] == "d3d11_default")
            .and_then(|probe| probe["implementation"].as_str()) {
            Some("wine_builtin") if result["context"]["requested_backend"].is_null() =>
                { ui.small("Direct3D 11 on this launch: Wine's built-in. The Wine D3D11 setting selects this same implementation."); }
            Some("wine_builtin") => { ui.small("Direct3D 11 on this launch: Wine's built-in."); }
            Some(_) => { ui.small("Direct3D 11 on this launch: the runner's own provider, not Wine's built-in."); }
            None => {}
        }
        ui.small("Independent runtime probes");
        for probe in probes {
            let renderer = probe["renderer"].as_str().unwrap_or("renderer unknown");
            // Wine's built-in Direct3D reports a fixed stand-in card.
            let note = if probe["implementation"] == "wine_builtin" && probe["renderer"].is_string() {
                " (Wine's stand-in adapter name, not the graphics device)" } else { "" };
            ui.small(format!("{}: {} · {renderer}{note}", probe["api"].as_str().unwrap_or("API"),
                probe["status"].as_str().unwrap_or("unknown")));
        }
    }
    ui.small("Recorded run; current driver/device freshness is unverified.");
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

/// Show only manager-offered actions. Visibility supplies no new authority;
/// disabled reasons, freshness and exact request validation still apply.
fn ordinary_actions(product:&Product, related:&[AvailableAction],
    primary:Option<&AvailableAction>) -> Vec<AvailableAction> {
    let installed_publication=product.active_revision.is_some()
        || matches!(product.disposition.as_str(),"ready"|"experimental"|"another_configuration");
    let mut actions=Vec::new();
    for offer in product.actions.iter().chain(related) {
        let show=match offer.action {
            Action::PluginReinspect{..}|Action::PluginPrepare{..} => installed_publication,
            Action::ExperimentalReplace{..}|Action::OrdinaryRollback{..}
                |Action::OrdinaryRestoreRecommended{..}|Action::ExperimentalDisable{..}
                |Action::CandidateWithdraw{..}|Action::QuarantinedModuleRetry{..}
                |Action::EnvironmentRescan{..}|Action::VendorApplicationFocus{..}
                |Action::VendorApplicationOpen{..}|Action::RendererOpen{..}
                |Action::RendererFocus{..} => true,
            _ => false,
        };
        if show && !guided_action_shown(product, primary, &offer.action)
            && !actions.iter().any(|shown:&AvailableAction|shown.action==offer.action) {
            actions.push(offer.clone());
        }
    }
    actions
}

fn guided_action_shown(product: &Product, primary: Option<&AvailableAction>, action: &Action) -> bool {
    primary.into_iter().chain(product.compatibility.iter()
        .flat_map(|workflow| workflow.alternatives.iter())).any(|shown| {
        shown.action == *action || matches!((action, &shown.action),
            (Action::ExperimentalReplace {candidate, expected_current},
                Action::CompatibilityPublishTest {candidate: shown_candidate,
                    expected_current: Some(shown_current)})
            if candidate == shown_candidate && expected_current == shown_current)
    })
}

fn routine_rank(action: &Action) -> Option<u8> {
    match action {
        Action::QuarantinedModuleRetry { .. } => Some(0),
        Action::PluginReinspect { .. } => Some(1),
        Action::PluginInspect { .. } => Some(2),
        Action::PluginPrepare { .. } => Some(3),
        Action::ExperimentalEnable { .. } => Some(4),
        Action::OrdinaryRestoreRecommended { .. } => Some(5),
        Action::EnvironmentRescan { .. } => Some(6),
        Action::VendorApplicationFocus { .. } | Action::RendererFocus { .. } => Some(7),
        Action::VendorApplicationOpen { .. } | Action::RendererOpen { .. } => Some(8),
        _ => None,
    }
}

fn emphasized_action(
    product: &Product,
    related: &[AvailableAction],
    busy: Option<&str>,
) -> Option<AvailableAction> {
    let published = matches!(product.disposition.as_str(), "ready" | "experimental");
    product
        .actions
        .iter()
        .chain(related)
        .filter(|offer| {
            offer.disabled_reason.is_none() && !(busy.is_some() && offer.action.requires_global_inactive())
        })
        .filter_map(|offer| {
            routine_rank(&offer.action).map(|rank| {
                // A published plug-in does not lead with housekeeping when its
                // manager-offered vendor application is available.
                let rank = if published {
                    match offer.action {
                        Action::VendorApplicationFocus { .. } | Action::RendererFocus { .. } => 5,
                        Action::VendorApplicationOpen { .. } | Action::RendererOpen { .. } => 6,
                        Action::EnvironmentRescan { .. } => 8,
                        _ => rank,
                    }
                } else {
                    rank
                };
                (rank, offer)
            })
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, offer)| offer.clone())
}

fn primary_refusal<'a>(action: &'a AvailableAction, busy: Option<&'a str>) -> Option<&'a str> {
    // Opening the result form must remain possible to report a problem while
    // the target is active. Successful submission uses its exact offered refusal.
    if matches!(action.action, Action::CompatibilityResult { .. }) { return None; }
    action.disabled_reason.as_deref().or_else(|| {
        action.action.requires_global_inactive().then_some(busy).flatten()
    })
}

fn emphasized_button(
    ui: &mut egui::Ui,
    offer: &AvailableAction,
    pending: bool,
    chosen: &mut Option<Action>,
) {
    let button = egui::Button::new(egui::RichText::new(&offer.label).strong())
        .fill(ui.visuals().selection.bg_fill)
        .min_size(egui::vec2(0.0, 46.0));
    if ui.add_enabled(!pending, button).clicked() {
        *chosen = Some(offer.action.clone());
    }
    if pending {
        ui.small("Waiting for the current request or manager readback");
    }
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
            let reason = primary_refusal(a, busy);
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
    // Keep every refusal visible to touch users, even when global inactivity
    // also blocks a manager-disabled action.
    for reason in visible_refusals(actions, busy, pending) {
        ui.small(reason);
    }
}

fn visible_refusals(actions: &[AvailableAction], busy: Option<&str>, pending: bool) -> Vec<String> {
    let mut reasons = Vec::new();
    let mut add = |reason: &str| {
        if !reasons.iter().any(|existing| existing == reason) {
            reasons.push(reason.to_owned());
        }
    };
    for offer in actions {
        if pending {
            add("Waiting for the current request or manager readback");
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

pub(crate) fn environment_installer_controls(ui: &mut egui::Ui,
    target: &crate::model::EnvironmentInstaller, pending: bool, chosen: &mut Option<Action>) {
    let controls:Vec<_>=target.actions.iter().filter(|offer|matches!(offer.action,
        Action::EnvironmentInstallerFocus { .. } | Action::EnvironmentInstallerStop { .. })).cloned().collect();
    crate::library::action_buttons(ui,&controls,None,pending,chosen);
    ui.label(&target.consequence);
    if target.affected.is_empty() {
        ui.small("This existing setup has no discovered plug-ins yet.");
    } else {
        ui.strong(format!("Affected plug-ins: {}", target.affected.join(", ")));
    }
    ui.small(format!("Compatibility space: {}",target.environment));
    if let Some(runtime)=&target.runtime {
        egui::CollapsingHeader::new("Compatibility runtime").show(ui,|ui| {
            ui.label(format!("Current runtime: {} · setup revision {}",runtime.runner,runtime.revision));
            ui.label(&runtime.consequence);
            ui.strong("Affected plug-ins and applications");
            for owner in &runtime.affected {ui.label(owner);}
            let key=egui::Id::new(("environment_runtime",&target.environment));
            let mut selected=ui.data_mut(|data|data.get_temp::<String>(key)).unwrap_or_default();
            egui::ComboBox::from_id_salt(("environment_runtime_choice",&target.environment))
                .selected_text(runtime.choices.iter().find(|offer|matches!(&offer.action,
                    Action::EnvironmentRuntimeSelect {runner,..} if runner==&selected))
                    .map(|offer|offer.label.as_str()).unwrap_or("Choose an installed runtime"))
                .show_ui(ui,|ui|for offer in &runtime.choices {
                    if let Action::EnvironmentRuntimeSelect {runner,..}=&offer.action {
                        ui.selectable_value(&mut selected,runner.clone(),&offer.label);
                    }
                });
            ui.data_mut(|data|data.insert_temp(key,selected.clone()));
            if let Some(offer)=runtime.choices.iter().find(|offer|matches!(&offer.action,
                Action::EnvironmentRuntimeSelect {runner,..} if runner==&selected)) {
                action_buttons(ui,std::slice::from_ref(offer),None,pending,chosen);
            }
            if runtime.choices.is_empty() {ui.small("Install another compatibility runtime through Setup to try it here.");}
        });
    }
    if target.operation.is_some() {
        let state = target.result["state"].as_str().unwrap_or("unavailable").replace('_'," ");
        ui.label(format!("Last installer: {state}"));
        if target.result["transaction"]["durable_installation"].is_string() {
            ui.label(presentation::installer_installation_label(&target.result["transaction"]));
        }
        if target.result["state"] == "failed" || target.result["state"] == "cancelled" {
            ui.colored_label(warning_color(ui),"Files or vendor state may already have changed. Rescan to see what remains, or retry in this same compatibility space. Stopping does not restore prior vendor state.");
        }
        egui::CollapsingHeader::new("Installer details").show(ui,|ui| {
            for line in presentation::installer_lines(&target.result) {ui.small(line);}
        });
    }
    let continuation:Vec<_>=target.actions.iter().filter(|offer|!matches!(offer.action,
        Action::EnvironmentInstallerFocus { .. } | Action::EnvironmentInstallerStop { .. })).cloned().collect();
    crate::library::action_buttons(ui,&continuation,None,pending,chosen);
    let key = egui::Id::new(("companion_installer",&target.environment));
    let mut selected = ui.data_mut(|data|data.get_temp::<String>(key)).unwrap_or_default();
    egui::ComboBox::from_id_salt(("companion_choice",&target.environment))
        .selected_text(target.choices.iter().find(|offer|matches!(&offer.action,
            Action::EnvironmentInstallerStart {installer,..} if installer == &selected))
            .map(|offer|offer.label.as_str()).unwrap_or("Choose an imported companion installer"))
        .show_ui(ui,|ui| {
            for offer in &target.choices {
                if let Action::EnvironmentInstallerStart {installer,..} = &offer.action {
                    ui.selectable_value(&mut selected,installer.clone(),&offer.label);
                }
            }
        });
    ui.data_mut(|data|data.insert_temp(key,selected.clone()));
    if let Some(offer) = target.choices.iter().find(|offer|matches!(&offer.action,
        Action::EnvironmentInstallerStart {installer,..} if installer == &selected)) {
        crate::library::action_buttons(ui,std::slice::from_ref(offer),None,pending,chosen);
    }
    if target.choices.is_empty() {ui.small("Choose a Windows installer to import a local companion or dependency first.");}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CompatibilityWorkflow, CompatibilityPhase};
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
                compatibility_settings(ui,&product,None,false,&mut None,true);
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape,&mut labels); }
            output.textures_delta.clear();
            for expected in ["Compatibility settings", "Graphics: Wine D3D11 fallback", "Accessibility: Windows default",
                "Source: Explicit local choice", "Selected runtime: selected-runtime-r7",
                "Selected environment: existing-environment", "Environment revision: 4", "Not yet qualified",
                "512 added bridge frames", "The DAW sets the live sample rate", "Prepared only.",
                "Effective editor renderer: not established", "Independent runtime probes", "Probe renderer only",
                "Direct3D 11 on this launch: Wine's built-in", "Wine's stand-in adapter name",
                "Prepare compatibility trial", "Assessment unavailable while this class is active"] {
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
                compatibility_settings(ui,&same_callback,None,false,&mut None,true);
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
                egui::epaint::Shape::Text(text) if text.galley.text() == "Prepare settings trial" =>
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
            let ctx = egui::Context::default();
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            for pressed in [None, Some(true), Some(false)] {
                let events = pressed.map(|pressed| vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output = ctx.run_ui(egui::RawInput {events,..Default::default()}, |ui| {
                    compatibility_settings(ui,&product,Some("Unrelated instances active"),false,&mut chosen,true);
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
            let mut library = Library::default();
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                library.show(ui, &snapshot, false, &mut chosen, |_, _| {});
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
            output.textures_delta.clear();
            for expected in ["Compatibility not checked", "Check compatibility",
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
        assert_eq!(status(product).0, "Problem recorded · awaiting retirement");
        assert_eq!(primary_refusal(product.compatibility.as_ref().unwrap().primary.as_ref().unwrap(),
            Some("Cleanup unconfirmed")), Some("Cleanup unconfirmed"));
    }

    #[test]
    fn existing_guided_product_exposes_update_and_rollback_without_expert_panel() {
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
        for width in [960.0,560.0] {
            let ctx=egui::Context::default();let mut library=Library::default();let mut chosen=None;
            let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
                ui.set_max_width(width-32.0);
                library.show(ui,&snapshot,false,&mut chosen,|_,_|{});
            });
            let mut labels=vec![];for clipped in &output.shapes {texts(&clipped.shape,&mut labels);}
            output.textures_delta.clear();
            for expected in ["Check compatibility again","Prepare a test bridge update",
                "Roll back to revision 6","Installed module changed; rescan first"] {
                assert!(labels.iter().any(|s|s.contains(expected)),"{expected} absent at {width}");
            }
            assert!(!library.expand_details);assert!(chosen.is_none());
        }
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
            let mut library = Library {expand_details:expanded,..Default::default()};
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                library.show(ui, &snapshot, false, &mut chosen, |_,_| {});
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
            output.textures_delta.clear();
            for offer in [&keep,&restore,&ready,&stale] {
                assert_eq!(labels.iter().filter(|label| **label == offer.label).count(), 1,
                    "{} must render once at {width}, expanded={expanded}", offer.label);
            }
            assert!(labels.iter().any(|label| Some(label) == stale.disabled_reason.as_ref()));
            assert_eq!(labels.iter().filter(|label| **label == expert.label).count(), usize::from(expanded),
                "expert section must actually be open only when requested");
            assert_eq!(library.expand_details, expanded);
            assert!(chosen.is_none());
        }
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
    fn current_product_selection_uses_exact_identity() {
        let snapshot = snapshot();
        let exact = ProductKey::from(&snapshot.products[0]);
        let other = ProductKey::from(&snapshot.products[1]);
        let mut library = Library::default();
        assert_eq!(library.focused_product(), None);
        library.focus_products(vec![exact.clone(), other]);
        assert_eq!(library.focused_product(), None);
        library.focus_product(exact.clone());
        assert_eq!(library.focused_product(), Some(&exact));
        assert_eq!(library.products(&snapshot).len(), 1);
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
        let mut library = Library::default();
        library.focus_products(vec![first.clone(), second.clone()]);
        assert!(library.focused_product().is_none());
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            library.show_current(ui, &snapshot, true, false, &mut false);
        });
        let mut labels = vec![];
        for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
        output.textures_delta.clear();
        assert_eq!(labels.iter().filter(|line| line.as_str() == "View compatibility controls").count(), 2);
        library.focus_product(second.clone());
        assert_eq!(library.focused_product(), Some(&second));
        assert_eq!(library.products(&snapshot).len(), 1);
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
            presentation::product_issue(&p).as_deref(),
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
    fn one_enabled_routine_action_is_emphasized_without_changing_offers() {
        let mut s = snapshot();
        let related = related_actions(&s.products[2], &s);
        let product = &mut s.products[2];
        let retry = AvailableAction {
            label: "Retry exact scan".into(),
            action: Action::PluginReinspect {
                selection: "exact".into(),
                audio_layout: None,
            },
            disabled_reason: Some("Current manager work blocks this scan".into()),
        };
        product.actions.push(retry.clone());
        assert!(emphasized_action(product, &related, Some("Busy")).is_none());
        assert_eq!(product.actions.len(), 1);
        assert_eq!(product.actions[0].action, retry.action);
        product.actions[0].disabled_reason = None;
        let selected = emphasized_action(product, &related, None).unwrap();
        assert_eq!(selected.action, retry.action);
        assert_eq!(product.actions.len(), 1);
        assert_eq!(selected.disabled_reason, None);
    }

    #[test]
    fn published_product_prefers_offered_vendor_application_over_rescan() {
        let s = snapshot();
        let product = &s.products[1];
        assert_eq!(product.disposition, "ready");
        let related = related_actions(product, &s);
        let chosen = emphasized_action(product, &related, None).unwrap();
        assert!(matches!(
            chosen.action,
            Action::VendorApplicationOpen { .. } | Action::VendorApplicationFocus { .. }
        ));
        assert!(related
            .iter()
            .any(|offer| matches!(offer.action, Action::EnvironmentRescan { .. })));
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
                let mut library = Library::default();
                let mut point = egui::Pos2::ZERO;
                let mut chosen = None;
                for pressed in [None,Some(true),Some(false)] {
                    let events = pressed.map(|pressed| vec![egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                            pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                    let mut output = ctx.run_ui(egui::RawInput {events,..Default::default()},|ui| {
                        ui.set_max_width(width-32.0);
                        library.show(ui,&s,false,&mut chosen,|_,_|{});
                    });
                    if pressed.is_none() {
                        point = output.shapes.iter().find_map(|shape|position(&shape.shape))
                            .expect("discovery must be visible with details collapsed");
                    }
                    output.textures_delta.clear();
                }
                assert!(!library.expand_details);
                assert_eq!(chosen,clickable.then(||action.clone()));
            }
        }
    }

    #[test]
    fn focused_product_is_selected_by_exact_identity_even_after_search() {
        let mut s = snapshot();
        let mut library = Library {
            search: "other".into(),
            ..Default::default()
        };
        let key = ProductKey::from(&s.products[2]);
        library.focus_product(key);
        assert_eq!(library.products(&s).len(), 1);
        assert_eq!(library.products(&s)[0].name, "Serum 2 FX");
        assert!(library.scroll_focus);
        let mut quarantined = s.products[0].clone();
        quarantined.name = "Quarantined module".into();
        quarantined.class_id.clear();
        s.products.push(quarantined.clone());
        library.focus_product(ProductKey::from(&quarantined));
        assert_eq!(library.products(&s).len(), 1);
        assert_eq!(library.products(&s)[0].name, "Quarantined module");
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
            (Action::EnvironmentRescan {environment:"environment".into()},None,false),
        ] {
            let offer = AvailableAction {label:"Selected action".into(),action:action.clone(),disabled_reason:refusal.map(str::to_owned)};
            let busy = Some("Another class is active");
            assert_eq!(primary_refusal(&offer,busy).is_none(),clickable);
            if !action.requires_global_inactive() {
                assert!(!visible_refusals(std::slice::from_ref(&offer),busy,false)
                    .iter().any(|reason| reason == "Another class is active"));
            }
            let ctx = egui::Context::default();
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            for pressed in [None,Some(true),Some(false)] {
                let events = pressed.map(|pressed| vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output = ctx.run_ui(egui::RawInput {events,..Default::default()},|ui| {
                    action_buttons(ui,std::slice::from_ref(&offer),busy,false,&mut chosen);
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
