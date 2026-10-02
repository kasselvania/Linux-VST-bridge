//! Native completion notification; no transport, allocation or mutex ownership.
//! The queue remains authoritative. Snapshot before inspecting it, then compare
//! that snapshot in the wait, so publication between inspection and sleep cannot
//! lose a wake. The fixed queue cannot cycle this counter 2^32 times in one wait.
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

pub(crate) struct Signal(AtomicU32);
impl Signal {
    pub(crate) fn new() -> Self { Self(AtomicU32::new(0)) }
    pub(crate) fn snapshot(&self) -> u32 { self.0.load(Ordering::Acquire) }
    pub(crate) fn notify(&self) {
        self.0.fetch_add(1, Ordering::Release);
        #[cfg(target_os = "linux")]
        unsafe {
            // One callback owns this instance. Unconditional wake avoids a
            // second, potentially racy waiter-registration protocol.
            libc::syscall(libc::SYS_futex, self.0.as_ptr(),
                libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG, 1);
        }
    }
    pub(crate) fn wait(&self, observed: u32, deadline: Instant) -> bool {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else { return false; };
        #[cfg(target_os = "linux")]
        unsafe {
            let timeout = libc::timespec {
                tv_sec: remaining.as_secs() as libc::time_t,
                tv_nsec: remaining.subsec_nanos() as libc::c_long,
            };
            let result = libc::syscall(libc::SYS_futex, self.0.as_ptr(),
                libc::FUTEX_WAIT | libc::FUTEX_PRIVATE_FLAG, observed,
                &timeout as *const libc::timespec);
            // Signals and a publication racing with entry merely request a
            // fresh queue inspection. The caller retains the original deadline.
            result == 0 || matches!(*libc::__errno_location(), libc::EAGAIN | libc::EINTR)
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Portable source-test fallback, not a non-Linux product claim.
            if self.snapshot() == observed {
                std::thread::sleep(remaining.min(std::time::Duration::from_micros(50)));
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn publication_between_inspection_and_sleep_cannot_lose_wake() {
        let signal = Signal::new();
        let before = signal.snapshot();
        signal.notify();
        let start = Instant::now();
        let (ready, allocations) = crate::allocation_test::measure(||
            signal.wait(before, start + Duration::from_millis(100)));
        assert!(ready);
        assert_eq!(allocations, [0; 3]);
        assert!(start.elapsed() < Duration::from_millis(50));
        assert!(!signal.wait(signal.snapshot(), start));
    }
}
