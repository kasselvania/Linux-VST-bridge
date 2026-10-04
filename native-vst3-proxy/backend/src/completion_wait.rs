//! Native completion notification; no transport, allocation or mutex ownership.
//! The queue remains authoritative. Snapshot before inspecting it, then compare
//! that snapshot in the wait, so publication between inspection and sleep cannot
//! lose a wake. The fixed queue cannot cycle this counter 2^32 times in one wait.
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use std::io;
#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

pub(crate) struct Signal(AtomicU32);
impl Signal {
    pub(crate) fn new() -> Self {
        let signal = Self(AtomicU32::new(0));
        #[cfg(target_os = "linux")]
        {
            // Resolve both imported libc functions during inactive allocation,
            // even when the DAW loads the proxy with lazy symbol binding.
            signal.notify();
            unsafe { libc::__errno_location(); }
        }
        signal
    }
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

// A separate local worker wake can be multiplexed with its Session sockets.
// Callback publication writes once to this prepared nonblocking kernel object;
// neither a socket nor a transport operation is touched by the callback.
pub(crate) struct WorkSignal {
    sequence: AtomicU32,
    #[cfg(target_os = "linux")]
    descriptor: OwnedFd,
    #[cfg(target_os = "linux")]
    failed: std::sync::atomic::AtomicBool,
    #[cfg(not(target_os = "linux"))]
    fallback: Signal,
}
impl WorkSignal {
    pub(crate) fn new() -> io::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            let descriptor = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
            if descriptor < 0 { return Err(io::Error::last_os_error()); }
            let signal = Self { sequence: AtomicU32::new(0),
                descriptor: unsafe { OwnedFd::from_raw_fd(descriptor) },
                failed: std::sync::atomic::AtomicBool::new(false) };
            // Resolve the callback's imports while inactive. The count is a
            // hint; one drain consumes all coalesced writes without a loop.
            signal.notify(); signal.drain()?;
            let mut poll = libc::pollfd { fd: signal.descriptor.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            if unsafe { libc::poll(&mut poll, 1, 0) } < 0 { return Err(io::Error::last_os_error()); }
            Ok(signal)
        }
        #[cfg(not(target_os = "linux"))]
        { Ok(Self { sequence: AtomicU32::new(0), fallback: Signal::new() }) }
    }
    pub(crate) fn snapshot(&self) -> u32 { self.sequence.load(Ordering::Acquire) }
    pub(crate) fn notify(&self) {
        self.sequence.fetch_add(1, Ordering::Release);
        #[cfg(target_os = "linux")]
        unsafe {
            let count = 1u64;
            let result = libc::write(self.descriptor.as_raw_fd(), std::ptr::addr_of!(count).cast(), 8);
            // Saturation leaves a readable pending wake. No retry occurs on
            // the callback; any other failed write is inspected by its worker.
            if result != 8 && !(result < 0 && *libc::__errno_location() == libc::EAGAIN) {
                self.failed.store(true, Ordering::Release);
            }
        }
        #[cfg(not(target_os = "linux"))]
        self.fallback.notify();
    }
    pub(crate) fn check(&self) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        if self.failed.load(Ordering::Acquire) { return Err(io::Error::other("local worker wake failed")); }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    fn drain(&self) -> io::Result<()> {
        self.check()?;
        let mut count = 0u64;
        let result = unsafe { libc::read(self.descriptor.as_raw_fd(), std::ptr::addr_of_mut!(count).cast(), 8) };
        if result == 8 { return Ok(()); }
        if result < 0 {
            let error = io::Error::last_os_error();
            if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) { return Ok(()); }
            return Err(error);
        }
        Err(io::Error::other("local worker wake extent"))
    }
    pub(crate) fn wait(&self, observed: u32, deadline: Instant,
        sockets: [(i32, bool); 2]) -> io::Result<()> {
        self.check()?;
        #[cfg(target_os = "linux")]
        {
            self.drain()?;
            if self.snapshot() != observed { return Ok(()); }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else { return Ok(()); };
            let mut descriptors = [
                libc::pollfd { fd: self.descriptor.as_raw_fd(), events: libc::POLLIN, revents: 0 },
                libc::pollfd { fd: sockets[0].0, events: libc::POLLIN | if sockets[0].1 { libc::POLLOUT } else { 0 }, revents: 0 },
                libc::pollfd { fd: sockets[1].0, events: libc::POLLIN | if sockets[1].1 { libc::POLLOUT } else { 0 }, revents: 0 },
            ];
            // This worker services health at <=4 ms; round the remaining wait
            // upward only within the caller's finite millisecond cadence.
            let milliseconds = remaining.as_millis().max(1).min(i32::MAX as u128) as i32;
            let result = unsafe { libc::poll(descriptors.as_mut_ptr(), 3, milliseconds) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted { return Err(error); }
            }
            if descriptors[0].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                return Err(io::Error::other("local worker wake disconnected"));
            }
            self.check()
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Portable source tests use the existing bounded signal fallback.
            // Production multiplexing is qualified on Linux.
            let _ = sockets;
            let ready = self.fallback.snapshot();
            if self.snapshot() == observed { self.fallback.wait(ready, deadline); }
            Ok(())
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
    #[test]
    fn coalesced_worker_publication_before_poll_has_no_callback_allocation_or_lost_wake() {
        let signal = WorkSignal::new().unwrap();
        let before = signal.snapshot();
        let (_, allocations) = crate::allocation_test::measure(|| {
            for _ in 0..1024 { signal.notify(); }
        });
        assert_eq!(allocations, [0; 3]);
        let start = Instant::now();
        signal.wait(before, start + Duration::from_secs(1), [(-1, false); 2]).unwrap();
        assert!(start.elapsed() < Duration::from_millis(50));
        assert_ne!(signal.snapshot(), before);
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn local_and_capture_readiness_share_one_worker_poll_without_losing_a_racing_publication() {
        use std::{io::Write, os::unix::net::UnixStream};
        let signal = WorkSignal::new().unwrap();
        let (socket, mut peer) = UnixStream::pair().unwrap();
        std::thread::scope(|scope| {
            let writer = scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(5)); peer.write_all(&[1]).unwrap();
            });
            let started = Instant::now();
            signal.wait(signal.snapshot(), started + Duration::from_secs(1),
                [(socket.as_raw_fd(), false), (-1, false)]).unwrap();
            assert!(started.elapsed() < Duration::from_millis(100)); writer.join().unwrap();
        });
        std::thread::scope(|scope| {
            let publisher = scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(5)); signal.notify();
            });
            let started = Instant::now();
            signal.wait(signal.snapshot(), started + Duration::from_secs(1), [(-1, false); 2]).unwrap();
            assert!(started.elapsed() < Duration::from_millis(100)); publisher.join().unwrap();
        });
    }
}
