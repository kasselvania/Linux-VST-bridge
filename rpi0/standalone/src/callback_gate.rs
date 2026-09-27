use std::{
    io,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    thread,
    time::{Duration, Instant},
};

/// Keeps control-plane lifecycle work from racing the JACK process callback.
/// Enter/drop is callback-safe: it uses only fixed-width atomics.
pub struct CallbackGate {
    enabled: AtomicBool,
    in_flight: AtomicU32,
}

pub struct CallbackLease<'a> {
    in_flight: &'a AtomicU32,
}

impl CallbackGate {
    pub const fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            in_flight: AtomicU32::new(0),
        }
    }

    pub fn enter(&self) -> Option<CallbackLease<'_>> {
        self.in_flight.fetch_add(1, Ordering::AcqRel);
        if self.enabled.load(Ordering::Acquire) {
            Some(CallbackLease {
                in_flight: &self.in_flight,
            })
        } else {
            self.in_flight.fetch_sub(1, Ordering::Release);
            None
        }
    }

    pub fn resume(&self) {
        self.enabled.store(true, Ordering::Release);
    }

    pub fn pause(&self, timeout: Duration) -> io::Result<()> {
        self.enabled.store(false, Ordering::Release);
        let deadline = Instant::now() + timeout;
        while self.in_flight.load(Ordering::Acquire) != 0 {
            if Instant::now() >= deadline {
                return Err(io::Error::other("JACK callback quiescence timeout"));
            }
            thread::sleep(Duration::from_micros(50));
        }
        Ok(())
    }
}

impl Drop for CallbackLease<'_> {
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc};

    #[test]
    fn paused_gate_refuses_callback_entry_and_resumes() {
        let gate = CallbackGate::new();
        assert!(gate.enter().is_none());
        gate.resume();
        assert!(gate.enter().is_some());
        gate.pause(Duration::from_millis(10)).unwrap();
        assert!(gate.enter().is_none());
    }

    #[test]
    fn pause_waits_for_the_exact_in_flight_callback() {
        let gate = Arc::new(CallbackGate::new());
        gate.resume();
        let lease = gate.enter().unwrap();
        let (sender, receiver) = mpsc::channel();
        let worker_gate = Arc::clone(&gate);
        let worker = thread::spawn(move || {
            sender.send(()).unwrap();
            worker_gate.pause(Duration::from_secs(1))
        });
        receiver.recv().unwrap();
        while gate.enabled.load(Ordering::Acquire) {
            thread::yield_now();
        }
        assert!(!worker.is_finished());
        drop(lease);
        worker.join().unwrap().unwrap();
        assert!(gate.enter().is_none());
    }
}
