//! One reservation, submission and retirement protocol for retained application sessions.
//! Record formats preserve old namespaces and receipts; they own no product policy.
use crate::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecordFormat {
    Application,
    Dependency,
}
impl RecordFormat {
    fn namespace(&self) -> &'static str {
        match self {
            Self::Application => "native-access",
            Self::Dependency => "native-access-dependency",
        }
    }
    fn tag(&self) -> &'static str {
        match self {
            Self::Application => "renderer",
            Self::Dependency => "dependency",
        }
    }
    fn problem(&self, suffix: &str) -> String {
        format!("{}_{}", self.tag(), suffix)
    }
    pub fn directory(&self, m: &Manager) -> PathBuf {
        m.root.join("vendor-applications").join(self.namespace())
    }
    pub fn operation_dir(&self, m: &Manager, op: &str) -> Result<PathBuf> {
        require(valid_hex(op, 32), &self.problem("operation_identity"))?;
        Ok(self.directory(m).join("operations").join(op))
    }
    pub fn unit(&self, op: &str) -> Result<String> {
        require(valid_hex(op, 32), &self.problem("operation_identity"))?;
        Ok(format!("linux-vst-bridge-{}-{op}.service", self.tag()))
    }
    pub fn current(&self, m: &Manager) -> Result<Value> {
        self.optional(&self.directory(m).join("current.json"))
    }
    pub fn result(&self, m: &Manager, op: &str) -> Result<Value> {
        let d = self.operation_dir(m, op)?;
        let recovery = self.optional(&d.join("recovery-result.json"))?;
        if !recovery.is_null() {
            Ok(recovery)
        } else {
            self.optional(&d.join("result.json"))
        }
    }
    fn optional(&self, p: &Path) -> Result<Value> {
        if p.exists() {
            read_json(p)
        } else {
            Ok(Value::Null)
        }
    }
    pub fn exact(&self, m: &Manager, op: &str) -> Result<()> {
        require(
            self.current(m)?["operation"] == op,
            &self.problem("operation_changed"),
        )
    }
    pub(crate) fn gate(&self, m: &Manager, op: &str) -> Result<operator_lock::LockAttempt> {
        let p = self.operation_dir(m, op)?.join("writer.lock");
        m.try_lock(
            p.strip_prefix(&m.root)?
                .to_str()
                .ok_or(self.problem("gate_path"))?,
        )
    }
    pub fn terminal(&self, v: &Value, op: &str) -> bool {
        v["schema"] == 1
            && v["operation"] == op
            && v["cleanup_confirmed"] == true
            && v["owned_live"] == 0
            && matches!(
                v["state"].as_str(),
                Some("completed" | "failed" | "cancelled")
            )
    }
    fn immutable(&self, p: &Path, v: &Value) -> Result<()> {
        let staging = p.with_extension(format!("stage-{}", random_id()?));
        atomic_json(&staging, v)?;
        let installed = fs::hard_link(&staging, p);
        fs::remove_file(&staging)?;
        installed?;
        File::open(p.parent().ok_or(self.problem("parent"))?)?.sync_all()?;
        Ok(())
    }
    pub fn inspect(&self, op: &str) -> Result<UnitState> {
        let name = self.unit(op)?;
        self.inspect_unit(&name)
    }
    #[doc(hidden)]
    pub fn inspect_unit(&self, name: &str) -> Result<UnitState> {
        let renderer = name
            .strip_prefix("linux-vst-bridge-renderer-")
            .and_then(|s| s.strip_suffix(".service"))
            .is_some_and(|id| valid_hex(id, 32));
        let vendor = name
            .strip_prefix("linux-vst-bridge-vendor-")
            .and_then(|s| s.strip_suffix(".service"))
            .is_some_and(|id| crate::vendor_application::ApplicationId::parse(id).is_ok());
        let dependency = name
            .strip_prefix("linux-vst-bridge-dependency-")
            .and_then(|s| s.strip_suffix(".service"))
            .is_some_and(|id| valid_hex(id, 32));
        require(renderer || vendor || dependency, "owned_unit_identity")?;
        let (success, output) = crate::owner::readiness::application_unit_properties(name)?;
        let fields: BTreeMap<_, _> = output.lines().filter_map(|s| s.split_once('=')).collect();
        require(
            fields.len() == 8 && fields.get("Id") == Some(&name),
            &self.problem("unit_state_unavailable"),
        )?;
        let load = *fields.get("LoadState").ok_or(self.problem("unit_load"))?;
        require(
            matches!(load, "loaded" | "not-found") && (success || load == "not-found"),
            &self.problem("unit_query_failed"),
        )?;
        let group = format!(
            "/user.slice/user-{}.slice/user@{}.service/app.slice/{name}",
            unsafe { libc::getuid() },
            unsafe { libc::getuid() }
        );
        require(
            fields["ControlGroup"].is_empty() || fields["ControlGroup"] == group,
            &self.problem("cgroup_changed"),
        )?;
        let mut pending = vec![PathBuf::from("/sys/fs/cgroup").join(group.trim_start_matches('/'))];
        let mut count = 0;
        let mut members = false;
        while let Some(path) = pending.pop() {
            if !path.try_exists()? {
                continue;
            }
            count += 1;
            require(count <= 128, &self.problem("cgroup_bound"))?;
            let bytes = fs::read(path.join("cgroup.procs"))?;
            require(bytes.len() <= 65536, &self.problem("members_bound"))?;
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
    pub(crate) fn close_empty(
        &self,
        m: &Manager,
        op: &str,
        probe: &UnitState,
    ) -> Result<Submission> {
        self.exact(m, op)?;
        let d = self.operation_dir(m, op)?;
        let prior = self.result(m, op)?;
        if self.terminal(&prior, op) {
            return Ok(Submission::SubmittedOrLive);
        }
        if !prior.is_null() {
            require(
                prior["operation"] == op,
                &self.problem("result_operation_changed"),
            )?;
            let mut recovered = prior.clone();
            recovered["state"] = json!("failed");
            recovered["cleanup_confirmed"] = json!(true);
            recovered["owned_live"] = json!(0);
            recovered["error"] = json!(self.problem("writer_retired_without_terminal_receipt"));
            // An empty cgroup after an interrupted writer proves process cleanup,
            // never a Windows SCM transition. Preserve the prior receipt by hash.
            recovered["dependency_retirement"] = json!({"service_stop_requested":null,
            "service_retirement_confirmed":false,"process_cleanup_confirmed":true,
            "forced_cleanup_used":true,"authority":"interrupted_writer_process_cleanup_only"});
            recovered["recovery"] = json!({"writer_gate":"exclusive","unit_empty":true,"prior_result_sha256":digest(&d.join("result.json"))?});
            self.immutable(&d.join("recovery-result.json"), &recovered)?;
            return Ok(Submission::SubmittedOrLive);
        }
        let writer = d.join("writer.json").exists();
        let spec: Value = read_json(&d.join("spec.json"))?;
        self.immutable(
        &d.join("result.json"),
        &json!({"schema":1,"operation":op,"application_identity":spec["application_identity"],
        "requested":spec["renderer_policy"],"state":"failed","effective":null,"outer_exit":null,"dependency_retirement":{"service_stop_requested":null,"service_retirement_confirmed":false,"process_cleanup_confirmed":true,"forced_cleanup_used":writer||probe.exists,"authority":"no_scm_receipt"},"cleanup_confirmed":true,"owned_live":0,"cancelled":false,
        "error":if writer||probe.exists{self.problem("retired_without_receipt")}else{self.problem("definitely_not_submitted")},
        (self.tag()):{"cause":"unresolved","complete":false},"recovery":{"writer_gate":"exclusive","unit_empty":true,"prior_writer":writer}}),
    )?;
        Ok(if writer || probe.exists {
            Submission::SubmittedOrLive
        } else {
            Submission::DefinitelyNotSubmitted
        })
    }
    pub(crate) fn record_probe(&self, m: &Manager, op: &str, p: &Result<UnitState>) -> Result<()> {
        let v = match p {
            Ok(p) => {
                json!({"operation":op,"available":true,"exists":p.exists,"empty":p.empty,"observation":p.observation})
            }
            Err(_) => json!({"operation":op,"available":false}),
        };
        let path = self
            .operation_dir(m, op)?
            .join("unit-observation.private.json");
        if self.optional(&path)? != v {
            atomic_json(&path, &v)?;
        }
        Ok(())
    }
    pub fn reconcile_with(
        &self,
        m: &Manager,
        op: &str,
        probe: impl FnOnce() -> Result<UnitState>,
    ) -> Result<Submission> {
        self.exact(m, op)?;
        let operator_lock::LockAttempt::Acquired(_gate) = self.gate(m, op)? else {
            return Ok(Submission::SubmittedOrLive);
        };
        let observed = probe();
        self.record_probe(m, op, &observed)?;
        match observed {
            Ok(p) if p.empty && !p.exists => self.close_empty(m, op, &p),
            Ok(p) if p.exists => Ok(Submission::SubmittedOrLive),
            _ => Ok(Submission::AcknowledgmentUncertain),
        }
    }
    pub fn reconcile(&self, m: &Manager, op: &str) -> Result<Submission> {
        self.reconcile_with(m, op, || self.inspect(op))
    }
    pub fn retired_with(
        &self,
        m: &Manager,
        op: &str,
        probe: impl FnOnce() -> Result<UnitState>,
    ) -> Result<bool> {
        let p = self.operation_dir(m, op)?.join("writer.lock");
        let operator_lock::LockAttempt::Acquired(_gate) = m.try_lock_shared(
            p.strip_prefix(&m.root)?
                .to_str()
                .ok_or(self.problem("gate_path"))?,
        )?
        else {
            return Ok(false);
        };
        // Retirement is a read-only observation. Hold shared custody through the
        // unit probe so readers coexist while every real writer remains excluded.
        self.exact(m, op)?;
        if !self.terminal(&self.result(m, op)?, op) {
            return Ok(false);
        }
        let empty = probe().is_ok_and(|p| p.empty);
        self.exact(m, op)?;
        Ok(empty && self.terminal(&self.result(m, op)?, op))
    }
    pub fn retired(&self, m: &Manager, op: &str) -> Result<bool> {
        self.retired_with(m, op, || self.inspect(op))
    }
    pub(crate) fn reserve(&self, m: &Manager, spec: &Value) -> Result<Reservation> {
        let op = spec["operation"]
            .as_str()
            .ok_or(self.problem("operation"))?;
        let directory = self.operation_dir(m, op)?;
        require(
            spec["report"] == directory.join("result.json").to_string_lossy().as_ref(),
            &self.problem("report_location"),
        )?;
        private_dir(&self.directory(m).join("operations"))?;
        require(!directory.exists(), &self.problem("duplicate_operation"))?;
        private_dir(&directory)?;
        let operator_lock::LockAttempt::Acquired(gate) = self.gate(m, op)? else {
            return Err(self.problem("new_gate_busy").into());
        };
        self.immutable(&directory.join("spec.json"), spec)?;
        atomic_json(
            &self.directory(m).join("current.json"),
            &json!({"operation":op,"application":spec["application_identity"],"policy":spec["renderer_policy"]}),
        )?;
        Ok(Reservation {
            format: *self,
            operation: op.into(),
            path: directory.join("spec.json"),
            _gate: gate,
        })
    }
    pub fn submit_with(
        &self,
        m: &Manager,
        spec: &Value,
        submit: impl FnOnce(&Path) -> Result<bool>,
        probe: impl FnOnce() -> Result<UnitState>,
    ) -> Result<Submission> {
        let reservation = self.reserve(m, spec)?;
        let acknowledged = submit(&reservation.path).unwrap_or(false);
        reservation.complete(m, acknowledged, probe)
    }
    #[cfg(test)]
    pub fn stop_with(
        &self,
        m: &Manager,
        op: &str,
        stop: impl FnOnce() -> Result<()>,
        mut probe: impl FnMut() -> Result<UnitState>,
    ) -> Result<()> {
        self.exact(m, op)?;
        let p = probe();
        let stop_error = if !p.as_ref().is_ok_and(|p| p.empty && !p.exists) {
            stop().err()
        } else {
            None
        };
        // Stop is a synchronous cancellation barrier for a loaded but inactive unit
        // or queued job. Ordinary lost-ack reconciliation never assumes that barrier.
        {
            let operator_lock::LockAttempt::Acquired(_gate) = self.gate(m, op)? else {
                return Err(self.problem("writer_still_active").into());
            };
            let observed = probe();
            self.record_probe(m, op, &observed)?;
            let observed = observed?;
            // A collected unit can disappear between the first query and Stop.
            // A failed acknowledgment is safe only with fresh exact absence, an
            // empty cgroup and this exclusive writer gate. Never retry the signal.
            if let Some(error) = stop_error {
                if observed.exists || !observed.empty {
                    return Err(error);
                }
            }
            require(observed.empty, &self.problem("members_remain"))?;
            self.close_empty(m, op, &observed)?;
        }
        require(
            self.retired_with(m, op, probe)?,
            &self.problem("cleanup_unconfirmed"),
        )
    }
}

/// The writer gate remains held between durable reservation and acknowledgment;
/// a late accepted supervisor cannot run past a no-submission tombstone.
pub(crate) struct Reservation {
    format: RecordFormat,
    operation: String,
    pub(crate) path: PathBuf,
    _gate: Lock,
}
impl Reservation {
    pub(crate) fn complete(
        self,
        m: &Manager,
        acknowledged: bool,
        probe: impl FnOnce() -> Result<UnitState>,
    ) -> Result<Submission> {
        let format = self.format;
        let op = &self.operation;
        let state = if acknowledged {
            Submission::SubmittedOrLive
        } else {
            let observed = probe();
            format.record_probe(m, op, &observed)?;
            match observed {
                Ok(probe) if probe.empty && !probe.exists => format.close_empty(m, op, &probe)?,
                Ok(probe) if probe.exists => Submission::SubmittedOrLive,
                _ => Submission::AcknowledgmentUncertain,
            }
        };
        atomic_json(
            &format.operation_dir(m, op)?.join("submission.json"),
            &json!({"schema":1,"operation":op,"acknowledged":acknowledged,"state":state}),
        )?;
        Ok(state)
    }
}
