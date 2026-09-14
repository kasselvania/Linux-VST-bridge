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
        busy: bool,
        pending: bool,
        chosen: &mut Option<Action>,
    ) {
        for a in actions {
            let reason = if a.action.requires_inactive() && busy {
                Some("Close active bridged instances before this action")
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
                    if s.schema != 1 {
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
                    self.refresh_after = true;
                }
                Reply::Activity(a) => {
                    if let Some(s) = &mut self.snapshot {
                        if s.system.dsp != a.system.dsp || s.operation != a.operation {
                            self.refresh_after = true;
                        }
                        s.system = a.system;
                        s.capture = a.capture;
                        s.operation = a.operation;
                    }
                }
                Reply::Error(e) => self.message = e,
            }
        }
        let mut refresh = false;
        let mut chosen = None;
        egui::CentralPanel::default().show(ui,|ui|{
            ui.heading("Linux Audio Compatibility Manager");
            ui.horizontal(|ui|{if ui.add_enabled(!self.pending,egui::Button::new("Refresh")).clicked(){refresh=true;}ui.label(&self.message);if self.pending{ui.spinner();}});
            ui.separator();
            egui::ScrollArea::vertical().show(ui,|ui|{
                let Some(s)=&self.snapshot else{ui.label("The installed Rust manager is the state authority. Waiting for readback.");return;};
                let busy=s.system.dsp>0 || s.system.cleanup_unconfirmed;
                ui.label(format!("Service: {}  ·  Keeper: {}  ·  DSP: {} / {}  ·  Pending: {}  ·  Stale transports: {}",s.system.service,s.system.keepers,s.system.dsp,s.system.ceiling,s.system.pending_transactions,s.system.stale_transports));
                ui.label("512 added frames recommended · 256 unqualified");
                ui.label(if s.capture["armed"]==true{"Crash capture: armed for next admitted launch"}else{"Crash capture: off"});
                if let Some(op)=&s.operation{egui::CollapsingHeader::new("Last operation receipt").show(ui,|ui|Self::value(ui,op));}
                egui::CollapsingHeader::new("Arturia environment and software center").default_open(true).show(ui,|ui|{
                    for app in &s.vendor_applications{ui.heading(&app.name);ui.label(format!("{} · {}",app.version,app.state));Self::buttons(ui,&app.actions,busy,self.pending,&mut chosen);}
                    for e in &s.environments{ui.label(format!("{} · revision {} · pinned runner {}",e.family,e.revision,e.runner));ui.small(&e.authorization);if !e.last_scan["id"].is_null(){ui.small(format!("Last scan: {} modules · completed at {}",e.last_scan["module_count"],e.last_scan["completed_at"]));}Self::buttons(ui,&e.actions,busy,self.pending,&mut chosen);}
                });
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
                    for incident in s.recent_incidents.iter().rev(){egui::CollapsingHeader::new(format!("{} · {}",incident.state,incident.id)).show(ui,|ui|{Self::value(ui,&incident.summary);if let Some(a)=&incident.export{Self::buttons(ui,std::slice::from_ref(a),busy,self.pending,&mut chosen);}});}
                });
                Self::buttons(ui,&s.actions,busy,self.pending,&mut chosen);
                ui.separator();ui.small("Closing this window does not stop bridged audio or vendor applications. Vendor sign-in and authorization stay in the vendor's own interface.");
            });
        });
        if let Some(a) = chosen {
            if let Some(s) = &self.snapshot {
                self.request(
                    Query::Action(Request {
                        schema: 1,
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
