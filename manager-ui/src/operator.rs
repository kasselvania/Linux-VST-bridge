use crate::{
    client::{self, Query, Reply},
    model::*,
};
use eframe::egui;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
pub struct Operator {
    snapshot: Option<Snapshot>,
    sender: mpsc::Sender<Reply>,
    receiver: mpsc::Receiver<Reply>,
    pending: bool,
    last_poll: Instant,
    message: String,
    filter: String,
    refresh_after: bool,
}
impl Operator {
    pub fn new(ctx: &egui::Context) -> Self {
        let (sender, receiver) = mpsc::channel();
        client::send(Query::Snapshot, sender.clone(), ctx.clone());
        Self {
            snapshot: None,
            sender,
            receiver,
            pending: true,
            last_poll: Instant::now(),
            message: "Reading installed manager…".into(),
            filter: String::new(),
            refresh_after: false,
        }
    }
    fn request(&mut self, q: Query, ctx: &egui::Context) {
        self.pending = true;
        self.last_poll = Instant::now();
        client::send(q, self.sender.clone(), ctx.clone());
    }
    fn buttons(
        ui: &mut egui::Ui,
        actions: &[AvailableAction],
        busy: Option<&str>,
        pending: bool,
        chosen: &mut Option<Action>,
    ) {
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
            if let Some(reason) = reason {
                response.on_disabled_hover_text(reason);
                ui.small(reason);
            }
        }
    }
    fn value(ui: &mut egui::Ui, v: &serde_json::Value) {
        ui.add(
            egui::Label::new(
                egui::RichText::new(serde_json::to_string_pretty(v).unwrap_or_default())
                    .monospace(),
            )
            .wrap(),
        );
    }
}
impl eframe::App for Operator {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(reply) = self.receiver.try_recv() {
            self.pending = false;
            match reply {
                Reply::Snapshot(s) => {
                    if s.schema != 2 {
                        self.message = "Unsupported manager schema".into();
                    } else {
                        self.snapshot = Some(*s);
                        self.message = "Canonical installed state refreshed".into();
                    }
                }
                Reply::Receipt(r) => {
                    self.message = if r.accepted {
                        format!("Operation accepted: {}", r.operation.unwrap_or_default())
                    } else {
                        r.refusal.unwrap_or_else(|| "Operation refused".into())
                    };
                    // Do not race a newly dispatched mutation for registry.lock.
                    // The lightweight receipt poll selects the completed readback.
                    self.refresh_after = !r.accepted;
                }
                Reply::Activity(a) => {
                    if let Some(op) = &a.operation {
                        if op["state"] == "refused" {
                            self.message = format!(
                                "Last operation refused: {}",
                                op["reason"].as_str().unwrap_or("see receipt")
                            );
                        } else if op["state"] == "completed" {
                            self.message = "Operation completed".into();
                        }
                    }
                    if let Some(s) = &mut self.snapshot {
                        if s.system.capacity_available() != a.system.capacity_available()
                            || s.system.dsp != a.system.dsp
                            || s.system.maintenance != a.system.maintenance
                            || s.system.cleanup_unconfirmed != a.system.cleanup_unconfirmed
                            || refresh_for_receipt(&s.operation, &a.operation)
                        {
                            self.refresh_after = true;
                        }
                        s.system = a.system;
                        s.capture = a.capture;
                        s.operation = a.operation;
                    }
                }
                Reply::Imported => {
                    self.message =
                        "Installer imported. Review its identity and choose a runner below.".into();
                    self.refresh_after = true;
                }
                Reply::Cancelled => {
                    self.message = "Installer selection cancelled".into();
                }
                Reply::Error(e) => {
                    self.message = e;
                    if let Some(s) = &mut self.snapshot {
                        s.system.service = "capacity unavailable".into();
                        s.system.cleanup_unconfirmed = true;
                    }
                }
            }
        }
        let mut refresh = false;
        let mut pick = false;
        let mut chosen = None;
        egui::CentralPanel::default().show(ui,|ui|{
            ui.heading("Linux Audio Compatibility Manager");
            ui.horizontal(|ui|{if ui.add_enabled(!self.pending,egui::Button::new("Refresh")).clicked(){refresh=true;}ui.label(&self.message);if self.pending{ui.spinner();}});
            ui.separator();
            egui::ScrollArea::vertical().show(ui,|ui|{
                let Some(s)=&self.snapshot else{ui.label("The installed Rust manager is the state authority. Waiting for readback.");return;};
                let busy=s.system.inactive_reason();
                if !s.system.capacity_available() { ui.colored_label(egui::Color32::YELLOW,"Service capacity unavailable — DSP, keeper and cleanup status cannot be confirmed"); }
                else { ui.label(format!("Service: {}  ·  Keeper: {}  ·  DSP: {} / {}  ·  Pending: {}  ·  Stale transports: {}",s.system.service,s.system.keepers,s.system.dsp,s.system.ceiling,s.system.pending_transactions,s.system.stale_transports)); }
                if s.system.capacity_available() { ui.label(format!("Installing / scanning: {}",s.system.maintenance)); }
                if s.system.capacity_available() && s.system.cleanup_unconfirmed { ui.colored_label(egui::Color32::YELLOW,"Previous instance cleanup is unconfirmed — retained leases are not proof of a live DSP; new admission is blocked"); }
                ui.label("512 added frames recommended · 256 unqualified");
                ui.label(if s.capture["armed"]==true{"Crash capture: armed for next admitted launch"}else if s.capture["active_retention"].as_u64().unwrap_or(0)>0{"Crash capture: retaining an active instance"}else{"Crash capture: off"});
                if s.capture["armed"]==true { for action in &s.actions { if matches!(action.action,Action::CaptureDisarm{}) { Self::buttons(ui,std::slice::from_ref(action),busy,self.pending,&mut chosen); } } }
                if let Some(op)=&s.operation{egui::CollapsingHeader::new("Last operation receipt").show(ui,|ui|Self::value(ui,op));}
                egui::CollapsingHeader::new("Arturia environment and software center").default_open(true).show(ui,|ui|{
                    for app in &s.vendor_applications{ui.heading(&app.name);ui.label(format!("{} · {}",app.version,app.state));Self::buttons(ui,&app.actions,busy,self.pending,&mut chosen);}
                    for e in &s.environments{ui.label(format!("{} · revision {} · pinned runner {}",e.family,e.revision,e.runner));ui.small(&e.authorization);if !e.last_scan["id"].is_null(){ui.small(format!("Last scan: {} modules · completed at {}",e.last_scan["module_count"],e.last_scan["completed_at"]));ui.small(format!("Changes: {} added · {} changed · {} removed · {} unchanged",e.last_scan["changes"]["added"],e.last_scan["changes"]["changed"],e.last_scan["changes"]["removed"],e.last_scan["changes"]["unchanged"]));}Self::buttons(ui,&e.actions,busy,self.pending,&mut chosen);}
                });
                ui.separator();ui.heading("Add a plug-in");
                if ui.add_enabled(!self.pending,egui::Button::new("Add Windows installer").min_size(egui::vec2(240.0,48.0))).clicked(){pick=true;}
                ui.small("Select a local installer → review → create isolated environment → install → scan. New products stay unpublished.");
                for o in &s.onboarding {egui::Frame::group(ui.style()).show(ui,|ui|{
                    ui.heading(o.state.replace('_'," "));ui.label(format!("{} · {} bytes · {}",o.name,o.byte_size,o.format));
                    ui.label(&o.required_human_action);
                    if let Some(failure)=&o.failure { for line in failure_lines(failure) { ui.colored_label(egui::Color32::YELLOW,line); } }
                    ui.small(format!("SHA-256: {}",o.installer));if let Some(id)=&o.environment{ui.small(format!("Isolated environment: {id}"));}
                    Self::buttons(ui,&o.actions,busy,self.pending,&mut chosen);
                    egui::CollapsingHeader::new("Installation and scan details").id_salt((&o.installer,&o.environment)).show(ui,|ui|Self::value(ui,&o.details));
                });}
                ui.separator();ui.horizontal(|ui|{ui.label("Find a product");ui.text_edit_singleline(&mut self.filter);});
                for (state,label) in [("ready","Ready"),("needs_attention","Needs attention"),("installed_unqualified","Installed but unqualified"),("quarantined","Quarantined")]{
                    let products:Vec<_>=s.products.iter().filter(|p|p.disposition==state && format!("{} {}",p.name,p.vendor).to_lowercase().contains(&self.filter.to_lowercase())).collect();
                    ui.heading(format!("{} ({})",label,products.len()));
                    for p in products{egui::Frame::group(ui.style()).show(ui,|ui|{
                        ui.heading(&p.name);ui.label(format!("{} · {} · {}",p.vendor,p.role,p.version));
                        if let Some(rev)=p.active_revision{ui.label(format!("Active ordinary revision: {rev} · Recommended: {}",p.recommended_revision.map(|v|v.to_string()).unwrap_or_else(||"none".into())));}
                        for limit in &p.limitations{ui.small(limit.replace('_'," "));}
                        Self::buttons(ui,&p.actions,busy,self.pending,&mut chosen);
                        egui::CollapsingHeader::new("Revision history and exact details").id_salt((&p.class_id,&p.module_sha256)).show(ui,|ui|{
                            for h in &p.history{ui.label(format!("Revision {} · {}{}{}",h.revision,h.claim,if h.active{" · active"}else{""},if h.rollback_allowed && !h.active{" · rollback available"}else{""}));}
                            ui.small("Review candidates are retained history and cannot be activated here.");
                            ui.label(format!("Class: {}\nModule SHA-256: {}\nEnvironment: {}\nRunner: {}",p.class_id,p.module_sha256,p.environment,p.runner));Self::value(ui,&p.details);
                        });
                    });}
                }
                ui.separator();egui::CollapsingHeader::new("Recent incidents").show(ui,|ui|{
                    for incident in s.recent_incidents.iter().rev(){egui::CollapsingHeader::new(format!("{} · {}",incident.state,incident.id)).show(ui,|ui|{
                        for line in incident_lines(&incident.summary){ui.label(line);}
                        if let Some(a)=&incident.export{Self::buttons(ui,std::slice::from_ref(a),busy,self.pending,&mut chosen);}
                        egui::CollapsingHeader::new("Sanitized technical report").id_salt(&incident.id).show(ui,|ui|Self::value(ui,&incident.summary));
                    });}
                });
                Self::buttons(ui,&s.actions,busy,self.pending,&mut chosen);
                ui.separator();ui.small("Closing this window does not stop bridged audio or vendor applications. Vendor sign-in and authorization stay in the vendor's own interface.");
            });
        });
        if pick {
            self.request(Query::PickInstaller, ui.ctx());
        } else if let Some(a) = chosen {
            if let Some(s) = &self.snapshot {
                self.request(
                    Query::Action(Request {
                        schema: 2,
                        state_token: s.state_token.clone(),
                        action: a,
                    }),
                    ui.ctx(),
                );
            }
        } else if !self.pending && (refresh || self.refresh_after) {
            self.refresh_after = false;
            self.request(Query::Snapshot, ui.ctx());
        } else if !self.pending && self.last_poll.elapsed() > Duration::from_secs(2) {
            self.request(Query::Activity, ui.ctx());
        }
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }
}
fn failure_lines(f: &crate::model::OperationFailure) -> Vec<String> {
    let mut lines = vec![match f.stage {
        crate::model::FailureStage::OperatorValidationReadback => {
            "Manager could not finish validating state."
        }
        crate::model::FailureStage::EnvironmentCreationAdmission => {
            "Manager was busy before environment creation could start."
        }
    }
    .into()];
    if !f.mutation_started && !f.environment_created {
        lines.push("Environment creation did not start.".into());
    }
    if !f.installer_launched {
        lines.push("The installer was not launched; this is not an installer failure.".into());
    }
    lines.push(
        if f.retryable {
            "Retry is safe once the manager refreshes and canonical state is healthy."
        } else {
            "Manager access needs attention before retrying."
        }
        .into(),
    );
    lines.push(format!(
        "Validation lock: {:?}; {:?}; {} attempts, {:.1} ms elapsed; holder unknown.",
        f.lock.name,
        f.lock.outcome,
        f.lock.attempts,
        f.lock.elapsed_wait_us as f64 / 1000.0
    ));
    lines
}
fn incident_lines(v: &serde_json::Value) -> Vec<String> {
    let word = |key: &str| v[key].as_str().unwrap_or("unavailable").replace('_', " ");
    let confirmed = |key: &str| match v[key].as_bool() {
        Some(true) => "confirmed",
        Some(false) => "not confirmed",
        None => "unavailable",
    };
    vec![
        format!("Outcome: {}", word("outcome")),
        format!(
            "Cleanup: {} · Transport retirement: {}",
            confirmed("cleanup_confirmed"),
            confirmed("transport_retired")
        ),
        format!(
            "Outer runner exit (separate from Windows): {}",
            v["outer_exit"]
        ),
        format!(
            "Windows reported exit codes: {}",
            v["windows_self_exit_codes"]
        ),
        format!(
            "Retained exception observations: {}",
            v["exceptions"]
                .as_array()
                .map(|items| items.len().to_string())
                .unwrap_or_else(|| "unavailable".into())
        ),
    ]
}
fn refresh_for_receipt(old: &Option<serde_json::Value>, new: &Option<serde_json::Value>) -> bool {
    old != new
        && new
            .as_ref()
            .and_then(|v| v["state"].as_str())
            .is_some_and(|s| matches!(s, "vendor_running" | "completed" | "refused"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timeout_card_distinguishes_manager_refusal_from_installer_failure() {
        let f:crate::model::OperationFailure=serde_json::from_value(serde_json::json!({
            "layer":"manager_control_plane","stage":"operator_validation_readback","code":"registry_lock_timeout",
            "retryable":true,"mutation_started":false,"environment_created":false,"installer_launched":false,
            "lock":{"name":"registry.lock","mode":"exclusive","purpose":"operator_validation_readback","operation":"ab".repeat(16),"policy":"bounded","elapsed_wait_us":100000,"attempts":11,"timeout_ms":100,"outcome":"timeout","holder":"unknown"}
        })).unwrap();
        let text = failure_lines(&f).join("\n");
        for expected in [
            "Manager",
            "Environment creation did not start",
            "installer was not launched",
            "not an installer failure",
            "Retry is safe",
            "holder unknown",
        ] {
            assert!(text.contains(expected));
        }
        let mut f = f;
        f.stage = crate::model::FailureStage::EnvironmentCreationAdmission;
        f.lock.purpose = crate::model::LockPurpose::EnvironmentCreationAdmission;
        let text = failure_lines(&f).join(" ");
        assert!(text.contains("before environment creation"));
        assert!(text.contains("installer was not launched"));
        assert!(text.contains("Retry is safe"));
        f.retryable = false;
        assert!(!failure_lines(&f).join(" ").contains("Retry is safe"));
    }
    #[test]
    fn incident_summary_keeps_runner_exit_separate_and_missing_cleanup_unknown() {
        let v = serde_json::json!({"outcome":"process_scoped_vendor_retirement","outer_exit":-15,"windows_self_exit_codes":[],"exceptions":[]});
        let lines = incident_lines(&v).join("\n");
        assert!(lines.contains("process scoped vendor retirement"));
        assert!(lines.contains("Cleanup: unavailable"));
        assert!(lines.contains("separate from Windows): -15"));
        assert!(!lines.contains("crash"));
    }
    #[test]
    fn readback_waits_for_mutation_or_vendor_launch_receipt() {
        let initial = None;
        for state in ["queued", "waiting", "running"] {
            assert!(!refresh_for_receipt(
                &initial,
                &Some(serde_json::json!({"state":state}))
            ));
        }
        for state in ["vendor_running", "completed", "refused"] {
            let value = Some(serde_json::json!({"state":state}));
            assert!(refresh_for_receipt(&initial, &value));
            assert!(!refresh_for_receipt(&value, &value));
        }
    }
}
