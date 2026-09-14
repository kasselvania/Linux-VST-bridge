//! Fixed manager entry point; no shell and no user-supplied executable/arguments.
use crate::model::{Activity, Receipt, Request, Snapshot};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::mpsc::Sender,
    time::{Duration, Instant},
};
pub enum Reply {
    Snapshot(Box<Snapshot>),
    Activity(Activity),
    Receipt(Receipt),
    Error(String),
}
pub enum Query {
    Snapshot,
    Activity,
    Action(Request),
}
fn call(query: Query) -> Result<Reply, String> {
    let home = std::env::var_os("HOME").ok_or("Home directory unavailable")?;
    let executable = std::path::PathBuf::from(home).join(".local/bin/linux-vst-bridge");
    let verb = match &query {
        Query::Snapshot => "snapshot",
        Query::Activity => "activity",
        Query::Action(_) => "request",
    };
    let mut child = Command::new(executable)
        .args(["operator", verb])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Installed manager is unavailable")?;
    let mut input = child.stdin.take().ok_or("Manager input unavailable")?;
    if let Query::Action(request) = &query {
        input
            .write_all(&serde_json::to_vec(request).map_err(|_| "Request encoding failed")?)
            .map_err(|_| "Request delivery failed")?;
    }
    drop(input);
    let collect = |mut pipe: Box<dyn Read + Send>, limit: u64| {
        std::thread::spawn(move || {
            let mut data = Vec::new();
            let result = pipe.by_ref().take(limit + 1).read_to_end(&mut data);
            let overflow = data.len() > limit as usize;
            let _ = std::io::copy(&mut pipe, &mut std::io::sink());
            (data, overflow, result.is_ok())
        })
    };
    let out = collect(
        Box::new(child.stdout.take().ok_or("Manager output unavailable")?),
        8 * 1024 * 1024,
    );
    let err = collect(
        Box::new(child.stderr.take().ok_or("Manager error unavailable")?),
        4096,
    );
    let deadline = Instant::now() + Duration::from_secs(45);
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|_| "Manager wait failed")? {
            break s;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(
                "Manager readback timed out. Refresh to inspect any accepted operation.".into(),
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (data, overflow, read) = out.join().map_err(|_| "Manager reader failed")?;
    let (error, _, _) = err.join().map_err(|_| "Manager error reader failed")?;
    if !status.success() {
        return Err(String::from_utf8_lossy(&error).chars().take(512).collect());
    }
    if overflow || !read {
        return Err("Manager response exceeded its bound".into());
    }
    match query {
        Query::Snapshot => {
            serde_json::from_slice::<Snapshot>(&data).map(|s| Reply::Snapshot(Box::new(s)))
        }
        Query::Activity => serde_json::from_slice::<Activity>(&data).map(Reply::Activity),
        Query::Action(_) => serde_json::from_slice::<Receipt>(&data).map(Reply::Receipt),
    }
    .map_err(|_| "Incompatible manager response; update the frontend and manager together".into())
}
pub fn send(query: Query, sender: Sender<Reply>, ctx: eframe::egui::Context) {
    std::thread::spawn(move || {
        let result = call(query).unwrap_or_else(Reply::Error);
        let _ = sender.send(result);
        ctx.request_repaint();
    });
}
