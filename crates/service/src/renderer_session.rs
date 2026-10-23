//! Compatibility record surface; lifecycle execution is shared by application sessions.
use crate::*;
use serde_json::Value;
#[cfg(test)] use serde_json::json;
pub use crate::application_session::{Submission, UnitState};
use crate::application_session::RecordFormat;
const FORMAT: RecordFormat = RecordFormat::Application;
pub fn directory(m: &Manager) -> PathBuf { FORMAT.directory(m) }
pub fn operation_dir(m: &Manager, op: &str) -> Result<PathBuf> { FORMAT.operation_dir(m, op) }
pub fn unit(op: &str) -> Result<String> { FORMAT.unit(op) }
pub fn current(m: &Manager) -> Result<Value> { FORMAT.current(m) }
pub fn result(m: &Manager, op: &str) -> Result<Value> { FORMAT.result(m, op) }
pub fn exact(m: &Manager, op: &str) -> Result<()> { FORMAT.exact(m, op) }
#[cfg(test)]
fn gate(m: &Manager, op: &str) -> Result<operator_lock::LockAttempt> { FORMAT.gate(m, op) }
pub fn terminal(value: &Value, op: &str) -> bool { FORMAT.terminal(value, op) }
pub fn inspect(op: &str) -> Result<UnitState> { FORMAT.inspect(op) }
pub fn reconcile_with(m: &Manager, op: &str, probe: impl FnOnce() -> Result<UnitState>) -> Result<Submission> {
    FORMAT.reconcile_with(m, op, probe)
}
pub fn reconcile(m: &Manager, op: &str) -> Result<Submission> { FORMAT.reconcile(m, op) }
pub fn retired_with(m: &Manager, op: &str, probe: impl FnOnce() -> Result<UnitState>) -> Result<bool> {
    FORMAT.retired_with(m, op, probe)
}
pub fn retired(m: &Manager, op: &str) -> Result<bool> { FORMAT.retired(m, op) }
pub fn submit_with(m: &Manager, spec: &Value, submit: impl FnOnce(&Path) -> Result<bool>,
    probe: impl FnOnce() -> Result<UnitState>) -> Result<Submission> {
    FORMAT.submit_with(m, spec, submit, probe)
}
#[cfg(test)]
pub fn stop_with(m: &Manager, op: &str, stop: impl FnOnce() -> Result<()>,
    probe: impl FnMut() -> Result<UnitState>) -> Result<()> { FORMAT.stop_with(m, op, stop, probe) }
pub fn stop(m: &Manager, op: &str) -> Result<()> { crate::owner::vendor_app::stop_retained(m, op, false) }
pub fn inspect_unit(name: &str) -> Result<UnitState> { FORMAT.inspect_unit(name) }

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
    fn concurrent_retirement_readers_share_custody_and_exclude_writers() {
        let f = Fixture::new();
        let op = "aa".repeat(16);
        submit_with(&f.m, &spec(&f.m, &op), |_| Ok(true), live).unwrap();
        let report = operation_dir(&f.m, &op).unwrap().join("result.json");
        atomic_json(&report, &json!({"schema":1,"operation":op,"state":"completed",
            "cleanup_confirmed":true,"owned_live":0})).unwrap();
        let before = fs::read(&report).unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let manager = &f.m;
            let operation = &op;
            let first = scope.spawn(move || retired_with(manager, operation, || {
                started_tx.send(()).unwrap();
                release_rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
                absent()
            }).unwrap());
            started_rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
            let writer = gate(&f.m, &op).unwrap();
            let second = retired_with(&f.m, &op, absent);
            release_tx.send(()).unwrap();
            assert!(first.join().unwrap());
            assert!(matches!(writer, operator_lock::LockAttempt::Busy),
                "the first observer must hold custody throughout its unit probe");
            assert!(second.unwrap(), "a retirement reader must not reject another reader");
        });
        let operator_lock::LockAttempt::Acquired(writer) = gate(&f.m, &op).unwrap() else {
            panic!("reader guard was not released")
        };
        assert!(!retired_with(&f.m, &op, || panic!("writer still owns admission")).unwrap());
        assert_eq!(fs::read(&report).unwrap(), before);
        drop(writer);
        assert!(retired_with(&f.m, &op, absent).unwrap());
    }
    #[test]
    fn retirement_observation_rechecks_current_owner_and_terminal_receipt() {
        for change in ["current", "result", "probe"] {
            let f = Fixture::new();
            let op = "aa".repeat(16);
            submit_with(&f.m, &spec(&f.m, &op), |_| Ok(true), live).unwrap();
            let report = operation_dir(&f.m, &op).unwrap().join("result.json");
            atomic_json(&report, &json!({"schema":1,"operation":op,"state":"completed",
                "cleanup_confirmed":true,"owned_live":0})).unwrap();
            let outcome = retired_with(&f.m, &op, || {
                match change {
                    "current" => atomic_json(&directory(&f.m).join("current.json"),
                        &json!({"operation":"bb".repeat(16)}))?,
                    "result" => atomic_json(&report, &json!({"schema":1,"operation":op,
                        "state":"running","cleanup_confirmed":false,"owned_live":1}))?,
                    _ => return Err("unit_query_unavailable".into()),
                }
                absent()
            });
            assert!(!outcome.unwrap_or(false), "changed or unavailable custody must refuse");
        }
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
    fn stop_ack_loss_requires_fresh_absence_and_exclusive_writer_custody() {
        for case in ["collected", "live", "inactive", "unavailable", "writer"] {
            let f = Fixture::new();
            let op = "ab".repeat(16);
            submit_with(&f.m, &spec(&f.m, &op), |_| Ok(true), live).unwrap();
            let held = if case == "writer" {
                Some(gate(&f.m, &op).unwrap())
            } else {
                None
            };
            let mut queries = 0;
            let mut stops = 0;
            let outcome = stop_with(
                &f.m,
                &op,
                || {
                    stops += 1;
                    Err("stop_ack_lost".into())
                },
                || {
                    queries += 1;
                    if queries == 1 {
                        return live();
                    }
                    match case {
                        "collected" | "writer" => absent(),
                        "live" => live(),
                        "inactive" => Ok(UnitState {
                            exists: true,
                            empty: true,
                            observation: None,
                        }),
                        _ => Err("query_unavailable".into()),
                    }
                },
            );
            assert_eq!(stops, 1);
            assert_eq!(outcome.is_ok(), case == "collected");
            if case != "collected" {
                assert!(result(&f.m, &op).unwrap().is_null());
            }
            drop(held);
        }
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
        let prior = json!({"schema":1,"operation":op,"state":"running","renderer":{"facts":[{"category":"gpu"}]},"effective":{"policy":"inherited"}});
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
        assert_eq!(retired["dependency_retirement"]["service_retirement_confirmed"], false);
        assert_eq!(retired["dependency_retirement"]["process_cleanup_confirmed"], true);
        assert_eq!(retired["renderer"], prior["renderer"]);
        assert_eq!(retired["effective"], prior["effective"]);
    }
}
