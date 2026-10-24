//! Independent observation connection. A blocked chooser or command cannot stop
//! heartbeats. There is one replaceable pending event, never a replay queue.
pub use lvb_model::contract::subscription::{Cursor, Event, State, STALE_AFTER};
use lvb_model::contract::subscription::Subscription;
use std::{path::PathBuf, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration};

struct Shared { pending: Mutex<Option<Event>>, stopped: AtomicBool }
pub struct Watch { shared: Arc<Shared> }
impl Watch {
    pub fn start(home: PathBuf, executable: PathBuf, wake: impl Fn() + Send + 'static) -> std::io::Result<Self> {
        let shared = Arc::new(Shared { pending: Mutex::new(None), stopped: AtomicBool::new(false) });
        let receiver = shared.clone();
        std::thread::Builder::new().name("bridge-observation-client".into()).spawn(move || {
            #[cfg(target_os = "linux")]
            let client = crate::Client::managed(home.clone(), executable);
            #[cfg(not(target_os = "linux"))]
            let _ = executable;
            let publish = |event: Event| {
                let mut pending = receiver.pending.lock().unwrap_or_else(|p| p.into_inner());
                // Preserve a not-yet-consumed replacement when its heartbeat arrives.
                if !matches!((&*pending, &event), (Some(Event::State(_)), Event::Heartbeat { .. })) {
                    *pending = Some(event);
                }
                drop(pending);
                wake();
            };
            while !receiver.stopped.load(Ordering::Acquire) {
                #[cfg(target_os = "linux")]
                match client.as_ref().map_err(Clone::clone).and_then(|client| client.current_frontend()) {
                    Ok(true) => {},
                    Ok(false) => { publish(Event::FrontendSuperseded); break; },
                    Err(reason) => {
                        publish(Event::Unavailable { reason });
                        std::thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                }
                let result = Subscription::installed(&home).and_then(|mut stream| {
                    while !receiver.stopped.load(Ordering::Acquire) {
                        let event = stream.next()?;
                        let ended = matches!(event, Event::FrontendSuperseded | Event::Unavailable { .. });
                        let superseded = matches!(event, Event::FrontendSuperseded);
                        publish(event);
                        if superseded { receiver.stopped.store(true, Ordering::Release); }
                        if ended { break; }
                    }
                    Ok(())
                });
                if result.is_err() {
                    publish(Event::Unavailable { reason: "Live status is unavailable. Reconnecting…".into() });
                }
                if !receiver.stopped.load(Ordering::Acquire) { std::thread::sleep(Duration::from_secs(1)); }
            }
        })?;
        Ok(Self { shared })
    }
    pub fn take(&self) -> Option<Event> {
        self.shared.pending.lock().unwrap_or_else(|p| p.into_inner()).take()
    }
}
impl Drop for Watch {
    fn drop(&mut self) { self.shared.stopped.store(true, Ordering::Release); }
}
