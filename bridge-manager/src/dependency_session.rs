//! NAD1 exact dependency reservation; same writer-gate protocol as renderer sessions.
//! A separate fixed namespace prevents changing historical renderer reservations.
use crate::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Submission {
    DefinitelyNotSubmitted,
    SubmittedOrLive,
    AcknowledgmentUncertain,
}
#[derive(Clone, Debug)]
pub struct UnitState {
    pub exists: bool,
    pub empty: bool,
    pub observation: Option<Value>,
}
pub fn directory(m: &Manager) -> PathBuf {
    m.root.join("vendor-applications/native-access-dependency")
}
pub fn operation_dir(m: &Manager, op: &str) -> Result<PathBuf> {
    require(valid_hex(op, 32), "dependency_operation_identity")?;
    Ok(directory(m).join("operations").join(op))
}
pub fn unit(op: &str) -> Result<String> {
    require(valid_hex(op, 32), "dependency_operation_identity")?;
    Ok(format!("linux-vst-bridge-dependency-{op}.service"))
}
pub fn current(m: &Manager) -> Result<Value> {
    optional(&directory(m).join("current.json"))
}
pub fn result(m: &Manager, op: &str) -> Result<Value> {
    let d = operation_dir(m, op)?;
    let recovery = optional(&d.join("recovery-result.json"))?;
    if !recovery.is_null() {
        Ok(recovery)
    } else {
        optional(&d.join("result.json"))
    }
}
fn optional(p: &Path) -> Result<Value> {
    if p.exists() {
        read_json(p)
    } else {
        Ok(Value::Null)
    }
}
pub fn exact(m: &Manager, op: &str) -> Result<()> {
    require(
        current(m)?["operation"] == op,
        "dependency_operation_changed",
    )
}
fn gate(m: &Manager, op: &str) -> Result<operator_lock::LockAttempt> {
    let p = operation_dir(m, op)?.join("writer.lock");
    m.try_lock(
        p.strip_prefix(&m.root)?
            .to_str()
            .ok_or("dependency_gate_path")?,
    )
}
pub fn terminal(v: &Value, op: &str) -> bool {
    v["schema"] == 1
        && v["operation"] == op
        && v["cleanup_confirmed"] == true
        && v["owned_live"] == 0
        && matches!(
            v["state"].as_str(),
            Some("completed" | "failed" | "cancelled")
        )
}
fn immutable(p: &Path, v: &Value) -> Result<()> {
    let staging = p.with_extension(format!("stage-{}", random_id()?));
    atomic_json(&staging, v)?;
    let installed = fs::hard_link(&staging, p);
    fs::remove_file(&staging)?;
    installed?;
    File::open(p.parent().ok_or("dependency_parent")?)?.sync_all()?;
    Ok(())
}
pub fn inspect(op: &str) -> Result<UnitState> {
    let name = unit(op)?;
    let output = Command::new("systemctl")
        .args([
            "--user",
            "show",
            &name,
            "--property=Id,LoadState,ActiveState,SubState,Result,ControlGroup,MainPID,ControlPID",
        ])
        .output()?;
    require(output.stdout.len() <= 16384, "dependency_unit_query_bound")?;
    let fields: BTreeMap<_, _> = std::str::from_utf8(&output.stdout)?
        .lines()
        .filter_map(|s| s.split_once('='))
        .collect();
    require(
        fields.len() == 8 && fields.get("Id") == Some(&name.as_str()),
        "dependency_unit_state_unavailable",
    )?;
    let load = *fields.get("LoadState").ok_or("dependency_unit_load")?;
    require(
        matches!(load, "loaded" | "not-found") && (output.status.success() || load == "not-found"),
        "dependency_unit_query_failed",
    )?;
    let group = format!(
        "/user.slice/user-{}.slice/user@{}.service/app.slice/{name}",
        unsafe { libc::getuid() },
        unsafe { libc::getuid() }
    );
    require(
        fields["ControlGroup"].is_empty() || fields["ControlGroup"] == group,
        "dependency_cgroup_changed",
    )?;
    let mut pending = vec![PathBuf::from("/sys/fs/cgroup").join(group.trim_start_matches('/'))];
    let mut count = 0;
    let mut members = false;
    while let Some(path) = pending.pop() {
        if !path.try_exists()? {
            continue;
        }
        count += 1;
        require(count <= 128, "dependency_cgroup_bound")?;
        let bytes = fs::read(path.join("cgroup.procs"))?;
        require(bytes.len() <= 65536, "dependency_members_bound")?;
        if !bytes.iter().all(u8::is_ascii_whitespace) {
            members = true;
        }
        for entry in fs::read_dir(path)? {
            let e = entry?;
            if e.file_type()?.is_dir() {
                pending.push(e.path());
            }
        }
    }
    let empty = matches!(fields["ActiveState"], "inactive" | "failed")
        && fields["MainPID"] == "0"
        && fields["ControlPID"] == "0"
        && !members;
    Ok(UnitState {
        exists: load == "loaded",
        empty,
        observation: Some(
            json!({"unit":name,"properties":fields,"cgroup":group,"cgroup_directories":count,"members_present":members}),
        ),
    })
}
fn close_empty(m: &Manager, op: &str, probe: &UnitState) -> Result<Submission> {
    exact(m, op)?;
    let d = operation_dir(m, op)?;
    let prior = result(m, op)?;
    if terminal(&prior, op) {
        return Ok(Submission::SubmittedOrLive);
    }
    if !prior.is_null() {
        require(
            prior["operation"] == op,
            "dependency_result_operation_changed",
        )?;
        let mut recovered = prior.clone();
        recovered["state"] = json!("failed");
        recovered["cleanup_confirmed"] = json!(true);
        recovered["owned_live"] = json!(0);
        recovered["error"] = json!("dependency_writer_retired_without_terminal_receipt");
        recovered["recovery"] = json!({"writer_gate":"exclusive","unit_empty":true,"prior_result_sha256":digest(&d.join("result.json"))?});
        immutable(&d.join("recovery-result.json"), &recovered)?;
        return Ok(Submission::SubmittedOrLive);
    }
    let writer = d.join("writer.json").exists();
    let spec: Value = read_json(&d.join("spec.json"))?;
    immutable(
        &d.join("result.json"),
        &json!({"schema":1,"operation":op,"application_identity":spec["application_identity"],
        "requested":spec["renderer_policy"],"state":"failed","effective":null,"outer_exit":null,"cleanup_confirmed":true,"owned_live":0,"cancelled":false,
        "error":if writer||probe.exists{"dependency_retired_without_receipt"}else{"dependency_definitely_not_submitted"},
        "dependency":{"cause":"unresolved","complete":false},"recovery":{"writer_gate":"exclusive","unit_empty":true,"prior_writer":writer}}),
    )?;
    Ok(if writer || probe.exists {
        Submission::SubmittedOrLive
    } else {
        Submission::DefinitelyNotSubmitted
    })
}
fn record_probe(m: &Manager, op: &str, p: &Result<UnitState>) -> Result<()> {
    let v = match p {
        Ok(p) => {
            json!({"operation":op,"available":true,"exists":p.exists,"empty":p.empty,"observation":p.observation})
        }
        Err(_) => json!({"operation":op,"available":false}),
    };
    let path = operation_dir(m, op)?.join("unit-observation.private.json");
    if optional(&path)? != v {
        atomic_json(&path, &v)?;
    }
    Ok(())
}
pub fn reconcile_with(
    m: &Manager,
    op: &str,
    probe: impl FnOnce() -> Result<UnitState>,
) -> Result<Submission> {
    exact(m, op)?;
    let operator_lock::LockAttempt::Acquired(_gate) = gate(m, op)? else {
        return Ok(Submission::SubmittedOrLive);
    };
    let observed = probe();
    record_probe(m, op, &observed)?;
    match observed {
        Ok(p) if p.empty && !p.exists => close_empty(m, op, &p),
        Ok(p) if p.exists => Ok(Submission::SubmittedOrLive),
        _ => Ok(Submission::AcknowledgmentUncertain),
    }
}
pub fn reconcile(m: &Manager, op: &str) -> Result<Submission> {
    reconcile_with(m, op, || inspect(op))
}
pub fn retired_with(
    m: &Manager,
    op: &str,
    probe: impl FnOnce() -> Result<UnitState>,
) -> Result<bool> {
    if !terminal(&result(m, op)?, op) {
        return Ok(false);
    }
    let operator_lock::LockAttempt::Acquired(_gate) = gate(m, op)? else {
        return Ok(false);
    };
    Ok(probe().is_ok_and(|p| p.empty))
}
pub fn retired(m: &Manager, op: &str) -> Result<bool> {
    retired_with(m, op, || inspect(op))
}
pub fn submit_with(
    m: &Manager,
    spec: &Value,
    submit: impl FnOnce(&Path) -> Result<bool>,
    probe: impl FnOnce() -> Result<UnitState>,
) -> Result<Submission> {
    let op = spec["operation"].as_str().ok_or("dependency_operation")?;
    let d = operation_dir(m, op)?;
    require(
        spec["report"] == d.join("result.json").to_string_lossy().as_ref(),
        "dependency_report_location",
    )?;
    private_dir(&directory(m).join("operations"))?;
    require(!d.exists(), "dependency_duplicate_operation")?;
    private_dir(&d)?;
    // Gate exists and is held before publishing the reservation. A late accepted
    // unit waits here and cannot pass a terminal no-submission tombstone.
    let operator_lock::LockAttempt::Acquired(_gate) = gate(m, op)? else {
        return Err("dependency_new_gate_busy".into());
    };
    immutable(&d.join("spec.json"), spec)?;
    atomic_json(
        &directory(m).join("current.json"),
        &json!({"operation":op,"application":spec["application_identity"],"policy":spec["renderer_policy"]}),
    )?;
    let acknowledged = submit(&d.join("spec.json")).unwrap_or(false);
    let state = if acknowledged {
        Submission::SubmittedOrLive
    } else {
        let observed = probe();
        record_probe(m, op, &observed)?;
        match observed {
            Ok(p) if p.empty && !p.exists => close_empty(m, op, &p)?,
            Ok(p) if p.exists => Submission::SubmittedOrLive,
            _ => Submission::AcknowledgmentUncertain,
        }
    };
    atomic_json(
        &d.join("submission.json"),
        &json!({"schema":1,"operation":op,"acknowledged":acknowledged,"state":state}),
    )?;
    Ok(state)
}
pub fn stop_with(
    m: &Manager,
    op: &str,
    stop: impl FnOnce() -> Result<()>,
    mut probe: impl FnMut() -> Result<UnitState>,
) -> Result<()> {
    exact(m, op)?;
    let p = probe();
    if !p.as_ref().is_ok_and(|p| p.empty && !p.exists) {
        stop()?;
    }
    // Stop is a synchronous cancellation barrier for a loaded but inactive unit
    // or queued job. Ordinary lost-ack reconciliation never assumes that barrier.
    {
        let operator_lock::LockAttempt::Acquired(_gate) = gate(m, op)? else {
            return Err("dependency_writer_still_active".into());
        };
        let observed = probe();
        record_probe(m, op, &observed)?;
        let observed = observed?;
        require(observed.empty, "dependency_members_remain")?;
        close_empty(m, op, &observed)?;
    }
    require(
        retired_with(m, op, probe)?,
        "dependency_cleanup_unconfirmed",
    )
}
pub fn stop(m: &Manager, op: &str) -> Result<()> {
    stop_with(
        m,
        op,
        || {
            require(
                Command::new("systemctl")
                    .args(["--user", "stop", &unit(op)?])
                    .status()?
                    .success(),
                "dependency_stop_unconfirmed",
            )
        },
        || inspect(op),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::Fixture;
    fn spec(m: &Manager, op: &str) -> Value {
        json!({"operation":op,"application_identity":"ab".repeat(32),"renderer_policy":"inherited","report":operation_dir(m,op).unwrap().join("result.json")})
    }
    fn absent() -> Result<UnitState> {
        Ok(UnitState {
            exists: false,
            empty: true,
            observation: None,
        })
    }
    fn live() -> Result<UnitState> {
        Ok(UnitState {
            exists: true,
            empty: false,
            observation: None,
        })
    }
    #[test]
    fn submission_three_states_and_delayed_result() {
        for case in [
            "success",
            "lost_ack_live",
            "lost_ack_loaded_inactive",
            "lost_ack_absent",
            "query_unavailable",
        ] {
            let f = Fixture::new();
            let op = "aa".repeat(16);
            let v = spec(&f.m, &op);
            let state = submit_with(
                &f.m,
                &v,
                |_| Ok(case == "success"),
                || match case {
                    "lost_ack_live" => live(),
                    "lost_ack_loaded_inactive" => Ok(UnitState {
                        exists: true,
                        empty: true,
                        observation: None,
                    }),
                    "lost_ack_absent" => absent(),
                    _ => Err("unavailable".into()),
                },
            )
            .unwrap();
            assert_eq!(
                state,
                match case {
                    "success" | "lost_ack_live" | "lost_ack_loaded_inactive" =>
                        Submission::SubmittedOrLive,
                    "lost_ack_absent" => Submission::DefinitelyNotSubmitted,
                    _ => Submission::AcknowledgmentUncertain,
                }
            );
            assert_eq!(
                terminal(&result(&f.m, &op).unwrap(), &op),
                case == "lost_ack_absent"
            );
            if case == "lost_ack_live" {
                let r = json!({"schema":1,"operation":op,"state":"completed","cleanup_confirmed":true,"owned_live":0});
                atomic_json(&operation_dir(&f.m, &op).unwrap().join("result.json"), &r).unwrap();
                assert_eq!(
                    reconcile_with(&f.m, &op, absent).unwrap(),
                    Submission::SubmittedOrLive
                );
                assert!(retired_with(&f.m, &op, absent).unwrap());
                assert_eq!(result(&f.m, &op).unwrap(), r);
            }
        }
    }
    #[test]
    fn interrupted_worker_and_writer_gate_prevent_late_launch_and_false_retirement() {
        let f = Fixture::new();
        let op = "aa".repeat(16);
        submit_with(
            &f.m,
            &spec(&f.m, &op),
            |_| Err("lost".into()),
            || Err("unknown".into()),
        )
        .unwrap();
        let operator_lock::LockAttempt::Acquired(held) = gate(&f.m, &op).unwrap() else {
            panic!()
        };
        assert_eq!(
            reconcile_with(&f.m, &op, || panic!("writer still owns launch")).unwrap(),
            Submission::SubmittedOrLive
        );
        assert!(result(&f.m, &op).unwrap().is_null());
        drop(held);
        assert_eq!(
            reconcile_with(&f.m, &op, absent).unwrap(),
            Submission::DefinitelyNotSubmitted
        );
        let p = operation_dir(&f.m, &op).unwrap().join("result.json");
        let bytes = fs::read(&p).unwrap();
        reconcile_with(&f.m, &op, absent).unwrap();
        assert_eq!(fs::read(&p).unwrap(), bytes);
        assert!(submit_with(
            &f.m,
            &spec(&f.m, &op),
            |_| panic!("no resubmission"),
            absent
        )
        .is_err());
    }
    #[test]
    fn stop_requires_exact_operation_and_preserves_interrupted_writer_evidence() {
        let f = Fixture::new();
        let op = "aa".repeat(16);
        submit_with(&f.m, &spec(&f.m, &op), |_| Ok(true), live).unwrap();
        assert!(stop_with(
            &f.m,
            &"bb".repeat(16),
            || panic!("wrong stop"),
            || panic!("wrong probe")
        )
        .is_err());
        let d = operation_dir(&f.m, &op).unwrap();
        let prior = json!({"schema":1,"operation":op,"state":"running","dependency":{"facts":[{"category":"gpu"}]},"effective":{"policy":"inherited"}});
        atomic_json(&d.join("result.json"), &prior).unwrap();
        let bytes = fs::read(d.join("result.json")).unwrap();
        let mut probes = 0;
        let mut stops = 0;
        stop_with(
            &f.m,
            &op,
            || {
                stops += 1;
                Ok(())
            },
            || {
                probes += 1;
                if probes == 1 {
                    live()
                } else {
                    absent()
                }
            },
        )
        .unwrap();
        assert_eq!(stops, 1);
        assert_eq!(fs::read(d.join("result.json")).unwrap(), bytes);
        let retired = result(&f.m, &op).unwrap();
        assert!(terminal(&retired, &op));
        assert_eq!(retired["dependency"], prior["dependency"]);
        assert_eq!(retired["effective"], prior["effective"]);
    }
}
