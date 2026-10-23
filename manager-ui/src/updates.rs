//! Egui drawing for the shared installed updater.
use eframe::egui;
use std::time::Instant;
use crate::screen_model::updates::{self as panel, Block, Link, Step, UpdateNotice as Notice};
pub struct Updates { core: lvb_client::updates::Updates, draft: String }
impl Updates {
    pub fn new() -> Self { Self { core: lvb_client::updates::Updates::new(), draft: String::new() } }
    pub fn disabled() -> Self { Self { core: lvb_client::updates::Updates::disabled(), draft: String::new() } }
    #[cfg(test)]
    pub fn sample(sample: &crate::screen_model::samples::UpdateSample) -> Self {
        Self { core: lvb_client::updates::Updates::sample(sample), draft: sample.draft.clone() }
    }
    pub fn headline(&self) -> Option<String> { self.core.headline() }
    pub fn poll(&mut self, ctx: &egui::Context, reading: Option<(bool, Instant)>) -> bool {
        let close = self.core.poll(reading);
        if let Some(delay) = self.core.next_poll() { ctx.request_repaint_after(delay); }
        close
    }
    /// Setup's "Beta updates" group, drawn from its screen model. A link
    /// change applies at once; the update's own controls apply at the end.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
            ui.add_enabled_ui(self.core.available(), |ui| {
            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
            let mut view = self.core.panel();
            ui.label(egui::RichText::new(view.title).size(18.0).strong());
            let mut link_step = None;
            match &view.link {
                Link::Entry { prompt, hint, save, keep, privacy } => {
                    ui.label(*prompt);
                    ui.horizontal_wrapped(|ui| {
                        let field = ui.add(egui::TextEdit::singleline(&mut self.draft)
                            .password(true).hint_text(*hint).desired_width(360.0));
                        let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui.add_enabled(panel::can_save(&self.draft),
                            crate::presentation::primary_button(*save, ui)).clicked() || entered {
                            self.core.step(Step::SaveLink, Some(self.draft.clone()));
                            link_step = Some(Step::SaveLink);
                        }
                        if let Some(keep) = keep {
                            if ui.button(keep.label).clicked() {
                                self.core.step(Step::KeepLink, None);
                                self.draft.clear();
                                link_step = Some(keep.does);
                            }
                        }
                    });
                    ui.small(*privacy);
                }
                Link::Saved { line, change } => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(*line);
                        if let Some(change) = change {
                            if ui.button(change.label).clicked() {
                                self.core.step(Step::ChangeLink, None);
                                link_step = Some(change.does);
                            }
                        }
                    });
                }
            }
            if link_step.is_some() {
                view = self.core.panel();
                if matches!(view.link, Link::Saved { .. }) { self.draft.clear(); }
            }
            if let Some(message) = &view.message {
                ui.colored_label(warning(ui), message);
            }
            let Some(status) = &view.status else { return };
            ui.add_space(4.0);
            let mut step = None;
            match status {
                Block::Idle => {}
                Block::Working(words) => { ui.horizontal(|ui| { ui.spinner(); ui.label(*words); }); }
                Block::Line(line) => { ui.label(line); }
                Block::Notice { notice, warn } => notice_line(ui, notice, *warn),
                Block::Available { headline, notes, notice, advice, download } => {
                    ui.label(egui::RichText::new(headline).strong());
                    if let Some((title, notes)) = notes {
                        egui::CollapsingHeader::new(*title).default_open(false)
                            .show(ui, |ui| { ui.label(plain_notes(notes)); });
                    }
                    if let Some(notice) = notice { notice_line(ui, notice, true); }
                    ui.label(*advice);
                    if ui.add(crate::presentation::primary_button(download.label, ui)).clicked() {
                        step = Some(download.does);
                    }
                }
                Block::Downloading { line, fraction, progress, note } => {
                    ui.label(line);
                    ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).show_percentage().text(progress));
                    ui.small(*note);
                }
                Block::Waiting { headline, warning: caution, continue_anyway, cancel, recheck } => {
                    ui.label(egui::RichText::new(headline).strong());
                    ui.colored_label(warning(ui), *caution);
                    ui.horizontal_wrapped(|ui| {
                        if let Some(proceed) = continue_anyway {
                            if ui.button(proceed.label).clicked() { step = Some(proceed.does); }
                        }
                        if ui.button(cancel.label).clicked() { step = Some(cancel.does); }
                    });
                    if let Some(recheck) = recheck { ui.small(*recheck); }
                }
            }
            if let Some(check) = &view.check {
                if ui.add_enabled(check.enabled, egui::Button::new(check.label)
                    .min_size(egui::vec2(0.0, 44.0))).clicked() {
                    self.core.step(Step::Check, None);
                }
            }
            if let Some(step) = step { self.core.step(step, None); }
            });
        });
    }

}

/// The notes as plain text in the window's font, which has no arrow glyphs.
fn plain_notes(notes: &str) -> String {
    notes.replace('\r', "").replace('→', "->").replace('←', "<-")
}

fn notice_line(ui: &mut egui::Ui, notice: &Notice, warn: bool) {
    if warn { ui.colored_label(warning(ui), &notice.text); } else { ui.label(&notice.text); }
    if let Some(technical) = &notice.technical {
        egui::CollapsingHeader::new(panel::TECHNICAL).show(ui, |ui| { ui.small(technical); });
    }
}

fn warning(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode { egui::Color32::from_rgb(245, 190, 100) } else { egui::Color32::from_rgb(140, 77, 0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_shared_update_samples_render_without_revealing_the_saved_link() {
        for (_, sample) in crate::screen_model::samples::updates() {
            let mut updates = Updates::sample(&sample);
            assert_eq!(updates.core.panel(), sample.panel());
            let ctx = egui::Context::default();
            let output = ctx.run_ui(egui::RawInput::default(), |ui| updates.show(ui));
            let rendered = format!("{:?}", output.shapes);
            assert!(!rendered.contains("s3cret") && !rendered.contains("pw="));
        }
    }
}
