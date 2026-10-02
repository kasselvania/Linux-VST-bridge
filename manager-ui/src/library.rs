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
                        } else if let Some(workflow) = &p.compatibility {
                            action_buttons(ui, &workflow.alternatives,
                                snapshot.system.inactive_reason(), pending, chosen);
                        }
                        action_buttons(ui, &management, snapshot.system.inactive_reason(), pending, chosen);
                        let buffering: Vec<_> = p.actions.iter().filter(|offer|
                            matches!(offer.action, Action::BufferingSet { .. })).cloned().collect();
                        if !buffering.is_empty() {
                            ui.label("Bridge audio buffering");
                            ui.small("1024 frames allows larger host blocks and adds 21.33 ms at 48 kHz. This configuration remains unqualified until tested.");
                            action_buttons(ui, &buffering, snapshot.system.inactive_reason(), pending, chosen);
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
                                        !matches!(action.action, Action::BufferingSet { .. })
                                            && !management.iter().any(|shown|shown.action==action.action) && primary
                                            .as_ref()
                                            .is_none_or(|primary| primary.action != action.action)
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
                                    action_buttons(ui, &workflow.alternatives,
                                        snapshot.system.inactive_reason(), pending, chosen);
                                }
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
        if show && primary.is_none_or(|p|p.action!=offer.action)
            && !actions.iter().any(|shown:&AvailableAction|shown.action==offer.action) {
            actions.push(offer.clone());
        }
    }
    actions
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
            offer.disabled_reason.is_none() && !(busy.is_some() && offer.action.requires_inactive())
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
    action.disabled_reason.as_deref().or_else(|| {
        (action.action.requires_inactive()
            && !matches!(action.action, Action::CompatibilityResult { .. }))
            .then_some(busy).flatten()
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
        if offer.action.requires_inactive() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CompatibilityWorkflow, CompatibilityPhase};
    fn snapshot() -> Snapshot {
        serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap()
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
}
