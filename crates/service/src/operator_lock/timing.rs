//! Fixed, private configuration-operation observations for pb0-c0-audit only.
//! These marks never grant admission or report successful publication. Normal
//! builds perform no observation; audio owners never call this module. A
//! returned edge describes a call return, not a successful operation receipt.
//! Scope-exit closes an unfinished span without inventing an outcome. On Linux
//! CLOCK_MONOTONIC is the native observer/SDK host's monotonic nanosecond clock.
//! Each timing-dispatch.json, timing-worker.json and timing-worker-early.json
//! retains its first exported attempt, even when a later retry succeeds. The
//! canonical result.json remains operation-outcome authority.
use crate::{operator_model::Action, Manager, Result};

#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Dispatch,
    RequestCreation,
    DispatchLock,
    RequestValidation,
    CurrentCapture,
    CaptureRegistry,
    CaptureSoftwareCatalogue,
    CaptureProducts,
    SelectedProjection,
    CurrentRecheck,
    SoftwareVerification,
    QueuedReceipt,
    WorkerLaunch,
    Worker,
    WorkerLock,
    WorkerRequestRead,
    WorkerReceiptRead,
    WorkerValidation,
    WorkerAdmission,
    CandidateVerification,
    ConfigurationAuthority,
    CandidateMutation,
    PublicationValidation,
    RegistryLock,
    CanonicalLock,
    ReceiptLock,
    ResumeLock,
    PublicationRecovery,
    PublicationRetainMutation,
    PublicationStageMutation,
    PublicationFinalAuthority,
    PublicationMutation,
    IntentMutation,
    PointerMutation,
    RegistryMutation,
    RestoreAuthority,
    RestoreMutation,
    FinalReceipt,
}

pub struct Session {
    #[cfg(feature = "pb0-c0-audit")]
    owns: bool,
}
impl Session {
    #[inline(always)]
    pub fn dispatch(m: &Manager, action: &Action) -> Self {
        #[cfg(feature = "pb0-c0-audit")]
        { enabled::start(m, None, Some(action), enabled::Lane::Dispatch) }
        #[cfg(not(feature = "pb0-c0-audit"))]
        { let _ = (m, action); Self {} }
    }
    #[inline(always)]
    pub fn worker(m: &Manager, operation: &str) -> Self {
        #[cfg(feature = "pb0-c0-audit")]
        { enabled::start(m, Some(operation), None, enabled::Lane::Worker) }
        #[cfg(not(feature = "pb0-c0-audit"))]
        { let _ = (m, operation); Self {} }
    }
}
#[inline(always)]
pub fn bind_operation(operation: &str) {
    #[cfg(feature = "pb0-c0-audit")]
    enabled::bind(operation);
    #[cfg(not(feature = "pb0-c0-audit"))]
    let _ = operation;
}
#[inline(always)]
pub fn select_action(action: &Action) {
    #[cfg(feature = "pb0-c0-audit")]
    enabled::select(action);
    #[cfg(not(feature = "pb0-c0-audit"))]
    let _ = action;
}

pub struct Span {
    #[cfg(feature = "pb0-c0-audit")]
    stage: Stage,
    #[cfg(feature = "pb0-c0-audit")]
    open: bool,
}
#[inline(always)]
pub fn span(stage: Stage) -> Span {
    #[cfg(feature = "pb0-c0-audit")]
    { enabled::mark(stage, enabled::Edge::Enter); Span {stage, open:true} }
    #[cfg(not(feature = "pb0-c0-audit"))]
    { let _ = stage; Span {} }
}
impl Span {
    #[inline(always)]
    pub fn end(mut self, successful_return: bool) {
        #[cfg(feature = "pb0-c0-audit")]
        {
            enabled::mark(self.stage, if successful_return {
                enabled::Edge::Returned
            } else { enabled::Edge::Failed });
            self.open = false;
        }
        #[cfg(not(feature = "pb0-c0-audit"))]
        let _ = (&mut self, successful_return);
    }
}
#[cfg(feature = "pb0-c0-audit")]
impl Drop for Span {
    fn drop(&mut self) {
        if self.open { enabled::mark(self.stage, enabled::Edge::ScopeExit); }
    }
}
#[inline(always)]
pub fn measure<T>(stage: Stage, run: impl FnOnce() -> Result<T>) -> Result<T> {
    let observed = span(stage);
    let result = run();
    observed.end(result.is_ok());
    result
}

#[cfg(feature = "pb0-c0-audit")]
mod enabled {
    use super::*;
    use crate::{atomic_json, private_dir, valid_hex};
    use std::{cell::RefCell, io::Read, os::unix::fs::MetadataExt, path::{Path,PathBuf}};
    const MAX_MARKS: usize = 128;
    #[derive(Clone, Copy, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    pub(super) enum Lane { Dispatch, Worker }
    #[derive(Clone, Copy, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Attempt { Primary, Early }
    #[derive(Clone, Copy, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    enum KindBasis { DispatchRequest, WorkerRequestRead, RecoveredAfterReturn }
    #[derive(Clone, Copy, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Kind { Prepare, Apply, Restore }
    fn kind(action: &Action) -> Option<Kind> {
        match action {
            Action::CandidateSettingsPrepare {..} | Action::CandidateGraphicsPrepare {..} => Some(Kind::Prepare),
            Action::ExperimentalEnable {..} | Action::ExperimentalReplace {..}
                | Action::CompatibilityPublishTest {..} => Some(Kind::Apply),
            Action::ExperimentalDisable {..} | Action::OrdinaryRollback {..}
                | Action::OrdinaryRestoreRecommended {..} => Some(Kind::Restore),
            _ => None,
        }
    }
    #[derive(Clone, Copy, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    pub(super) enum Edge { Enter, Returned, Failed, ScopeExit }
    #[derive(Clone, Copy, serde::Serialize)]
    struct Mark { stage: Stage, edge: Edge, monotonic_ns: u64 }
    struct Buffer {
        root: PathBuf,
        operation: Option<String>,
        kind: Option<Kind>,
        lane: Lane,
        marks: [Mark; MAX_MARKS],
        len: usize,
        omitted: u64,
        clock_failures: u64,
    }
    thread_local! { static ACTIVE: RefCell<Option<Buffer>> = const { RefCell::new(None) }; }
    fn now() -> Option<u64> {
        let mut time = libc::timespec {tv_sec:0, tv_nsec:0};
        if unsafe {libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time)} != 0 { return None; }
        u64::try_from(time.tv_sec).ok()?.checked_mul(1_000_000_000)?
            .checked_add(u64::try_from(time.tv_nsec).ok()?)
    }
    pub(super) fn start(m: &Manager, operation: Option<&str>, action: Option<&Action>, lane: Lane) -> Session {
        let selected = action.and_then(kind);
        if action.is_some() && selected.is_none()
            || operation.is_some_and(|id| !valid_hex(id,32)) { return Session {owns:false}; }
        let owns = ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active.is_some() { return false; }
            *active = Some(Buffer {root:m.root.clone(), operation:operation.map(Into::into),
                kind:selected,lane,marks:[Mark {stage:Stage::Dispatch,edge:Edge::Enter,monotonic_ns:0};MAX_MARKS],
                len:0,omitted:0,clock_failures:0});
            true
        });
        Session {owns}
    }
    pub(super) fn bind(operation: &str) {
        if valid_hex(operation,32) {
            ACTIVE.with(|active| if let Some(buffer) = active.borrow_mut().as_mut() {
                if buffer.operation.is_none() { buffer.operation = Some(operation.into()); }
            });
        }
    }
    pub(super) fn select(action: &Action) {
        ACTIVE.with(|active| if let Some(buffer) = active.borrow_mut().as_mut() {buffer.kind = kind(action);});
    }
    pub(super) fn mark(stage: Stage, edge: Edge) {
        ACTIVE.with(|active| if let Some(buffer) = active.borrow_mut().as_mut() {
            if buffer.len == MAX_MARKS { buffer.omitted = buffer.omitted.saturating_add(1); return; }
            if let Some(monotonic_ns) = now() {
                buffer.marks[buffer.len] = Mark {stage,edge,monotonic_ns};
                buffer.len += 1;
            } else {buffer.clock_failures = buffer.clock_failures.saturating_add(1);}
        });
    }
    #[derive(serde::Serialize)]
    struct Export<'a> {
        schema: u32, operation: &'a str, kind: Kind, lane: Lane, process: u32,
        attempt: Attempt, kind_basis: KindBasis,
        clock: &'static str, max_marks: usize, omitted: u64, clock_failures: u64,
        export_started_ns: Option<u64>, marks: &'a [Mark],
    }
    impl Buffer {
        fn export(self) -> Result<()> {
            let Some(operation) = self.operation.as_deref() else {return Ok(())};
            let directory = self.root.join("operator").join(operation);
            // Only the existing private receipt directory may receive an audit.
            // Never create an observation directory or include its path in data.
            if !directory.is_dir() { return Ok(()); }
            private_dir(&directory)?;
            let (kind,attempt,kind_basis,filename) = match (self.kind,self.lane) {
                (Some(kind),Lane::Dispatch) => (kind,Attempt::Primary,KindBasis::DispatchRequest,"timing-dispatch.json"),
                (Some(kind),Lane::Worker) => (kind,Attempt::Primary,KindBasis::WorkerRequestRead,"timing-worker.json"),
                (None,Lane::Dispatch) => return Ok(()),
                (None,Lane::Worker) => {
                    // Supplemental metadata read after the failed/early call
                    // returned and released its guards. It grants no admission
                    // and never replaces the canonical worker observation.
                    let file = crate::file(&directory.join("request.json"))?;
                    let meta = file.metadata()?;
                    crate::require(meta.mode() & 0o077 == 0 && meta.len() <= 16_385,
                        "operator_audit_request_extent_or_privacy")?;
                    let request:crate::operator_model::Request = serde_json::from_reader(file.take(16_386))?;
                    let Some(kind) = (request.schema == crate::operator_model::OPERATOR_SCHEMA)
                        .then(||kind(&request.action)).flatten() else {return Ok(())};
                    (kind,Attempt::Early,KindBasis::RecoveredAfterReturn,"timing-worker-early.json")
                }
            };
            let value = Export {schema:1,operation,kind,lane:self.lane,attempt,kind_basis,
                process:std::process::id(),clock:"clock_monotonic",max_marks:MAX_MARKS,
                omitted:self.omitted,clock_failures:self.clock_failures,export_started_ns:now(),marks:&self.marks[..self.len]};
            retain_first(&directory.join(filename),&value)
        }
    }
    fn retain_first(path: &Path, value: &Export<'_>) -> Result<()> {
        let staged = path.with_extension(format!("stage-{}",crate::random_id()?));
        let result = (|| {
            atomic_json(&staged,value)?;
            // Atomic, immutable exposure: an existing attempt wins.
            std::fs::hard_link(&staged,path)?;
            std::fs::File::open(path.parent().ok_or("operator_audit_parent")?)?.sync_all()?;
            Ok(())
        })();
        let _ = std::fs::remove_file(staged);
        result
    }
    impl Drop for Session {
        fn drop(&mut self) {
            if self.owns {
                let buffer = ACTIVE.with(|active| active.borrow_mut().take());
                // Observation failure never rewrites or refuses a canonical result.
                if let Some(buffer) = buffer { let _ = buffer.export(); }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{private_dir, random_id, test_fixture::Fixture};
    fn directory(f: &Fixture, id: &str) -> std::path::PathBuf {
        let directory = f.m.root.join("operator").join(id);
        private_dir(&directory).unwrap();
        directory
    }
    #[test]
    fn audit_observation_never_changes_return_or_creates_receipt_authority() {
        let f = Fixture::new();
        let id = random_id().unwrap();
        let error = {
            let _observed = Session::dispatch(&f.m, &Action::ExperimentalEnable {candidate:"private payload".into()});
            bind_operation(&id);
            measure::<()>(Stage::CandidateMutation, || Err("original_error".into())).unwrap_err()
        };
        assert_eq!(error.to_string(), "original_error");
        assert!(!f.m.root.join("operator").exists());
    }
    #[test]
    fn audit_normal_build_and_unrelated_actions_do_not_write_observations() {
        let f = Fixture::new();
        let id = random_id().unwrap();
        let directory = directory(&f, &id);
        {
            let _observed = Session::dispatch(&f.m, &Action::SupportExport {});
            bind_operation(&id);
            measure(Stage::Dispatch, || Ok(())).unwrap();
        }
        assert!(!directory.join("timing-dispatch.json").exists());
        #[cfg(not(feature = "pb0-c0-audit"))]
        {
            {
                let _observed = Session::dispatch(&f.m, &Action::ExperimentalEnable {candidate:"private".into()});
                bind_operation(&id);
                measure(Stage::PublicationMutation, || Ok(())).unwrap();
            }
            assert!(!directory.join("timing-dispatch.json").exists());
        }
    }
    #[cfg(feature = "pb0-c0-audit")]
    #[test]
    fn audit_is_bounded_private_monotonic_and_separates_lanes() {
        use crate::read_json;
        use std::os::unix::fs::PermissionsExt;
        let f = Fixture::new();
        let id = random_id().unwrap();
        let directory = directory(&f, &id);
        let action = Action::CompatibilityPublishTest {candidate:"private candidate path".into(),
            expected_current:Some(crate::operator_model::PublicationIdentity {id:"private revision".into(),sha256:"private hash".into()})};
        {
            let _observed = Session::dispatch(&f.m, &action);
            bind_operation(&id);
            measure(Stage::RequestCreation, || Ok(())).unwrap();
            measure::<()>(Stage::RequestValidation, || Err("private error".into())).unwrap_err();
            drop(span(Stage::DispatchLock));
            for _ in 0..100 {measure(Stage::SelectedProjection, || Ok(())).unwrap();}
        }
        let path = directory.join("timing-dispatch.json");
        let value: serde_json::Value = read_json(&path).unwrap();
        assert_eq!(value.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["attempt","clock","clock_failures","export_started_ns","kind","kind_basis","lane","marks","max_marks","omitted","operation","process","schema"]);
        let marks = value["marks"].as_array().unwrap();
        assert_eq!(marks.len(),128);
        assert_eq!(value["omitted"],78);
        assert_eq!(value["clock_failures"],0);
        assert_eq!(value["clock"],"clock_monotonic");
        assert_eq!(value["kind"],"apply");
        assert_eq!(marks[3]["edge"],"failed");
        assert_eq!(marks[5]["edge"],"scope_exit");
        assert!(marks.windows(2).all(|pair|pair[0]["monotonic_ns"].as_u64().unwrap() <= pair[1]["monotonic_ns"].as_u64().unwrap()));
        assert!(value["export_started_ns"].as_u64().unwrap() >= marks.last().unwrap()["monotonic_ns"].as_u64().unwrap());
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() < 24 * 1024);
        assert!(!String::from_utf8(bytes.clone()).unwrap().contains("private"));
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,0o600);
        {
            let _observed = Session::worker(&f.m,&id);
            let worker = span(Stage::Worker);
            select_action(&action);
            worker.end(true);
        }
        assert_eq!(std::fs::read(&path).unwrap(),bytes);
        let worker:serde_json::Value = read_json(&directory.join("timing-worker.json")).unwrap();
        assert_eq!(worker["lane"],"worker");
        assert_eq!(worker["operation"],value["operation"]);
    }
    #[cfg(feature = "pb0-c0-audit")]
    #[test]
    fn audit_failed_observation_write_preserves_the_original_return() {
        let f = Fixture::new();
        let id = random_id().unwrap();
        let directory = directory(&f,&id);
        std::fs::create_dir(directory.join("timing-dispatch.json")).unwrap();
        let result = {
            let _observed = Session::dispatch(&f.m,&Action::ExperimentalEnable {candidate:"private".into()});
            bind_operation(&id);
            measure(Stage::Dispatch,|| Ok(42))
        };
        assert_eq!(result.unwrap(),42);
        assert!(directory.join("timing-dispatch.json").is_dir());
        assert!(!directory.join("result.json").exists());
        assert_eq!(std::fs::read_dir(directory).unwrap().count(),1);
    }
    #[cfg(feature = "pb0-c0-audit")]
    #[test]
    fn audit_recovers_early_worker_kind_after_return_without_replacing_either_trace() {
        let f = Fixture::new();
        let id = random_id().unwrap();
        let directory = directory(&f,&id);
        crate::atomic_json(&directory.join("request.json"),&crate::operator_model::Request {
            schema:crate::operator_model::OPERATOR_SCHEMA,state_token:"private state".into(),
            action:Action::CompatibilityPublishTest {candidate:"private candidate".into(),expected_current:None},
        }).unwrap();
        let canonical = directory.join("timing-worker.json");
        crate::atomic_json(&canonical,&serde_json::json!({"retained":"prior canonical trace"})).unwrap();
        let before = std::fs::read(&canonical).unwrap();
        let lockname = format!("operator-worker-{id}.lock");
        let held = f.m.lock(&lockname).unwrap();
        let result = {
            let _observed = Session::worker(&f.m,&id);
            measure(Stage::Worker,|| measure(Stage::WorkerLock,|| f.m.lock(&lockname)))
        };
        assert!(result.err().unwrap().is::<crate::operator_lock::LockBusy>());
        drop(held);
        let early_path = directory.join("timing-worker-early.json");
        let first = std::fs::read(&early_path).unwrap();
        let observed:serde_json::Value = crate::read_json(&early_path).unwrap();
        assert_eq!(observed["kind"],"apply");
        assert_eq!(observed["attempt"],"early");
        assert_eq!(observed["kind_basis"],"recovered_after_return");
        assert!(position(&observed,"worker_lock","failed").is_some());
        assert!(position(&observed,"worker","failed").is_some());
        assert_eq!(std::fs::read(&canonical).unwrap(),before);
        {
            let _observed = Session::worker(&f.m,&id);
            measure::<()>(Stage::Worker,|| Err("second early attempt".into())).unwrap_err();
        }
        assert_eq!(std::fs::read(&canonical).unwrap(),before);
        assert_eq!(std::fs::read(&early_path).unwrap(),first);
        assert!(std::fs::read_dir(&directory).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().contains(".stage-")));
    }
    #[cfg(feature = "pb0-c0-audit")]
    fn observed<T>(f: &Fixture, action: Action, run: impl FnOnce() -> Result<T>) -> (Result<T>,serde_json::Value) {
        let id = random_id().unwrap();
        let directory = directory(f,&id);
        let result = {
            let _observed = Session::worker(&f.m,&id);
            select_action(&action);
            measure(Stage::Worker,run)
        };
        let value = crate::read_json(&directory.join("timing-worker.json")).unwrap();
        (result,value)
    }
    #[cfg(feature = "pb0-c0-audit")]
    fn position(value: &serde_json::Value, stage: &str, edge: &str) -> Option<usize> {
        value["marks"].as_array().unwrap().iter()
            .position(|mark|mark["stage"] == stage && mark["edge"] == edge)
    }
    #[cfg(feature = "pb0-c0-audit")]
    #[test]
    fn audit_actual_configuration_owners_bracket_prepare_apply_restore_and_refuse_stale_mutation() {
        use crate::{preparation as prep, operator_model::{LocalSettings,AccessibilityChoice,GraphicsBackend,PublicationIdentity}};
        let (f,base) = prep::tests::fixture();
        prep::record_candidate(&f.m,&base).unwrap();
        let original = prep::enable(&f.m,&base,false).unwrap();
        let original_registration = f.m.load_revision(&base.selection.class.id,&original).unwrap().registration;
        let candidate_bytes = serde_json::to_vec(&base).unwrap();
        let environment_bytes = std::fs::read(base.selection.environment.root.join("environment.json")).unwrap();
        let settings = LocalSettings {graphics:Some(GraphicsBackend::WineD3d11),accessibility:AccessibilityChoice::DisabledForHost};
        let prepare_action = Action::CandidateSettingsPrepare {candidate:base.id().unwrap(),settings:settings.clone(),
            expected_current:Some(PublicationIdentity {id:original.id.clone(),sha256:original.sha256.clone()})};
        let (result,prepare) = observed(&f,prepare_action.clone(),||
            prep::configuration::prepare_settings(&f.m,&base,&settings,Some(&original)));
        let trial = result.unwrap();
        assert!(position(&prepare,"configuration_authority","returned").unwrap()
            < position(&prepare,"candidate_mutation","enter").unwrap());
        assert!(position(&prepare,"candidate_mutation","returned").is_some());
        assert!(position(&prepare,"pointer_mutation","enter").is_none());
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision,Some(original.clone()));

        let mut stale = original.clone(); stale.sha256 = "ff".repeat(32);
        let (result,refused) = observed(&f,prepare_action,||
            prep::configuration::prepare_settings(&f.m,&base,&settings,Some(&stale)));
        assert!(result.is_err());
        assert!(position(&refused,"worker","failed").is_some());
        assert!(position(&refused,"candidate_mutation","enter").is_none());
        let apply_action = Action::CompatibilityPublishTest {candidate:trial.id().unwrap(),
            expected_current:Some(PublicationIdentity {id:original.id.clone(),sha256:original.sha256.clone()})};
        let (result,refused) = observed(&f,apply_action.clone(),|| prep::replace(&f.m,&trial,&stale));
        assert!(result.is_err());
        assert!(position(&refused,"candidate_mutation","enter").is_none());
        assert!(position(&refused,"intent_mutation","enter").is_none());
        let (result,apply) = observed(&f,apply_action,|| prep::replace(&f.m,&trial,&original));
        let selected = result.unwrap();
        assert!(position(&apply,"publication_stage_mutation","returned").unwrap()
            < position(&apply,"publication_final_authority","enter").unwrap());
        assert!(position(&apply,"publication_final_authority","returned").unwrap()
            < position(&apply,"intent_mutation","enter").unwrap());
        assert!(position(&apply,"pointer_mutation","returned").unwrap()
            < position(&apply,"registry_mutation","enter").unwrap());
        assert!(position(&apply,"publication_mutation","returned").is_some());
        let restore_action = Action::ExperimentalDisable {candidate:trial.id().unwrap()};
        let mut stale = selected.clone(); stale.sha256 = "ff".repeat(32);
        let (result,refused) = observed(&f,restore_action.clone(),||
            f.m.rollback_exact(&base.selection.class.id,&original.id,&stale));
        assert!(result.is_err());
        assert!(position(&refused,"restore_mutation","enter").is_none());
        let (result,restore) = observed(&f,restore_action,|| prep::disable_exact(&f.m,&trial,&selected));
        result.unwrap();
        assert!(position(&restore,"restore_authority","returned").unwrap()
            < position(&restore,"restore_mutation","enter").unwrap());
        assert!(position(&restore,"pointer_mutation","returned").is_some());
        assert!(position(&restore,"restore_mutation","returned").is_some());
        let entry = &f.m.registry().unwrap().classes[&base.selection.class.id];
        assert_eq!(entry.managed_revision,Some(original));
        assert_eq!(entry.registration,original_registration);
        assert_eq!(serde_json::to_vec(&base).unwrap(),candidate_bytes);
        assert_eq!(std::fs::read(base.selection.environment.root.join("environment.json")).unwrap(),environment_bytes);
        for value in [prepare,apply,restore] {assert_eq!(value["omitted"],0);assert_eq!(value["clock_failures"],0);}
    }
}
