//! The native drawing adapter for the shared package setup client.
use eframe::egui;
#[allow(unused_imports)]
pub use lvb_client::bootstrap::{entry, launch_selected, Entry};
use lvb_client::bootstrap::{State, Worker};

pub struct Bootstrap { state: State, worker: Worker }
impl Bootstrap {
    fn from_state(state: State) -> Self { Self { state, worker: Worker::new() } }
    #[allow(dead_code)]
    pub fn new() -> Self { Self::from_state(State::new()) }
    #[allow(dead_code)]
    pub fn needs_activation() -> Self {
        let mut state = State::new(); state.receive(Ok(lvb_client::bootstrap::Completion::NeedsActivation)); Self::from_state(state)
    }
    #[allow(dead_code)]
    pub fn legacy_adoptable_preview() -> Self {
        let mut state = State::new(); state.receive(Ok(lvb_client::bootstrap::Completion::LegacyAdoptable)); Self::from_state(state)
    }
    #[allow(dead_code)]
    pub fn repair_active_preview() -> Self {
        let mut state = State::new(); state.receive(Ok(lvb_client::bootstrap::Completion::RepairActive)); Self::from_state(state)
    }
    pub fn checking() -> Self { Self::from_state(State::checking()) }
    pub fn continuing_update() -> Self { Self::from_state(State::continuing_update()) }
    pub fn attention(error: String) -> Self { Self::from_state(State::attention(error)) }
}
impl eframe::App for Bootstrap {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(reply) = self.worker.receive() {
            if self.state.receive(reply) {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
        if let Some(operation) = self.state.take_operation() {
            if let Err(error) = self.worker.send(operation) { self.state.receive(Err(error)); }
        }
        let panel = self.state.panel();
        if panel.progress.is_some() { ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)); }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(panel.title);
            ui.label(panel.introduction);
            ui.label(panel.advice);
            if let Some(progress) = panel.progress { ui.strong(progress); }
            if !panel.detail.is_empty() { ui.small(panel.detail); }
            for control in panel.controls {
                if ui.add_sized([250.0, 48.0], egui::Button::new(control.label)).clicked() {
                    self.state.press(control.step);
                    ui.ctx().request_repaint();
                }
            }
            if let Some(result) = panel.result { ui.colored_label(egui::Color32::YELLOW, result); }
        });
    }
}
