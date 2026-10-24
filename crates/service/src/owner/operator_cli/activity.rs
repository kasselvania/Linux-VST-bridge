//! Bounded status summaries. No diagnostic reconstruction or executable reads.
use super::*;
use ui::Observation;

fn observed<T>(read: impl FnOnce() -> Result<T>) -> Observation<T> {
    match read() { Ok(value) => Observation::Observed(value),
        Err(error) => Observation::Unavailable(error.to_string().chars().take(512).collect()) }
}
fn status(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&crate::read_control_bytes(path, 4096)?)?)
}
/// The latest saved receipt, not recomposed journey or current process health.
/// Keep ordinary Activity bounded independently of the technical snapshot.
pub(super) fn latest_operation(m: &Manager) -> Result<Option<Value>> {
    let bytes = match crate::read_control_bytes(&m.root.join("operator/latest.json"), 1024 * 1024) {
        Ok(bytes) => bytes,
        Err(error) if error.downcast_ref::<std::io::Error>().is_some_and(|error|
            error.kind() == std::io::ErrorKind::NotFound) => return Ok(None),
        Err(error) => return Err(error),
    };
    let record: Value = serde_json::from_slice(&bytes)?;
    let operation = record["operation"].as_str().filter(|id| valid_hex(id, 32)).ok_or("activity_receipt_identity")?;
    let state = record["state"].as_str().filter(|state| !state.is_empty() && state.len() <= 64).ok_or("activity_receipt_state")?;
    let mut summary = json!({"summary":true,"operation":operation,"state":state});
    // Deliberately omit live-derived owned_service_pause and all request/result
    // payloads. The exact retained Operation read remains the full receipt path.
    for key in ["error", "refusal"] {
        if let Some(reason) = record[key].as_str() { summary[key] = json!(reason.chars().take(512).collect::<String>()); }
    }
    Ok(Some(summary))
}
pub(super) fn incidents(m: &Manager) -> Observation<Vec<ui::IncidentSummary>> {
    observed(|| {
        let root = m.root.join("runtime/incidents");
        let metadata = match fs::symlink_metadata(&root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(error) => return Err(error.into()),
        };
        use std::os::unix::fs::MetadataExt;
        require(metadata.is_dir() && !metadata.file_type().is_symlink() && metadata.uid() == unsafe { libc::getuid() }, "incident_directory")?;
        let mut paths = fs::read_dir(root)?.take(66).map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        require(paths.len() <= 65, "incident_summary_bound")?;
        paths.sort();
        let mut summaries = Vec::new();
        for path in paths {
            if path.file_name().is_some_and(|name| name == "next.json") {
                linux_vst_bridge::file(&path)?; // the one declared arming record, not an incident
                continue;
            }
            require(summaries.len() < 64, "incident_summary_bound")?;
            let id = path.file_name().and_then(|s| s.to_str()).ok_or("incident_identity")?;
            let metadata = fs::symlink_metadata(&path)?;
            require(valid_hex(id, 32) && metadata.is_dir() && !metadata.file_type().is_symlink()
                && metadata.uid() == unsafe { libc::getuid() }, "incident_identity")?;
            let status = status(&path.join("status.json"))?;
            let word = |key: &str| status[key].as_str().map(|v| Observation::Observed(v.to_owned()))
                .unwrap_or_else(|| Observation::Unavailable("Not included in the incident status.".into()));
            let fact = |key: &str| status[key].as_bool().map(Observation::Observed)
                .unwrap_or_else(|| Observation::Unavailable("Read the retained technical report for this observation.".into()));
            let report_retained = match linux_vst_bridge::file(&path.join("share.json")) {
                Ok(_) => true,
                Err(error) if error.downcast_ref::<std::io::Error>().is_some_and(|error|
                    error.kind() == std::io::ErrorKind::NotFound) => false,
                Err(error) => return Err(error),
            };
            summaries.push(ui::IncidentSummary { id: id.into(), state: status["state"].as_str()
                .ok_or("incident_status_state")?.into(), outcome: word("outcome"),
                cleanup_confirmed: fact("cleanup_confirmed"), transport_retired: fact("transport_retired"), report_retained });
        }
        Ok(summaries)
    })
}
pub(super) fn recent(m: &Manager) -> Observation<Vec<ui::RecentInstance>> {
    observed(|| {
        require(!operator_lock::registry_writer_waiting(m), "activity_registry_busy")?;
        let _lock = match m.try_lock_shared("registry.lock")? {
            operator_lock::LockAttempt::Acquired(lock) => lock,
            operator_lock::LockAttempt::Busy => return Err("activity_registry_busy".into()),
        };
        // At most 128 small retained terminal records. No runtime, process or
        // diagnostic-content reads occur under this shared snapshot guard.
        let summaries = capacity::terminal_summaries(m)?;
        Ok(summaries.into_iter().map(|summary| {
            let report = m.root.join("runtime/results").join(format!("windows-{}.json", summary.session));
            let product = observed(|| {
                let owner = m.retained_lease_owner(&summary.session, &report)?;
                let environment: Environment = serde_json::from_value(owner["registration"]["environment"].clone())?;
                let module: Artifact = serde_json::from_value(owner["registration"]["module"].clone())?;
                let key = ui::ProductIdentity { environment: environment.id,
                    module_sha256: module.sha256, class_id: text(&owner["registration"]["metadata"], "class_id") };
                // Admitted setup identities include historical UUIDs. The exact
                // retained owner already checked its environment/path binding;
                // do not impose a newer identifier spelling on readable history.
                require(!key.environment.is_empty() && valid_hex(&key.module_sha256, 64)
                    && key.class_id.eq_ignore_ascii_case(&summary.class_id), "recent_instance_binding")?;
                Ok(key)
            });
            ui::RecentInstance { session: summary.session, class_id: summary.class_id, product,
                failure: match summary.terminal {
                    capacity::InstanceTerminal::WindowsHostExited => ui::InstanceFailure::WindowsHostExited,
                    capacity::InstanceTerminal::EditorControllerFailed => ui::InstanceFailure::EditorControllerFailed,
                    capacity::InstanceTerminal::TransportFailed => ui::InstanceFailure::TransportFailed,
                }, cleanup_confirmed: summary.cleanup_confirmed, transport_retired: summary.transport_retired,
                observed_at: summary.observed_at }
        }).collect())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incident_summaries_read_bounded_status_without_parsing_report_contents() {
        let fixture = test_fixture::Fixture::new();
        let folder = fixture.m.root.join("runtime/incidents").join("ab".repeat(16));
        private_dir(&folder).unwrap();
        fs::write(folder.join("status.json"), br#"{"state":"retained"}"#).unwrap();
        // A large/malformed diagnostic body cannot make a lightweight summary
        // reconstruct or parse the report; presence is the only declared fact.
        fs::write(folder.join("share.json"), vec![b'x'; 65536]).unwrap();
        // The sibling arming record is not an incident or an unreadable census.
        atomic_json(&fixture.m.root.join("runtime/incidents/next.json"), &json!({"armed":true})).unwrap();
        let Observation::Observed(rows) = incidents(&fixture.m) else { panic!() };
        assert_eq!(rows.len(), 1); assert!(rows[0].report_retained);
        assert!(matches!(rows[0].cleanup_confirmed, Observation::Unavailable(_)));
        assert!(matches!(rows[0].outcome, Observation::Unavailable(_)));
        fs::write(folder.join("status.json"), vec![b' '; 4097]).unwrap();
        assert!(matches!(incidents(&fixture.m), Observation::Unavailable(_)));
        fs::remove_file(folder.join("status.json")).unwrap();
        std::os::unix::fs::symlink(folder.join("share.json"), folder.join("status.json")).unwrap();
        assert!(matches!(incidents(&fixture.m), Observation::Unavailable(_)));
    }
    #[test]
    fn retained_operation_summary_never_exposes_cached_live_pause_or_hides_read_failure() {
        let fixture = test_fixture::Fixture::new();
        private_dir(&fixture.m.root.join("operator")).unwrap();
        let path = fixture.m.root.join("operator/latest.json");
        assert!(latest_operation(&fixture.m).unwrap().is_none());
        atomic_json(&path, &json!({"operation":"ab".repeat(16),"state":"running",
            "owned_service_pause":{"schema":1,"owner_operation":"dead-owner"},
            "journey":{"phase":"preparing_plugins"},"result":{"unneeded":"payload"}})).unwrap();
        let summary = latest_operation(&fixture.m).unwrap().unwrap();
        assert_eq!(summary["operation"], "ab".repeat(16)); assert_eq!(summary["state"], "running");
        assert_eq!(summary["summary"], true);
        assert!(summary.get("owned_service_pause").is_none() && summary.get("journey").is_none() && summary.get("result").is_none());
        fs::write(&path, b"invalid").unwrap();
        assert!(latest_operation(&fixture.m).is_err());
        assert!(ActivityComposer::default().sample(&fixture.m, ui::LiveActivity::unavailable("fixture"), None, None).is_err());
        fs::write(&path, vec![b' '; 1024 * 1024 + 1]).unwrap();
        assert!(latest_operation(&fixture.m).is_err());
    }
    #[test]
    fn recent_observation_yields_to_exclusive_custody_and_empty_is_explicit() {
        let fixture = test_fixture::Fixture::new();
        let held = fixture.m.lock("registry.lock").unwrap();
        assert!(matches!(recent(&fixture.m), Observation::Unavailable(_)));
        drop(held);
        assert!(matches!(recent(&fixture.m), Observation::Observed(rows) if rows.is_empty()));
    }
    #[test]
    fn summary_cache_is_only_for_the_exact_covered_revision() {
        let fixture = test_fixture::Fixture::new();
        let mut composer = ActivityComposer::default();
        let live = || ui::LiveActivity::unavailable("fixture");
        let first = composer.sample(&fixture.m, live(), None, Some(10)).unwrap();
        assert!(matches!(first.incidents, Observation::Observed(rows) if rows.is_empty()));
        let folder = fixture.m.root.join("runtime/incidents").join("ab".repeat(16));
        private_dir(&folder).unwrap();
        fs::write(folder.join("status.json"), br#"{"state":"retained"}"#).unwrap();
        let reused = composer.sample(&fixture.m, live(), None, Some(10)).unwrap();
        assert!(matches!(reused.incidents, Observation::Observed(rows) if rows.is_empty()));
        let changed = composer.sample(&fixture.m, live(), None, Some(11)).unwrap();
        assert!(matches!(changed.incidents, Observation::Observed(rows) if rows.len() == 1));
        fs::write(folder.join("status.json"), b"invalid").unwrap();
        assert!(matches!(composer.sample(&fixture.m, live(), None, None).unwrap().incidents, Observation::Unavailable(_)));
    }
    #[test]
    fn recent_summary_keeps_exact_uuid_setup_and_refuses_unbound_owner_data() {
        let f = test_fixture::Fixture::new();
        let session = "ab".repeat(16);
        let report = f.m.root.join("runtime/results").join(format!("windows-{session}.json"));
        let lease = f.m.root.join("runtime/leases").join(format!("{session}.json"));
        let temporary = f.r.environment.root.join("compatdata/pfx/drive_c/bridge/sessions")
            .join(&session).join("owner.json");
        private_dir(temporary.parent().unwrap()).unwrap();
        atomic_json(&temporary, &json!({"session":session,"report":report,"lease":lease,
            "registration":f.r})).unwrap();
        {
            let _guard = f.m.lock("registry.lock").unwrap();
            f.m.retain_lease_owner(&temporary).unwrap();
        }
        let directory = f.m.root.join("runtime/terminal-summaries");
        private_dir(&directory).unwrap();
        atomic_json(&directory.join(format!("{}.json", f.r.key())), &capacity::TerminalSummary {
            schema:1, session:session.clone(), class_id:f.r.key(), terminal:capacity::InstanceTerminal::WindowsHostExited,
            producer:2, status_domain:3, cleanup_confirmed:true, transport_retired:true, observed_at:1,
        }).unwrap();
        let Observation::Observed(rows) = recent(&f.m) else { panic!() };
        assert_eq!(rows[0].product, Observation::Observed(ui::ProductIdentity {
            environment:f.r.environment.id.clone(), module_sha256:f.r.module.sha256.clone(), class_id:f.r.key() }));
        assert!(f.r.environment.id.contains('-'), "the historical UUID must remain readable");
        let retained = f.m.root.join("runtime/lease-owners").join(format!("{session}.json"));
        let original: Value = read_json(&retained).unwrap();
        // Once retired, no lookup of the former Windows tree is needed.
        fs::remove_file(&temporary).unwrap();
        for case in 0..4 {
            let mut changed = original.clone();
            match case {
                0 => changed["owner"]["registration"]["metadata"]["class_id"] = "cd".repeat(16).into(),
                1 => changed["owner"]["registration"]["module"]["sha256"] = "malformed".into(),
                2 => changed["owner"]["report"] = "/wrong/report".into(),
                _ => changed["owner"]["registration"]["environment"]["id"] = "another-setup".into(),
            }
            atomic_json(&retained, &changed).unwrap();
            let Observation::Observed(rows) = recent(&f.m) else { panic!() };
            assert_eq!(rows.len(), 1, "terminal evidence must not disappear: {case}");
            assert!(matches!(rows[0].product, Observation::Unavailable(_)), "case {case}");
            assert_eq!(rows[0].failure, ui::InstanceFailure::WindowsHostExited);
        }
        atomic_json(&retained, &original).unwrap();
        let Observation::Observed(rows) = recent(&f.m) else { panic!() };
        assert!(rows[0].product.observed().is_some());
        fs::remove_file(&retained).unwrap();
        let Observation::Observed(rows) = recent(&f.m) else { panic!() };
        assert!(matches!(rows[0].product, Observation::Unavailable(_)));
    }

}
