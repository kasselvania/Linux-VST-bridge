//! One-shot diagnostics for an exact published registration. No launch authority.
use super::*;
use crate::publication::RevisionRef;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    schema: u32,
    id: String,
    pub directory: PathBuf,
    registration: Registration,
    publication: RevisionRef,
    profile_fingerprint: String,
    software: serde_json::Value,
    #[serde(default)]
    session: Option<String>,
}
fn root(m: &Manager) -> PathBuf {
    m.root.join("runtime/incidents")
}
fn folder(m: &Manager, id: &str) -> Result<PathBuf> {
    require(valid_hex(id, 32), "capture incident identity")?;
    Ok(root(m).join(id))
}
fn incidents(m: &Manager) -> Result<Vec<PathBuf>> {
    if !root(m).exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for e in fs::read_dir(root(m))?.take(66) {
        let p = e?.path();
        if p.is_dir() {
            paths.push(p);
        }
    }
    require(
        paths.len() <= 64,
        "capture retention full; retain/remove private incidents explicitly",
    )?;
    paths.sort();
    Ok(paths)
}
fn arm(m: &Manager, class: Option<&str>) -> Result<()> {
    // Admission independently verifies exact physical publication and artifacts.
    let admission = match class {
        Some(class) => ui_observation::admit(m, class)?,
        None => ui_observation::admit_if1(m)?,
    };
    arm_admitted(m, admission)
}
fn arm_admitted(m: &Manager, admission: ui_observation::Admission) -> Result<()> {
    let _registry = m.lock("registry.lock")?;
    let _capture = m.lock("capture.lock")?;
    private_dir(&root(m))?;
    require(!root(m).join("next.json").exists(), "capture already armed")?;
    require(incidents(m)?.len() < 64, "capture incident capacity (64)")?;
    let db = m.registry()?;
    let entry = db
        .classes
        .get(&admission.registration.key())
        .ok_or("capture publication absent")?;
    require(
        entry.registration == admission.registration,
        "capture publication changed",
    )?;
    let publication = entry
        .managed_revision
        .clone()
        .ok_or("capture publication absent")?;
    let revision = m.load_revision(&entry.registration.key(), &publication)?;
    require(
        revision.profile.fingerprint()? == admission.profile_fingerprint,
        "capture profile changed",
    )?;
    m.verify_completed_publication(&revision, &publication)?;
    require(
        fs::read_link(m.link(&entry.registration.key()))? == revision.target,
        "capture physical target changed",
    )?;
    let id = random_id()?;
    let directory = folder(m, &id)?;
    private_dir(&directory)?;
    let capture = Capture {
        schema: 1,
        id: id.clone(),
        directory: directory.clone(),
        registration: admission.registration,
        publication,
        profile_fingerprint: admission.profile_fingerprint,
        software: read_json(&m.root.join("software.json"))?,
        session: None,
    };
    atomic_json(&directory.join("request.json"), &capture)?;
    atomic_json(&root(m).join("next.json"), &capture)?;
    println!(
        "{}",
        serde_json::json!({"incident":id,"state":"armed","scope":"next exact admitted instance; existing instances are not restarted"})
    );
    Ok(())
}
/// Called under the existing registry admission lock. Failure never grants a
/// different binding or prevents normal diagnostics-off admission.
pub fn claim(m: &Manager, reg: &Registration, session: &str) -> Result<Option<Capture>> {
    if !root(m).join("next.json").exists() {
        return Ok(None);
    }
    let _capture = m.lock("capture.lock")?;
    if !root(m).join("next.json").exists() {
        return Ok(None);
    }
    let mut c: Capture = read_json(&root(m).join("next.json"))?;
    if c.registration.key() != reg.key() {
        return Ok(None);
    }
    require(
        c.schema == 1 && c.directory == folder(m, &c.id)? && valid_hex(session, 32),
        "capture identity",
    )?;
    let db = m.registry()?;
    let e = db.classes.get(&reg.key()).ok_or("capture class absent")?;
    if c.registration != *reg || e.managed_revision.as_ref() != Some(&c.publication) {
        atomic_json(
            &c.directory.join("status.json"),
            &serde_json::json!({"state":"stale_binding","capture_enabled":false}),
        )?;
        fs::remove_file(root(m).join("next.json"))?;
        return Ok(None);
    }
    c.session = Some(session.into());
    atomic_json(&c.directory.join("request.json"), &c)?;
    atomic_json(
        &c.directory.join("status.json"),
        &serde_json::json!({"state":"claimed","session":session,"capture_enabled":false}),
    )?;
    fs::remove_file(root(m).join("next.json"))?;
    Ok(Some(c))
}
fn disarm(m: &Manager) -> Result<()> {
    let _lock = m.lock("capture.lock")?;
    if root(m).join("next.json").exists() {
        let c: Capture = read_json(&root(m).join("next.json"))?;
        require(c.directory == folder(m, &c.id)?, "capture identity")?;
        atomic_json(
            &c.directory.join("status.json"),
            &serde_json::json!({"state":"cancelled_before_launch","capture_enabled":false}),
        )?;
        fs::remove_file(root(m).join("next.json"))?;
    }
    // This cancels retention, never the process or environment owner. The Wine
    // flags cannot be removed from an already-running process without restart.
    for path in incidents(m)? {
        if path.join("status.json").exists() {
            let s: serde_json::Value = read_json(&path.join("status.json"))?;
            if matches!(s["state"].as_str(), Some("claimed" | "collecting")) {
                atomic_json(&path.join("cancel.json"), &true)?;
            }
        }
    }
    println!("Capture disarmed. Active retention cancels at the next owner turn; logging pipes continue draining until normal session exit.");
    Ok(())
}
pub fn run(m: &Manager, args: &[String]) -> Result<()> {
    match args {
        [action,selection] if action=="arm"=>{
            let db=m.registry()?;let i=selection.parse::<usize>()?.checked_sub(1).ok_or("product selection")?;
            let key=db.classes.keys().nth(i).ok_or("product selection")?;arm(m,Some(key))
        }
        [action] if action=="arm-failure"=>arm(m,None),
        [action] if action=="disarm"=>disarm(m),
        [action] if action=="status"=>{
            let mut values=Vec::new();for path in incidents(m)? {
                let id=path.file_name().and_then(|s|s.to_str()).ok_or("capture identity")?;
                let state=if path.join("status.json").exists(){read_json::<serde_json::Value>(&path.join("status.json"))?}else{serde_json::json!({"state":"armed"})};
                values.push(serde_json::json!({"incident":id,"status":state}));
            }
            println!("{}",serde_json::json!({"schema":1,"armed":root(m).join("next.json").exists(),"incidents":values}));Ok(())
        }
        [action,id] if matches!(action.as_str(),"report"|"summary"|"export")=>{
            let path=folder(m,id)?.join(match action.as_str(){"report"=>"incident.json","summary"=>"summary.txt",_=>"share.json"});
            let mut f=file(&path)?;require(f.metadata()?.len()<=8*1024*1024,"capture report capacity")?;
            let mut s=String::new();f.read_to_string(&mut s)?;println!("{s}");Ok(())
        }
        _=>Err("usage: capture arm SELECTION | arm-failure | status | report ID | summary ID | export ID | disarm".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::prepared;
    fn ready() -> (crate::test_fixture::Fixture, ui_observation::Admission) {
        let (f, p, c, n) = prepared();
        let reg = observation::derive(&p, &c, &n).unwrap();
        let reference =
            f.m.managed_publish(&p, &c, reg, &c.host, &c.host_source_sha256, None)
                .unwrap();
        let revision = f.m.load_revision(&f.r.key(), &reference).unwrap();
        atomic_json(
            &f.m.root.join("software.json"),
            &serde_json::json!({"fixture":true}),
        )
        .unwrap();
        let a = ui_observation::Admission {
            schema: 1,
            profile_fingerprint: p.fingerprint().unwrap(),
            registration: revision.registration,
            accessibility_probe_permitted: false,
            maximum_seconds: 180,
            maximum_records: 16384,
        };
        (f, a)
    }
    #[test]
    fn one_exact_claim_preserves_other_instances_and_disarms() {
        let (f, a) = ready();
        let reg = a.registration.clone();
        arm_admitted(&f.m, a).unwrap();
        let mut sibling = reg.clone();
        sibling.metadata.class_id = "ff".repeat(16);
        assert!(claim(&f.m, &sibling, &"aa".repeat(16)).unwrap().is_none());
        let first = claim(&f.m, &reg, &"aa".repeat(16)).unwrap().unwrap();
        assert_eq!(first.session, Some("aa".repeat(16)));
        assert!(claim(&f.m, &reg, &"bb".repeat(16)).unwrap().is_none());
        disarm(&f.m).unwrap();
        assert!(first.directory.join("cancel.json").exists());
        assert_eq!(
            f.m.registry().unwrap().classes[&reg.key()].registration,
            reg
        );
    }
    #[test]
    fn changed_binding_does_not_capture_new_bytes_or_block_admission() {
        for field in 0..7 {
            let (f, a) = ready();
            let mut reg = a.registration.clone();
            arm_admitted(&f.m, a).unwrap();
            match field {
                0 => reg.native.sha256 = "01".repeat(32),
                1 => reg.host.sha256 = "02".repeat(32),
                2 => reg.module.sha256 = "03".repeat(32),
                3 => reg.host_source_sha256 = "04".repeat(32),
                4 => reg.environment.revision += 1,
                5 => reg.environment.runner.id = "different".into(),
                _ => {
                    reg.compatibility.disable_windows_accessibility =
                        !reg.compatibility.disable_windows_accessibility
                }
            }
            assert!(claim(&f.m, &reg, &"aa".repeat(16)).unwrap().is_none());
            assert!(!root(&f.m).join("next.json").exists());
            let status: serde_json::Value =
                read_json(&incidents(&f.m).unwrap()[0].join("status.json")).unwrap();
            assert_eq!(status["state"], "stale_binding");
        }
    }
    #[test]
    fn unused_arm_cancel_and_foreign_ids_have_no_launch_authority() {
        let (f, a) = ready();
        arm_admitted(&f.m, a).unwrap();
        disarm(&f.m).unwrap();
        assert!(!root(&f.m).join("next.json").exists());
        for id in ["../software", "", "gggggggggggggggggggggggggggggggg"] {
            assert!(folder(&f.m, id).is_err());
        }
        assert!(run(&f.m, &["launch".into(), "/bin/sh".into()]).is_err());
    }
}
