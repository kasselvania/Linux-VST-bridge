//! Stable setup surfaces: all ownership checks precede any installed-authority change.
use super::*;
use std::os::unix::fs::MetadataExt;
const DESKTOP_OWNER: &str = "[Desktop Entry]\nX-LinuxVSTBridge-Owner=MF1\n";
const SERVICE_OWNER: &str = "[Unit]\nDescription=Linux VST Bridge registered host\n";

/// Explicit packages select their own schema/capabilities. Only an in-generation
/// acceptance transition may retain the prior adapter: a legacy manager rejects
/// the newer Software field even when it names an otherwise valid artifact.
pub(super) fn installer_launch_inputs(
    package: Option<&Path>,
    prior: Option<&Software>,
) -> Result<(Option<PathBuf>, Option<Artifact>)> {
    if let Some(package) = package {
        let input = package.join("installer-launch.exe");
        return Ok((input.try_exists()?.then_some(input), None));
    }
    let retained = prior.and_then(|s| s.installer_launch.clone());
    if let Some(a) = &retained {
        a.verify()?;
    }
    Ok((None, retained))
}

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
    // Exact pre-IS2 Software schema, including its unknown-field refusal.
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LegacySoftware {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preparation_kit: Option<Artifact>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        operator_frontend: Option<Artifact>,
        manager: Artifact,
        supervisor: Artifact,
        ownership: Artifact,
        host: Artifact,
        source_manifest: Artifact,
        source_sha256: String,
        #[serde(default)]
        native_catalogue: Option<Artifact>,
    }
    fn with_adapter(mut software: Software) -> Software {
        let path = software.manager.path.with_file_name("installer-launch.exe");
        fs::write(&path, b"source-owned adapter fixture").unwrap();
        software.installer_launch = Some(Artifact {
            sha256: digest(&path).unwrap(),
            path,
        });
        software
    }
    fn generation_bytes(dir: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
        fs::read_dir(dir)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect()
    }
    #[test]
    fn explicit_legacy_package_omits_adapter_and_remains_legacy_readable() {
        let f = test_fixture::Fixture::new();
        let home = f.outer.join("home");
        let old = with_adapter(software_fixture(&f, "is2"));
        let old_dir = old.manager.path.parent().unwrap();
        for path in generation_bytes(old_dir).keys() {
            fs::set_permissions(path, fs::Permissions::from_mode(0o444)).unwrap();
        }
        let before = generation_bytes(old_dir);
        commit(&f.m, &home, &old, None, None).unwrap();
        assert!(serde_json::from_slice::<LegacySoftware>(
            &fs::read(f.m.root.join("software.json")).unwrap()
        )
        .is_err());

        let mut legacy = software_fixture(&f, "legacy");
        let package = legacy.manager.path.parent().unwrap();
        let (input, retained) = installer_launch_inputs(Some(package), Some(&old)).unwrap();
        assert!(input.is_none());
        assert!(retained.is_none());
        legacy.installer_launch = retained;
        commit(&f.m, &home, &legacy, Some(&old), None).unwrap();
        let bytes = fs::read(f.m.root.join("software.json")).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(value.get("installer_launch").is_none());
        let parsed: LegacySoftware = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
        assert_eq!(software(&f.m).unwrap().manager, legacy.manager);
        assert_eq!(generation_bytes(old_dir), before);
        old.installer_launch.as_ref().unwrap().verify().unwrap();
    }
    #[test]
    fn is4_operation_policy_never_leaks_into_legacy_software_or_environment() {
        use linux_vst_bridge::installer_policy::{bind, Powershell};
        let f=test_fixture::Fixture::new();let home=f.outer.join("home");
        let current=with_adapter(software_fixture(&f,"is4"));
        let before=generation_bytes(current.manager.path.parent().unwrap());
        commit(&f.m,&home,&current,None,None).unwrap();
        let sw_before=fs::read(f.m.root.join("software.json")).unwrap();
        let mut environment=f.r.environment.clone();environment.id="ab".repeat(16);
        let env_before=serde_json::to_vec(&environment).unwrap();
        let mut spec=serde_json::json!({"schema":2,"operation":"cd".repeat(16),"environment":environment,"installer":f.r.module,"format":"pe_executable","installer_launch":current.installer_launch});
        for format in ["msi_compound", "unknown", ""] {
            let mut bad=spec.clone();bad["format"]=serde_json::json!(format);
            assert!(bind(&mut bad,&current,Powershell::Inherited).is_err());
            assert_eq!(bad["schema"],2);
        }
        let mut missing=current.clone();missing.installer_launch=None;
        assert!(bind(&mut spec.clone(),&missing,Powershell::Inherited).is_err());
        let mut bad=spec.clone();bad["installer_launch"]=serde_json::json!(current.manager);
        assert!(bind(&mut bad,&current,Powershell::Inherited).is_err());
        let adapter=current.installer_launch.as_ref().unwrap();let bytes=fs::read(&adapter.path).unwrap();
        fs::write(&adapter.path,b"changed adapter bytes").unwrap();
        assert!(bind(&mut spec.clone(),&current,Powershell::Inherited).is_err());
        fs::write(&adapter.path,bytes).unwrap();
        let original=spec.clone();
        bind(&mut spec,&current,Powershell::IntentionallyUnavailable).unwrap();
        assert_eq!(spec["schema"],3);
        assert_eq!(spec["installer_capability"]["format"],"pe_executable");
        assert_eq!(spec["installer_capability"]["installer_launch"],original["installer_launch"]);
        assert_eq!(spec["installer_capability"]["operation"],original["operation"]);
        assert_eq!(spec["installer_capability"]["environment"],original["environment"]);
        assert_eq!(spec["installer_capability"]["owners"]["supervisor"],serde_json::to_value(&current.supervisor).unwrap());
        let policy_spec=serde_json::to_vec(&spec).unwrap();
        assert!(bind(&mut spec,&current,Powershell::Inherited).is_err());
        assert_eq!(serde_json::to_vec(&spec).unwrap(),policy_spec);
        assert_eq!(fs::read(f.m.root.join("software.json")).unwrap(),sw_before);
        assert_eq!(serde_json::to_vec(&environment).unwrap(),env_before);
        let legacy=software_fixture(&f,"pre-policy");
        commit(&f.m,&home,&legacy,Some(&current),None).unwrap();
        let bytes=fs::read(f.m.root.join("software.json")).unwrap();
        let _:LegacySoftware=serde_json::from_slice(&bytes).unwrap();
        let value:serde_json::Value=serde_json::from_slice(&bytes).unwrap();
        assert!(value.get("installer_capability").is_none());
        assert_eq!(generation_bytes(current.manager.path.parent().unwrap()),before);
        assert_eq!(serde_json::to_vec(&spec).unwrap(),policy_spec);
        // A new legacy launch has its unchanged schema-2 input; no environment
        // default or software field can transfer the old operation's selection.
        assert!(original.get("installer_capability").is_none());
        assert_eq!(original["schema"],2);
    }
    #[test]
    fn explicit_current_package_selects_its_exact_adapter_not_the_prior_one() {
        let f = test_fixture::Fixture::new();
        let home = f.outer.join("home");
        let old = with_adapter(software_fixture(&f, "old"));
        let mut new = with_adapter(software_fixture(&f, "current"));
        let expected = new.installer_launch.clone().unwrap();
        fs::write(&expected.path, b"distinct current adapter").unwrap();
        let expected = Artifact {
            sha256: digest(&expected.path).unwrap(),
            path: expected.path,
        };
        let (input, retained) =
            installer_launch_inputs(new.manager.path.parent(), Some(&old)).unwrap();
        assert!(retained.is_none());
        let input = input.unwrap();
        // Production setup copies this input into the new immutable generation
        // and constructs its Artifact from those staged bytes.
        new.installer_launch = Some(Artifact {
            sha256: digest(&input).unwrap(),
            path: input,
        });
        assert_eq!(new.installer_launch.as_ref(), Some(&expected));
        assert_ne!(new.installer_launch, old.installer_launch);
        commit(&f.m, &home, &old, None, None).unwrap();
        commit(&f.m, &home, &new, Some(&old), None).unwrap();
        assert_eq!(software(&f.m).unwrap().installer_launch, Some(expected));
    }
    #[test]
    fn acceptance_without_package_preserves_verified_adapter_exactly() {
        let f = test_fixture::Fixture::new();
        let home = f.outer.join("home");
        let old = with_adapter(software_fixture(&f, "old"));
        let mut accepted = software_fixture(&f, "accepted");
        let before = generation_bytes(old.manager.path.parent().unwrap());
        let (input, retained) = installer_launch_inputs(None, Some(&old)).unwrap();
        assert!(input.is_none());
        assert_eq!(retained, old.installer_launch);
        accepted.installer_launch = retained;
        commit(&f.m, &home, &old, None, None).unwrap();
        commit(&f.m, &home, &accepted, Some(&old), None).unwrap();
        assert_eq!(software(&f.m).unwrap().installer_launch, old.installer_launch);
        assert_eq!(generation_bytes(old.manager.path.parent().unwrap()), before);
        fs::write(&old.installer_launch.as_ref().unwrap().path, b"changed").unwrap();
        assert!(installer_launch_inputs(None, Some(&old)).is_err());
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
