//! Console support composition. Owner data and offers come from the shared view model.
use super::*;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step { Read, Back, Previous, Next, Save, Done }

/// Local presentation of an exact, already completed owner export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub identity: vm::request::SupportReport,
    pub pages: Option<Arc<Vec<String>>>,
    pub page: usize,
    pub inspecting: bool,
    pub busy: bool,
    pub notice: Option<String>,
    pub saved: Option<PathBuf>,
}
impl Report {
    pub fn new(identity: vm::request::SupportReport) -> Self {
        Self { identity, pages: None, page: 0, inspecting: false, busy: false, notice: None, saved: None }
    }
    pub fn contents(&mut self, json: &str) {
        // Every character remains inspectable, without an unbounded native
        // label. Split on UTF-8 boundaries and cap both lines and bytes/page.
        let mut pages = vec![];
        let mut page = String::new();
        let mut lines = 0;
        for ch in json.chars() {
            if page.len() + ch.len_utf8() > 4096 || lines >= 48 {
                pages.push(std::mem::take(&mut page)); lines = 0;
            }
            page.push(ch);
            if ch == '\n' { lines += 1; }
        }
        if !page.is_empty() || pages.is_empty() { pages.push(page); }
        self.pages = Some(Arc::new(pages)); self.page = 0;
        self.inspecting = true; self.notice = None; self.busy = false;
    }
}

impl Builder<'_> {
    fn report_control(&mut self, report: &Report, label: &str, step: Step, weight: Importance,
        unavailable: Option<&str>) -> ActionView {
        let reason = if self.why.superseded { Some("Open the current manager to continue.") }
            else if report.busy || self.why.in_flight { Some("Waiting for the current request to finish.") }
            else { unavailable };
        self.local(label, weight, Target::Report { report: report.identity.clone(), step }, reason.map(str::to_owned))
    }

    pub(super) fn report_page(&mut self, report: &Report) -> Vec<Node> {
        let mut out = vec![Node::Label("Support".into()), Node::Title(if report.inspecting {
            "Read the report contents"
        } else if report.saved.is_some() { "Report saved" } else { "Make a support report" }.into())];
        if let Some(notice) = &report.notice { out.push(words(Role::Caution, notice)); }
        if report.busy { out.push(words(Role::Body, "Waiting for the report file…")); }
        if report.inspecting {
            let pages = report.pages.as_deref().map(Vec::as_slice).unwrap_or_default();
            out.push(words(Role::Body, "This is the complete JSON content used by Save report. Nothing is uploaded."));
            out.push(words(Role::Detail, format!("Page {} of {}", report.page + 1, pages.len())));
            let back = self.report_control(report, "Back to report", Step::Back, Importance::Secondary, None);
            let previous = self.report_control(report, "Previous page", Step::Previous, Importance::Quiet,
                (report.page == 0).then_some("This is the first page."));
            let next = self.report_control(report, "Next page", Step::Next, Importance::Quiet,
                (report.page + 1 >= pages.len()).then_some("This is the last page."));
            out.push(self.actions("report-pages", vec![back, previous, next]));
            out.push(Node::Panel { kind: PanelKind::Raised, children: vec![words(Role::Record,
                pages.get(report.page).map(String::as_str).unwrap_or_default())] });
            return out;
        }
        if let Some(path) = &report.saved {
            out.push(Node::Panel { kind: PanelKind::Card, children: vec![
                Node::Label("Saved file".into()), words(Role::Strong, path.file_name().unwrap_or_default().to_string_lossy()),
                words(Role::Code, path.display().to_string()),
            ] });
            out.push(words(Role::Lead, "Attach this JSON file to your support email or beta thread. Nothing has been uploaded."));
            let done = self.report_control(report, "Done", Step::Done, Importance::Primary, None);
            let read = self.report_control(report, "Read the contents", Step::Read, Importance::Secondary, None);
            out.push(self.actions("saved-report", vec![done, read]));
        } else {
            out.push(words(Role::Lead, "Problem report created on this computer. You can read everything before choosing where to save it."));
            out.extend(report_contents_summary());
            let save = self.report_control(report, "Save report…", Step::Save, Importance::Primary, None);
            let read = self.report_control(report, "Read the contents", Step::Read, Importance::Secondary, None);
            let back = self.report_control(report, "Back to Support", Step::Done, Importance::Quiet, None);
            out.push(self.actions("report", vec![save, read, back]));
        }
        out
    }

    pub(super) fn diagnostics(&mut self, diagnostics: &Diagnostics, frame: &Frame) -> Vec<Node> {
        let Diagnostics { heading: _, version, system, actions_title, actions, records, technical } = diagnostics;
        let VersionSection { installed, setup, apply, runtime, copy, copied } = version;
        let mut out = vec![Node::Title("Support".into())];
        out.push(Node::Heading("Make a support report".into()));
        out.push(words(Role::Lead, "Create a local report to inspect and save. Nothing is uploaded."));
        out.extend(report_contents_summary());
        if let Some(export) = &frame.title.export { out.extend(self.row(export)); }
        let mut children = vec![Node::Label("Version details".into()), words(Role::Body, installed.clone())];
        if let Some(setup) = setup {
            children.push(words(Role::Code, setup.clone()));
        }
        if let Some(apply) = apply {
            children.push(words(Role::Body, *apply));
        }
        children.push(words(Role::Code, runtime.clone()));
        let copy = self.control(copy);
        children.push(self.one(copy));
        if self.ui.copied {
            children.push(words(Role::Detail, *copied));
        }
        let versions = Node::Panel { kind: PanelKind::Card, children };
        let mut rows = vec![Node::Heading((*actions_title).into())];
        for row in actions {
            // The primary row already prints this exact offer, refusal and
            // consequence. Retain any distinct owner offer or explanation.
            if frame.title.export.as_ref() == Some(row) { continue; }
            rows.extend(self.row(row));
        }
        out.push(Node::Columns(vec![versions, Node::Panel { kind: PanelKind::Card, children: rows }]));
        out.push(self.fold(system));
        out.extend(self.service_details(frame));
        out.push(self.fold(records));
        out.push(self.fold(technical));
        out
    }

}

fn report_contents_summary() -> Vec<Node> {
    let card = |label: &str, lines: &[&str]| Node::Panel { kind: PanelKind::Card,
        children: std::iter::once(Node::Label(label.into()))
            .chain(lines.iter().map(|line| words(Role::Body, *line))).collect() };
    vec![Node::Columns(vec![card("Included", &[
        "Manager and runtime versions, plus computer checks",
        "Setup history and selected bridge problem records",
        "Recorded incident categories and cleanup status",
    ]), card("Never included", &[
        "Projects, recordings, presets, samples or libraries",
        "Licences, serials or account details",
        "Plug-in files, installers, prefixes or raw logs",
    ])])]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_pages_preserve_every_unicode_byte_with_bounded_native_labels() {
        let json = format!("{}{}", "🎹é".repeat(10000), "\n".repeat(100));
        let mut report = Report::new(vm::request::SupportReport { id: "ab".repeat(16), sha256: "cd".repeat(32) });
        report.contents(&json);
        let pages = report.pages.unwrap();
        assert_eq!(pages.concat(), json);
        assert!(pages.iter().all(|page| page.len() <= 4096 && page.chars().filter(|ch| *ch == '\n').count() <= 48));
    }
}
