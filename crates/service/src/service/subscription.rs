//! One sampler and one replaceable state shared by every subscribed window.
//! Slow subscribers never queue history or occupy native admission workers.
use crate::{operator_model::{Activity, Observation}, service::contract::subscription as wire};
use std::{io, os::unix::net::UnixStream, sync::{Arc, Condvar, Mutex, atomic::{AtomicUsize, Ordering}}, time::{Duration, Instant}};

const SUBSCRIBERS: usize = 4;
const RECORD_INTERVAL: Duration = Duration::from_millis(100);
const LIVE_INTERVAL: Duration = Duration::from_millis(250);
/// Explicit reads can arrive between a record change and its next sample.
/// Wait off the window thread, within four normal sampling intervals.
const READ_SETTLE: Duration = Duration::from_secs(1);
struct Held { state: wire::State, encoded: Vec<u8>, activity_at: Instant, records_at: Instant }
pub(crate) struct Publisher {
    held: Mutex<Held>,
    changed: Condvar,
    subscribers: AtomicUsize,
}
impl Publisher {
    /// Wait for the shared sampler to cover the current records. This never
    /// composes another census or renews the sampler's freshness timestamps.
    pub fn activity_when_current(&self, records: impl Fn() -> Option<u64>) -> crate::Result<Activity> {
        self.activity_within(records, READ_SETTLE)
    }
    fn activity_within(&self, records: impl Fn() -> Option<u64>, bound: Duration) -> crate::Result<Activity> {
        let deadline = Instant::now() + bound;
        loop {
            let revision = records();
            let held = self.held.lock().unwrap_or_else(|p| p.into_inner());
            let mut reason = "Current activity is not yet available. Try again shortly.".to_owned();
            if revision.is_some() && held.state.records_revision == revision
                && held.activity_at.elapsed() < wire::STALE_AFTER && held.records_at.elapsed() < wire::STALE_AFTER {
                match &held.state.activity {
                    Observation::Observed(activity) => return Ok(activity.clone()),
                    Observation::Unavailable(detail) => reason = detail.clone(),
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() { return Err(reason.into()); }
            // Drop the publisher lock before consulting the record watch.
            // The short cap also covers a record change between reading its
            // revision and taking this lock, even if its notification passed.
            drop(self.changed.wait_timeout(held, remaining.min(RECORD_INTERVAL))
                .unwrap_or_else(|p| p.into_inner()).0);
        }
    }
    /// Explicit Activity reads consume the same observation as subscribers.
    /// Reading never renews its timestamp or reconstructs a second census.
    #[cfg(test)]
    fn activity(&self, records_revision: Option<u64>) -> crate::Result<Activity> {
        let held = self.held.lock().unwrap_or_else(|p| p.into_inner());
        crate::require(records_revision.is_some() && held.state.records_revision == records_revision
            && held.activity_at.elapsed() < wire::STALE_AFTER && held.records_at.elapsed() < wire::STALE_AFTER,
            "Current activity is not yet available for these records")?;
        match &held.state.activity {
            Observation::Observed(activity) => Ok(activity.clone()),
            Observation::Unavailable(reason) => Err(reason.clone().into()),
        }
    }
    pub fn start(
        mut records: impl FnMut() -> Option<u64> + Send + 'static,
        mut activity: impl FnMut() -> (Option<u64>, Observation<Activity>) + Send + 'static,
    ) -> crate::Result<Arc<Self>> {
        let publisher = Arc::new(Self {
            held: Mutex::new(Held { state: wire::State {
                cursor: wire::Cursor { service_epoch: crate::random_id()?, revision: 1 },
                records_revision: None, activity: Observation::Unavailable("Reading current activity".into()),
            }, encoded: vec![], activity_at: Instant::now(), records_at: Instant::now() }), changed: Condvar::new(), subscribers: AtomicUsize::new(0),
        });
        let sample = publisher.clone();
        std::thread::Builder::new().name("bridge-record-changes".into()).spawn(move || {
            loop {
                let started = Instant::now();
                let revision = records();
                sample.replace(Some(if started.elapsed() < wire::STALE_AFTER { revision } else { None }), None);
                std::thread::sleep(RECORD_INTERVAL);
            }
        })?;
        let sample = publisher.clone();
        std::thread::Builder::new().name("bridge-live-activity".into()).spawn(move || {
            loop {
                let started = Instant::now();
                let (revision, mut observed) = activity();
                if started.elapsed() >= wire::STALE_AFTER {
                    observed = Observation::Unavailable("Live activity sampling exceeded its freshness bound".into());
                }
                sample.replace(None, Some((revision, observed)));
                std::thread::sleep(LIVE_INTERVAL);
            }
        })?;
        Ok(publisher)
    }
    fn replace(&self, records_revision: Option<Option<u64>>, activity: Option<(Option<u64>, Observation<Activity>)>) {
        let mut held = self.held.lock().unwrap_or_else(|p| p.into_inner());
        if records_revision.is_some() { held.records_at = Instant::now(); }
        let records_revision = records_revision.unwrap_or(held.state.records_revision);
        if activity.is_some() { held.activity_at = Instant::now(); }
        let activity = match activity {
            Some((revision, activity)) if revision == records_revision => activity,
            Some(_) => Observation::Unavailable("Activity changed while it was read".into()),
            None if records_revision != held.state.records_revision =>
                Observation::Unavailable("Reading activity for the changed records".into()),
            None => held.state.activity.clone(),
        };
        let mut encoded = serde_json::to_vec(&activity).unwrap_or_default();
        // Do not truncate a census into a falsely complete list.
        let activity = if encoded.len() + 1024 > wire::FRAME_LIMIT {
            let unavailable = Observation::Unavailable("Activity exceeds its observation bound".into());
            encoded = serde_json::to_vec(&unavailable).unwrap_or_default();
            unavailable
        } else { activity };
        if held.state.records_revision == records_revision && held.encoded == encoded {
            // Freshness may have recovered without a semantic change. Wake
            // explicit readers without publishing a new cursor or UI frame.
            self.changed.notify_all();
            return;
        }
        held.state.records_revision = records_revision;
        held.state.activity = activity;
        held.state.cursor.revision += 1;
        held.encoded = encoded;
        self.changed.notify_all();
    }
    /// Caller has already authenticated uid and the selected frontend. Recheck
    /// selection before each state/heartbeat, including after package changes.
    pub fn attach(self: &Arc<Self>, mut peer: UnixStream,
        selected: impl Fn(&UnixStream) -> crate::Result<()> + Send + 'static) -> io::Result<()> {
        peer.set_write_timeout(Some(wire::HEARTBEAT))?;
        if self.subscribers.fetch_update(Ordering::AcqRel, Ordering::Acquire,
            |count| (count < SUBSCRIBERS).then_some(count + 1)).is_err() {
            wire::write(&mut peer, &wire::Event::Unavailable { reason: "Too many observation windows are open".into() })?;
            return Ok(());
        }
        let publisher = self.clone();
        let result = std::thread::Builder::new().name("bridge-subscriber".into()).spawn(move || {
            struct Slot(Arc<Publisher>);
            impl Drop for Slot { fn drop(&mut self) { self.0.subscribers.fetch_sub(1, Ordering::AcqRel); } }
            let _slot = Slot(publisher.clone());
            let mut previous = 0;
            loop {
                let mut held = publisher.held.lock().unwrap_or_else(|p| p.into_inner());
                if held.state.cursor.revision == previous {
                    held = publisher.changed.wait_timeout_while(held, wire::HEARTBEAT,
                        |held| held.state.cursor.revision == previous).unwrap_or_else(|p| p.into_inner()).0;
                }
                if held.activity_at.elapsed() >= wire::STALE_AFTER || held.records_at.elapsed() >= wire::STALE_AFTER {
                    drop(held);
                    let _ = wire::write(&mut peer, &wire::Event::Unavailable { reason: "Live status sampling is delayed".into() });
                    break;
                }
                let event = if held.state.cursor.revision == previous {
                    wire::Event::Heartbeat { cursor: held.state.cursor.clone() }
                } else { wire::Event::State(held.state.clone()) };
                previous = held.state.cursor.revision;
                drop(held);
                if let Err(error) = selected(&peer) {
                    let event = if error.to_string().starts_with("operator_frontend_superseded:") {
                        wire::Event::FrontendSuperseded
                    } else { wire::Event::Unavailable { reason: "Current frontend identity could not be checked".into() } };
                    let _ = wire::write(&mut peer, &event);
                    break;
                }
                if wire::write(&mut peer, &event).is_err() { break; }
            }
        });
        if let Err(error) = result {
            self.subscribers.fetch_sub(1, Ordering::AcqRel);
            return Err(error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    fn fixture() -> Arc<Publisher> {
        Arc::new(Publisher { held: Mutex::new(Held { state: wire::State {
            cursor: wire::Cursor { service_epoch: "ab".repeat(16), revision: 1 },
            records_revision: Some(1), activity: Observation::Unavailable("fixture".into()),
        }, encoded: vec![], activity_at: Instant::now(), records_at: Instant::now() }), changed: Condvar::new(),
            subscribers: AtomicUsize::new(0) })
    }
    fn receive(peer: &mut UnixStream) -> wire::Event {
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut length = [0; 4]; peer.read_exact(&mut length).unwrap();
        let length = u32::from_le_bytes(length) as usize;
        assert!(length <= wire::FRAME_LIMIT);
        let mut bytes = vec![0; length]; peer.read_exact(&mut bytes).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
    #[test]
    fn slow_readers_get_one_complete_latest_state_and_never_a_history_queue() {
        let publisher = fixture();
        for revision in 2..1000 { publisher.replace(Some(Some(revision)), None); }
        let (mut client, server) = UnixStream::pair().unwrap();
        publisher.attach(server, |_| Ok(())).unwrap();
        let wire::Event::State(state) = receive(&mut client) else { panic!("replacement state") };
        assert_eq!(state.records_revision, Some(999));
        assert_eq!(publisher.held.lock().unwrap().state.records_revision, Some(999));
        drop(client);
    }
    #[test]
    fn oversize_activity_is_unavailable_without_truncating_a_census() {
        let publisher = fixture();
        publisher.replace(Some(Some(2)), Some((Some(2), Observation::Unavailable("x".repeat(wire::FRAME_LIMIT)))));
        let held = publisher.held.lock().unwrap();
        assert!(held.encoded.len() < 1024);
        assert!(matches!(&held.state.activity, Observation::Unavailable(reason) if reason.contains("bound")));
    }
    #[test]
    fn heartbeat_cannot_keep_a_stalled_sampler_current() {
        let publisher = fixture();
        publisher.held.lock().unwrap().activity_at = Instant::now() - wire::STALE_AFTER;
        let (mut client, server) = UnixStream::pair().unwrap();
        publisher.attach(server, |_| Ok(())).unwrap();
        assert!(matches!(receive(&mut client), wire::Event::Unavailable { .. }));
    }
    #[test]
    fn live_samples_cannot_keep_a_stalled_record_watch_current() {
        let publisher = fixture();
        publisher.held.lock().unwrap().records_at = Instant::now() - wire::STALE_AFTER;
        publisher.replace(None, Some((Some(1), Observation::Unavailable("fresh live sample".into()))));
        let (mut client, server) = UnixStream::pair().unwrap();
        publisher.attach(server, |_| Ok(())).unwrap();
        assert!(matches!(receive(&mut client), wire::Event::Unavailable { .. }));
    }
    #[test]
    fn explicit_reads_use_the_same_state_and_cannot_renew_or_cross_records() {
        let publisher = fixture();
        let snapshot: crate::operator_model::Snapshot = serde_json::from_str(include_str!("../../../../manager-ui/examples/library-preview.json")).unwrap();
        let activity = Activity { schema: crate::operator_model::OPERATOR_SCHEMA,
            live: crate::operator_model::LiveActivity::unavailable("fixture"), recent: Observation::Observed(vec![]),
            incidents: Observation::Observed(vec![]), system: snapshot.system, capture: serde_json::Value::Null, operation: None };
        publisher.replace(None, Some((Some(1), Observation::Observed(activity.clone()))));
        assert_eq!(serde_json::to_value(publisher.activity(Some(1)).unwrap()).unwrap(), serde_json::to_value(activity).unwrap());
        assert!(publisher.activity(None).is_err());
        assert!(publisher.activity(Some(2)).is_err());
        let before = Instant::now() - wire::STALE_AFTER;
        publisher.held.lock().unwrap().activity_at = before;
        assert!(publisher.activity(Some(1)).is_err());
        assert_eq!(publisher.held.lock().unwrap().activity_at, before);
    }
    fn sample_activity(marker: u64) -> Activity {
        let snapshot: crate::operator_model::Snapshot = serde_json::from_str(include_str!("../../../../manager-ui/examples/library-preview.json")).unwrap();
        Activity { schema: crate::operator_model::OPERATOR_SCHEMA,
            live: crate::operator_model::LiveActivity::unavailable("fixture"), recent: Observation::Observed(vec![]),
            incidents: Observation::Observed(vec![]), system: snapshot.system,
            capture: serde_json::json!(marker), operation: None }
    }
    #[test]
    fn explicit_read_waits_for_matching_sample_and_follows_changed_records() {
        use std::sync::{atomic::AtomicU64, mpsc};
        let publisher = fixture();
        let revision = AtomicU64::new(1);
        let (entered, entering) = mpsc::channel();
        let (result, received) = mpsc::channel();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                result.send(publisher.activity_within(|| {
                    let _ = entered.send(());
                    Some(revision.load(Ordering::Acquire))
                }, Duration::from_secs(2)).map(|activity| activity.capture).map_err(|error| error.to_string())).unwrap();
            });
            entering.recv_timeout(Duration::from_secs(1)).unwrap();
            revision.store(2, Ordering::Release);
            publisher.replace(Some(Some(2)), Some((Some(1), Observation::Observed(sample_activity(1)))));
            assert!(received.recv_timeout(Duration::from_millis(20)).is_err(), "an old sample cannot satisfy the read");
            publisher.replace(None, Some((Some(2), Observation::Observed(sample_activity(2)))));
            assert_eq!(received.recv_timeout(Duration::from_secs(1)).unwrap().unwrap(), serde_json::json!(2));
        });
    }
    #[test]
    fn explicit_read_timeout_preserves_reason_and_does_not_renew_freshness() {
        let publisher = fixture();
        let before = publisher.held.lock().unwrap().activity_at;
        let started = Instant::now();
        let error = publisher.activity_within(|| Some(1), Duration::from_millis(10)).unwrap_err();
        assert_eq!(error.to_string(), "fixture");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(publisher.held.lock().unwrap().activity_at, before);
        publisher.replace(None, Some((Some(1), Observation::Observed(sample_activity(1)))));
        let stale = Instant::now() - wire::STALE_AFTER;
        publisher.held.lock().unwrap().activity_at = stale;
        assert!(publisher.activity_within(|| Some(1), Duration::from_millis(10)).is_err());
        assert_eq!(publisher.held.lock().unwrap().activity_at, stale);
    }
    #[test]
    fn identical_sample_restores_freshness_without_changing_cursor() {
        let publisher = fixture();
        publisher.replace(None, Some((Some(1), Observation::Observed(sample_activity(1)))));
        let cursor = publisher.held.lock().unwrap().state.cursor.clone();
        publisher.held.lock().unwrap().activity_at = Instant::now() - wire::STALE_AFTER;
        std::thread::scope(|scope| {
            let reader = scope.spawn(|| publisher.activity_within(|| Some(1), Duration::from_secs(1)).unwrap());
            publisher.replace(None, Some((Some(1), Observation::Observed(sample_activity(1)))));
            assert_eq!(reader.join().unwrap().capture, serde_json::json!(1));
        });
        assert_eq!(publisher.held.lock().unwrap().state.cursor, cursor);
    }
    #[test]
    fn record_changes_retire_activity_until_a_matching_sample_arrives() {
        let publisher = fixture();
        publisher.replace(None, Some((Some(1), Observation::Unavailable("sample one".into()))));
        publisher.replace(Some(Some(2)), None);
        {
            let held = publisher.held.lock().unwrap();
            assert_eq!(held.state.records_revision, Some(2));
            assert!(matches!(&held.state.activity, Observation::Unavailable(reason)
                if reason == "Reading activity for the changed records"));
        }
        publisher.replace(None, Some((Some(1), Observation::Unavailable("late sample one".into()))));
        {
            let held = publisher.held.lock().unwrap();
            assert_eq!(held.state.records_revision, Some(2));
            assert!(matches!(&held.state.activity, Observation::Unavailable(reason)
                if reason == "Activity changed while it was read"));
        }
        publisher.replace(None, Some((Some(2), Observation::Unavailable("sample two".into()))));
        let held = publisher.held.lock().unwrap();
        assert!(matches!(&held.state.activity, Observation::Unavailable(reason) if reason == "sample two"));
    }
    #[test]
    fn subscriptions_have_a_separate_ceiling_and_recheck_frontend_selection() {
        let publisher = fixture();
        let mut clients = Vec::new();
        for _ in 0..SUBSCRIBERS {
            let (mut client, server) = UnixStream::pair().unwrap();
            publisher.attach(server, |_| Ok(())).unwrap();
            assert!(matches!(receive(&mut client), wire::Event::State(_)));
            clients.push(client);
        }
        let (mut extra, server) = UnixStream::pair().unwrap();
        publisher.attach(server, |_| Ok(())).unwrap();
        assert!(matches!(receive(&mut extra), wire::Event::Unavailable { .. }));
        assert_eq!(publisher.subscribers.load(Ordering::Acquire), SUBSCRIBERS);
        drop(clients);
        let independent = fixture();
        let (mut old, server) = UnixStream::pair().unwrap();
        independent.attach(server, |_| Err(crate::service::contract::FRONTEND_SUPERSEDED.into())).unwrap();
        assert!(matches!(receive(&mut old), wire::Event::FrontendSuperseded));
    }
}
