//! Measured time per stage and per return, printed and, when asked, written
//! as Markdown for the workflow summary. Written even when the test fails.
use crate::session::Return;
use std::{fmt::Write, path::PathBuf, time::Duration};

pub struct Report {
    revision: String,
    path: Option<PathBuf>,
    stages: Vec<(String, Duration)>,
    returns: Vec<Row>,
    notes: Vec<String>,
}
struct Row {
    case: String,
    outcome: String,
    total: Duration,
    checks: Option<u64>,
    to_second_copy: Option<u64>,
    second_copy: Option<u64>,
    answer: Option<u64>,
}

impl Report {
    pub fn new(revision: &str, path: Option<PathBuf>) -> Self {
        Self { revision: revision.into(), path, stages: vec![], returns: vec![], notes: vec![] }
    }
    pub fn stage(&mut self, name: &str, elapsed: Duration) {
        self.stages.push((name.into(), elapsed));
    }
    pub fn note(&mut self, note: String) {
        self.notes.push(note);
    }
    /// One return: the handler's own checks until it reached the supervisor;
    /// from there until the second copy ran; the second copy's own time until
    /// it handed over or ended; and from that until the handler had its answer.
    pub fn browser_return(&mut self, case: &str, outcome: &str, r: &Return, second_copy: Option<u64>,
        handed_over: Option<u64>) {
        let between = |from: Option<u64>, to: Option<u64>| to?.checked_sub(from?);
        self.returns.push(Row { case: case.into(), outcome: outcome.into(), total: r.elapsed,
            checks: between(Some(r.started_ms), r.connected_ms),
            to_second_copy: between(r.connected_ms, second_copy),
            second_copy: between(second_copy, handed_over),
            answer: between(handed_over.or(second_copy), Some(r.exited_ms)) });
    }

    fn markdown(&self) -> String {
        let mut text = format!("### Native Access browser return, {}\n\n", self.revision);
        let seconds = |d: Duration| format!("{:.1} s", d.as_secs_f64());
        let ms = |v: Option<u64>| v.map_or("–".into(), |v| format!("{:.1} s", v as f64 / 1000.0));
        text += "| Stage | Time |\n|---|---|\n";
        for (name, elapsed) in &self.stages {
            let _ = writeln!(text, "| {name} | {} |", seconds(*elapsed));
        }
        text += "\n| Return | Result | Handler total | Handler checks | To second copy | Second copy | Answer |\n";
        text += "|---|---|---|---|---|---|---|\n";
        for row in &self.returns {
            let _ = writeln!(text, "| {} | {} | {} | {} | {} | {} | {} |", row.case, row.outcome,
                seconds(row.total), ms(row.checks), ms(row.to_second_copy), ms(row.second_copy), ms(row.answer));
        }
        for note in &self.notes {
            let _ = writeln!(text, "\n{note}");
        }
        text
    }
}
impl Drop for Report {
    fn drop(&mut self) {
        let mut text = self.markdown();
        if std::thread::panicking() { text += "\n**The test failed; the table ends at the failure.**\n"; }
        println!("{text}");
        if let Some(path) = &self.path {
            if let Err(error) = std::fs::write(path, &text) { eprintln!("report {}: {error}", path.display()); }
        }
    }
}
