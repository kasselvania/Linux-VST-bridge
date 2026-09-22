//! RPI1-only, single-producer JACK callback phase recorder. The reader lives
//! outside the callback. Overflow drops observations, never audio work.
use crate::queue::Queue;
use std::sync::atomic::{AtomicU64, Ordering};

pub const CAPACITY: usize = 65_536;
pub const CALLBACK_ENTER: u32 = 1;
pub const JACK_CYCLE: u32 = 2;
pub const PAYLOAD_READY: u32 = 3;
pub const REQUEST_SLOT_INSPECT: u32 = 4;
pub const REQUEST_PUBLISHED: u32 = 5;
pub const REPLY_OBSERVED: u32 = 6;
pub const REPLY_COPY_COMPLETE: u32 = 7;
pub const OUTPUT_COPY_COMPLETE: u32 = 8;
pub const CALLBACK_EXIT: u32 = 9;
pub const GAP_COMMITTED: u32 = 10;
pub const FAULT_COMMITTED: u32 = 11;
pub const CALLBACK_PAUSED: u32 = 12;
pub const JACK_NEXT: u32 = 13;
pub const WORKER_REQUEST_OBSERVED: u32 = 14;
pub const WORKER_PROCESS_BEGIN: u32 = 15;
pub const WORKER_PREPARED: u32 = 16;
pub const WORKER_SENT: u32 = 17;
pub const WORKER_REPLIED: u32 = 18;
pub const WORKER_VALIDATED: u32 = 19;
pub const WORKER_RESULT_PUBLISHED: u32 = 20;

pub const SITE_UNKNOWN: u32 = 0;
pub const SITE_PUBLISH_REQUEST: u32 = 1;
pub const SITE_RESULT_SEQUENCE: u32 = 2;
pub const SITE_APPEND_RETURNED: u32 = 3;
pub const SITE_AUDIO_BACKLOG: u32 = 4;
pub const SITE_PRESENT_POSITION: u32 = 5;
pub const SITE_JACK_PROCESS: u32 = 6;
pub const SITE_RESULT_PUBLISH: u32 = 7;
pub const SITE_WORKER_FAILURE: u32 = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Event {
    pub monotonic_ns: u64,
    pub callback_sequence: u64,
    pub bridge_position: u64,
    pub kind: u32,
    pub detail: u32,
    pub value_1: u64,
    pub value_2: u64,
}

pub struct Ring {
    queue: Queue<Event>,
    enabled: bool,
    dropped: AtomicU64,
    producer_tid: AtomicU64,
}

impl Default for Ring {
    fn default() -> Self {
        Self::new()
    }
}
impl Ring {
    pub fn new() -> Self {
        Self {
            queue: Queue::new(CAPACITY),
            enabled: true,
            dropped: AtomicU64::new(0),
            producer_tid: AtomicU64::new(0),
        }
    }
    pub fn disabled() -> Self {
        Self {
            queue: Queue::new(1),
            enabled: false,
            dropped: AtomicU64::new(0),
            producer_tid: AtomicU64::new(0),
        }
    }
    /// Called on the producer thread before the first event. A changed TID is
    /// visible to the consumer and must not be described as one continuous run.
    pub fn bind_producer(&self, tid: u64) {
        if !self.enabled {
            return;
        }
        self.producer_tid.store(tid, Ordering::Release);
    }
    pub fn record(
        &self,
        callback_sequence: u64,
        bridge_position: u64,
        kind: u32,
        detail: u32,
        value_1: u64,
        value_2: u64,
    ) {
        if !self.enabled {
            return;
        }
        let event = Event {
            monotonic_ns: crate::observer::monotonic_ns(),
            callback_sequence,
            bridge_position,
            kind,
            detail,
            value_1,
            value_2,
        };
        if !self.queue.push(event) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub fn pop(&self) -> Option<Event> {
        self.queue.pop()
    }
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Acquire)
    }
    pub fn producer_tid(&self) -> u64 {
        self.producer_tid.load(Ordering::Acquire)
    }
}

pub fn event_name(kind: u32) -> &'static str {
    match kind {
        CALLBACK_ENTER => "callback_enter",
        JACK_CYCLE => "jack_cycle",
        PAYLOAD_READY => "request_payload_ready",
        REQUEST_SLOT_INSPECT => "request_slot_inspect",
        REQUEST_PUBLISHED => "request_published",
        REPLY_OBSERVED => "reply_observed",
        REPLY_COPY_COMPLETE => "reply_copy_complete",
        OUTPUT_COPY_COMPLETE => "output_copy_complete",
        CALLBACK_EXIT => "callback_exit",
        GAP_COMMITTED => "gap_committed",
        FAULT_COMMITTED => "fault_committed",
        CALLBACK_PAUSED => "callback_paused",
        JACK_NEXT => "jack_next",
        WORKER_REQUEST_OBSERVED => "worker_request_observed",
        WORKER_PROCESS_BEGIN => "worker_process_begin",
        WORKER_PREPARED => "worker_prepared",
        WORKER_SENT => "worker_sent",
        WORKER_REPLIED => "worker_replied",
        WORKER_VALIDATED => "worker_validated",
        WORKER_RESULT_PUBLISHED => "worker_result_published",
        _ => "unknown",
    }
}
pub fn fault_name(raw: u64, site: u32) -> &'static str {
    match (raw, site) {
        (2, SITE_PUBLISH_REQUEST) => "request_queue_full",
        (2, SITE_APPEND_RETURNED) => "returned_events_overflow",
        (2, SITE_AUDIO_BACKLOG) => "audio_result_backlog_full",
        (2, SITE_RESULT_PUBLISH) => "result_queue_full",
        (4, SITE_RESULT_SEQUENCE) => "result_sequence_mismatch",
        (4, SITE_PRESENT_POSITION) => "presentation_position_mismatch",
        _ => match raw {
            2 => "overflow",
            3 => "worker_failure",
            4 => "correlation_failure",
            5 => "returned_result_failure",
            _ => "unknown",
        },
    }
}
pub fn site_name(site: u32) -> &'static str {
    match site {
        SITE_PUBLISH_REQUEST => "publish_request",
        SITE_RESULT_SEQUENCE => "result_sequence",
        SITE_APPEND_RETURNED => "append_returned",
        SITE_AUDIO_BACKLOG => "audio_backlog",
        SITE_PRESENT_POSITION => "present_position",
        SITE_JACK_PROCESS => "jack_process",
        SITE_RESULT_PUBLISH => "publish_result",
        SITE_WORKER_FAILURE => "worker_failure",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_ring_preserves_order_and_counts_drops() {
        let ring = Ring::new();
        ring.bind_producer(7);
        for n in 0..CAPACITY as u64 + 2 {
            ring.record(n, n * 256, REQUEST_PUBLISHED, 0, n, 0);
        }
        assert_eq!(ring.producer_tid(), 7);
        assert_eq!(ring.dropped(), 2);
        for n in 0..CAPACITY as u64 {
            let event = ring.pop().unwrap();
            assert_eq!(event.callback_sequence, n);
            assert_eq!(event.value_1, n);
        }
        assert!(ring.pop().is_none());
    }
}
