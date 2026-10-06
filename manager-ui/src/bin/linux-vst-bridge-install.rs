//! First-run user-space installer. It never selects or stops an existing bridge.
use eframe::egui;
use linux_vst_bridge::portable_package::{Bundle, Trust};
use std::{path::PathBuf, process::Command, sync::mpsc};

struct Installer {
    bundle: Option<Bundle>, home: PathBuf, version: String, internal: bool,
    status: String, running: Option<mpsc::Receiver<Result<PathBuf,String>>>,
    frontend: Option<PathBuf>,
}
impl eframe::App for Installer {
    fn ui(&mut self, ui:&mut egui::Ui, _frame:&mut eframe::Frame) {
        let context=ui.ctx().clone();
        if let Some(receiver)=&self.running {
            if let Ok(result)=receiver.try_recv() {
                self.running=None;
                match result {
                    Ok(path)=> {self.frontend=Some(path); self.status="Application files installed. Open Setup to select this version and start the bridge.".into();},
                    Err(reason)=> self.status=format!("Installation stopped: {reason}. Your selected application and projects were retained."),
                }
            } else {context.request_repaint_after(std::time::Duration::from_millis(100));}
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Install Linux VST Bridge");
            if self.internal { ui.colored_label(egui::Color32::YELLOW,"Internal test build · not a customer release"); }
            if !self.version.is_empty() {ui.label(format!("Version {}",self.version));}
            ui.label("Install the application in your account. The compatibility runtime is acquired later through Setup. Supported plug-ins need no compiler or Steam installation.");
            ui.separator();ui.label(&self.status);
            if self.running.is_some() {ui.spinner();}
            if self.bundle.is_some() && self.running.is_none() && ui.button("Install application files").clicked() {
                let mut bundle=self.bundle.take().unwrap(); let home=self.home.clone();
                let (send,receive)=mpsc::channel();self.running=Some(receive);
                self.status="Verifying and installing application files…".into();
                std::thread::spawn(move|| {
                    let result=bundle.stage(&home).and_then(|frontend| {
                        linux_vst_bridge::portable_package::expose_setup(&frontend,&home)?;
                        Ok(frontend)
                    }).map_err(|e|e.to_string());
                    let _=send.send(result);context.request_repaint();
                });
            }
            if let Some(frontend)=&self.frontend {
                if ui.button("Open Setup").clicked() {
                    match Command::new(frontend).spawn() {
                        Ok(_)=>ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                        Err(_)=>self.status="Setup could not open. Use Linux VST Bridge Setup and Updates in Applications.".into(),
                    }
                }
            }
        });
    }
}
fn main()->eframe::Result {
    unsafe {libc::umask(0o077);}
    if std::env::args_os().len()!=1 {
        eprintln!("This installer accepts no command, path or operation arguments.");
        std::process::exit(64);
    }
    let home=PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let inspected=std::env::current_exe().map_err(|e|e.to_string())
        .and_then(|p| Trust::compiled().and_then(|trust|Bundle::open(&p,trust)).map_err(|e|e.to_string()));
    let mut installer=Installer {bundle:None,home,version:String::new(),internal:false,
        status:String::new(),running:None,frontend:None};
    match inspected {
        Ok(bundle)=> {installer.version=bundle.release().version.clone();installer.internal=bundle.key_class()=="internal_test";
            installer.status="The package signature is verified. Choose Install to stage these application files.".into();installer.bundle=Some(bundle);},
        Err(e)=>installer.status=format!("Package verification failed: {e}. No application files were installed."),
    }
    eframe::run_native("Linux VST Bridge Installer",eframe::NativeOptions {
        renderer:eframe::Renderer::Glow,
        viewport:egui::ViewportBuilder::default().with_inner_size([600.0,360.0]),
        ..Default::default()
    }, Box::new(move|_|Ok(Box::new(installer))))
}
