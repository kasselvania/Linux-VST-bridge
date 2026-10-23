//! Native file dialogs shared by the installed frontends.
use crate::{FileSelection, SupportReport};
use std::path::PathBuf;

pub struct RfdSelection;
impl FileSelection for RfdSelection {
    fn pick_installer(&self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .set_title("Choose Windows installer")
            .add_filter("Windows installers", &["exe", "msi"])
            .pick_file()
    }
    fn pick_program(&self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .set_title("Choose Windows program")
            .add_filter("Windows programs", &["exe"])
            .pick_file()
    }
    fn save_report(&self, report: &SupportReport) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .set_title("Save problem report to send")
            .set_file_name(format!("linux-vst-bridge-problem-{}.json", report.id))
            .add_filter("Problem report", &["json"])
            .save_file()
    }
}
