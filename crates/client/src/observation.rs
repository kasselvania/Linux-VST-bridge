//! Independent observation connection. A blocked chooser or command cannot stop
//! heartbeats. There is one replaceable pending event, never a replay queue.
pub use lvb_model::contract::subscription::{Cursor, Event, State, STALE_AFTER};
use lvb_model::contract::subscription::Subscription;
use std::{path::PathBuf, sync::{Arc, Mutex}, time::{Duration, Instant}};

/// Local receipt time, never the later UI consumption time.
pub struct Received { pub event: Event, pub at: Instant }
impl Received {
    pub fn now(event: Event) -> Self { Self { event, at: Instant::now() } }
}

#[derive(Default)]
struct Pending { event: Option<Received>, restart: bool, stopped: bool }
impl Pending {
    fn publish(&mut self, received: Received) -> bool {
        if self.stopped { return false; }
        // Terminal publication and stopping share restart/take's lock. Neither
        // restart nor a late nonterminal result can erase this notification.
        if matches!(received.event, Event::FrontendSuperseded) {
            self.stopped = true;
            self.restart = false;
            self.event = Some(received);
            return false;
        }
        if self.restart { return false; }
        // Keep the replacement payload while renewing only its exact cursor.
        if let (Some(Received { event: Event::State(state), at }), Event::Heartbeat { cursor }) = (&mut self.event, &received.event) {
            if state.cursor == *cursor { *at = (*at).max(received.at); }
        } else {
            self.event = Some(received);
        }
        true
    }
}
struct Shared { pending: Mutex<Pending> }
pub struct Watch { shared: Arc<Shared> }
impl Watch {
    pub fn start(home: PathBuf, executable: PathBuf, wake: impl Fn() + Send + 'static) -> std::io::Result<Self> {
        Self::start_with_admission(home.clone(), move || {
            #[cfg(target_os = "linux")]
            let client = crate::Client::managed(home, executable);
            #[cfg(not(target_os = "linux"))]
            let _ = (home, executable);
            move || {
                #[cfg(target_os = "linux")]
                { client.as_ref().map_err(Clone::clone).and_then(|client| client.current_frontend()) }
                #[cfg(not(target_os = "linux"))]
                { Ok(true) }
            }
        }, wake)
    }
    fn start_with_admission<A: FnMut() -> Result<bool, String> + Send + 'static>(
        home: PathBuf, admission: impl FnOnce() -> A + Send + 'static,
        wake: impl Fn() + Send + 'static,
    ) -> std::io::Result<Self> {
        let shared = Arc::new(Shared { pending: Mutex::new(Pending::default()) });
        let receiver = shared.clone();
        std::thread::Builder::new().name("bridge-observation-client".into()).spawn(move || {
            let mut admit = admission();
            let publish = |received: Received| {
                let keep_receiving = receiver.pending.lock().unwrap_or_else(|p| p.into_inner()).publish(received);
                wake();
                keep_receiving
            };
            loop {
                {
                    let mut pending = receiver.pending.lock().unwrap_or_else(|p| p.into_inner());
                    if pending.stopped { break; }
                    pending.restart = false;
                }
                match admit() {
                    Ok(true) => {},
                    Ok(false) => { publish(Received::now(Event::FrontendSuperseded)); break; },
                    Err(reason) => {
                        publish(Received::now(Event::Unavailable { reason }));
                        std::thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                }
                let result = Subscription::installed(&home).and_then(|mut stream| {
                    loop {
                        if receiver.pending.lock().unwrap_or_else(|p| p.into_inner()).stopped { break; }
                        let received = Received::now(stream.receive()?);
                        let ended = matches!(received.event, Event::FrontendSuperseded | Event::Unavailable { .. });
                        // Do not discard terminal input even when restart was
                        // requested while receive was blocked.
                        if !publish(received) || ended { break; }
                    }
                    Ok(())
                });
                if result.is_err() {
                    publish(Received::now(Event::Unavailable { reason: "Live status is unavailable. Reconnecting…".into() }));
                }
                if !receiver.pending.lock().unwrap_or_else(|p| p.into_inner()).stopped { std::thread::sleep(Duration::from_secs(1)); }
            }
        })?;
        Ok(Self { shared })
    }
    pub fn restart(&self) {
        let mut pending = self.shared.pending.lock().unwrap_or_else(|p| p.into_inner());
        if pending.stopped { return; }
        pending.event = None;
        pending.restart = true;
    }
    pub fn take(&self) -> Option<Received> {
        self.shared.pending.lock().unwrap_or_else(|p| p.into_inner()).event.take()
    }
}
impl Drop for Watch {
    fn drop(&mut self) { self.shared.pending.lock().unwrap_or_else(|p| p.into_inner()).stopped = true; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn coalesced_state_keeps_exact_cursor_latest_receipt() {
        let cursor = Cursor { service_epoch: "ab".repeat(16), revision: 1 };
        let old = Instant::now() - Duration::from_secs(10);
        let fresh = Instant::now();
        let mut pending = Pending::default();
        pending.publish(Received { event: Event::State(Box::new(State {
            cursor: cursor.clone(), records_revision: Some(1),
            activity: lvb_model::operator_model::Observation::Unavailable("fixture".into()),
        })), at: old });
        let mut wrong = cursor.clone(); wrong.revision += 1;
        pending.publish(Received { event: Event::Heartbeat { cursor: wrong }, at: fresh });
        assert_eq!(pending.event.as_ref().unwrap().at, old);
        pending.publish(Received { event: Event::Heartbeat { cursor: cursor.clone() }, at: fresh });
        let received = pending.event.take().unwrap();
        assert_eq!(received.at, fresh);
        assert!(matches!(received.event, Event::State(state) if state.cursor == cursor));
    }

    #[test]
    fn restart_between_take_and_admission_false_exit_preserves_terminal() {
        let (admit, admission) = mpsc::channel();
        let (published, publication) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let watch = Watch::start_with_admission("unused".into(), move || move || {
            admission.recv().unwrap(); Ok(false)
        }, move || {
            published.send(()).unwrap(); released.recv().unwrap();
        }).unwrap();
        assert!(watch.take().is_none());
        admit.send(()).unwrap();
        publication.recv_timeout(Duration::from_secs(2)).unwrap();
        // Worker is held after publication, before returning from admission.
        watch.restart();
        assert!(matches!(watch.take().unwrap().event, Event::FrontendSuperseded));
        watch.restart();
        let pending = watch.shared.pending.lock().unwrap();
        assert!(pending.stopped && !pending.restart);
        drop(pending);
        release.send(()).unwrap();
    }

    #[test]
    fn terminal_receive_wins_restart_and_late_nonterminal_publication() {
        let watch = Watch { shared: Arc::new(Shared { pending: Mutex::new(Pending::default()) }) };
        assert!(watch.take().is_none());
        for restart_first in [false, true] {
            *watch.shared.pending.lock().unwrap() = Pending::default();
            if restart_first { watch.restart(); }
            assert!(!watch.shared.pending.lock().unwrap().publish(Received::now(Event::FrontendSuperseded)));
            watch.restart();
            assert!(!watch.shared.pending.lock().unwrap().publish(Received::now(Event::Unavailable { reason: "late".into() })));
            assert!(matches!(watch.take().unwrap().event, Event::FrontendSuperseded));
            watch.restart();
            let pending = watch.shared.pending.lock().unwrap();
            assert!(pending.stopped && !pending.restart);
        }
    }
}
