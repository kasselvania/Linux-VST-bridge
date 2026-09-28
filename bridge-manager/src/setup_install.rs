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
/// Edit only this scheme's default. Unrelated associations are byte-preserved;
/// a foreign default is never silently replaced.
fn callback_default(before: &[u8]) -> Result<Vec<u8>> {
    const KEY: &str = "x-scheme-handler/native-access";
    const VALUE: &str = "linux-vst-bridge-native-access.desktop;";
    let text=std::str::from_utf8(before)?;
    require(!text.contains('\0'),"callback_association_invalid")?;
    let mut inside=false;let mut section=false;let mut found=false;
    let mut insertion=text.len();let mut offset=0;
    for line in text.split_inclusive('\n') {
        let trim=line.trim();
        if trim.starts_with('[') {
            if inside {insertion=offset;}
            inside=trim=="[Default Applications]";
            if inside {require(!section,"callback_association_duplicate_section")?;section=true;}
        } else if inside {
            if let Some((key,value))=trim.split_once('=') {
                if key.trim()==KEY {
                    require(!found,"callback_association_duplicate")?;found=true;
                    require(value.trim()==VALUE||value.trim()==VALUE.trim_end_matches(';'),"callback_association_foreign")?;
                }
            }
        }
        offset+=line.len();
    }
    if found{return Ok(before.to_vec());}
    let entry=format!("{KEY}={VALUE}\n");
    let mut out=text[..insertion].to_string();
    if !out.is_empty()&&!out.ends_with('\n'){out.push('\n');}
    if !section{out.push_str("[Default Applications]\n");}
    out.push_str(&entry);out.push_str(&text[insertion..]);Ok(out.into_bytes())
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
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileChange {
    path: PathBuf,
    before: Option<Vec<u8>>,
    after: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkChange {
    path: PathBuf,
    before: Option<PathBuf>,
    after: PathBuf,
}
/// A complete, exact switch of the user-facing command, frontend, service and
/// software authority. PKG0 retains this before changing any of those routes.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Plan {
    links: Vec<LinkChange>,
    files: Vec<FileChange>,
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
    let plan = plan(m, home, installed, previous)?;
    apply(&plan, fail_after)
}

pub(super) fn plan(
    m: &Manager,
    home: &Path,
    installed: &Software,
    previous: Option<&Software>,
) -> Result<Plan> {
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
    // Scheme-only desktop entry; this command cannot launch an application. The
    // current manager must admit an existing exact operation before forwarding.
    // A legacy rollback receives no new Software field; its unknown command
    // refuses rather than launching the callback elsewhere.
    let handler=home.join(".local/share/applications/linux-vst-bridge-native-access.desktop");
    let executable=home.join(".local/bin/linux-vst-bridge");
    let executable=executable.to_str().ok_or("callback_desktop_path")?;
    require(!executable.chars().any(char::is_control),"callback_desktop_path")?;
    let escaped=executable.replace('\\',"\\\\").replace('"',"\\\"").replace('`',"\\`").replace('$',"\\$").replace('%',"%%");
    files.push(FileChange{before:owned_file(&handler,Some(DESKTOP_OWNER))?,path:handler,after:format!("{DESKTOP_OWNER}Type=Application\nName=Native Access login return (Linux Audio Compatibility Manager)\nExec=\"{escaped}\" native-access-callback %u\nTerminal=false\nNoDisplay=true\nMimeType=x-scheme-handler/native-access;\n").into_bytes()});
    let associations=home.join(".config/mimeapps.list");
    let before=owned_file(&associations,None)?;
    let after=callback_default(before.as_deref().unwrap_or(b""))?;
    files.push(FileChange{path:associations,before,after});
    let unit = home.join(".config/systemd/user/linux-vst-bridge.service");
    files.push(FileChange{before:owned_file(&unit,Some(SERVICE_OWNER))?,path:unit,after:format!("{SERVICE_OWNER}After=graphical-session.target\n\n[Service]\nType=simple\nExecStart={} serve\nUMask=0077\nRestart=on-failure\nRestartSec=2\nKillMode=control-group\nTimeoutStopSec=30\n\n[Install]\nWantedBy=default.target\n",systemd(installed.manager.path.to_str().ok_or("executable path encoding")?)).into_bytes()});
    let manifest = m.root.join("software.json");
    files.push(FileChange {
        before: owned_file(&manifest, None)?,
        path: manifest,
        after: serde_json::to_vec(installed)?,
    });
    // All command, desktop, unit and manifest checks above are read-only.
    Ok(Plan { links, files })
}

fn apply(plan: &Plan, fail_after: Option<usize>) -> Result<()> {
    let Plan { links, files } = plan;
    let mut linked = 0;
    let mut written = Vec::new();
    let mut step = 0;
    let result = (|| -> Result<()> {
        for l in links {
            publication::install_command(&l.path, &l.after, l.before.as_deref())?;
            linked += 1;
            step += 1;
            require(fail_after != Some(step), "setup_injected_failure")?;
        }
        for (index, f) in files.iter().enumerate() {
            require(owned_file(&f.path, None)? == f.before, "setup_file_changed")?;
            if f.before.as_deref() != Some(f.after.as_slice()) {
                put(&f.path, &f.after)?;
                written.push(index);
            }
            step += 1;
            require(fail_after != Some(step), "setup_injected_failure")?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        for index in written.into_iter().rev() {
            let f = &files[index];
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

/// The package owner writes this private journal before changing any route.
/// It is kept on ordinary failure until the exact old or new state is proven.
pub(super) fn commit_journaled(m: &Manager, home: &Path, installed: &Software,
    previous: Option<&Software>, verify_effective: impl Fn(Option<&Software>) -> Result<()>) -> Result<()> {
    let journal = m.root.join("package-transition.json");
    require(!journal.try_exists()?, "package_transition_pending")?;
    let switch = plan(m, home, installed, previous)?;
    switch.validate_paths(m, home)?;
    atomic_json(&journal, &switch)?;
    match apply(&switch, None) {
        Ok(()) => {
            require(switch.is_after()?, "package_transition_incomplete")?;
            verify_effective(Some(installed))?;
            fs::remove_file(journal)?;
            Ok(())
        }
        Err(error) => {
            if switch.is_before()? { fs::remove_file(journal)?; }
            Err(error)
        }
    }
}

impl Plan {
    fn verify_software(m: &Manager, software: &Software) -> Result<()> {
        for artifact in [&software.manager, &software.supervisor, &software.ownership,
            &software.host, &software.source_manifest] { artifact.verify()?; }
        for artifact in [&software.operator_frontend, &software.installer_launch,
            &software.preparation_kit].into_iter().flatten() { artifact.verify()?; }
        require(software.source_sha256 == software.source_manifest.sha256,
            "package_transition_host_source")?;
        if let Some(catalogue) = &software.native_catalogue {
            software.catalogue(m)?;
            require(fs::metadata(&catalogue.path)?.permissions().mode() & 0o222 == 0,
                "package_transition_catalogue_writable")?;
        }
        Ok(())
    }
    fn validate_paths(&self, m: &Manager, home: &Path) -> Result<()> {
        let links = [home.join(".local/bin/linux-vst-bridge"),
            home.join(".local/bin/linux-audio-compatibility-manager")];
        require((1..=2).contains(&self.links.len())
            && self.links.iter().enumerate().all(|(i, change)| change.path == links[i]),
            "package_transition_paths")?;
        let files = [
            home.join(".local/share/applications/linux-audio-compatibility-manager.desktop"),
            home.join(".local/share/applications/linux-vst-bridge-native-access.desktop"),
            home.join(".config/mimeapps.list"),
            home.join(".config/systemd/user/linux-vst-bridge.service"),
            m.root.join("software.json"),
        ];
        let expected = if self.links.len() == 2 { &files[..] } else { &files[1..] };
        require(self.files.len() == expected.len()
            && self.files.iter().zip(expected).all(|(change, path)| change.path == *path),
            "package_transition_paths")?;
        for link in &self.links {
            require(link.after.starts_with(m.root.join("software"))
                && link.before.as_ref().is_none_or(|p| p.starts_with(m.root.join("software"))),
                "package_transition_link")?;
        }
        let after: Software = serde_json::from_slice(&self.files.last().ok_or("package_transition_paths")?.after)?;
        Self::verify_software(m, &after)?;
        require(after.manager.path == self.links[0].after
            && after.operator_frontend.as_ref().map(|a| &a.path) == self.links.get(1).map(|l| &l.after),
            "package_transition_software")?;
        if let Some(before) = &self.files.last().ok_or("package_transition_paths")?.before {
            let old: Software = serde_json::from_slice(before)?;
            Self::verify_software(m, &old)?;
            require(self.links[0].before.as_ref().is_none_or(|p| p == &old.manager.path)
                && self.links.get(1).is_none_or(|l| l.before.as_ref().is_none_or(|p|
                    old.operator_frontend.as_ref().is_some_and(|a| &a.path == p))),
                "package_transition_software")?;
        }
        Ok(())
    }
    fn link_state(change: &LinkChange) -> Result<Option<PathBuf>> {
        match fs::read_link(&change.path) {
            Ok(path) => Ok(Some(path)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    fn states(&self) -> Result<(bool, bool)> {
        let mut before = true;
        let mut after = true;
        for link in &self.links {
            let actual = Self::link_state(link)?;
            require(actual == link.before || actual.as_ref() == Some(&link.after),
                "package_transition_foreign_link")?;
            before &= actual == link.before;
            after &= actual.as_ref() == Some(&link.after);
        }
        for file in &self.files {
            let actual = owned_file(&file.path, None)?;
            require(actual == file.before || actual.as_ref() == Some(&file.after),
                "package_transition_foreign_file")?;
            before &= actual == file.before;
            after &= actual.as_ref() == Some(&file.after);
        }
        Ok((before, after))
    }
    fn is_before(&self) -> Result<bool> { Ok(self.states()?.0) }
    fn is_after(&self) -> Result<bool> { Ok(self.states()?.1) }
}

/// After a crash, finish an entirely applied switch or restore the exact
/// predecessor. A foreign edit refuses; the journal remains for inspection.
pub(super) fn recover_journaled(m: &Manager, home: &Path,
    verify_effective: impl Fn(Option<&Software>) -> Result<()>) -> Result<bool> {
    let journal = m.root.join("package-transition.json");
    if !journal.try_exists()? { return Ok(false); }
    let switch: Plan = read_json(&journal)?;
    switch.validate_paths(m, home)?;
    if !switch.is_after()? && !switch.is_before()? {
        for file in switch.files.iter().rev() {
            let actual = owned_file(&file.path, None)?;
            if actual.as_ref() == Some(&file.after) && actual != file.before {
                if let Some(bytes) = &file.before { put(&file.path, bytes)?; }
                else { fs::remove_file(&file.path)?; }
            }
        }
        for link in switch.links.iter().rev() {
            if Plan::link_state(link)?.as_ref() == Some(&link.after)
                && link.before.as_ref() != Some(&link.after) {
                if let Some(prior) = &link.before {
                    publication::install_command(&link.path, prior, Some(&link.after))?;
                } else { fs::remove_file(&link.path)?; }
            }
        }
    }
    require(switch.is_after()? || switch.is_before()?, "package_transition_recovery_incomplete")?;
    let chosen = if switch.is_after()? {
        Some(serde_json::from_slice::<Software>(&switch.files.last().ok_or("package_transition_paths")?.after)?)
    } else {
        switch.files.last().ok_or("package_transition_paths")?.before.as_ref()
            .map(|bytes| serde_json::from_slice::<Software>(bytes)).transpose()?
    };
    verify_effective(chosen.as_ref())?;
    fs::remove_file(journal)?;
    Ok(true)
}

#[cfg(test)]
pub(super) fn interrupt_journaled_for_test(m: &Manager, home: &Path,
    installed: &Software, previous: Option<&Software>, complete: bool) -> Result<()> {
    let switch = plan(m, home, installed, previous)?;
    atomic_json(&m.root.join("package-transition.json"), &switch)?;
    if complete { apply(&switch, None) }
    else {
        let first = switch.links.first().ok_or("package_transition_paths")?;
        publication::install_command(&first.path, &first.after, first.before.as_deref())
    }
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
            home.join(".local/share/applications/linux-vst-bridge-native-access.desktop"),
            home.join(".config/mimeapps.list"),
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
        for foreign in ["frontend", "desktop", "callback", "association", "unit"] {
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
                "association" => fs::write(home.join(".config/mimeapps.list"), "[Default Applications]\nx-scheme-handler/native-access=foreign.desktop;\n").unwrap(),
                "callback" => fs::write(home.join(".local/share/applications/linux-vst-bridge-native-access.desktop"), "foreign callback").unwrap(),
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
        for boundary in 1..=7 {
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

#[cfg(test)]
mod callback_association_tests {
    use super::*;
    #[test]
    fn only_own_default_changes_and_conflicts_refuse() {
        let before=b"# user comment\n[Default Applications]\ntext/plain=editor.desktop;\n[Added Associations]\nx-test=other.desktop;\n";
        let after=callback_default(before).unwrap();
        assert_eq!(after,b"# user comment\n[Default Applications]\ntext/plain=editor.desktop;\nx-scheme-handler/native-access=linux-vst-bridge-native-access.desktop;\n[Added Associations]\nx-test=other.desktop;\n");
        assert_eq!(callback_default(&after).unwrap(),after);
        assert!(callback_default(b"[Default Applications]\nx-scheme-handler/native-access=foreign.desktop;\n").is_err());
        assert!(callback_default(b"[Default Applications]\n[Default Applications]\n").is_err());
        assert!(callback_default(b"\0").is_err());
        assert_eq!(callback_default(b"# preserved").unwrap(),b"# preserved\n[Default Applications]\nx-scheme-handler/native-access=linux-vst-bridge-native-access.desktop;\n");
    }
}
