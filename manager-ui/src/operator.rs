use crate::{
    client::{self, Query, Reply},
    model::*,
};
use eframe::egui;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
#[derive(Clone, Debug)]
struct RequestFeedback {
    action: Action,
    operation: Option<String>,
    text: String,
    terminal: bool,
    blocking: bool,
    release_after_snapshot: bool,
    acknowledgment_uncertain: bool,
    transitions: Vec<String>,
}
impl RequestFeedback {
    fn captured(action: Action) -> Self {
        Self {
            action,
            operation: None,
            text: "Click received. Waiting for the current readback; no operation submitted yet."
                .into(),
            terminal: false,
            blocking: true,
            release_after_snapshot: false,
            acknowledgment_uncertain: false,
            transitions: vec!["captured".into()],
        }
    }
    fn transition(&mut self, state: &str) {
        if self.transitions.last().is_none_or(|s| s != state) {
            if self.transitions.len() == 12 {
                self.transitions.remove(0);
            }
            self.transitions.push(state.into());
        }
    }
    fn receipt(&mut self, r: &Receipt) {
        self.acknowledgment_uncertain = r.accepted && r.operation.is_none();
        self.transition(if r.accepted { "accepted" } else { "refused" });
        self.operation = r.operation.clone();
        self.terminal = !r.accepted;
        self.release_after_snapshot = !r.accepted;
        self.text = if r.accepted {
            format!(
                "Request accepted · {}. Waiting for its result.",
                r.operation.as_deref().unwrap_or("identity unavailable")
            )
        } else {
            format!(
                "This request was refused: {}",
                r.refusal.as_deref().unwrap_or("reason unavailable")
            )
        };
    }
    fn observe(&mut self, op: &serde_json::Value) {
        if self.terminal
            || self.operation.as_deref() != op["operation"].as_str()
            || self.operation.is_none()
        {
            return;
        }
        if let Some(
            state @ ("refused" | "completed" | "waiting" | "validating" | "vendor_running"
            | "queued" | "running"),
        ) = op["state"].as_str()
        {
            self.transition(state);
        }
        self.text = match op["state"].as_str() {
            Some("refused") => {
                self.terminal = true;
                self.release_after_snapshot = true;
                format!(
                    "This request was refused: {}",
                    op["reason"].as_str().unwrap_or("reason unavailable")
                )
            }
            Some("completed") => {
                self.terminal = true;
                self.release_after_snapshot = true;
                "This operation completed. Refreshing its result…".into()
            }
            Some("waiting" | "validating") => {
                "This operation is validating manager state. Do not click again.".into()
            }
            Some("vendor_running") => {
                self.release_after_snapshot = true;
                "Vendor operation is running. Refreshing its controls…".into()
            }
            Some("queued" | "running") => {
                "This operation is in progress. Do not click again.".into()
            }
            _ => return,
        };
    }
    fn refreshed(&mut self) {
        if self.release_after_snapshot {
            self.blocking = false;
            self.release_after_snapshot = false;
            if self.text == "This operation completed. Refreshing its result…" {
                self.text = "This operation completed. Its current result is shown below.".into();
            } else if self.text == "Vendor operation is running. Refreshing its controls…" {
                self.text = "Installer is supervised and running. Use its exact Focus or Stop control below.".into();
            }
        }
    }
    fn reconcile_snapshot(&mut self, s: &Snapshot) {
        // A lost acknowledgment is not a failed operation. Only exact canonical
        // onboarding ownership plus its offered recovery action can resolve it.
        if self.acknowledgment_uncertain && self.operation.is_none() {
            if let Action::InstallerStart { onboarding } = &self.action {
                let owners: Vec<_> = s.onboarding.iter().filter(|r| r.environment.as_ref() == Some(onboarding))
                    .filter_map(|r| {
                        let op = r.details["installation"]["operation"].as_str()?;
                        let exact = r.actions.iter().any(|a| matches!(&a.action,
                            Action::InstallerStop { onboarding: id, operation } if id == onboarding && operation == op));
                        (exact && matches!(r.details["installation"]["state"].as_str().unwrap_or(r.state.as_str()), "starting" | "running" | "unknown")).then_some(op.to_owned())
                    }).collect();
                if owners.len() == 1 {
                    self.operation = Some(owners[0].clone());
                    self.transition("live_owner_reconciled");
                    self.release_after_snapshot = true;
                    self.text = "Submission acknowledgment was unavailable. The manager confirms this exact live installer; recovery controls are available. No request was resubmitted.".into();
                }
            }
        }
        let matching_operation = self.operation.is_some()
            && s.operation
                .as_ref()
                .is_some_and(|op| op["operation"].as_str() == self.operation.as_deref());
        let exact_recovery = if let Action::InstallerStart { onboarding } = &self.action {
            s.onboarding.iter().any(|r| {
                r.environment.as_ref() == Some(onboarding)
                    && r.actions.iter().any(|a| {
                        matches!(&a.action, Action::InstallerStop{onboarding:id,operation}
                    if id==onboarding && Some(operation.as_str())==self.operation.as_deref())
                    })
            })
        } else {
            false
        };
        for product in &s.products {
            if let Some(op)=product.details["preparation"].get("operation") {self.observe(op);}
            if let Some(rows)=product.details["preparation"]["candidates"].as_array(){for row in rows{if let Some(op)=row.get("operation"){self.observe(op);}}}
        }
        if let Some(op) = &s.operation {
            self.observe(op);
        }
        if self.terminal || matching_operation || exact_recovery {
            self.refreshed();
        }
    }
    fn for_installer(&self, id: &str) -> bool {
        matches!(&self.action,Action::InstallerEnvironmentCreate{installer,..} if installer==id)
    }
}
pub struct Operator {
    snapshot: Option<Snapshot>,
    sender: mpsc::Sender<Reply>,
    receiver: mpsc::Receiver<Reply>,
    pending: bool,
    background_poll: bool,
    action_inflight: bool,
    queued_action: Option<Request>,
    feedback: Option<RequestFeedback>,
    last_poll: Instant,
    message: String,
    filter: String,
    refresh_after: bool,
    product_form: Option<Action>,
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
            background_poll: false,
            action_inflight: false,
            queued_action: None,
            feedback: None,
            last_poll: Instant::now(),
            message: "Reading installed manager…".into(),
            filter: String::new(),
            refresh_after: false,
            product_form: None,
        }
    }
    fn preparation_details(ui:&mut egui::Ui,v:&serde_json::Value) {
        if v["publication"]=="ordinary" {ui.strong("Ordinarily published");}
        if let Some(history)=v["candidates"].as_array(){
            egui::CollapsingHeader::new("Candidate generations and their own results").id_salt((v["selection"].as_str(),"candidate-history")).default_open(history.len()>1).show(ui,|ui|{
                for c in history {ui.group(|ui|{ui.label(format!("Candidate {} · {} · {}",c["id"].as_str().unwrap_or("?"),c["disposition"].as_str().unwrap_or("?"),c["publication"].as_str().unwrap_or("?")));if c["publication_acceptance_sealed"]==true && c["unmet_requirements"].as_array().is_some_and(|r|!r.is_empty()){ui.colored_label(egui::Color32::YELLOW,"This publication retains its original acceptance. New evidence needs review; use Withdraw to disable it.");}Self::value(ui,c);});}
            });
        }
        if let Some(reason)=v["recovery"].as_str(){ui.colored_label(egui::Color32::YELLOW,reason);}
        if let Some(reason)=v["build_prerequisite"].as_str(){ui.label(reason);}
        egui::CollapsingHeader::new("Inspection generations").id_salt((v["selection"].as_str(),"inspection-history")).show(ui,|ui|Self::value(ui,&v["inspections"]));
        ui.label(format!("Inspection: {} · Preparation: {} · Publication: {}",v["inspection"].as_str().unwrap_or("unknown"),v["preparation"].as_str().unwrap_or("unknown"),v["publication"].as_str().unwrap_or("unknown")));
        ui.small("Preliminary inspection is not a guarantee of DAW audio, editor, automation, or state recall.");
        if v["origin"]=="retained_sv1" {ui.small("Adopted exact SV1 work; originally prepared through the engineering CLI.");}
        if let Some(reason)=v["controller"]["reason"].as_str(){ui.small(reason);}
        if !v["preliminary"].is_null() {
            egui::CollapsingHeader::new("Preliminary compatibility details").id_salt(("inspection",v["selection"].as_str())).show(ui,|ui|{
                let f=&v["preliminary"];
                ui.label(format!("Parameters: {} · Positive inspection cleanup: {}",f["parameter_count"],f["cleanup_confirmed"]));
                if f["editor_interface"].is_null(){ui.label("Editor interface: not retained by the original inspector");}
                else {ui.label(format!("Editor created: {} · HWND interface result: {} · Not attached",f["editor_interface"]["created"],f["editor_interface"]["hwnd_result"]));}
                ui.monospace(serde_json::to_string_pretty(f).unwrap_or_default());
            });
        }
        if let Some(op)=v.get("operation").filter(|o|!o.is_null()) {
            ui.label(format!("This product's operation: {}",op["state"].as_str().unwrap_or("unknown")));
            if let Some(stage)=op["preparation_failure"]["attempted_stage"].as_str(){ui.label(format!("Stopped during {}",stage.replace('_'," ")));}
            if let Some(reason)=op["reason"].as_str(){ui.colored_label(egui::Color32::YELLOW,reason);}
            if let Some(recovery)=op["preparation_failure"]["recovery"].as_str(){ui.small(recovery);}
        }
        egui::CollapsingHeader::new("Recorded tests and remaining qualification").id_salt(v["selection"].as_str()).default_open(false).show(ui,|ui|{
            for area in ["daw_load","midi","audio","editor","parameters","automation","state_recall","processing_restart","retirement"] {
                let latest=v["evidence"].as_array().and_then(|rows|rows.iter().rev().find(|r|r["area"]==area));
                ui.label(format!("{}: {}",area.replace('_'," "),latest.and_then(|r|r["status"].as_str()).unwrap_or("not tested")));
                if let Some(row)=latest{ui.small(format!("{} · {}",row["witness"].as_str().unwrap_or("unavailable"),row["detail"].as_str().unwrap_or("")));}
            }
            if let Some(rows)=v["unmet_requirements"].as_array(){for row in rows{if let Some(text)=row.as_str(){ui.small(text);}}}
        });
    }
    fn request(&mut self, q: Query, ctx: &egui::Context) {
        self.pending = true;
        self.background_poll = matches!(&q, Query::Activity);
        self.action_inflight = matches!(&q, Query::Action(_));
        if self.action_inflight {
            if let Some(f) = &mut self.feedback {
                f.text = "Submitting this request to the manager…".into();
            }
        }
        self.last_poll = Instant::now();
        client::send(q, self.sender.clone(), ctx.clone());
    }
    fn controls_pending(&self) -> bool {
        self.queued_action.is_some()
            || self.feedback.as_ref().is_some_and(|f| f.blocking)
            || (self.pending && !self.background_poll)
    }
    fn capture_action(&mut self, request: Request) {
        self.feedback = Some(RequestFeedback::captured(request.action.clone()));
        self.queued_action = Some(request);
    }
    fn next_action(&mut self) -> Option<Request> {
        if self.pending {
            None
        } else {
            self.queued_action.take()
        }
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
            if let Some(reason) = reason.or(pending.then_some(
                "Waiting for this request or canonical readback; duplicate submission is blocked",
            )) {
                response.on_disabled_hover_text(reason);
                ui.small(reason);
            }
        }
    }
    fn handle_reply(&mut self, reply: Reply) {
        let was_action = self.action_inflight;
        self.pending = false;
        self.background_poll = false;
        self.action_inflight = false;
        match reply {
            Reply::Snapshot(s) => {
                if s.schema != 4 {
                    self.message = "Unsupported manager schema".into();
                } else {
                    if let Some(f) = &mut self.feedback {
                        f.reconcile_snapshot(&s);
                    }
                    self.snapshot = Some(*s);
                    self.message = "Canonical installed state refreshed".into();
                }
            }
            Reply::Receipt(r) => {
                if let Some(f) = &mut self.feedback {
                    f.receipt(&r);
                }
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
                    if let Some(f) = &mut self.feedback {
                        f.observe(op);
                    }
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
                if was_action {
                    if let Some(f) = &mut self.feedback {
                        f.acknowledgment_uncertain = true;
                        f.transition("acknowledgment_unconfirmed");
                        f.text=format!("Request acknowledgment is unavailable: {e}. Reconciling manager ownership; no automatic resubmission.");
                        self.refresh_after = true;
                    }
                }
                self.message = e;
                if let Some(s) = &mut self.snapshot {
                    s.system.service = "capacity unavailable".into();
                    s.system.cleanup_unconfirmed = true;
                }
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
            self.handle_reply(reply);
        }

        let controls_pending = self.controls_pending();
        let mut refresh = false;
        let mut pick = false;
        let mut chosen = None;
        egui::CentralPanel::default().show(ui,|ui|{
            ui.heading("Linux Audio Compatibility Manager");
            ui.horizontal(|ui|{if ui.add_enabled(!self.pending,egui::Button::new("Refresh")).clicked(){refresh=true;}ui.label(self.feedback.as_ref().map_or(self.message.as_str(),|f|f.text.as_str()));if self.pending{ui.spinner();}});
            if let Some(f) = &self.feedback {
                ui.collapsing("Request status", |ui| {
                    ui.label(f.operation.as_deref().unwrap_or("Operation identity not yet confirmed"));
                    ui.label(f.transitions.join(" → "));
                });
            }
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
                if s.capture["armed"]==true { for action in &s.actions { if matches!(action.action,Action::CaptureDisarm{}) { Self::buttons(ui,std::slice::from_ref(action),busy,controls_pending,&mut chosen); } } }
                if let Some(op)=&s.operation{egui::CollapsingHeader::new("Last operation receipt").show(ui,|ui|Self::value(ui,op));}
                ui.separator();ui.horizontal(|ui|{ui.label("Find a product");ui.text_edit_singleline(&mut self.filter);});
                for (state,label) in [("another_configuration","Another configuration published"),("experimental","Experimental use enabled"),("prepared","Test instances prepared"),("ready","Ready"),("needs_attention","Needs attention"),("installed_unqualified","Installed but unqualified"),("quarantined","Quarantined")]{
                    let products:Vec<_>=s.products.iter().filter(|p|p.disposition==state && format!("{} {}",p.name,p.vendor).to_lowercase().contains(&self.filter.to_lowercase())).collect();
                    ui.heading(format!("{} ({})",label,products.len()));
                    for p in products{egui::Frame::group(ui.style()).show(ui,|ui|{
                        ui.heading(&p.name);ui.label(format!("{} · {} · {}",p.vendor,p.role,p.version));
                        let live=s.active_sessions.iter().filter(|o|o["class_id"]==p.class_id).count();if live>0{ui.label(format!("Live instances: {live}. Closing the manager leaves them running."));}
                        if let Some(rev)=p.active_revision{ui.label(format!("Published revision: {rev} · Recommended: {}",p.recommended_revision.map(|v|v.to_string()).unwrap_or_else(||"none".into())));}
                        for limit in &p.limitations{ui.small(limit.replace('_'," "));}
                        if let Some(hint)=p.details["inspection_hint"].as_str(){ui.label(hint);}
                        Self::buttons(ui,&p.actions,busy,controls_pending,&mut chosen);
                        if let Some(v)=p.details.get("preparation") { Self::preparation_details(ui,v); }
                        if let Some(reason)=p.details["preparation_failure"].as_str(){ui.colored_label(egui::Color32::YELLOW,reason);}
                        egui::CollapsingHeader::new("Revision history and exact details").id_salt((&p.class_id,&p.module_sha256)).show(ui,|ui|{
                            for h in &p.history{ui.label(format!("Revision {} · {}{}{}",h.revision,h.claim,if h.active{" · active"}else{""},if h.rollback_allowed && !h.active{" · rollback available"}else{""}));}
                            ui.small("Experimental publication is a separate explicit choice. Candidate history is not a qualification decision.");
                            ui.label(format!("Class: {}\nModule SHA-256: {}\nEnvironment: {}\nRunner: {}",p.class_id,p.module_sha256,p.environment,p.runner));Self::value(ui,&p.details);
                        });
                    });}
                }
                egui::CollapsingHeader::new("Arturia environment and software center").default_open(true).show(ui,|ui|{
                    for app in &s.vendor_applications{ui.heading(&app.name);ui.label(format!("{} · {}",app.version,app.state));Self::buttons(ui,&app.actions,busy,controls_pending,&mut chosen);}
                    for e in &s.environments{ui.label(format!("{} · revision {} · pinned runner {}",e.family,e.revision,e.runner));ui.small(&e.authorization);if !e.last_scan["id"].is_null(){ui.small(format!("Last scan: {} modules · completed at {}",e.last_scan["module_count"],e.last_scan["completed_at"]));ui.small(format!("Changes: {} added · {} changed · {} removed · {} unchanged",e.last_scan["changes"]["added"],e.last_scan["changes"]["changed"],e.last_scan["changes"]["removed"],e.last_scan["changes"]["unchanged"]));}Self::buttons(ui,&e.actions,busy,controls_pending,&mut chosen);}
                });
                ui.separator();ui.heading("Add a plug-in");
                if ui.add_enabled(!self.pending,egui::Button::new("Add Windows installer").min_size(egui::vec2(240.0,48.0))).clicked(){pick=true;}
                ui.small("Select a local installer → review → create isolated environment → install → scan. New products stay unpublished.");
                for o in s.onboarding.iter().rev() {let historical=matches!(o.state.as_str(),"cancelled"|"failed"|"no_audio_plugin_discovered");egui::CollapsingHeader::new(if historical{format!("Earlier attempt — {}",o.state.replace('_'," "))}else{format!("Current attempt — {}",o.state.replace('_'," "))}).id_salt((&o.installer,&o.environment,"attempt")).default_open(!historical).show(ui,|ui|{
                    ui.heading(o.state.replace('_'," "));ui.label(format!("{} · {} bytes · {}",o.name,o.byte_size,o.format));
                    ui.label(&o.required_human_action);
                    for line in installer_lines(&o.details["installation"]) { ui.label(line); }
                    if let Some(f)=self.feedback.as_ref().filter(|f|f.for_installer(&o.installer) || matches!(&f.action, Action::InstallerNewAttempt {previous,..} if o.environment.as_ref()==Some(previous))) {ui.colored_label(egui::Color32::YELLOW,&f.text);}
                    else if let Some(reason)=o.details["request_result"]["reason"].as_str(){ui.colored_label(egui::Color32::YELLOW,format!("Last request refused before worker launch: {reason}"));}
                    if let Some(failure)=&o.failure { for line in failure_lines(failure) { ui.colored_label(egui::Color32::YELLOW,line); } }
                    ui.small(format!("SHA-256: {}",o.installer));if let Some(id)=&o.environment{ui.small(format!("Isolated environment: {id}"));}
                    Self::buttons(ui,&o.actions,busy,controls_pending,&mut chosen);
                    egui::CollapsingHeader::new("Installation and scan details").id_salt((&o.installer,&o.environment)).show(ui,|ui|Self::value(ui,&o.details));
                });}
                ui.separator();egui::CollapsingHeader::new("Recent incidents").show(ui,|ui|{
                    for incident in s.recent_incidents.iter().rev(){egui::CollapsingHeader::new(format!("{} · {}",incident.state,incident.id)).show(ui,|ui|{
                        for line in incident_lines(&incident.summary){ui.label(line);}
                        if let Some(a)=&incident.export{Self::buttons(ui,std::slice::from_ref(a),busy,controls_pending,&mut chosen);}
                        egui::CollapsingHeader::new("Sanitized technical report").id_salt(&incident.id).show(ui,|ui|Self::value(ui,&incident.summary));
                    });}
                });
                Self::buttons(ui,&s.actions,busy,controls_pending,&mut chosen);
                ui.separator();ui.small("Closing this window does not stop bridged audio or vendor applications. Vendor sign-in and authorization stay in the vendor's own interface.");
            });
        });
        if let Some(a)=chosen.take() {
            if matches!(a,Action::CandidateObserve{..}|Action::CandidateReview{..}) {self.product_form=Some(a);} else {chosen=Some(a);}
        }
        if let Some(form)=self.product_form.as_mut() {
            let mut submit=false;let mut cancel=false;
            egui::Window::new("Exact candidate observation / review").collapsible(false).show(ui.ctx(),|ui|{
                match form {
                    Action::CandidateObserve{area,status,note,..}=>{
                        ui.label("This records your observation. It does not create a machine measurement or qualification decision.");
                        egui::ComboBox::from_id_salt("area").selected_text(area.as_str()).show_ui(ui,|ui|{for a in ["daw_load","midi","audio","editor","parameters","automation","state_recall","processing_restart","retirement"]{ui.selectable_value(area,a.into(),a.replace('_'," "));}});
                        egui::ComboBox::from_id_salt("status").selected_text(status.as_str()).show_ui(ui,|ui|{for a in ["passed","failed","not_tested","unavailable","not_applicable"]{ui.selectable_value(status,a.into(),a.replace('_'," "));}});
                        ui.add(egui::TextEdit::multiline(note).char_limit(512));
                        submit=ui.add_enabled(!note.trim().is_empty(),egui::Button::new("Record operator observation")).clicked();
                    }
                    Action::CandidateReview{accept,rationale,..}=>{
                        ui.label(if *accept{"Maintainer review: accept this exact local configuration against the retained results. This is not a project-wide support claim. Publication remains a separate action."}else{"Record why this exact configuration needs more work. Publication is unchanged."});
                        ui.add(egui::TextEdit::multiline(rationale).char_limit(512));
                        submit=ui.add_enabled(!rationale.trim().is_empty(),egui::Button::new("Record explicit review decision")).clicked();
                    }
                    _=>cancel=true,
                }
                cancel|=ui.button("Cancel").clicked();
            });
            if submit{chosen=self.product_form.take();}else if cancel{self.product_form=None;}
        }
        if pick {
            self.request(Query::PickInstaller, ui.ctx());
        } else if let Some(a) = chosen {
            if let Some(s) = &self.snapshot {
                self.capture_action(Request {
                    schema: 4,
                    state_token: s.state_token.clone(),
                    action: a,
                });
            }
        }
        if let Some(request) = self.next_action() {
            self.request(Query::Action(request), ui.ctx());
        } else if !self.pending && (refresh || self.refresh_after) {
            self.refresh_after = false;
            self.request(Query::Snapshot, ui.ctx());
        } else if !self.pending && self.last_poll.elapsed() > Duration::from_secs(2) {
            self.request(Query::Activity, ui.ctx());
        }
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }
}
fn installer_lines(v: &serde_json::Value) -> Vec<String> {
    let t=&v["transaction"];
    if t["schema"] != 1 || t["operation"] != v["operation"] {
        return if v["error"] == "installer_launcher_failed" {
            vec!["Earlier installer result: outer launch route exited nonzero; the failing later stage and installation completeness were not captured.".into()]
        } else {vec![]};
    }
    let outcome=match t["outcome"].as_str() {
        Some("in_progress")=>"Installer supervision is ongoing. Exact Focus and Stop controls remain available; cleanup will be checked after retirement.",
        Some("outer_nonzero_stage_unknown")=>"The outer installer route exited nonzero. The failing child or stage is not established.",
        Some("installed_dependency_failed")=>"Application files are installed, but a process performing service/dependency work exited nonzero. Review that stage before reinstalling.",
        Some("child_failed")=>"An owned child exited nonzero. Its role and underlying cause may still be unknown.",
        Some("cancelled")=>"This attempt was cancelled. Earlier failure observations remain retained.",
        Some("cleanup_unconfirmed")=>"Installer cleanup is unconfirmed. Further work is blocked.",
        Some("installed")=>"Application files and installation registration were observed. First launch and dependency health remain unproved.",
        Some("partial_installation")=>"Partial installation: durable changes exist, but a complete application installation is not established.",
        Some("not_installed")=>"No durable installation was found in the inspected surfaces.",
        _=>"Review the installer stage record; completion does not qualify or publish a plug-in.",
    };
    let mut lines=vec![outcome.into()];
    if let Some(code)=v["startup"]["first_problem"]["code"].as_str() {
        let observation=match code {
            "native_steamclient_load_failed"=>"native runtime dependency load failed",
            "native_steamclient_export_unavailable"=>"native runtime export unavailable",
            "runtime_assertion_observed"=>"runtime assertion observed",
            "prefix_initialization_failed"|"prefix_initialization_timeout"=>"environment initialization did not complete",
            _=>"startup problem retained; inspect bounded details",
        };
        lines.push(format!("Earlier startup observation: {observation}. Cancellation and cleanup do not erase it."));
    }
    if let Some(n)=t["outer_launcher_exit"].as_i64(){lines.push(format!("Outer launcher exit: {n} (separate from payload and service exits)"));}
    lines.push(format!("Durable installation: {}",t["durable_installation"].as_str().unwrap_or("unavailable").replace('_'," ")));
    if !t["first_failure"].is_null() {
        let f=&t["first_failure"];
        lines.push(format!("First retained process result: phase {} · role {} · relationship {} · domain {} · status {} · cause unestablished",
            f["phase"].as_str().unwrap_or("unknown"), f["role"].as_str().unwrap_or("unknown"),
            f["relationship"].as_str().unwrap_or("unknown"), f["domain"].as_str().unwrap_or("unknown"), f["status"]));
    }
    let binding=&t["launch_binding"];
    if binding["schema"]==1 && binding["operation"]==v["operation"] {
        lines.push(if binding["status"]=="bound" {"Installer Windows root: exact operation and launch generation observed.".into()}
            else {format!("Installer Windows root unavailable: {}. Child attribution may be incomplete.",binding["reason"].as_str().unwrap_or("missing observation"))});
    }
    if t["presence_close"]["schema"]==1 {
        lines.push(format!("Application presence/close requests observed: {}. Helper success does not prove a match or successful closure; exact match and recheck results remain unavailable.",t["presence_close"]["observation_count"]));
        if t["presence_close"]["operation_classes"].as_array().is_some_and(|rows| rows.iter().any(|r| r=="presence_query" || r=="close_request")) {
            lines.push("Observed request mechanism: PowerShell/CIM process query or script process close. The matched object and actual close outcome are unavailable.".into());
        }
    }
    lines.push(if v["cleanup_confirmed"]==true {"Owned process cleanup confirmed."} else {"Owned process cleanup not yet confirmed."}.into());
    lines
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
    #[test]
    fn installer_root_and_presence_do_not_claim_close_from_helper_success() {
        let mut v=serde_json::json!({"operation":"exact","cleanup_confirmed":true,"transaction":{
            "schema":1,"operation":"exact","outcome":"outer_nonzero_stage_unknown","durable_installation":"partial_installation",
            "launch_binding":{"schema":1,"operation":"exact","status":"bound"},
            "presence_close":{"schema":1,"observation_count":3,"operation_classes":["presence_query","close_request","presence_query"]}}});
        let text=super::installer_lines(&v).join(" ");assert!(text.contains("exact operation and launch generation"));
        assert!(text.contains("Helper success does not prove") && text.contains("results remain unavailable"));
        assert!(text.contains("PowerShell/CIM") && text.contains("matched object and actual close outcome are unavailable"));
        v["transaction"]["launch_binding"]["operation"]="unrelated".into();
        assert!(!super::installer_lines(&v).join(" ").contains("exact operation and launch generation"));
        v["transaction"]["launch_binding"]["operation"]="exact".into();v["transaction"]["launch_binding"]["status"]="unavailable".into();
        assert!(super::installer_lines(&v).join(" ").contains("Child attribution may be incomplete"));
    }
    #[test]
    fn first_failure_presents_exact_stage_role_relationship_domain_without_guessing() {
        for role in [Some("service_dependency"), None] {
            for (domain, status) in [("wine_self_exit_observation", 3010), ("linux_wait", 2)] {
                let v = serde_json::json!({"operation":"exact","cleanup_confirmed":true,
                    "transaction":{"schema":1,"operation":"exact","outcome":"cancelled",
                        "outer_launcher_exit":-15,"durable_installation":"partial_installation",
                        "first_failure":{"phase":"target_runner","role":role,
                            "relationship":"descendant","domain":domain,"status":status}}});
                let lines = super::installer_lines(&v);
                let failure = lines.iter().find(|l| l.starts_with("First retained")).unwrap();
                assert_eq!(failure, &format!("First retained process result: phase target_runner · role {} · relationship descendant · domain {domain} · status {status} · cause unestablished", role.unwrap_or("unknown")));
                assert!(lines.iter().any(|l| l.contains("Outer launcher exit: -15")));
                assert!(lines.iter().any(|l| l.contains("cleanup confirmed")));
            }
        }
    }
    #[test]
    fn installer_partial_nonzero_cancellation_and_cleanup_stay_separate() {
        let mut v=serde_json::json!({"operation":"exact","cleanup_confirmed":true,"startup":{"first_problem":{"code":"runtime_assertion_observed"}},"transaction":{"schema":1,"operation":"exact","outcome":"cancelled","outer_launcher_exit":-15,"durable_installation":"partial_installation","first_failure":{"domain":"linux_wait","status":37}}});
        let lines=super::installer_lines(&v).join(" ");
        assert!(lines.contains("runtime assertion observed") && lines.contains("cancelled") && lines.contains("status 37") && lines.contains("partial installation") && lines.contains("cleanup confirmed"));
        v["transaction"]["outcome"]="in_progress".into();
        let ongoing=super::installer_lines(&v).join(" ");assert!(ongoing.contains("Exact Focus and Stop"));assert!(!ongoing.contains("Further work is blocked"));
        v["transaction"]["operation"]="other".into();assert!(super::installer_lines(&v).is_empty());
        let legacy=serde_json::json!({"error":"installer_launcher_failed"});
        assert!(super::installer_lines(&legacy)[0].contains("failing later stage"));
    }

    use super::*;
    fn state_fixture() -> Operator {
        let (sender, receiver) = mpsc::channel();
        Operator {
            snapshot: None,
            sender,
            receiver,
            pending: false,
            background_poll: false,
            action_inflight: false,
            queued_action: None,
            feedback: None,
            last_poll: Instant::now(),
            message: String::new(),
            filter: String::new(),
            refresh_after: false,
            product_form: None,
        }
    }
    fn create_request() -> Request {
        Request {
            schema: 4,
            state_token: "snapshot".into(),
            action: Action::InstallerEnvironmentCreate {
                installer: "ab".repeat(32),
                runner: "cd".repeat(32),
            },
        }
    }
    fn running_snapshot(operation: &str) -> Snapshot {
        let id = "aa".repeat(16);
        Snapshot {
            schema: 4,
            state_token: "current".into(),
            system: System {
                service: "capacity unavailable".into(),
                keepers: 0,
                dsp: 0,
                maintenance: 0,
                ceiling: 0,
                pending_transactions: 0,
                stale_transports: 0,
                cleanup_unconfirmed: true,
            },
            onboarding: vec![Onboarding {
                failure: None,
                installer: "bb".repeat(32),
                name: "Installer".into(),
                byte_size: 10,
                format: "pe_executable".into(),
                environment: Some(id.clone()),
                state: "running".into(),
                required_human_action: "installer_ui".into(),
                details: serde_json::json!({"installation":{"operation":operation}}),
                actions: vec![AvailableAction {
                    label: "Stop installer".into(),
                    action: Action::InstallerStop {
                        onboarding: id,
                        operation: operation.into(),
                    },
                    disabled_reason: None,
                }],
            }],
            environments: vec![],
            vendor_applications: vec![],
            products: vec![],
            active_sessions: vec![],
            capture: serde_json::Value::Null,
            recent_incidents: vec![],
            actions: vec![],
            operation: Some(serde_json::json!({"operation":operation,"state":"vendor_running"})),
        }
    }
    fn start_feedback(o: &mut Operator, acknowledged: bool) {
        o.feedback = Some(RequestFeedback::captured(Action::InstallerStart {
            onboarding: "aa".repeat(16),
        }));
        if acknowledged {
            o.handle_reply(Reply::Receipt(Receipt {
                schema: 4,
                accepted: true,
                operation: Some("current-op".into()),
                refusal: None,
            }));
        } else {
            o.action_inflight = true;
            o.handle_reply(Reply::Error("ack lost".into()));
        }
    }
    fn activity_from(s: &Snapshot) -> Reply {
        Reply::Activity(Activity {
            schema: 4,
            system: s.system.clone(),
            capture: s.capture.clone(),
            operation: s.operation.clone(),
        })
    }
    #[test]
    fn terminal_retry_offers_survive_snapshot_and_poll_without_frontend_inference() {
        // These are manager-projected offers, not frontend eligibility rules.
        // Cover both eligible completed outcomes and the three explicit refusals.
        for (durable, outcome, linked, offered, disabled) in [
            ("not_installed", "not_installed", false, true, None),
            ("partial_installation", "partial_installation", false, true, None),
            ("installed", "installed", false, false, None),
            ("partial_installation", "cleanup_unconfirmed", false, false, None),
            ("partial_installation", "partial_installation", true, false, None),
            ("partial_installation", "partial_installation", false, true, Some("active DSP")),
        ] {
            let mut s = running_snapshot("prior-op");
            s.system.service = "active".into();
            s.system.cleanup_unconfirmed = outcome == "cleanup_unconfirmed";
            s.operation = Some(serde_json::json!({"operation":"prior-op","state":"completed"}));
            let card = &mut s.onboarding[0];
            card.state = "completed".into();
            card.details = serde_json::json!({"installation":{"operation":"prior-op","state":"completed",
                "transaction":{"schema":1,"operation":"prior-op","outcome":outcome,"durable_installation":durable}},"linked_attempt":linked});
            let action = Action::InstallerNewAttempt { previous:card.environment.clone().unwrap(), runner:"cd".repeat(32) };
            card.actions = if offered { vec![AvailableAction { label:"New isolated attempt".into(),
                action:action.clone(), disabled_reason:disabled.map(Into::into) }] } else { vec![] };
            let mut o = state_fixture(); // reopening receives canonical snapshot
            o.handle_reply(Reply::Snapshot(Box::new(s.clone())));
            o.handle_reply(activity_from(&s));
            let ctx = egui::Context::default();
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            for pressed in [None, Some(true), Some(false)] {
                o.pending = pressed == Some(false);
                o.background_poll = o.pending;
                let events = pressed.map(|pressed| vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton { pos:point, button:egui::PointerButton::Primary,
                        pressed, modifiers:egui::Modifiers::NONE }]).unwrap_or_default();
                let mut output = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
                    point = ui.next_widget_position() + egui::vec2(12.0, 12.0);
                    let projected = o.snapshot.as_ref().unwrap();
                    Operator::buttons(ui, &projected.onboarding[0].actions, projected.system.inactive_reason(),
                        o.controls_pending(), &mut chosen);
                });
                output.textures_delta.clear();
            }
            assert_eq!(chosen, if offered && disabled.is_none() { Some(action) } else { None },
                "{durable}/{outcome}, linked={linked}");
        }
    }
    #[test]
    fn integrated_snapshot_first_and_activity_first_reconcile_stop() {
        for snapshot_first in [true, false] {
            let mut o = state_fixture();
            start_feedback(&mut o, true);
            assert!(o.controls_pending());
            let s = running_snapshot("current-op");
            if !snapshot_first {
                o.handle_reply(activity_from(&s));
                assert!(o.controls_pending());
            }
            o.handle_reply(Reply::Snapshot(Box::new(s.clone())));
            assert!(!o.controls_pending());
            o.handle_reply(activity_from(&s));
            assert!(
                !o.controls_pending(),
                "equal running must not require another transition"
            );
            let ctx = egui::Context::default();
            let mut chosen = None;
            let mut point = egui::Pos2::ZERO;
            for pressed in [None, Some(true), Some(false)] {
                o.pending = pressed == Some(false);
                o.background_poll = o.pending;
                let events = pressed
                    .map(|pressed| {
                        vec![
                            egui::Event::PointerMoved(point),
                            egui::Event::PointerButton {
                                pos: point,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ]
                    })
                    .unwrap_or_default();
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        point = ui.next_widget_position() + egui::vec2(12.0, 12.0);
                        Operator::buttons(
                            ui,
                            &s.onboarding[0].actions,
                            s.system.inactive_reason(),
                            o.controls_pending(),
                            &mut chosen,
                        );
                    },
                );
                output.textures_delta.clear();
            }
            assert!(matches!(chosen, Some(Action::InstallerStop { .. })));
            assert!(!s.onboarding[0]
                .actions
                .iter()
                .any(|a| matches!(a.action, Action::InstallerStart { .. })));
        }
    }
    #[test]
    fn integrated_unrelated_snapshot_cannot_unlock_request() {
        let mut o = state_fixture();
        start_feedback(&mut o, true);
        o.handle_reply(Reply::Snapshot(Box::new(running_snapshot("old-op"))));
        assert!(o.controls_pending());
        assert!(!o.feedback.as_ref().unwrap().terminal);
        o.handle_reply(activity_from(&running_snapshot("old-op")));
        assert!(o.controls_pending());
        o.handle_reply(activity_from(&running_snapshot("current-op")));
        o.handle_reply(Reply::Snapshot(Box::new(running_snapshot("old-op"))));
        assert!(
            o.controls_pending(),
            "unrelated snapshot cannot release a prior running transition"
        );
    }
    #[test]
    fn uncertain_ack_reconciles_exact_live_owner_without_resubmission() {
        let mut o = state_fixture();
        start_feedback(&mut o, false);
        assert!(!o.feedback.as_ref().unwrap().terminal);
        assert!(o.controls_pending());
        let mut unrelated = running_snapshot("other-op");
        unrelated.onboarding[0].environment = Some("cc".repeat(16));
        o.handle_reply(Reply::Snapshot(Box::new(unrelated)));
        assert!(o.controls_pending());
        let s = running_snapshot("current-op");
        o.handle_reply(Reply::Snapshot(Box::new(s)));
        assert!(!o.controls_pending());
        assert!(o.next_action().is_none());
        assert_eq!(
            o.feedback.as_ref().unwrap().operation.as_deref(),
            Some("current-op")
        );
        assert!(o
            .feedback
            .as_ref()
            .unwrap()
            .transitions
            .iter()
            .any(|s| s == "live_owner_reconciled"));
        for _ in 0..100 {
            o.feedback.as_mut().unwrap().transition("running");
            o.feedback.as_mut().unwrap().transition("validating");
        }
        assert_eq!(o.feedback.as_ref().unwrap().transitions.len(), 12);
    }
    #[test]
    fn background_poll_click_is_captured_then_submitted_once() {
        let mut o = state_fixture();
        o.pending = true;
        o.background_poll = true;
        assert!(!o.controls_pending());
        o.capture_action(create_request());
        assert!(o.controls_pending());
        assert!(o.next_action().is_none());
        o.feedback.as_mut().unwrap().observe(
            &serde_json::json!({"operation":"old","state":"refused","reason":"old lock failure"}),
        );
        assert!(o.feedback.as_ref().unwrap().text.contains("Click received"));
        o.pending = false;
        o.background_poll = false;
        assert_eq!(o.next_action().unwrap().action, create_request().action);
        assert!(o.next_action().is_none());
        assert!(o.controls_pending());
    }
    #[test]
    fn old_poll_cannot_overwrite_current_prequeue_refusal_or_inflight_request() {
        let mut f = RequestFeedback::captured(create_request().action);
        let old =
            serde_json::json!({"operation":"old","state":"refused","reason":"old lock failure"});
        f.receipt(&Receipt {
            schema: 4,
            accepted: false,
            operation: Some("new".into()),
            refusal: Some("fresh refusal".into()),
        });
        f.observe(&old);
        assert!(f.text.contains("fresh refusal"));
        assert!(!f.text.contains("old lock"));
        assert!(f.for_installer(&"ab".repeat(32)));
        assert!(!f.for_installer(&"ef".repeat(32)));
        f = RequestFeedback::captured(create_request().action);
        f.receipt(&Receipt {
            schema: 4,
            accepted: true,
            operation: Some("new".into()),
            refusal: None,
        });
        f.observe(&old);
        assert!(!f.terminal);
        f.observe(&serde_json::json!({"operation":"new","state":"completed"}));
        assert!(f.terminal);
        f.observe(&old);
        assert!(f.text.contains("completed"));
    }
    #[test]
    fn vendor_controls_unlock_only_after_current_snapshot_without_losing_feedback() {
        let mut f = RequestFeedback::captured(create_request().action);
        f.receipt(&Receipt {
            schema: 4,
            accepted: true,
            operation: Some("new".into()),
            refusal: None,
        });
        f.observe(&serde_json::json!({"operation":"new","state":"vendor_running"}));
        assert!(f.blocking);
        f.refreshed();
        assert!(!f.blocking && !f.terminal);
        f.observe(&serde_json::json!({"operation":"new","state":"completed"}));
        assert!(f.terminal);
        let mut f = RequestFeedback::captured(create_request().action);
        f.receipt(&Receipt {
            schema: 4,
            accepted: false,
            operation: Some("refused".into()),
            refusal: Some("reason".into()),
        });
        assert!(f.blocking);
        f.refreshed();
        assert!(!f.blocking);
        assert!(f.text.contains("reason"));
    }
    #[test]
    fn real_button_release_survives_background_poll_start() {
        fn click(old_gate: bool) -> bool {
            let ctx = egui::Context::default();
            let mut o = state_fixture();
            let actions = [AvailableAction {
                label: "Create isolated environment".into(),
                action: create_request().action,
                disabled_reason: None,
            }];
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            let mut frame = |events: Vec<egui::Event>, pending: bool| {
                o.pending = pending;
                o.background_poll = pending;
                let input = egui::RawInput {
                    events,
                    ..Default::default()
                };
                let mut output = ctx.run_ui(input, |ui| {
                    point = ui.next_widget_position() + egui::vec2(12.0, 12.0);
                    Operator::buttons(
                        ui,
                        &actions,
                        None,
                        if old_gate {
                            o.pending
                        } else {
                            o.controls_pending()
                        },
                        &mut chosen,
                    );
                });
                output.textures_delta.clear();
                point
            };
            let p = frame(vec![], false);
            frame(
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                false,
            );
            frame(
                vec![egui::Event::PointerButton {
                    pos: p,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                true,
            );
            chosen.is_some()
        }
        assert!(
            !click(true),
            "reviewed global-pending gate loses this click"
        );
        assert!(
            click(false),
            "production action control must remain enabled through background polling"
        );
    }
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
    #[test]
    fn matching_product_receipt_survives_a_newer_unrelated_latest_operation() {
        let mut f=RequestFeedback::captured(Action::PluginPrepare{selection:"ab".repeat(32),inspection:"cd".repeat(32),recipe:"ef".repeat(32),predecessor:None});
        f.receipt(&Receipt{schema:4,accepted:true,operation:Some("11".repeat(16)),refusal:None});
        let mut s=running_snapshot(&"22".repeat(16));
        s.operation=Some(serde_json::json!({"operation":"22".repeat(16),"state":"completed"}));
        s.products.push(Product{class_id:"aa".repeat(16),name:"Generated instrument".into(),vendor:"Fixture".into(),role:"instrument".into(),version:"1".into(),disposition:"prepared".into(),active_revision:None,recommended_revision:None,environment:"ef".repeat(16),runner:"pinned".into(),module_sha256:"ff".repeat(32),limitations:vec![],history:vec![],actions:vec![],details:serde_json::json!({"preparation":{"operation":{"operation":"11".repeat(16),"state":"completed"}}})});
        f.reconcile_snapshot(&s);assert!(f.terminal);assert!(!f.blocking);
        let mut other=RequestFeedback::captured(Action::CandidateObserve{candidate:"bc".repeat(32),area:"processing_restart".into(),status:"failed".into(),note:"Exact candidate observation".into()});
        other.receipt(&Receipt{schema:4,accepted:true,operation:Some("33".repeat(16)),refusal:None});
        s.products[0].details["preparation"]["candidates"]=serde_json::json!([
            {"id":"aa".repeat(32),"operation":{"operation":"11".repeat(16),"state":"completed"}},
            {"id":"bc".repeat(32),"operation":{"operation":"33".repeat(16),"state":"refused"}}
        ]);
        other.reconcile_snapshot(&s);assert!(other.terminal);assert!(!other.blocking);assert_eq!(other.transitions.last().map(String::as_str),Some("refused"));
    }

}
