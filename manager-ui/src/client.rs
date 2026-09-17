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
    Imported,
    Cancelled,
    Error(String),
}
pub enum Query {
    PickInstaller,
    Snapshot,
    Activity,
    Action(Request),
}
fn call(query: Query) -> Result<Reply, String> {
    let home = std::env::var_os("HOME").ok_or("Home directory unavailable")?;
    let executable = std::path::PathBuf::from(home).join(".local/bin/linux-vst-bridge");
    let selected = if matches!(query, Query::PickInstaller) {
        let selected = rfd::FileDialog::new()
            .set_title("Add Windows installer")
            .add_filter("Windows installers", &["exe", "msi"])
            .pick_file();
        let Some(path) = selected else {
            return Ok(Reply::Cancelled);
        };
        Some(open_selected(&path)?)
    } else {
        None
    };
    let verb = match &query {
        Query::PickInstaller => "import-installer",
        Query::Snapshot => "snapshot",
        Query::Activity => "activity",
        Query::Action(_) => "request",
    };
    let mut command = Command::new(executable);
    if matches!(query, Query::PickInstaller) {
        command.arg(verb);
    } else {
        command.args(["operator", verb]);
    }
    let mut child = command
        .stdin(selected.map(Stdio::from).unwrap_or_else(Stdio::piped))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Installed manager is unavailable")?;
    if let Query::Action(request) = &query {
        child
            .stdin
            .as_mut()
            .ok_or("Manager input unavailable")?
            .write_all(&serde_json::to_vec(request).map_err(|_| "Request encoding failed")?)
            .map_err(|_| "Request delivery failed")?;
    }
    drop(child.stdin.take());
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
    let deadline = Instant::now()
        + Duration::from_secs(if matches!(query, Query::PickInstaller) {
            300
        } else {
            45
        });
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
    if matches!(query, Query::PickInstaller) {
        let value: serde_json::Value =
            serde_json::from_slice(&data).map_err(|_| "Invalid import receipt")?;
        if value["schema"] != 1 || value["import_result"] != "imported" {
            return Err("Installer import refused".into());
        }
        return Ok(Reply::Imported);
    }
    let envelope:serde_json::Value=serde_json::from_slice(&data).map_err(|_|"Invalid manager response")?;
    if envelope["schema"]!=5 {return Err("Update the frontend and manager together: operator model 5 required".into());}
    match query {
        Query::PickInstaller => unreachable!(),
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

fn open_selected(path: &std::path::Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "Choose a regular local installer file; links are not accepted")?;
    let md = f.metadata().map_err(|_| "Cannot read selected file")?;
    if !md.is_file() || md.uid() != unsafe { libc::getuid() } {
        return Err("Choose an owned regular installer file".into());
    }
    Ok(f)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_descriptor_refuses_links_and_directories() {
        let root = std::env::temp_dir().join(format!("mf2-picker-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let p = root.join("selected.exe");
        std::fs::write(&p, b"bytes").unwrap();
        assert!(open_selected(&p).is_ok());
        let l = root.join("link");
        std::os::unix::fs::symlink(&p, &l).unwrap();
        assert!(open_selected(&l).is_err());
        assert!(open_selected(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
