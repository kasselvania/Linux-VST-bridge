//! Presentation over the manager snapshot. No discovery, publication or launch authority.
use crate::model::{Action, AvailableAction, CompatibilityPhase, Product, Snapshot};
use crate::presentation::{self, ProductKey, setup_choice_label};
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
    if product.availability.is_none() {
        return ("DAW connection status unavailable", "attention");
    }
    if product.availability.as_ref().is_some_and(|availability| availability.available) {
        return ("Available in your DAW", "published");
    }
    // Removed on purpose: not a fault, and not yet-to-be-installed either.
    if product.disposition == "removed_from_daw" {
        return ("Removed from your DAW", "unpublished");
    }
    if product.availability.as_ref().is_some_and(|availability|
        !availability.available && availability.reason.is_some()) {
        return ("DAW connection needs attention", "attention");
    }
    match product.disposition.as_str() {
        "ready" | "experimental" => ("DAW connection unavailable", "attention"),
        "another_configuration" => ("Another setup is available in your DAW", "attention"),
        "prepared" => ("Prepared · not available in your DAW yet", "unpublished"),
        "installed_unqualified" => ("Installed · not available in your DAW yet", "unpublished"),
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
                "Current plug-in status could not be loaded. Refresh on Home before continuing.");
        } else if details_attempted {
            ui.colored_label(warning_color(ui),
                "Plug-in controls could not be loaded. The last plug-in summary is shown below.");
            if ui.add_sized([240.0, 44.0],
                egui::Button::new("Load plug-in controls again")).clicked() {
                *retry_details = true;
            }
        } else {
            ui.label(if self.focused_product().is_some() {
                "Loading this plug-in's setup and settings…"
            } else {
                "Open a plug-in to manage its setup, updates and settings."
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
                "The selected plug-in is missing from the current list. Refresh before continuing."
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
                    if fresh { ui.label(presentation::installation_progress(current, product)
                        .unwrap_or_else(|| status(product).0)); }
                    if let Some(refused) = presentation::last_load_text(product) {
                        ui.colored_label(warning_color(ui), refused);
                    }
                    if let Some(problem) = presentation::problem_text(&product.details["problem"]) {
                        ui.colored_label(warning_color(ui), problem);
                    } else {
                        ui.small("Setup actions and settings appear after the current controls load.");
                    }
                    // A Setup discovery may focus several exact classes. Each
                    // row must still let the user select one current detail.
                    if self.focused_product().is_none()
                        && ui.add_sized([240.0, 44.0],
                            egui::Button::new("Manage plug-in")).clicked() {
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
        let single_product = self.focused_product().is_some();
        ui.horizontal_wrapped(|ui| {
            ui.heading("Plug-ins");
            if self.focus.is_some() && ui.add_sized([180.0, 44.0],
                egui::Button::new("Show all plug-ins")).clicked() {self.focus = None;}
        });
        if !single_product {
            ui.add(egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search name, vendor, type or version")
                .desired_width(f32::INFINITY));
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("library-role")
                    .selected_text(match self.role.as_str() {
                        "instrument" => "Instruments", "effect" => "Effects", _ => "All types",
                    }).show_ui(ui, |ui| {
                    for (value,label) in [("","All types"),("instrument","Instruments"),("effect","Effects")] {
                        ui.selectable_value(&mut self.role,value.into(),label);
                    }
                });
                egui::ComboBox::from_id_salt("library-status")
                    .selected_text(match self.status.as_str() {
                        "published" => "Available in DAW", "unpublished" => "Not available yet",
                        "attention" => "Needs attention", _ => "All statuses",
                    }).show_ui(ui, |ui| {
                    for (value,label) in [("","All statuses"),("published","Available in DAW"),
                        ("unpublished","Not available yet"),("attention","Needs attention")] {
                        ui.selectable_value(&mut self.status,value.into(),label);
                    }
                });
                if (!self.search.is_empty() || !self.role.is_empty() || !self.status.is_empty())
                    && ui.button("Clear filters").clicked() {*self = Self::default();}
            });
        }
        let products = self.products(snapshot);
        if !single_product {
            ui.small(format!("{} of {} plug-ins · grouped by vendor",products.len(),snapshot.products.len()));
            ui.small("Available plug-ins appear as native VST3s. Rescan VST3 plug-ins in your DAW if one is missing.");
        }
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
            if !single_product && group != previous_vendor {
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
                        let progress = presentation::installation_progress(snapshot, p);
                        let primary = p.compatibility.as_ref().and_then(|workflow| workflow.primary.clone())
                            .or_else(|| p.compatibility.is_none().then(||
                                emphasized_action(p, &related, snapshot.system.inactive_reason())).flatten())
                            .map(|mut offer| {
                                if recovery_pending(p) && matches!(offer.action, Action::CompatibilityFinishResult { .. }) {
                                    offer.label="Finish plug-in recovery".into();
                                }
                                offer
                            });
                        let optional_result = primary.as_ref().filter(|offer| optional_test_result(p, &offer.action));
                        let optional_preparation = primary.as_ref().filter(|offer|
                            (status(p).1 == "published" || progress.is_some()) && other_setup_option(&offer.action));
                        let everyday_primary = primary.clone().filter(|_| optional_result.is_none()
                            && optional_preparation.is_none());
                        let management = ordinary_actions(p, &related, primary.as_ref());
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(&p.name).size(19.0).strong());
                            let (label, category) = progress.map(|label| (label, "installing")).unwrap_or_else(|| status(p));
                            let color = if category == "attention" {
                                warning_color(ui)
                            } else {
                                ui.visuals().text_color()
                            };
                            ui.label(egui::RichText::new(label).color(color));
                        });
                        ui.small(format!("{} · {}", role(p), if p.version.trim().is_empty() {
                            "Version unavailable" } else { &p.version }));
                        if recovery_pending(p) {
                            if let Some(workflow) = &p.compatibility {
                                ui.colored_label(warning_color(ui), &workflow.summary);
                            }
                        } else if status(p).1 == "published" {
                            ui.label(presentation::daw_guidance(presentation::product_daw(snapshot, p)));
                            if let Some(workflow) = p.compatibility.as_ref().filter(|workflow|
                                workflow.phase == CompatibilityPhase::NeedsWork) {
                                ui.colored_label(warning_color(ui), &workflow.summary);
                            }
                        } else if p.disposition == "removed_from_daw" {
                            if let Some(reason) = p.availability.as_ref().and_then(|a| a.reason.as_ref()) {
                                ui.label(reason);
                            }
                        } else if let Some(workflow) = p.compatibility.as_ref().filter(|_| progress.is_none()) {
                            if workflow.phase == CompatibilityPhase::ReadyForTest {
                                ui.label("The native bridge is prepared. Make it available below, then choose it in your DAW.");
                            } else { ui.label(&workflow.summary); }
                        }
                        let instances = presentation::product_activity(snapshot, p);
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
                            presentation::product_issue(p).filter(|_| progress.is_none() && status(p).1 == "attention")
                        {
                            ui.colored_label(warning_color(ui), reason);
                        }
                        if let Some(refused) = presentation::last_load_text(p) {
                            ui.colored_label(warning_color(ui), refused);
                        }
                        if let Some(action) = &everyday_primary {
                            if let Some(consequence) = presentation::vendor_install_consequence(snapshot, &action.action) {
                                ui.label(consequence);
                            }
                            let reason = primary_refusal(action, snapshot.system.inactive_reason());
                            if let Some(reason) = reason {
                                ui.add_enabled(false, egui::Button::new(presentation::action_label(action))
                                    .min_size(egui::vec2(240.0, 46.0)));
                                ui.small(reason);
                                if let Some(consequence) = presentation::action_consequence(&action.action) { ui.small(consequence); }
                            } else { emphasized_button(ui, action, pending, chosen); }
                        }
                        let offers = management.iter().chain(p.compatibility.iter()
                            .flat_map(|workflow|workflow.alternatives.iter())).chain(optional_preparation);
                        let (previous, current): (Vec<_>, Vec<_>) = offers.cloned()
                            .partition(|offer| matches!(offer.action,
                                Action::OrdinaryRollback { .. } | Action::OrdinaryRestoreRecommended { .. }));
                        let (availability, current): (Vec<_>, Vec<_>) = current.into_iter()
                            .partition(|offer| matches!(offer.action, Action::ExperimentalDisable { .. } | Action::CandidateWithdraw { .. }));
                        let (options, current): (Vec<_>, Vec<_>) = current.into_iter()
                            .filter(|offer| !everyday_primary.as_ref().is_some_and(|shown| shown.action == offer.action))
                            .partition(|offer| other_setup_option(&offer.action));
                        if let Some(consequence) = current.iter().find_map(|offer|
                            presentation::vendor_install_consequence(snapshot, &offer.action)) {
                            ui.label(consequence);
                        }
                        action_buttons(ui, &current, snapshot.system.inactive_reason(), pending, chosen);
                        compatibility_settings(ui, p, snapshot.system.inactive_reason(), pending,
                            chosen, self.expand_details);
                        if p.compatibility.is_some() || settings_actions(p).iter().any(|offer|
                            matches!(offer.action, Action::CandidateGraphicsAssess { .. })) {
                            egui::CollapsingHeader::new("Help and testing")
                                .open(self.expand_details.then_some(true)).show(ui, |ui| {
                                    if let Some(offer) = optional_result {
                                        action_buttons(ui, std::slice::from_ref(offer), snapshot.system.inactive_reason(), pending, chosen);
                                    }
                                    let editor_checks: Vec<_> = settings_actions(p).into_iter()
                                        .filter(|offer| matches!(offer.action, Action::CandidateGraphicsAssess { .. })).collect();
                                    action_buttons(ui, &editor_checks, snapshot.system.inactive_reason(), pending, chosen);
                                    if let Some(workflow) = &p.compatibility {
                                        if workflow.phase == CompatibilityPhase::AvailableForTest {
                                            ui.label("Available in your DAW using this setup. You can record a result after trying it.");
                                        } else { ui.label(&workflow.summary); }
                                        for established in &workflow.established { ui.small(format!("Observed: {established}")); }
                                        if !workflow.remaining.is_empty() {
                                            ui.small(format!("Not yet checked: {}", workflow.remaining.iter().map(|area| presentation::check_area_label(area)).collect::<Vec<_>>().join(", ")));
                                        }
                                    }
                                    ui.small("Recorded checks describe only what was observed with that setup. They do not replace your vendor's authorization or your DAW's project save and reopen.");
                                    egui::CollapsingHeader::new("Editor check details").open(self.expand_details.then_some(true)).show(ui, |ui| {
                                        let configuration = &p.details["configuration"];
                                        let settings = if configuration.is_null() { &p.details["graphics"] } else { configuration };
                                        graphics_observation(ui, "Configuration assessment", &settings["assessment"]);
                                        ui.small("This supervised editor check does not establish uninterrupted audio, saved-project recall or a known fix.");
                                    });
                                });
                        }
                        if !availability.is_empty() {
                            egui::CollapsingHeader::new("DAW availability")
                                .open(self.expand_details.then_some(true)).show(ui, |ui| {
                                    action_buttons(ui, &availability, snapshot.system.inactive_reason(), pending, chosen);
                                });
                        }
                        if !options.is_empty() {
                            egui::CollapsingHeader::new(format!("Other setup options ({})", options.len()))
                                .id_salt("other-setup-options")
                                .open(self.expand_details.then_some(true)).show(ui,|ui| {
                                    ui.label("Prepare an alternative or select a retained setup. Preparing one keeps your current DAW selection.");
                                    for offer in &options {
                                        setup_choice(ui, p, offer, snapshot.system.inactive_reason(), pending, chosen);
                                    }
                                });
                        }
                        if !previous.is_empty() {
                            egui::CollapsingHeader::new(format!("Previous setups ({})", previous.len()))
                                .id_salt("previous-setups")
                                .open(self.expand_details.then_some(true))
                                .show(ui, |ui| {
                                    for offer in &previous {
                                        setup_choice(ui, p, offer, snapshot.system.inactive_reason(), pending, chosen);
                                    }
                                });
                        }
                        if let Some(target) = snapshot.environment_installers.iter()
                            .find(|target|target.environment == p.environment) {
                            egui::CollapsingHeader::new("Manage shared setup")
                                .default_open(target.actions.iter().any(|offer|
                                    matches!(offer.action,Action::EnvironmentInstallerStop { .. })))
                                .show(ui,|ui|environment_installer_controls(ui,target,pending,chosen));
                        }
                        egui::CollapsingHeader::new(if p.compatibility.is_some() {
                            "Technical details and expert actions"
                        } else { "Technical details and advanced actions" })
                            .open(self.expand_details.then_some(true))
                            .show(ui, |ui| {
                                ui.small(instances.detail_label());
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
                                if let Some(workflow) = &p.compatibility {
                                    ui.label(&workflow.summary);
                                    for established in &workflow.established { ui.small(format!("Observed: {established}")); }
                                }
                                details(ui, p);
                            });
                    });
            });
        }
    }
}

pub(crate) fn recovery_pending(product: &Product) -> bool {
    product.compatibility.as_ref().is_some_and(|workflow| matches!(workflow.phase,
        CompatibilityPhase::AwaitingRetirement | CompatibilityPhase::PublicationNeedsAttention))
}

fn optional_test_result(product: &Product, action: &Action) -> bool {
    matches!(action, Action::CompatibilityResult { .. })
        || (matches!(action, Action::CompatibilityFinishResult { .. }) && !recovery_pending(product))
}

fn setup_choice(ui: &mut egui::Ui, product: &Product, offer: &AvailableAction,
    busy: Option<&str>, pending: bool, chosen: &mut Option<Action>) {
    ui.push_id(serde_json::to_string(&offer.action).unwrap_or_default(), |ui| {
        let label = setup_choice_label(product, offer);
        // Context-specific labels already describe the typed action; do not map them a second time.
        let reason = primary_refusal(offer, busy);
        if ui.add_enabled(!pending && reason.is_none(), egui::Button::new(label)
            .min_size(egui::vec2(0.0, 44.0))).clicked() { *chosen = Some(offer.action.clone()); }
        for refusal in visible_refusals(std::slice::from_ref(offer), busy, pending) { ui.small(refusal); }
        if let Action::OrdinaryRollback { publication, .. } = &offer.action {
            if let Some(record) = product.history.iter().find(|record| record.publication == *publication) {
                if !record.description.is_empty() { ui.small(&record.description); }
            }
        }
        if let Some(consequence) = presentation::action_consequence(&offer.action) { ui.small(consequence); }
        egui::CollapsingHeader::new("Exact setup identity").show(ui, |ui| {
            ui.small(format!("Manager action: {}", offer.label));
            ui.small(serde_json::to_string(&offer.action).unwrap_or_default());
        });
    });
}

/// Display manager-resolved facts and only its offered, typed controls.
fn compatibility_settings(ui: &mut egui::Ui, product: &Product, busy: Option<&str>,
    pending: bool, chosen: &mut Option<Action>, expanded: bool) {
    let configuration = &product.details["configuration"];
    let legacy = &product.details["graphics"];
    let actions: Vec<_> = settings_actions(product).into_iter()
        .filter(|offer| !matches!(offer.action, Action::CandidateGraphicsAssess { .. })).collect();
    let buffering: Vec<_> = product.actions.iter().filter(|offer|
        matches!(offer.action, Action::BufferingSet { .. } | Action::DeliverySet { .. })).cloned().collect();
    if configuration.is_null() && legacy.is_null() && actions.is_empty() && buffering.is_empty() {
        return;
    }
    egui::CollapsingHeader::new("Settings").open(expanded.then_some(true)).show(ui, |ui| {
        let settings = if configuration.is_null() { legacy } else { configuration };
        if !configuration.is_null() {
            ui.strong("Launch settings");
            configuration_choices(ui, configuration, expanded);
            egui::CollapsingHeader::new("Technical setup details").open(expanded.then_some(true)).show(ui, |ui| {
                for (key, label) in [("runtime", "Selected runtime"), ("environment", "Selected environment"),
                    ("environment_revision", "Environment revision")] {
                    ui.small(format!("{label}: {}", configuration_value(&configuration["launch"][key])));
                }
                for (key, label) in [("module", "Module"), ("class_id", "Class"),
                    ("host", "Windows host"), ("native", "Native bridge")] {
                    ui.small(format!("{label}: {}", configuration_value(&configuration["launch"][key])));
                }
                ui.small(format!("Candidate: {}", configuration_value(&configuration["candidate"])));
                ui.small(format!("Publication: {}", configuration_value(&configuration["publication"])));
                ui.label(configuration["qualification"].as_str().unwrap_or("Qualification unavailable"));
            });
        } else if !legacy.is_null() {
            ui.label(legacy["requested"].as_str().unwrap_or("Requested settings unavailable"));
            ui.small(legacy["reason"].as_str().unwrap_or_default());
        }
        if settings["publication"] == "another_configuration" {
            ui.strong("Prepared only. Your previous configuration is still selected.");
        }
        ui.small("Prepare changed launch settings below, then select the prepared setup under Other setup options. Your current setup stays selected until then.");
        action_buttons(ui, &actions, busy, pending, chosen);
        if !configuration["buffering"].is_null() || !buffering.is_empty() {
            ui.separator();
            ui.strong("Bridge audio delivery");
            let audio = &configuration["buffering"];
            if let Some(frames) = audio["added_frames"].as_u64()
                .or_else(||product.details["added_frames"].as_u64()) {
                ui.label(format!("{frames} added bridge frames"));
                if audio["delivery_mode"] == "same_callback" || product.details["delivery_mode"] == "same_callback" {
                    ui.label("Same-callback delivery · test mode");
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
                    configuration_choices(ui, &settings["before"], expanded);
                } else {
                    ui.label(settings["before"]["requested"].as_str().unwrap_or("Retained previous configuration"));
                }
                graphics_observation(ui, "Previous assessment", &settings["before"]["assessment"]);
            });
        }
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

fn other_setup_option(action: &Action) -> bool {
    matches!(action,Action::CompatibilityCheck { .. } | Action::CompatibilityPublishTest { .. }
        |Action::PluginInspect { .. } |Action::PluginReinspect { .. } |Action::PluginPrepare { .. }
        |Action::ExperimentalReplace { .. } |Action::ExperimentalEnable { .. }
        |Action::CandidateSettingsPrepare { .. } |Action::CandidateGraphicsPrepare { .. })
}

fn configuration_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        _ => "Unavailable".into(),
    }
}

fn configuration_choices(ui: &mut egui::Ui, configuration: &serde_json::Value, expanded: bool) {
    let Some(choices) = configuration["choices"].as_array().filter(|choices| !choices.is_empty()) else {
        ui.small("Requested choices unavailable.");
        return;
    };
    for choice in choices {
        ui.label(format!("{}: {}", configuration_value(&choice["name"]), configuration_value(&choice["value"])));
        egui::CollapsingHeader::new(format!("{} setting details", configuration_value(&choice["name"]))).open(expanded.then_some(true)).show(ui, |ui| {
            ui.small(format!("Source: {} · Scope: {}", configuration_value(&choice["source"]), configuration_value(&choice["scope"])));
            ui.small(format!("Application: {}", configuration_value(&choice["change"])));
        });
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
                |Action::PluginRemoveFromDaw{..}|Action::PluginAddBackToDaw{..}
                |Action::OrdinaryRestoreRecommended{..}|Action::ExperimentalDisable{..}
                |Action::CandidateWithdraw{..}|Action::QuarantinedModuleRetry{..}
                |Action::EnvironmentRescan{..}|Action::VendorApplicationFocus{..}
                |Action::VendorSoftwareInstall{..}|Action::VendorApplicationOpen{..}|Action::RendererOpen{..}
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
        Action::QuarantinedModuleRetry { .. } | Action::PluginAddBackToDaw { .. } => Some(0),
        Action::PluginReinspect { .. } => Some(1),
        Action::PluginInspect { .. } => Some(2),
        Action::PluginPrepare { .. } => Some(3),
        Action::ExperimentalEnable { .. } => Some(4),
        Action::OrdinaryRestoreRecommended { .. } => Some(5),
        Action::EnvironmentRescan { .. } => Some(6),
        Action::VendorApplicationFocus { .. } | Action::RendererFocus { .. } => Some(7),
        Action::VendorSoftwareInstall { .. } | Action::VendorApplicationOpen { .. } | Action::RendererOpen { .. } => Some(8),
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
                        Action::VendorSoftwareInstall { .. } | Action::VendorApplicationOpen { .. } | Action::RendererOpen { .. } => 6,
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
    let button = egui::Button::new(egui::RichText::new(presentation::action_label(offer)).strong())
        .fill(ui.visuals().selection.bg_fill)
        .min_size(egui::vec2(0.0, 46.0));
    if ui.add_enabled(!pending, button).clicked() {
        *chosen = Some(offer.action.clone());
    }
    if pending {
        ui.small("Waiting for the current request or manager readback");
    }
    if let Some(consequence) = presentation::action_consequence(&offer.action) { ui.small(consequence); }
}

pub(crate) fn primary_action(ui: &mut egui::Ui, offer: &AvailableAction,
    busy: Option<&str>, pending: bool, chosen: &mut Option<Action>) {
    let reason = primary_refusal(offer,busy);
    if ui.add_enabled(!pending && reason.is_none(),
        presentation::primary_button(egui::RichText::new(presentation::action_label(offer)).strong(),ui)).clicked() {
        *chosen=Some(offer.action.clone());
    }
    if let Some(reason)=reason {ui.small(reason);}
    else if pending {ui.small("Waiting for the current request to finish.");}
    if let Some(consequence) = presentation::action_consequence(&offer.action) { ui.small(consequence); }
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
        actions.extend(app.actions.iter().filter(|offer| matches!(offer.action,
            Action::VendorSoftwareInstall { .. } | Action::VendorApplicationOpen { .. } | Action::VendorApplicationFocus { .. }
                | Action::RendererOpen { .. } | Action::RendererFocus { .. }))
            .map(|offer| {
                let mut named = offer.clone();
                if !app.name.trim().is_empty() && !matches!(offer.action, Action::VendorSoftwareInstall {..}) {
                    named.label = format!("{} {}", if matches!(offer.action,
                        Action::VendorApplicationOpen { .. } | Action::RendererOpen { .. }) {
                        "Open" } else { "Show" }, app.name);
                }
                named
            }));
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
                egui::Button::new(presentation::action_label(a)).min_size(egui::vec2(0.0, 44.0)),
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
    for reason in visible_refusals(actions, busy, pending) { ui.small(reason); }
    let mut consequences = Vec::new();
    for offer in actions {
        if let Some(text) = presentation::action_consequence(&offer.action) {
            if !consequences.contains(&text) { ui.small(text); consequences.push(text); }
        }
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

pub(crate) fn installer_retry(offer:&crate::model::AvailableAction)->bool {
    offer.label=="Retry" && offer.action.environment_installer_start().is_some()
}
pub(crate) fn environment_installer_controls(ui: &mut egui::Ui,
    target: &crate::model::EnvironmentInstaller, pending: bool, chosen: &mut Option<Action>) {
    ui.strong("Programs");
    ui.small("Choose a Windows program to remember and open in this setup. Its files and resources stay in place.");
    let programs:Vec<_>=target.actions.iter().filter(|offer|matches!(offer.action,
        Action::ProgramChoose {..})).cloned().collect();
    action_buttons(ui,&programs,None,pending,chosen);
    let controls:Vec<_>=target.actions.iter().filter(|offer|matches!(offer.action,
        Action::EnvironmentInstallerFocus { .. } | Action::EnvironmentInstallerStop { .. })).cloned().collect();
    crate::library::action_buttons(ui,&controls,None,pending,chosen);
    ui.label(&target.consequence);
    if target.affected.is_empty() {
        ui.small("This existing setup has no discovered plug-ins yet.");
    } else {
        ui.strong(format!("Affected plug-ins: {}", target.affected.join(", ")));
    }
    egui::CollapsingHeader::new("Setup identity").show(ui, |ui| {
        ui.small(&target.environment);
    });
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
            if runtime.choices.is_empty() {ui.small("To download Windows plug-in support, open Diagnostics → Manager actions.");}
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
    let continuation:Vec<_>=target.actions.iter().filter(|offer|installer_retry(offer)
        || offer.action.environment_installer_start().is_none() && !matches!(offer.action,
        Action::EnvironmentInstallerFocus { .. } | Action::EnvironmentInstallerStop { .. }
        | Action::ProgramChoose {..})).cloned().collect();
    crate::library::action_buttons(ui,&continuation,None,pending,chosen);
    let key = egui::Id::new(("companion_installer",&target.environment));
    let mut selected = ui.data_mut(|data|data.get_temp::<String>(key))
        .unwrap_or_else(|| target.installer.clone().unwrap_or_default());
    egui::ComboBox::from_id_salt(("companion_choice",&target.environment))
        .selected_text(target.choices.iter().find(|offer|offer.action.environment_installer_start()
            .is_some_and(|(_,installer,policy)|installer==selected && policy.is_none()))
            .map(|offer|offer.label.as_str()).unwrap_or("Choose an added Windows installer"))
        .show_ui(ui,|ui| {
            for offer in &target.choices {
                if let Some((_,installer,None))=offer.action.environment_installer_start() {
                    ui.selectable_value(&mut selected,installer.into(),&offer.label);
                }
            }
        });
    ui.data_mut(|data|data.insert_temp(key,selected.clone()));
    if let Some(offer) = target.choices.iter().find(|offer|offer.action.environment_installer_start()
        .is_some_and(|(_,installer,policy)|installer==selected && policy.is_none())) {
        crate::library::action_buttons(ui,std::slice::from_ref(offer),None,pending,chosen);
        let advanced:Vec<_>=target.actions.iter().filter(|offer|!installer_retry(offer)
            && offer.action.environment_installer_start().is_some_and(|(_,installer,policy)|
                installer==selected && policy.is_some())).cloned().collect();
        if !advanced.is_empty() {
            egui::CollapsingHeader::new("Advanced installer options").show(ui,|ui| {
                crate::library::action_buttons(ui,&advanced,None,pending,chosen);
            });
        }
    }
    if target.choices.is_empty() {ui.small("Choose a Windows installer to add a local installer first.");}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CompatibilityWorkflow, CompatibilityPhase};
    fn snapshot() -> Snapshot {
        serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap()
    }
    #[test]
    fn current_native_availability_is_independent_of_compatibility_reports() {
        let mut product = snapshot().products.remove(0);
        product.availability=None;
        assert_eq!(status(&product), ("DAW connection status unavailable", "attention"));
        for phase in [CompatibilityPhase::NotChecked, CompatibilityPhase::CheckFailed,
            CompatibilityPhase::AvailableForTest, CompatibilityPhase::TestResultIncomplete,
            CompatibilityPhase::NeedsWork, CompatibilityPhase::OrdinarySupported] {
            product.compatibility = Some(CompatibilityWorkflow {
                phase, summary:"Optional compatibility record".into(), established:vec![],
                remaining:vec![], current_inspection:None, current_candidate:None,
                primary:None, alternatives:vec![],
            });
            product.availability = Some(crate::model::NativeAvailability { available:true, reason:None });
            assert_eq!(status(&product), ("Available in your DAW", "published"));
            product.availability = Some(crate::model::NativeAvailability {
                available:false, reason:Some("The selected native connection is missing.".into()),
            });
            assert_eq!(status(&product), ("DAW connection needs attention", "attention"));
            assert_eq!(presentation::product_issue(&product).as_deref(),
                Some("The selected native connection is missing."));
        }
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
                let mut library = Library { expand_details: true, ..Default::default() };
                library.show(ui, &current, false, &mut None, |_,_|{});
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
            let mut library = Library {expand_details:true,..Default::default()};
            let ctx = egui::Context::default();
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            for pressed in [None, Some(true), Some(false)] {
                let events = pressed.map(|pressed| vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output = ctx.run_ui(egui::RawInput {events,..Default::default()}, |ui| {
                    library.show(ui,&snapshot,false,&mut chosen,|_,_|{});
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
            let mut library = Library::default();
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                library.show(ui, &snapshot, false, &mut chosen, |_, _| {});
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
        assert_eq!(status(product).0, "Available in your DAW");
        assert!(!optional_test_result(product,
            &product.compatibility.as_ref().unwrap().primary.as_ref().unwrap().action));
        assert_eq!(primary_refusal(product.compatibility.as_ref().unwrap().primary.as_ref().unwrap(),
            Some("Cleanup unconfirmed")), Some("Cleanup unconfirmed"));
        for width in [560.0,960.0] {
            let ctx=egui::Context::default();
            let mut library=Library::default();
            let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
                ui.set_max_width(width-32.0);
                library.show(ui,&snapshot,false,&mut None,|_,_|{});
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
            assert!(!text.iter().any(|line|line==presentation::daw_guidance(None)));
            assert!(!library.expand_details);
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
            let ctx=egui::Context::default();let mut library=Library {expand_details:expanded,..Default::default()};let mut chosen=None;
            let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
                ui.set_max_width(width-32.0);
                library.show(ui,&snapshot,false,&mut chosen,|_,_|{});
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
            assert_eq!(library.expand_details,expanded);assert!(chosen.is_none());
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
            let mut library = Library::default();
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                library.show(ui, snapshot, false, &mut chosen, |_,_| {});
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
        assert_eq!(status(&snapshot.products[0]), ("Removed from your DAW", "unpublished"));
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
            let mut library = Library {expand_details:expanded,..Default::default()};
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                library.show(ui, &snapshot, false, &mut chosen, |_,_| {});
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
            output.textures_delta.clear();
            for offer in [&keep,&restore] {
                assert_eq!(labels.iter().filter(|label| **label == presentation::action_label(offer)).count(), usize::from(expanded),
                    "optional controls are disclosed at {width}, expanded={expanded}");
            }
            for offer in [&ready,&stale] {
                assert_eq!(labels.iter().filter(|label|**label==setup_choice_label(&snapshot.products[0],offer)).count(),usize::from(expanded));
            }
            assert_eq!(labels.iter().any(|label|Some(label)==stale.disabled_reason.as_ref()),expanded);
            assert_eq!(labels.iter().filter(|label| **label == expert.label).count(), usize::from(expanded),
                "expert section must actually be open only when requested");
            assert_eq!(library.expand_details, expanded);
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
            let mut library=Library::default();let mut chosen=None;let mut point=egui::Pos2::ZERO;
            for frame in 0..9 {
                let click=matches!(frame,3|4|7|8);
                let events=if click {vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                    pos:point,button:egui::PointerButton::Primary,pressed:matches!(frame,3|7),modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,egui::vec2(560.0,2000.0))),..Default::default()},|ui| {
                    library.show(ui,&snapshot,pending,&mut chosen,|_,_|{});
                });
                let mut text=vec![];for shape in &output.shapes {centers(&shape.shape,&mut text);}
                if frame==2 {
                    assert!(!text.iter().any(|(label,_)|label.starts_with("Restore previous setup")));
                    point=text.iter().find(|(label,_)|label=="Previous setups (12)").unwrap().1;
                    assert!(!text.iter().any(|(label,_)|label=="Check this plug-in again"));
                    assert!(text.iter().find(|(label,_)|label==presentation::daw_guidance(None)).unwrap().1.y<700.0);
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
            let mut library=Library::default();library.focus_product(crate::presentation::ProductKey::from(&*product));
            let ctx=egui::Context::default();presentation::configure_style(&ctx);
            ctx.global_style_mut(|style|style.animation_time=0.0);
            let mut chosen=None;let mut point=egui::Pos2::ZERO;
            for frame in 0..9 {
                let events=if matches!(frame,3|4|7|8) {vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                    pos:point,button:egui::PointerButton::Primary,pressed:matches!(frame,3|7),modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,egui::vec2(560.0,2400.0))),..Default::default()},|ui|library.show(ui,&snapshot,pending,&mut chosen,|_,_|{}));
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
    fn search_combines_terms_and_filters_without_losing_unknown_states() {
        let mut snapshot = snapshot();
        snapshot.products[0].disposition = "future_state".into();
        snapshot.products[0].availability=None;
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
        for fresh in [true, false] {
            let mut library = Library::default();
            library.focus_products(vec![first.clone(), second.clone()]);
            assert!(library.focused_product().is_none());
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                library.show_current(ui, &snapshot, fresh, false, &mut false);
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
            output.textures_delta.clear();
            assert_eq!(labels.iter().filter(|line| line.as_str() == "Manage plug-in").count(), 2,
                "read-only detail navigation remains available, fresh={fresh}");
            library.focus_product(second.clone());
            assert_eq!(library.focused_product(), Some(&second));
            assert_eq!(library.products(&snapshot).len(), 1);
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
            let mut library = Library::default(); library.focus_product(ProductKey::from(&*product));
            let ctx = egui::Context::default(); presentation::configure_style(&ctx);
            ctx.global_style_mut(|style| style.animation_time = 0.0);
            let mut chosen = None; let mut point = egui::Pos2::ZERO;
            for frame in 0..9 {
                let events = if matches!(frame, 3|4|7|8) { vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton { pos:point, button:egui::PointerButton::Primary,
                        pressed:matches!(frame,3|7), modifiers:egui::Modifiers::NONE }] } else { vec![] };
                let mut output = ctx.run_ui(egui::RawInput { events, screen_rect:Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO, egui::vec2(560.0, 1200.0))), ..Default::default() },
                    |ui| library.show(ui, &snapshot, pending, &mut chosen, |_,_|{}));
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
    fn installation_and_preparation_do_not_imply_publication() {
        let mut p = snapshot().products.remove(0);
        for state in ["prepared", "installed_unqualified"] {
            p.disposition = state.into();
            p.availability=Some(crate::model::NativeAvailability {available:false,reason:None});
            assert_eq!(status(&p).1, "unpublished");
        }
        p.disposition = "experimental".into();
        p.availability=Some(crate::model::NativeAvailability {available:true,reason:None});
        assert_eq!(status(&p).0, "Available in your DAW");
        p.disposition = "another_configuration".into();
        p.availability=Some(crate::model::NativeAvailability {available:false,reason:None});
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
    fn retained_explanation_wins_over_generic_availability_text() {
        let mut p = snapshot().products.remove(0);
        p.disposition = "installed_unqualified".into();
        p.availability = Some(crate::model::NativeAvailability {available:false,
            reason:Some("This installed plug-in has no current native DAW connection.".into())});
        p.details["inspection_error"] = serde_json::json!("Windows SDK host failed: 90");
        assert_eq!(presentation::product_issue(&p).as_deref(),
            Some("This installed plug-in has no current native DAW connection."));
        p.details["problem"] = serde_json::json!({"code":"sdk_component_start",
            "what_happened":"The plug-in loaded, but it refused to start its audio engine.",
            "likely_cause":"It is usually missing something it needs at start-up.",
            "what_to_try":"Use the vendor's installer or app to repair or activate the plug-in, then run the check again."});
        assert_eq!(presentation::product_issue(&p).as_deref(),
            Some("The plug-in loaded, but it refused to start its audio engine. It is usually missing something it needs at start-up. Use the vendor's installer or app to repair or activate the plug-in, then run the check again."));
        p.details["problem"] = serde_json::json!({"code":"x","what_happened":" "});
        assert_eq!(presentation::product_issue(&p).as_deref(),
            Some("This installed plug-in has no current native DAW connection."));
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
