//! egui drawing of the screen-model vocabulary: words, offers, setup choices
//! and disclosures. Drawing only; every decision and word comes from the model.
use crate::model::{Action, AvailableAction};
use crate::screen_model::journey::Results;
use crate::screen_model::offers::{self, Availability, Importance, OfferRow, SetupChoice};
use crate::screen_model::{Detail, Disclosure, Text, TextRole};
use eframe::egui;

pub fn warning_color(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgb(245, 190, 100)
    } else {
        egui::Color32::from_rgb(140, 77, 0)
    }
}

pub fn text(ui: &mut egui::Ui, text: &Text) {
    match text.role {
        TextRole::Body => ui.label(&text.words),
        TextRole::Strong => ui.strong(&text.words),
        TextRole::Detail => ui.small(&text.words),
        TextRole::Caution => ui.colored_label(warning_color(ui), &text.words),
        TextRole::Alert => ui.colored_label(egui::Color32::YELLOW, &text.words),
        TextRole::Hint => ui.label(egui::RichText::new(&text.words).small()
            .color(ui.visuals().weak_text_color())),
        TextRole::CautionHint => ui.label(egui::RichText::new(&text.words).small().color(warning_color(ui))),
        TextRole::Record => ui.add(egui::Label::new(egui::RichText::new(&text.words).monospace()).wrap()),
        TextRole::Code => ui.monospace(&text.words),
    };
}

/// A wrapped row of offers, then every refusal and consequence once.
pub fn offer_row(ui: &mut egui::Ui, row: &OfferRow, chosen: &mut Option<Action>) {
    if let Some(preface) = &row.preface {
        ui.label(preface);
    }
    if row.offers.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for offer in &row.offers {
            let response = ui.add_enabled(
                offer.availability.enabled(),
                egui::Button::new(&offer.label).min_size(egui::vec2(0.0, 44.0)),
            );
            if response.clicked() {
                *chosen = Some(offer.intent.clone());
            }
            // A held offer's reason is said once by its screen.
            if let Availability::Waiting | Availability::Refused(_) = offer.availability {
                if let Some(reason) = offer.availability.reason() { response.on_disabled_hover_text(reason); }
            }
        }
    });
    // Keep every refusal visible to touch users, even when global inactivity
    // also blocks a manager-disabled action.
    for note in &row.notes { ui.small(note); }
    for consequence in &row.consequences { ui.small(*consequence); }
}

/// Manager-offered actions as one row, for screens not yet built from the model.
pub fn action_buttons(
    ui: &mut egui::Ui,
    actions: &[AvailableAction],
    busy: Option<&str>,
    pending: bool,
    chosen: &mut Option<Action>,
) {
    offer_row(ui, &offers::row(actions, Importance::Secondary, busy, pending), chosen);
}

pub fn setup_choice(ui: &mut egui::Ui, choice: &SetupChoice, chosen: &mut Option<Action>) {
    ui.push_id(&choice.exact_action, |ui| {
        if ui.add_enabled(choice.offer.availability.enabled(), egui::Button::new(&choice.offer.label)
            .min_size(egui::vec2(0.0, 44.0))).clicked() { *chosen = Some(choice.offer.intent.clone()); }
        for note in &choice.notes { ui.small(note); }
        if let Some(description) = &choice.description { ui.small(description); }
        if let Some(consequence) = choice.offer.consequence { ui.small(consequence); }
        disclosure(ui, &choice.identity, chosen);
    });
}

pub fn details(ui: &mut egui::Ui, content: &[Detail], chosen: &mut Option<Action>) {
    for detail in content {
        match detail {
            Detail::Text(line) => text(ui, line),
            Detail::Offers(row) => offer_row(ui, row, chosen),
            Detail::Held { actionable, row } => { ui.add_enabled_ui(*actionable, |ui| offer_row(ui, row, chosen)); }
            Detail::Choice(choice) => setup_choice(ui, choice, chosen),
            Detail::Disclosure(inner) => disclosure(ui, inner, chosen),
            Detail::Group(inner) => { ui.group(|ui| details(ui, inner, chosen)); }
            Detail::Divider => { ui.separator(); }
        }
    }
}

pub fn disclosure(ui: &mut egui::Ui, disclosure: &Disclosure, chosen: &mut Option<Action>) {
    disclosure_with(ui, disclosure, chosen, |_| {});
}

/// A disclosure whose content ends with records the caller still draws itself.
pub fn disclosure_with(ui: &mut egui::Ui, disclosure: &Disclosure, chosen: &mut Option<Action>,
    tail: impl FnOnce(&mut egui::Ui)) {
    let mut header = egui::CollapsingHeader::new(&disclosure.title);
    if let Some(salt) = disclosure.id_salt {
        header = header.id_salt(salt);
    }
    header.default_open(disclosure.start_open)
        .open(disclosure.force_open.then_some(true))
        .show(ui, |ui| {
            details(ui, &disclosure.content, chosen);
            tail(ui);
        });
}

/// The one emphasised next step. A refused offer stays visible, plainly
/// disabled, with the manager's reason beside it.
pub fn lead_offer(ui: &mut egui::Ui, offer: &offers::Offer, chosen: &mut Option<Action>) {
    if let Availability::Refused(reason) = &offer.availability {
        ui.add_enabled(false, egui::Button::new(&offer.label).min_size(egui::vec2(240.0, 46.0)));
        ui.small(reason);
        if let Some(consequence) = offer.consequence { ui.small(consequence); }
        return;
    }
    let button = egui::Button::new(egui::RichText::new(&offer.label).strong())
        .fill(ui.visuals().selection.bg_fill)
        .min_size(egui::vec2(0.0, 46.0));
    if ui.add_enabled(offer.availability.enabled(), button).clicked() {
        *chosen = Some(offer.intent.clone());
    }
    if offer.availability == Availability::Waiting {
        ui.small(offers::WAITING);
    }
    if let Some(consequence) = offer.consequence { ui.small(consequence); }
}

/// The main next step of a Home or Setup card: a filled button, then its
/// refusal or why it waits, then what it changes.
pub fn main_offer(ui: &mut egui::Ui, offer: &offers::Offer, chosen: &mut Option<Action>) {
    if ui.add_enabled(offer.availability.enabled(), crate::presentation::primary_button(
        egui::RichText::new(&offer.label).strong(), ui)).clicked() {
        *chosen = Some(offer.intent.clone());
    }
    match &offer.availability {
        Availability::Refused(reason) => { ui.small(reason); }
        Availability::Waiting => { ui.small(offers::WAITING_TO_FINISH); }
        // Held: the card's screen says once that actions wait for current status.
        Availability::Enabled | Availability::Held => {}
    }
    if let Some(consequence) = offer.consequence { ui.small(consequence); }
}

/// One installation journey's DAW, problem and per-plug-in results.
pub fn journey(ui: &mut egui::Ui, results: &Results) {
    ui.small(&results.daw);
    if let Some(empty) = results.empty { ui.label(empty); }
    if let Some(problem) = &results.problem { ui.colored_label(warning_color(ui), problem); }
    for plugin in &results.plugins {
        ui.horizontal_wrapped(|ui| {
            ui.strong(&plugin.name);
            ui.label(plugin.status.words);
        });
        if let Some(problem) = &plugin.problem { ui.colored_label(warning_color(ui), problem); }
        if let Some(hint) = plugin.hint { ui.small(hint); }
    }
}
