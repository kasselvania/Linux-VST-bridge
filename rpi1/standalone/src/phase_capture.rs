//! RPI1 control-plane drain for the fixed callback ring. All formatting and
//! filesystem work happen on this ordinary thread, never in JACK's callback.
use ap2_backend::rpi1_phase::{self as phase, Event, Ring};
use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const WINDOW_NS: u64 = 60_000_000_000;
const MAX_EVENTS: usize = 65_536;

#[derive(Clone, Copy)]
struct SourceEvent {
    event: Event,
    domain: &'static str,
    tid: u64,
}

pub struct PhaseCapture {
    stop: Arc<AtomicBool>,
    markers: mpsc::Sender<(u64, String)>,
    thread: Option<JoinHandle<io::Result<()>>>,
}
impl PhaseCapture {
    pub fn start(
        rings: (Arc<Ring>, Arc<Ring>),
        directory: &Path,
        session: &str,
    ) -> io::Result<Self> {
        let path = directory.join(format!("{session}-phase.jsonl"));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        let (markers, receiver) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let session = session.to_owned();
        let thread = thread::Builder::new()
            .name("rpi1-phase-drain".into())
            .spawn(move || drain(file, &session, rings, receiver, thread_stop))?;
        Ok(Self {
            stop,
            markers,
            thread: Some(thread),
        })
    }
    pub fn mark(&self, marker: &str) -> io::Result<()> {
        self.markers
            .send((monotonic_ns(), marker.to_owned()))
            .map_err(|_| io::Error::other("phase capture retired"))
    }
    pub fn finish(mut self) -> io::Result<()> {
        self.stop.store(true, Ordering::Release);
        self.thread
            .take()
            .unwrap()
            .join()
            .map_err(|_| io::Error::other("phase capture panicked"))?
    }
}
impl Drop for PhaseCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn drain(
    mut file: File,
    session: &str,
    rings: (Arc<Ring>, Arc<Ring>),
    markers: mpsc::Receiver<(u64, String)>,
    stop: Arc<AtomicBool>,
) -> io::Result<()> {
    let mut history = VecDeque::<SourceEvent>::with_capacity(MAX_EVENTS);
    let mut last_drop = [0, 0];
    let mut last_incident_flush = 0;
    loop {
        let mut incident = false;
        let mut terminal = false;
        for (ring, domain) in [
            (&rings.0, "native_jack_callback"),
            (&rings.1, "native_bridge_worker"),
        ] {
            while let Some(event) = ring.pop() {
                if event.kind == phase::GAP_COMMITTED || event.kind == phase::FAULT_COMMITTED {
                    incident = true;
                }
                if event.kind == phase::FAULT_COMMITTED {
                    terminal = true;
                }
                while history.front().is_some_and(|old| {
                    event.monotonic_ns.saturating_sub(old.event.monotonic_ns) > WINDOW_NS
                }) {
                    history.pop_front();
                }
                if history.len() == MAX_EVENTS {
                    history.pop_front();
                }
                history.push_back(SourceEvent {
                    event,
                    domain,
                    tid: ring.producer_tid(),
                });
            }
        }
        let mut marked = false;
        while let Ok((at, marker)) = markers.try_recv() {
            writeln!(
                file,
                "{}",
                serde_json::json!({"event":"rpi1_phase_marker",
                "session":session, "name":marker,
                "monotonic_ns":at})
            )?;
            incident = true;
            marked = true;
        }
        for (index, ring) in [&rings.0, &rings.1].into_iter().enumerate() {
            if ring.dropped() != last_drop[index] {
                last_drop[index] = ring.dropped();
                writeln!(
                    file,
                    "{}",
                    serde_json::json!({"event":"rpi1_phase_dropped",
                    "session":session, "producer":index, "count":last_drop[index],
                    "monotonic_ns":monotonic_ns()})
                )?;
                incident = true;
            }
        }
        let now = monotonic_ns();
        let should_flush = terminal
            || marked
            || stop.load(Ordering::Acquire)
            || incident
                && (last_incident_flush == 0
                    || now.saturating_sub(last_incident_flush) >= 5_000_000_000);
        if should_flush {
            flush_history(&mut file, session, &mut history)?;
            file.sync_data()?;
            last_incident_flush = now;
        }
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn flush_history(
    file: &mut File,
    session: &str,
    history: &mut VecDeque<SourceEvent>,
) -> io::Result<()> {
    while let Some(source) = history.pop_front() {
        let event = source.event;
        let converted = matches!(
            event.kind,
            phase::WORKER_PROCESS_BEGIN
                | phase::WORKER_PREPARED
                | phase::WORKER_SENT
                | phase::WORKER_REPLIED
                | phase::WORKER_VALIDATED
        );
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "event":"rpi1_phase", "session":session,
                "monotonic_ns":if converted { event.value_1 } else { event.monotonic_ns },
                "recorded_ns":event.monotonic_ns,
                "domain":source.domain, "pid":std::process::id(), "tid":source.tid,
                "callback_sequence":event.callback_sequence,
                "bridge_position":event.bridge_position,
                "phase":phase::event_name(event.kind), "detail":event.detail,
                "value_1":event.value_1, "value_2":event.value_2,
                "raw_fault_code":if event.kind == phase::FAULT_COMMITTED {
                    Some(event.value_1) } else { None },
            "symbolic_fault":if event.kind == phase::FAULT_COMMITTED {
                Some(phase::fault_name(event.value_1, event.detail)) } else { None },
                "fault_site":if event.kind == phase::FAULT_COMMITTED {
                    Some(phase::site_name(event.detail)) } else { None },
            })
        )?;
    }
    Ok(())
}
fn monotonic_ns() -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time);
    }
    (time.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(time.tv_nsec as u64)
}
