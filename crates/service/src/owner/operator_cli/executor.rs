//! The service owns execution and the journal across client disconnection.
//! A retained process generation distinguishes interrupted service work from
//! historical operation units; neither is replayed during recovery.
use super::*;
use std::collections::BTreeSet;

#[cfg(all(feature = "service-executor-fixture", not(debug_assertions)))]
compile_error!("service-executor-fixture is only permitted in non-release test builds");
#[cfg(feature = "service-executor-fixture")]
fn fixture_barrier(m: &Manager, operation: &str) -> Result<()> {
    let directory = m.root.join("test-executor-control");
    if !directory.join("hold").try_exists()? { return Ok(()); }
    atomic_json(&directory.join(format!("{operation}.held")), &serde_json::json!({"pid":std::process::id()}))?;
    let deadline = Instant::now() + Duration::from_secs(25);
    while directory.join("hold").try_exists()? && !directory.join(format!("{operation}.release")).try_exists()? {
        require(Instant::now() < deadline, "service_executor_fixture_timeout")?;
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    schema: u32,
    operation: String,
    boot: String,
    pid: i32,
    start: u64,
    active: bool,
}
fn path(m: &Manager, operation: &str) -> Result<PathBuf> {
    Ok(job_dir(m, operation)?.join("service-executor.json"))
}
/// None identifies the historical external-worker format.
pub(super) fn live(m: &Manager, operation: &str) -> Result<Option<bool>> {
    let path = path(m, operation)?;
    if !path.try_exists()? { return Ok(None); }
    let owner: Owner = serde_json::from_slice(&control_artifact(path)?.record_bytes(4096)?)?;
    require(owner.schema == 1 && owner.operation == operation && owner.pid > 0
        && owner.start > 0 && valid_kernel_boot(&owner.boot), "operation_executor_identity")?;
    if !owner.active || kernel_boot()?.as_deref() != Some(owner.boot.as_str()) { return Ok(Some(false)); }
    match transport_storage::process_start_ticks(owner.pid) {
        Ok(start) => Ok(Some(start == owner.start)),
        Err(error) if error.downcast_ref::<std::io::Error>()
            .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) => Ok(Some(false)),
        Err(error) => Err(error),
    }
}
type Executions = BTreeSet<(PathBuf, String)>;
static EXECUTIONS: std::sync::LazyLock<Mutex<Executions>> = std::sync::LazyLock::new(Default::default);

pub(super) fn launch(m: &Manager, operation: &str) -> Result<bool> {
    launch_with_directory(m,operation,None)
}
pub(super) fn launch_with_directory(m:&Manager,operation:&str,input:Option<fs::File>)->Result<bool> {
    let key = (m.root.clone(), operation.to_owned());
    {
        let mut executions = EXECUTIONS.lock().map_err(|_| "operation_executor_lock")?;
        require(executions.len() < capacity::service_limits().service_workers, "operation_executor_capacity")?;
        require(executions.insert(key.clone()), "operation_executor_duplicate")?;
    }
    let retained: Result<Owner> = (|| {
        let owner = Owner { schema: 1, operation: operation.into(),
            boot: kernel_boot()?.ok_or("operation_requires_linux_kernel")?,
            pid: std::process::id() as i32,
            start: transport_storage::process_start_ticks(std::process::id() as i32)?, active: true };
        atomic_json(&path(m, operation)?, &owner)?;
        Ok(owner)
    })();
    let mut owner = match retained {
        Ok(owner) => owner,
        Err(error) => { EXECUTIONS.lock().map_err(|_| "operation_executor_lock")?.remove(&key); return Err(error); }
    };
    let manager = Manager::new(m.root.clone(), m.publications.clone());
    let id = operation.to_owned();
    let thread_key = key.clone();
    let launched = std::thread::Builder::new().name("bridge-operation".into()).spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operator_wait(|| {
            #[cfg(feature = "service-executor-fixture")]
            fixture_barrier(&manager, &id)?;
            library_location::with_input(input,||worker(&manager, &id))
        })));
        if !matches!(result, Ok(Ok(()))) {
            // Only a named operation failure is logged; selected file contents
            // and application/browser inputs never belong in this message.
            eprintln!("Operation {id} ended before its normal receipt");
        }
        if let Err(error) = finish_operation(&manager, &id) {
            eprintln!("Operation {id} cleanup remains unconfirmed: {error}");
        }
        owner.active = false;
        if let Err(error) = path(&manager, &id).and_then(|path| atomic_json(&path, &owner)) {
            eprintln!("Operation {id} executor retirement could not be retained: {error}");
        }
        if let Ok(mut executions) = EXECUTIONS.lock() { executions.remove(&thread_key); }
    });
    if let Err(error) = launched {
        EXECUTIONS.lock().map_err(|_| "operation_executor_lock")?.remove(&key);
        // No executor exists. The admission owner writes the refusal; retain
        // exact creation identity with active=false rather than erase evidence.
        let mut owner: Owner = read_json(&path(m, operation)?)?;
        owner.active = false;
        atomic_json(&path(m, operation)?, &owner)?;
        return Err(error.into());
    }
    Ok(true)
}
