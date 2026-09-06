//! Optional reference work has its own consumer. The transport only makes a
//! bounded copy into an SPSC queue; it never waits for observation or a reader.
use crate::{queue::Queue, queued::Observation, state::Witness};
use ap1_native_client::CAP;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
const CAPACITY: usize = 64;
#[derive(Clone, Copy, Debug, Default)]
pub struct Trace {
    pub epoch: u64,
    pub position: u64,
    pub frames: u64,
    pub sequence: u64,
    pub queued: Option<Instant>,
    pub started: Option<Instant>,
    pub prepared: Option<Instant>,
    pub sent: Option<Instant>,
    pub replied: Option<Instant>,
    pub validated: Option<Instant>,
    pub published: Option<Instant>,
}
#[derive(Clone, Copy, Debug)]
pub struct Gap {
    pub epoch: u64,
    pub position: u64,
    pub frames: u64,
    pub at: Instant,
}
#[derive(Clone, Copy)]
struct Job {
    sequence: u64,
    n: usize,
    gain: f64,
    state: Option<([u8; 12], bool)>,
    input: [[f32; CAP]; 2],
    output: [[u32; CAP + 2]; 2],
    trace: Trace,
}
#[derive(Default)]
pub struct Report {
    pub observation: Observation,
    pub unchecked: u64,
    pub reader_skips: u64,
    pub traces: Vec<(Gap, Option<Trace>)>,
}
pub struct Shared {
    jobs: Queue<Job>,
    pub gaps: Queue<Gap>,
    pub report: Mutex<Report>,
    pub offered: AtomicU64,
    pub dropped: AtomicU64,
    pub dropped_samples: AtomicU64,
    pub gap_drops: AtomicU64,
    stop: AtomicBool,
}
impl Shared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            jobs: Queue::new(CAPACITY),
            gaps: Queue::new(CAPACITY),
            report: Mutex::new(Report::default()),
            offered: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            dropped_samples: AtomicU64::new(0),
            gap_drops: AtomicU64::new(0),
            stop: AtomicBool::new(false),
        })
    }
}
pub struct Observer {
    pub shared: Arc<Shared>,
    sequence: u64,
    thread: Option<JoinHandle<()>>,
}
impl Observer {
    #[cfg(test)]
    pub(crate) fn paused() -> Self {
        Self {
            shared: Shared::new(),
            sequence: 0,
            thread: None,
        }
    }
    pub fn new() -> std::io::Result<Self> {
        let shared = Shared::new();
        let peer = shared.clone();
        let thread = thread::Builder::new()
            .name("ap7-observer".into())
            .spawn(move || {
                let mut consumer = Consumer::new();
                loop {
                    let did_work = consumer.step(&peer);
                    if !did_work && peer.stop.load(Ordering::Acquire) {
                        break;
                    }
                    if !did_work {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            })?;
        Ok(Self {
            shared,
            sequence: 0,
            thread: Some(thread),
        })
    }
    fn submit(&mut self, mut job: Job) {
        job.sequence = self.sequence;
        self.sequence += 1;
        self.shared
            .offered
            .fetch_add((job.n * 2) as u64, Ordering::Relaxed);
        if !self.shared.jobs.push(job) {
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            self.shared
                .dropped_samples
                .fetch_add((job.n * 2) as u64, Ordering::Relaxed);
        }
    }
    pub fn state(&mut self, bytes: &[u8], restore: bool) {
        // The mandatory state path validated this fixed reference-fixture extent.
        let Ok(bytes) = bytes.try_into() else {
            return;
        };
        self.submit(Job {
            sequence: 0,
            n: 0,
            gain: f64::NAN,
            state: Some((bytes, restore)),
            input: [[0.; CAP]; 2],
            output: [[0; CAP + 2]; 2],
            trace: Trace::default(),
        });
    }
    pub fn audio(
        &mut self,
        n: usize,
        gain: f64,
        input: [&[f32]; 2],
        output: [[u32; CAP + 2]; 2],
        trace: Trace,
    ) {
        let mut planes = [[0.; CAP]; 2];
        for ch in 0..2 {
            planes[ch][..n].copy_from_slice(input[ch]);
        }
        self.submit(Job {
            sequence: 0,
            n,
            gain,
            state: None,
            input: planes,
            output,
            trace,
        });
    }
    pub fn finish(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        self.finish();
    }
}
struct Consumer {
    witness: Witness,
    next: u64,
    unchecked: u64,
    reader_skips: u64,
    recent: std::collections::VecDeque<Trace>,
    gaps: Vec<(Gap, Option<Trace>)>,
}
fn covers(t: Trace, g: Gap) -> bool {
    t.epoch == g.epoch && t.position < g.position + g.frames && g.position < t.position + t.frames
}
impl Consumer {
    fn new() -> Self {
        Self {
            witness: Witness::new(),
            next: 0,
            unchecked: 0,
            reader_skips: 0,
            recent: std::collections::VecDeque::with_capacity(CAPACITY),
            gaps: Vec::with_capacity(32),
        }
    }
    fn step(&mut self, s: &Shared) -> bool {
        // Bounded per iteration; diagnostics never delay the transport producer.
        for _ in 0..CAPACITY {
            let Some(g) = s.gaps.pop() else {
                break;
            };
            if self.gaps.len() < 32 {
                let t = self.recent.iter().find(|t| covers(**t, g)).copied();
                self.gaps.push((g, t));
            } else {
                s.gap_drops.fetch_add(1, Ordering::Relaxed);
            }
        }
        let job = s.jobs.pop();
        if let Some(job) = job {
            if job.sequence != self.next {
                self.witness.ready = false;
            }
            self.next = job.sequence + 1;
            if let Some((bytes, restore)) = job.state {
                if self.witness.state(&bytes, restore).is_err() {
                    self.witness.ready = false;
                }
            } else {
                if self
                    .witness
                    .compare(
                        job.n,
                        job.gain,
                        [&job.input[0][..job.n], &job.input[1][..job.n]],
                        &job.output,
                    )
                    .is_err()
                {
                    self.unchecked += (job.n * 2) as u64;
                }
                for (g, t) in &mut self.gaps {
                    if t.is_none() && covers(job.trace, *g) {
                        *t = Some(job.trace);
                    }
                }
                if self.recent.len() == CAPACITY {
                    self.recent.pop_front();
                }
                self.recent.push_back(job.trace);
            }
        }
        // A poisoned/paused diagnostic reader only loses report coverage.
        if let Ok(mut report) = s.report.try_lock() {
            report.observation = Observation {
                comparison: self.witness.report,
                input_hash: self.witness.input_hash,
                output_hash: self.witness.output_hash,
            };
            report.unchecked = self.unchecked;
            report.reader_skips = self.reader_skips;
            report.traces.clone_from(&self.gaps);
        } else {
            self.reader_skips += 1;
        }
        job.is_some()
    }
}
pub fn micros(from: Option<Instant>, to: Option<Instant>) -> i64 {
    match (from, to) {
        (Some(a), Some(b)) => {
            if b >= a {
                b.duration_since(a).as_micros().min(i64::MAX as u128) as i64
            } else {
                -(a.duration_since(b).as_micros().min(i64::MAX as u128) as i64)
            }
        }
        _ => -1,
    }
}

pub fn report_text(s: &Shared) -> String {
    use std::fmt::Write;
    let Ok(r) = s.report.try_lock() else {
        return "{\"event\":\"ap7_observation\",\"report_error\":\"unavailable\"}\n".into();
    };
    let w = r.observation.comparison;
    let mut text = format!("{{\"event\":\"ap7_observation\",\"offered_samples\":{},\"verified_returned_samples\":{},\"unchecked_samples\":{},\"dropped_jobs\":{},\"dropped_samples\":{},\"reader_skips\":{},\"unretained_gap_traces\":{},\"maximum_error\":{},\"before_edit_samples\":{},\"edits\":{}}}\n",
        s.offered.load(Ordering::Acquire), w.samples, r.unchecked,
        s.dropped.load(Ordering::Relaxed), s.dropped_samples.load(Ordering::Relaxed),
        r.reader_skips, s.gap_drops.load(Ordering::Relaxed), w.maximum_error,
        w.before_edit_samples, w.edits);
    for (g, t) in &r.traces {
        if let Some(t) = t {
            let _ = writeln!(text, "{{\"event\":\"ap7_gap_request\",\"epoch\":{},\"gap_position\":{},\"gap_frames\":{},\"request_position\":{},\"request_frames\":{},\"sequence\":{},\"output_published\":{},\"queue_us\":{},\"prepare_us\":{},\"send_us\":{},\"reply_us\":{},\"validation_us\":{},\"publication_us\":{},\"publication_after_gap_us\":{},\"admission_to_gap_us\":{}}}",
                g.epoch, g.position, g.frames, t.position, t.frames, t.sequence, t.published.is_some(),
                micros(t.queued,t.started), micros(t.started,t.prepared), micros(t.prepared,t.sent),
                micros(t.sent,t.replied), micros(t.replied,t.validated), micros(t.validated,t.published),
                micros(Some(g.at),t.published), micros(t.queued,Some(g.at)));
        } else {
            let _ = writeln!(text, "{{\"event\":\"ap7_gap_request\",\"epoch\":{},\"gap_position\":{},\"gap_frames\":{},\"request_trace_missing\":true}}", g.epoch, g.position, g.frames);
        }
    }
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paused_consumer_and_reader_drop_only_observation_coverage() {
        let shared = Shared::new();
        let mut producer = Observer {
            shared: shared.clone(),
            sequence: 0,
            thread: None,
        };
        let mut consumer = Consumer::new();
        let bytes = [0.5f32.to_le_bytes(), 0f32.to_le_bytes(), 0u32.to_le_bytes()].concat();
        producer.state(&bytes, true);
        assert!(consumer.step(&shared));
        let input = [[0.25; CAP]; 2];
        let output = [[0.125f32.to_bits(); CAP + 2]; 2];
        // No consumer runs. Filling and overflowing the queue returns normally.
        for _ in 0..CAPACITY + 3 {
            producer.audio(
                CAP,
                f64::NAN,
                [&input[0], &input[1]],
                output,
                Trace::default(),
            );
        }
        assert_eq!(shared.dropped.load(Ordering::Relaxed), 3);
        let reader = shared.report.lock().unwrap();
        for _ in 0..CAPACITY {
            assert!(consumer.step(&shared));
        }
        assert_eq!(consumer.witness.report.samples, (CAPACITY * CAP * 2) as u64);
        assert_eq!(consumer.reader_skips, CAPACITY as u64);
        drop(reader);
        // A dropped job may have changed any reference state. Never guess it.
        producer.audio(
            CAP,
            f64::NAN,
            [&input[0], &input[1]],
            output,
            Trace::default(),
        );
        consumer.step(&shared);
        assert_eq!(consumer.unchecked, (CAP * 2) as u64);
        producer.state(&bytes, false);
        consumer.step(&shared);
        producer.audio(
            CAP,
            f64::NAN,
            [&input[0], &input[1]],
            output,
            Trace::default(),
        );
        consumer.step(&shared);
        let r = shared.report.lock().unwrap();
        assert_eq!(
            r.observation.comparison.samples,
            ((CAPACITY + 1) * CAP * 2) as u64
        );
        assert_eq!(r.observation.comparison.maximum_error, 0.);
        assert_eq!(
            shared.offered.load(Ordering::Relaxed),
            r.observation.comparison.samples
                + r.unchecked
                + shared.dropped_samples.load(Ordering::Relaxed)
        );
    }
}
