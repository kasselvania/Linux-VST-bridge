//! Stable setup surfaces: all ownership checks precede any installed-authority change.
use super::*;
use std::os::unix::fs::MetadataExt;
const DESKTOP_OWNER: &str = "[Desktop Entry]\nX-LinuxVSTBridge-Owner=MF1\n";
const SERVICE_OWNER: &str = "[Unit]\nDescription=Linux VST Bridge registered host\n";

pub(super) fn retained_frontend(
    input: Option<&Path>,
    prior: Option<&Software>,
) -> Result<Option<Artifact>> {
    if input.is_some() {
        return Ok(None);
    }
    let artifact = prior.and_then(|s| s.operator_frontend.clone());
    if let Some(a) = &artifact {
        a.verify()?;
    }
    Ok(artifact)
}
fn owned_file(path: &Path, prefix: Option<&str>) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
        Ok(md) => {
            require(
                md.is_file() && md.uid() == unsafe { libc::getuid() },
                "setup_file_foreign",
            )?;
            require(md.len() <= 1024 * 1024, "setup_file_bound")?;
            let bytes = fs::read(path)?;
            if let Some(prefix) = prefix {
                require(bytes.starts_with(prefix.as_bytes()), "setup_file_foreign")?;
            }
            Ok(Some(bytes))
        }
    }
}
fn put(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("setup_parent")?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".lvb-setup-{}", random_id()?));
    let result = (|| -> Result<()> {
        let mut f = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&temp, path)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if temp.exists() {
        let _ = fs::remove_file(temp);
    }
    result
}
struct FileChange {
    path: PathBuf,
    before: Option<Vec<u8>>,
    after: Vec<u8>,
}
struct LinkChange {
    path: PathBuf,
    before: Option<PathBuf>,
    after: PathBuf,
}
fn command(path: PathBuf, after: &Path, prior: Option<&Path>) -> Result<LinkChange> {
    publication::preflight_command(&path, after, prior)?;
    let before = match fs::read_link(&path) {
        Ok(p) => Some(p),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    Ok(LinkChange {
        path,
        before,
        after: after.into(),
    })
}
/// Failure injection is internal test instrumentation, never a CLI option.
pub(super) fn commit(
    m: &Manager,
    home: &Path,
    installed: &Software,
    previous: Option<&Software>,
    fail_after: Option<usize>,
) -> Result<()> {
    let mut links = vec![command(
        home.join(".local/bin/linux-vst-bridge"),
        &installed.manager.path,
        previous.map(|s| s.manager.path.as_path()),
    )?];
    let mut files = Vec::new();
    if let Some(frontend) = &installed.operator_frontend {
        frontend.verify()?;
        links.push(command(
            home.join(".local/bin/linux-audio-compatibility-manager"),
            &frontend.path,
            previous
                .and_then(|s| s.operator_frontend.as_ref())
                .map(|a| a.path.as_path()),
        )?);
        let path = frontend.path.to_str().ok_or("operator_frontend_path")?;
        require(
            !path.chars().any(char::is_control),
            "operator_frontend_path",
        )?;
        let escaped = path
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
            .replace('%', "%%");
        let desktop =
            home.join(".local/share/applications/linux-audio-compatibility-manager.desktop");
        files.push(FileChange{before:owned_file(&desktop,Some(DESKTOP_OWNER))?,path:desktop,after:format!("{DESKTOP_OWNER}Type=Application\nName=Linux Audio Compatibility Manager\nExec=\"{escaped}\"\nTerminal=false\nCategories=AudioVideo;Audio;\n").into_bytes()});
    } else {
        // Omission is not an uninstall operation. Never orphan a stable UI.
        require(
            previous.is_none_or(|p| p.operator_frontend.is_none()),
            "operator_frontend_retention_required",
        )?;
    }
    let unit = home.join(".config/systemd/user/linux-vst-bridge.service");
    files.push(FileChange{before:owned_file(&unit,Some(SERVICE_OWNER))?,path:unit,after:format!("{SERVICE_OWNER}After=graphical-session.target\n\n[Service]\nType=simple\nExecStart={} serve\nUMask=0077\nRestart=on-failure\nRestartSec=2\nKillMode=control-group\nTimeoutStopSec=30\n\n[Install]\nWantedBy=default.target\n",systemd(installed.manager.path.to_str().ok_or("executable path encoding")?)).into_bytes()});
    let manifest = m.root.join("software.json");
    files.push(FileChange {
        before: owned_file(&manifest, None)?,
        path: manifest,
        after: serde_json::to_vec(installed)?,
    });
    // All command, desktop, unit and manifest checks above are read-only.
    let mut linked = 0;
    let mut written = 0;
    let mut step = 0;
    let result = (|| -> Result<()> {
        for l in &links {
            publication::install_command(&l.path, &l.after, l.before.as_deref())?;
            linked += 1;
            step += 1;
            require(fail_after != Some(step), "setup_injected_failure")?;
        }
        for f in &files {
            require(owned_file(&f.path, None)? == f.before, "setup_file_changed")?;
            put(&f.path, &f.after)?;
            written += 1;
            step += 1;
            require(fail_after != Some(step), "setup_injected_failure")?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        for f in files[..written].iter().rev() {
            require(
                owned_file(&f.path, None)?.as_ref() == Some(&f.after),
                "setup_rollback_file_changed",
            )?;
            if let Some(old) = &f.before {
                put(&f.path, old)?;
            } else {
                fs::remove_file(&f.path)?;
            }
        }
        for l in links[..linked].iter().rev() {
            require(
                fs::read_link(&l.path)? == l.after,
                "setup_rollback_command_changed",
            )?;
            if let Some(old) = &l.before {
                publication::install_command(&l.path, old, Some(&l.after))?;
            } else {
                fs::remove_file(&l.path)?;
            }
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    fn software_fixture(f: &test_fixture::Fixture, name: &str) -> Software {
        let dir = f.m.root.join("software").join(name);
        private_dir(&dir).unwrap();
        let a = |name: &str| {
            let p = dir.join(name);
            fs::write(&p, format!("{name}-{dir:?}")).unwrap();
            Artifact {
                sha256: digest(&p).unwrap(),
                path: p,
            }
        };
        let source = a("source");
        Software {
            installer_launch: None,
            preparation_kit: None,
        manager: a("manager"),
            operator_frontend: Some(a("frontend")),
            supervisor: a("session"),
            ownership: a("ownership"),
            host: a("host"),
            source_sha256: source.sha256.clone(),
            source_manifest: source,
            native_catalogue: None,
        }
    }
    fn state(m: &Manager, home: &Path) -> Vec<Vec<u8>> {
        [
            m.root.join("software.json"),
            home.join(".local/bin/linux-vst-bridge"),
            home.join(".local/bin/linux-audio-compatibility-manager"),
            home.join(".local/share/applications/linux-audio-compatibility-manager.desktop"),
            home.join(".config/systemd/user/linux-vst-bridge.service"),
        ]
        .into_iter()
        .map(|p| {
            if let Ok(link) = fs::read_link(&p) {
                link.as_os_str().as_encoded_bytes().to_vec()
            } else {
                fs::read(p).unwrap()
            }
        })
        .collect()
    }
    #[test]
    fn foreign_preflight_and_each_commit_failure_leave_all_stable_surfaces_unchanged() {
        for foreign in ["frontend", "desktop", "unit"] {
            let f = test_fixture::Fixture::new();
            let home = f.outer.join("home");
            let old = software_fixture(&f, "old");
            let new = software_fixture(&f, "new");
            commit(&f.m, &home, &old, None, None).unwrap();
            match foreign {
                "frontend" => {
                    let p = home.join(".local/bin/linux-audio-compatibility-manager");
                    fs::remove_file(&p).unwrap();
                    symlink(f.outer.join("foreign"), p).unwrap();
                }
                "desktop" => fs::write(
                    home.join(
                        ".local/share/applications/linux-audio-compatibility-manager.desktop",
                    ),
                    "foreign desktop",
                )
                .unwrap(),
                _ => fs::write(
                    home.join(".config/systemd/user/linux-vst-bridge.service"),
                    "foreign service",
                )
                .unwrap(),
            }
            let before = state(&f.m, &home);
            assert!(commit(&f.m, &home, &new, Some(&old), None).is_err());
            assert_eq!(state(&f.m, &home), before);
        }
        for boundary in 1..=5 {
            let f = test_fixture::Fixture::new();
            let home = f.outer.join("home");
            let old = software_fixture(&f, "old");
            let new = software_fixture(&f, "new");
            commit(&f.m, &home, &old, None, None).unwrap();
            let before = state(&f.m, &home);
            assert!(commit(&f.m, &home, &new, Some(&old), Some(boundary)).is_err());
            assert_eq!(state(&f.m, &home), before);
        }
    }
    #[test]
    fn omitted_frontend_retains_exact_artifact_and_normal_software_checks_it() {
        let f = test_fixture::Fixture::new();
        let home = f.outer.join("home");
        let old = software_fixture(&f, "old");
        let mut new = software_fixture(&f, "new");
        commit(&f.m, &home, &old, None, None).unwrap();
        new.operator_frontend = retained_frontend(None, Some(&old)).unwrap();
        assert_eq!(new.operator_frontend, old.operator_frontend);
        commit(&f.m, &home, &new, Some(&old), None).unwrap();
        assert_eq!(
            software(&f.m).unwrap().operator_frontend,
            old.operator_frontend
        );
        let a = old.operator_frontend.unwrap();
        assert_eq!(
            fs::read_link(home.join(".local/bin/linux-audio-compatibility-manager")).unwrap(),
            a.path
        );
        fs::write(&a.path, "changed frontend").unwrap();
        assert!(software(&f.m).is_err());
    }
}
