//! Non-RT logical-instance control. Neither sockets nor these mutexes are used
//! by process(). The owner timer takes fixed events; slow work runs here.
#[cfg(target_os = "linux")]
use crate::need;
use crate::state;
use ap1_native_client::recovery as wire;
#[cfg(target_os = "linux")]
use std::io::Write;
use std::{
    collections::BTreeMap,
    io,
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, Weak,
    },
    thread,
    time::{Duration, Instant},
};

// Logical close remains reachable while Registry::update excludes callback
// leases. Weak references neither keep a removed instance alive nor retain state.
static CONTROLS: Mutex<BTreeMap<u64, Weak<Control>>> = Mutex::new(BTreeMap::new());
pub(crate) fn lookup(id: u64) -> Option<Arc<Control>> {
    CONTROLS.lock().ok()?.get(&id)?.upgrade()
}
fn remember(id: u64, control: &Arc<Control>) {
    if let Ok(mut controls) = CONTROLS.lock() {
        controls.retain(|_, control| control.strong_count() != 0);
        controls.insert(id, Arc::downgrade(control));
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Event {
    pub stage: u32,
    pub reason: u32,
    pub generation: u64,
    pub operation: [u8; 16],
    pub active: u32,
    pub processing: u32,
}
#[derive(Clone, Copy)]
struct Desired {
    maximum: u32,
    mode: u32,
    active: bool,
    processing: bool,
}
struct State {
    command: Option<wire::Command>,
    phase: u32,
    reason: u32,
    start_taken: bool,
    readback: Option<Vec<u8>>,
    readback_taken: bool,
    controller: Option<bool>,
    final_ready: bool,
    final_taken: bool,
    desired: Option<Desired>,
    generation: u64,
    accepted: Option<Instant>,
}
pub struct Control {
    original: [u8; 16],
    root: PathBuf,
    inner: Mutex<State>,
    pub(crate) quit: AtomicBool,
    started: Instant,
    timer: AtomicU64,
    job: AtomicBool,
}
impl Control {
    #[cfg(test)]
    pub(crate) fn fixture_running(self: &Arc<Self>, id: u64) {
        remember(id, self);
        self.job.store(true, Ordering::Release);
    }
    #[cfg(test)]
    pub(crate) fn fixture_finish(&self, id: u64) {
        self.finish_closed(id);
    }
    pub fn new(original: [u8; 16], root: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            original,
            root,
            inner: Mutex::new(State {
                command: None,
                phase: wire::IDLE,
                reason: 0,
                start_taken: false,
                readback: None,
                readback_taken: false,
                controller: None,
                final_ready: false,
                final_taken: false,
                desired: None,
                generation: 1,
                accepted: None,
            }),
            quit: AtomicBool::new(false),
            started: Instant::now(),
            timer: AtomicU64::new(0),
            job: AtomicBool::new(false),
        })
    }
    pub fn start(self: &Arc<Self>, id: u64) {
        remember(id, self);
        let this = self.clone();
        let _ = thread::Builder::new()
            .name("bridge-proxy-control".into())
            .spawn(move || this.run(id));
    }
    fn status(&self, id: u64, last: wire::Status) -> wire::Status {
        let mut status = crate::queued::recovery_status(id).unwrap_or(last);
        status.flags &= !wire::OWNER_TIMER;
        let timer = self.timer.load(Ordering::Acquire);
        if timer != 0 && self.millis().saturating_sub(timer) < 2000 {
            status.flags |= wire::OWNER_TIMER;
        }
        if let Ok(mut state) = self.inner.lock() {
            if state.phase == wire::ACCEPTED
                && state
                    .accepted
                    .is_some_and(|at| at.elapsed() >= Duration::from_secs(5))
            {
                state.phase = wire::REFUSED;
                state.reason = 2;
            }
            status.operation = state.command.map_or([0; 16], |c| c.operation);
            status.phase = state.phase;
            status.reason = state.reason;
        }
        status
    }
    fn millis(&self) -> u64 {
        self.started.elapsed().as_millis().min(u64::MAX as u128 - 1) as u64 + 1
    }
    /// The restart worker owns eventual cleanup; SDK removal never waits for
    /// discovery, Windows startup or state restoration to finish.
    pub(crate) fn request_close(&self) -> bool {
        self.quit.store(true, Ordering::Release);
        self.job.load(Ordering::Acquire)
    }
    #[cfg(target_os = "linux")]
    fn connect(&self, status: &wire::Status) -> io::Result<UnixStream> {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        for (path, directory) in [(&self.root, true), (&self.root.join("owner.sock"), false)] {
            let meta = std::fs::symlink_metadata(path)?;
            need(
                meta.uid() == unsafe { libc::getuid() }
                    && meta.mode() & 0o077 == 0
                    && if directory {
                        meta.is_dir()
                    } else {
                        meta.file_type().is_socket()
                    },
                "proxy control custody",
            )?;
        }
        // AF_UNIX connect is nonblocking through the shared fixed helper.
        let mut stream = crate::control_socket::connect(
            &self.root.join("owner.sock"),
            Duration::from_millis(250),
        )?;
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_millis(250)))?;
        stream.set_write_timeout(Some(Duration::from_millis(250)))?;
        let mut deadline = wire::SocketDeadline::new(&mut stream, Duration::from_millis(250));
        deadline.write_all(wire::GREETING)?;
        deadline.write_all(&self.original)?;
        status.write(&mut deadline)?;
        need(
            wire::Command::read(&mut deadline)? == wire::Command::default(),
            "proxy initial command",
        )?;
        Ok(stream)
    }
    #[cfg(not(target_os = "linux"))]
    fn connect(&self, _status: &wire::Status) -> io::Result<UnixStream> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Bridge proxy control requires Linux",
        ))
    }
    fn run(self: Arc<Self>, id: u64) {
        let mut last = wire::Status {
            session: self.original,
            generation: 1,
            ..wire::Status::default()
        };
        while !self.quit.load(Ordering::Acquire) {
            last = self.status(id, last);
            if let Ok(mut stream) = self.connect(&last) {
                while !self.quit.load(Ordering::Acquire) {
                    last = self.status(id, last);
                    let mut deadline =
                        wire::SocketDeadline::new(&mut stream, Duration::from_millis(250));
                    let command = last
                        .write(&mut deadline)
                        .and_then(|_| wire::Command::read(&mut deadline));
                    match command {
                        Ok(command) if command.operation != [0; 16] => self.receive(command, &last),
                        Ok(_) => (),
                        Err(_) => break,
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
            // Reconnect re-establishes observation only. It never replays a command.
            for _ in 0..10 {
                if self.quit.load(Ordering::Acquire) {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    fn receive(&self, command: wire::Command, status: &wire::Status) {
        let Ok(mut state) = self.inner.lock() else {
            return;
        };
        if state
            .command
            .is_some_and(|prior| prior.operation == command.operation)
        {
            return;
        }
        if !matches!(state.phase, wire::IDLE | wire::COMPLETE | wire::REFUSED) {
            return;
        }
        if !status.ready() || !command.matches(status) {
            state.command = Some(command);
            state.phase = wire::REFUSED;
            state.reason = 1;
            return;
        }
        state.command = Some(command);
        state.phase = wire::ACCEPTED;
        state.reason = 0;
        state.start_taken = false;
        state.accepted = Some(Instant::now());
        state.readback = None;
        state.readback_taken = false;
        state.controller = None;
        state.final_ready = false;
        state.final_taken = false;
    }
    fn refuse(&self, reason: u32) {
        if let Ok(mut state) = self.inner.lock() {
            state.phase = wire::REFUSED;
            state.reason = reason;
        }
    }
    fn wait(&self, read: impl Fn(&State) -> Option<bool>) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.quit.load(Ordering::Acquire) && Instant::now() < deadline {
            if let Ok(state) = self.inner.lock() {
                if let Some(result) = read(&state) {
                    return result;
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        false
    }
    fn restore(self: Arc<Self>, id: u64, command: wire::Command, desired: Desired) {
        let result = unsafe { crate::queued::recover_exact(id, &command, self.original) };
        let (readback, generation) = match result {
            Ok(value) => value,
            Err(_) => {
                // Admission/startup may have accepted a replacement before its
                // response was lost. Do not turn that into permission to replay.
                if let Ok(mut state) = self.inner.lock() {
                    state.phase = wire::UNCERTAIN;
                    state.reason = 8;
                }
                self.finish_closed(id);
                return;
            }
        };
        if let Ok(mut state) = self.inner.lock() {
            state.phase = wire::CONTROLLER;
            state.readback = Some(readback);
            state.generation = generation;
        }
        self.start_restored(id, desired);
    }
    // Shared by ordinary restoration and the test-only installed-transport seam.
    fn start_restored(self: Arc<Self>, id: u64, desired: Desired) {
        if !self.wait(|s| s.controller) {
            unsafe { crate::queued::abandon_recovery(id) };
            self.refuse(6);
            self.finish_closed(id);
            return;
        }
        let active = (!desired.active
            || unsafe { crate::queued::ap4_activate(id, desired.maximum, desired.mode) } == 0)
            && (!desired.processing
                || (unsafe { crate::queued::ap3_transition(id, 10) } == 0
                    && crate::queued::wait_started(id).is_ok()));
        if !active {
            unsafe { crate::queued::abandon_recovery(id) };
            self.refuse(7);
            self.finish_closed(id);
            return;
        }
        if let Ok(mut state) = self.inner.lock() {
            state.final_ready = true;
        }
        if !self.wait(Self::completion_result) && self.final_timeout_requires_cleanup() {
            unsafe { crate::queued::abandon_recovery(id) };
        }
        self.finish_closed(id);
    }
    fn final_timeout_requires_cleanup(&self) -> bool {
        let Ok(mut state) = self.inner.lock() else {
            return true;
        };
        // The owner acknowledgement and expiry choose one terminal outcome
        // under this lock, before any replacement retirement starts.
        if state.phase == wire::COMPLETE {
            return false;
        }
        if !matches!(state.phase, wire::REFUSED | wire::UNCERTAIN) {
            state.phase = wire::UNCERTAIN;
            state.reason = 8;
        }
        true
    }
    fn completion_result(state: &State) -> Option<bool> {
        match state.phase {
            wire::COMPLETE => Some(true),
            wire::REFUSED | wire::UNCERTAIN => Some(false),
            _ => None,
        }
    }
    fn finish_closed(&self, id: u64) {
        self.job.store(false, Ordering::Release);
        if self.quit.load(Ordering::Acquire) {
            unsafe { crate::queued::close_abandoned_recovery(id) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> wire::Status {
        wire::Status {
            session: [1; 16],
            generation: 7,
            revision: 9,
            digest: [8; 32],
            flags: wire::FAILED | wire::RETIRED | wire::SNAPSHOT | wire::OWNER_TIMER,
            ..Default::default()
        }
    }
    fn command() -> wire::Command {
        wire::Command {
            operation: [2; 16],
            generation: 7,
            revision: 9,
            digest: [8; 32],
        }
    }
    #[test]
    fn owner_timer_expires_and_accepted_but_undispatched_command_cannot_start_late() {
        let control = Control::new([1; 16], PathBuf::new());
        assert_eq!(
            control.status(u64::MAX, ready()).flags & wire::OWNER_TIMER,
            0
        );
        control.timer.store(control.millis(), Ordering::Release);
        assert_ne!(
            control.status(u64::MAX, ready()).flags & wire::OWNER_TIMER,
            0
        );
        let mut expired = Control::new([1; 16], PathBuf::new());
        Arc::get_mut(&mut expired).unwrap().started = Instant::now() - Duration::from_secs(3);
        expired.timer.store(1, Ordering::Release);
        assert_eq!(
            expired.status(u64::MAX, ready()).flags & wire::OWNER_TIMER,
            0
        );
        control.receive(command(), &ready());
        control.inner.lock().unwrap().accepted = Some(Instant::now() - Duration::from_secs(6));
        let status = control.status(u64::MAX, ready());
        assert_eq!((status.phase, status.reason), (wire::REFUSED, 2));
        control.receive(command(), &ready()); // delayed duplicate never reopens it
        assert_eq!(control.inner.lock().unwrap().phase, wire::REFUSED);
        assert!(!control.job.load(Ordering::Acquire));
    }
    #[test]
    fn changed_snapshot_refuses_and_one_accepted_operation_is_never_replayed() {
        let control = Control::new([1; 16], PathBuf::new());
        let mut changed = ready();
        changed.digest[0] ^= 1;
        control.receive(command(), &changed);
        assert_eq!(control.inner.lock().unwrap().phase, wire::REFUSED);
        control.receive(command(), &ready());
        assert_eq!(control.inner.lock().unwrap().phase, wire::REFUSED);
        let mut next = command();
        next.operation = [3; 16];
        control.receive(next, &ready());
        assert_eq!(control.inner.lock().unwrap().phase, wire::ACCEPTED);
        control.inner.lock().unwrap().phase = wire::UNCERTAIN;
        let mut another = next;
        another.operation = [4; 16];
        control.receive(another, &ready());
        let state = control.inner.lock().unwrap();
        assert_eq!(state.command, Some(next));
        assert_eq!(state.phase, wire::UNCERTAIN);
    }
    #[test]
    fn final_timeout_fences_late_poll_and_ack_without_overwriting_acknowledged_completion() {
        let control = Control::new([1; 16], PathBuf::new());
        let id = u64::MAX - 2;
        remember(id, &control);
        let prepare = || {
            let mut state = control.inner.lock().unwrap();
            state.command = Some(command());
            state.phase = wire::CONTROLLER;
            state.reason = 0;
            state.readback_taken = true;
            state.final_ready = true;
            state.final_taken = false;
        };
        let poll = || {
            let mut event = Event::default();
            let mut bytes = state::OwnedState {
                data: std::ptr::null(),
                length: 0,
                abi_version: 1,
            };
            assert_eq!(
                unsafe { lvb_recovery_poll_v1(id, 1, &mut event, &mut bytes) },
                0
            );
            assert!(bytes.data.is_null());
            event.stage
        };
        let acknowledge = || unsafe { lvb_recovery_ack_v1(id, command().operation.as_ptr(), 3, 1) };

        prepare();
        assert!(control.final_timeout_requires_cleanup());
        assert_eq!(poll(), 4); // timer resumes after the deadline
        assert!(!control.inner.lock().unwrap().final_taken);
        assert_ne!(acknowledge(), 0);
        assert_eq!(control.inner.lock().unwrap().phase, wire::UNCERTAIN);

        prepare();
        assert_eq!(poll(), 3);
        assert!(control.final_timeout_requires_cleanup()); // expires between event and ack
        assert_ne!(acknowledge(), 0);
        assert_eq!(control.inner.lock().unwrap().phase, wire::UNCERTAIN);

        prepare();
        assert_eq!(poll(), 3);
        assert_eq!(acknowledge(), 0); // the acknowledgement wins the same lock
        assert!(!control.final_timeout_requires_cleanup());
        assert_eq!(control.inner.lock().unwrap().phase, wire::COMPLETE);

        prepare();
        control.quit.store(true, Ordering::Release);
        assert_eq!(poll(), 0);
        assert!(!control.inner.lock().unwrap().final_taken);
        assert_ne!(acknowledge(), 0);
    }

    #[test]
    fn negative_owner_final_ack_refuses_and_releases_replacement_cleanup_wait() {
        let control = Control::new([1; 16], PathBuf::new());
        let id = u64::MAX - 1;
        remember(id, &control);
        {
            let mut state = control.inner.lock().unwrap();
            state.command = Some(command());
            state.phase = wire::CONTROLLER;
            state.final_taken = true;
        }
        assert_eq!(
            unsafe { lvb_recovery_ack_v1(id, command().operation.as_ptr(), 3, 0) },
            0
        );
        let start = Instant::now();
        assert!(!control.wait(Control::completion_result));
        assert!(start.elapsed() < Duration::from_millis(500));
        let state = control.inner.lock().unwrap();
        assert_eq!((state.phase, state.reason), (wire::REFUSED, 9));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lvb_recovery_poll_v1(
    id: u64,
    timer: u32,
    event: *mut Event,
    bytes: *mut state::OwnedState,
) -> u32 {
    crate::ffi(|| {
        if event.is_null() || !state::owned_output_ready(bytes) {
            return 1;
        }
        *event = Event::default();
        let Some(control) = crate::queued::proxy_control(id) else {
            return 0;
        };
        control.timer.store(
            if timer == 1 { control.millis() } else { 0 },
            Ordering::Release,
        );
        let Ok(mut state) = control.inner.try_lock() else {
            return 0;
        };
        if control.quit.load(Ordering::Acquire) {
            return 0;
        }
        let Some(command) = state.command else {
            return 0;
        };
        event.write(Event {
            operation: command.operation,
            generation: state.generation,
            reason: state.reason,
            active: state.desired.map_or(0, |d| d.active as u32),
            processing: state.desired.map_or(0, |d| d.processing as u32),
            ..Event::default()
        });
        if state.phase == wire::ACCEPTED && !state.start_taken {
            state.start_taken = true;
            (*event).stage = 1;
        } else if state.phase == wire::CONTROLLER && !state.readback_taken {
            if let Some(readback) = state.readback.take() {
                state.readback_taken = true;
                state::give_owned(bytes, readback);
                (*event).stage = 2;
            }
        } else if state.phase == wire::CONTROLLER && state.final_ready && !state.final_taken {
            state.final_taken = true;
            (*event).stage = 3;
        } else if state.phase >= wire::REFUSED {
            (*event).stage = 4;
        }
        0
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn lvb_recovery_begin_v1(
    id: u64,
    operation: *const u8,
    maximum: u32,
    mode: u32,
    active: u32,
    processing: u32,
) -> u32 {
    crate::ffi(|| {
        if operation.is_null() || active > 1 || processing > active {
            return 1;
        }
        let Some(control) = crate::queued::proxy_control(id) else {
            return 1;
        };
        let operation: [u8; 16] = std::slice::from_raw_parts(operation, 16)
            .try_into()
            .unwrap();
        let desired = Desired {
            maximum,
            mode,
            active: active != 0,
            processing: processing != 0,
        };
        let command = {
            let Ok(mut state) = control.inner.lock() else {
                return 1;
            };
            let Some(command) = state.command else {
                return 1;
            };
            if command.operation != operation
                || state.phase != wire::ACCEPTED
                || !state.start_taken
                || control.quit.load(Ordering::Acquire)
            {
                return 1;
            }
            control.job.store(true, Ordering::Release);
            state.phase = wire::RESTORING;
            state.desired = Some(desired);
            command
        };
        let worker = control.clone();
        if thread::Builder::new()
            .name("bridge-instance-restart".into())
            .spawn(move || worker.restore(id, command, desired))
            .is_err()
        {
            control.refuse(2);
            control.finish_closed(id);
            return 1;
        }
        0
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn lvb_recovery_ack_v1(
    id: u64,
    operation: *const u8,
    stage: u32,
    success: u32,
) -> u32 {
    crate::ffi(|| {
        if operation.is_null() || success > 1 {
            return 1;
        }
        let Some(control) = crate::queued::proxy_control(id) else {
            return 1;
        };
        let operation: [u8; 16] = std::slice::from_raw_parts(operation, 16)
            .try_into()
            .unwrap();
        let Ok(mut state) = control.inner.lock() else {
            return 1;
        };
        if state.command.is_none_or(|c| c.operation != operation)
            || state.phase != wire::CONTROLLER
            || control.quit.load(Ordering::Acquire)
        {
            return 1;
        }
        match stage {
            2 if state.readback_taken => state.controller = Some(success != 0),
            3 if state.final_taken => {
                state.phase = if success == 1 {
                    wire::COMPLETE
                } else {
                    wire::REFUSED
                };
                if success == 0 {
                    state.reason = 9;
                }
            }
            _ => return 1,
        }
        0
    }) as u32
}

// Test-only seam after admission/opaque restoration. START, its socket reply,
// final polling/acknowledgement, real deadline and abandonment remain production.
#[cfg(feature = "recovery-integration")]
impl Control {
    pub(crate) fn integration_busy(&self) -> bool { self.job.load(Ordering::Acquire) }
    pub(crate) fn integration_begin(self: &Arc<Self>, id: u64, maximum: u32, mode: u32) {
        remember(id, self);
        let desired = Desired { maximum, mode, active: true, processing: true };
        {
            let mut state = self.inner.lock().unwrap();
            state.command = Some(wire::Command { operation: [2;16], ..Default::default() });
            state.phase = wire::CONTROLLER;
            state.generation = 8;
            state.desired = Some(desired);
            state.readback = Some(vec![]);
        }
        self.job.store(true, Ordering::Release);
        let control = self.clone();
        thread::spawn(move || control.start_restored(id, desired));
    }
}
#[cfg(feature = "recovery-integration")]
#[no_mangle]
pub extern "C" fn lvb_test_recovery_phase(id: u64) -> u32 {
    let control = lookup(id).unwrap();
    let state = control.inner.lock().unwrap();
    if state.phase == wire::CONTROLLER && state.final_ready { 3 }
    else if state.phase >= wire::REFUSED { 4 }
    else { 0 }
}

#[cfg(feature = "recovery-integration")]
#[no_mangle]
pub extern "C" fn lvb_test_recovery_busy(id: u64) -> u32 {
    u32::from(lookup(id).is_some_and(|control| control.integration_busy()))
}
