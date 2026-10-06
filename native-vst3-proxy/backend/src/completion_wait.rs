//! Native completion notification; no transport, allocation or mutex ownership.
//! The queue remains authoritative. Snapshot before inspecting it, then compare
//! that snapshot in the wait, so publication between inspection and sleep cannot
//! lose a wake. The fixed queue cannot cycle this counter 2^32 times in one wait.
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use std::io;
#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

#[cfg(target_os = "linux")]
fn monotonic_now() -> Option<libc::timespec> {
    let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) } != 0
        || time.tv_sec < 0 || !(0..1_000_000_000).contains(&time.tv_nsec) {
        return None;
    }
    Some(time)
}

#[cfg(target_os = "linux")]
fn absolute_target_from(deadline: Instant, monotonic_before: Option<libc::timespec>,
    instant_after: Instant) -> Option<libc::timespec> {
    let mut target = monotonic_before?;
    if target.tv_sec < 0 || !(0..1_000_000_000).contains(&target.tv_nsec) { return None; }
    let remaining = deadline.checked_duration_since(instant_after)?;
    let seconds: libc::time_t = remaining.as_secs().try_into().ok()?;
    let nanos = target.tv_nsec.checked_add(remaining.subsec_nanos() as libc::c_long)?;
    let carry = nanos / 1_000_000_000;
    target.tv_nsec = nanos % 1_000_000_000;
    target.tv_sec = target.tv_sec.checked_add(seconds)?
        .checked_add(carry as libc::time_t)?;
    Some(target)
}

#[cfg(target_os = "linux")]
fn absolute_target(deadline: Instant) -> Option<libc::timespec> {
    // Sample CLOCK_MONOTONIC first. Instant is sampled afterward, so any
    // interruption between them can only move the kernel target earlier.
    let monotonic_before = monotonic_now();
    let instant_after = Instant::now();
    absolute_target_from(deadline, monotonic_before, instant_after)
}

/// Shared-file futex boundary for delivery v4. No private-key optimization is
/// legal here: the reciprocal waiter is in Wine, with a different virtual VA.
#[cfg(target_os = "linux")]
pub(crate) fn wait_shared(word: &AtomicU32, observed: u32, deadline: Instant) -> bool {
    let Some(timeout)=absolute_target(deadline) else {return false;};
    let result=unsafe {libc::syscall(libc::SYS_futex,word.as_ptr(),libc::FUTEX_WAIT_BITSET,
        observed,&timeout,std::ptr::null_mut::<u32>(),libc::FUTEX_BITSET_MATCH_ANY as u32)};
    let error=unsafe {*libc::__errno_location()};
    wait_result(result,error,deadline,Instant::now)
}

#[cfg(target_os = "linux")]
#[inline]
fn wait_result<F>(result: libc::c_long, error: libc::c_int, deadline: Instant,
    now: F) -> bool where F: FnOnce() -> Instant {
    result == 0 || matches!(error, libc::EAGAIN | libc::EINTR)
        || error == libc::ETIMEDOUT && now() < deadline
}

pub(crate) struct Signal(AtomicU32);
impl Signal {
    pub(crate) fn new() -> Self {
        let signal = Self(AtomicU32::new(0));
        #[cfg(target_os = "linux")]
        {
            // Resolve the required libc functions during inactive allocation,
            // even when the DAW loads the proxy with lazy symbol binding.
            signal.notify();
            let _ = monotonic_now();
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
        #[cfg(target_os = "linux")]
        {
            self.wait_with(observed, deadline, |address, operation, value, timeout, address2, bitset| unsafe {
                let result = libc::syscall(libc::SYS_futex,
                    address, operation, value, timeout, address2, bitset);
                (result, *libc::__errno_location())
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else { return false; };
            // Portable source-test fallback, not a non-Linux product claim.
            if self.snapshot() == observed {
                std::thread::sleep(remaining.min(std::time::Duration::from_micros(50)));
            }
            true
        }
    }
    #[cfg(target_os = "linux")]
    fn wait_with<F>(&self, observed: u32, deadline: Instant, enter: F) -> bool
    where F: FnOnce(*mut u32, libc::c_int, u32, *const libc::timespec,
        *mut u32, u32) -> (libc::c_long, libc::c_int) {
        let Some(timeout) = absolute_target(deadline) else { return false; };
        let (result, error) = enter(self.0.as_ptr(),
            libc::FUTEX_WAIT_BITSET | libc::FUTEX_PRIVATE_FLAG,
            observed, &timeout, std::ptr::null_mut(), libc::FUTEX_BITSET_MATCH_ANY as u32);
        // Signals, interruption and a publication racing with entry request a
        // fresh queue inspection. Conservative clock conversion can expire
        // early; preserve the caller's original Instant deadline in that case.
        wait_result(result, error, deadline, Instant::now)
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
    #[cfg(target_os = "linux")]
    #[test]
    fn futex_wait_never_renews_the_deadline_before_kernel_entry() {
        fn monotonic_ns() -> u128 {
            let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
            assert_eq!(unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) }, 0);
            assert!(time.tv_sec >= 0 && time.tv_nsec >= 0);
            time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128
        }
        fn timeout_ns(timeout: *const libc::timespec) -> u128 {
            assert!(!timeout.is_null());
            let timeout = unsafe { &*timeout };
            assert!(timeout.tv_sec >= 0 && timeout.tv_nsec >= 0);
            timeout.tv_sec as u128 * 1_000_000_000 + timeout.tv_nsec as u128
        }
        let signal = Signal::new();
        let observed = signal.snapshot();
        let allowance = Duration::from_secs(10);
        let instant = Instant::now();
        let deadline = instant + allowance;
        // The later monotonic sample plus the full allowance is a conservative
        // upper bracket for the original Instant deadline.
        let deadline_upper = monotonic_ns() + allowance.as_nanos();
        let fake_kernel_entry = deadline_upper + Duration::from_secs(5).as_nanos();
        let mut entered = false;
        let ready = signal.wait_with(observed, deadline,
            |_address, operation, _value, timeout, _address2, _bitset| {
                entered = true;
                assert_eq!(operation & libc::FUTEX_CLOCK_REALTIME, 0);
                let timeout = timeout_ns(timeout);
                let effective_expiry = match operation & libc::FUTEX_CMD_MASK {
                    libc::FUTEX_WAIT => fake_kernel_entry + timeout,
                    libc::FUTEX_WAIT_BITSET => timeout,
                    command => panic!("unexpected futex command {command}"),
                };
                assert!(effective_expiry <= deadline_upper,
                    "kernel expiry {effective_expiry} renewed original deadline upper {deadline_upper}");
                (0, 0)
            });
        assert!(entered);
        assert!(ready);
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn futex_wait_uses_the_full_bitset_syscall_abi() {
        let signal = Signal::new();
        let observed = signal.snapshot();
        let mut entered = false;
        let ready = signal.wait_with(observed, Instant::now() + Duration::from_secs(1),
            |address, operation, value, target, address2, bitset| {
                entered = true;
                assert_eq!(address, signal.0.as_ptr());
                assert_eq!(operation, libc::FUTEX_WAIT_BITSET | libc::FUTEX_PRIVATE_FLAG);
                assert_eq!(value, observed);
                assert!(!target.is_null());
                assert!(address2.is_null());
                assert_eq!(bitset, libc::FUTEX_BITSET_MATCH_ANY as u32);
                (0, 0)
            });
        assert!(entered);
        assert!(ready);
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn absolute_target_is_conservative_and_refuses_invalid_clock_or_overflow() {
        let instant_after = Instant::now();
        let deadline = instant_after + Duration::new(123, 456_000_000);
        let target = absolute_target_from(deadline,
            Some(libc::timespec { tv_sec: 7, tv_nsec: 900_000_000 }), instant_after).unwrap();
        assert_eq!((target.tv_sec, target.tv_nsec), (131, 356_000_000));
        assert!(absolute_target_from(deadline, None, instant_after).is_none());
        assert!(absolute_target_from(deadline,
            Some(libc::timespec { tv_sec: -1, tv_nsec: 0 }), instant_after).is_none());
        assert!(absolute_target_from(deadline,
            Some(libc::timespec { tv_sec: 0, tv_nsec: 1_000_000_000 }), instant_after).is_none());
        assert!(absolute_target_from(instant_after + Duration::from_secs(1),
            Some(libc::timespec { tv_sec: libc::time_t::MAX, tv_nsec: 0 }),
            instant_after).is_none());
        assert!(absolute_target_from(instant_after, Some(libc::timespec {
            tv_sec: 1, tv_nsec: 0 }), instant_after + Duration::from_nanos(1)).is_none());
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn expired_absolute_target_reaches_linux_and_interruption_keeps_existing_result() {
        let signal = Signal::new();
        let observed = signal.snapshot();
        let mut kernel_result = None;
        let _ = signal.wait_with(observed, Instant::now() + Duration::from_secs(60),
            |address, operation, value, _timeout, address2, bitset| unsafe {
                let mut expired = monotonic_now().unwrap();
                if expired.tv_nsec == 0 {
                    expired.tv_sec -= 1;
                    expired.tv_nsec = 999_999_999;
                } else {
                    expired.tv_nsec -= 1;
                }
                let result = libc::syscall(libc::SYS_futex,
                    address, operation, value, &expired, address2, bitset);
                let error = *libc::__errno_location();
                kernel_result = Some((result, error));
                (result, error)
            });
        assert_eq!(kernel_result, Some((-1, libc::ETIMEDOUT)));
        let interrupted = signal.wait_with(observed, Instant::now() + Duration::from_secs(1),
            |_address, _operation, _value, _timeout, _address2, _bitset| (-1, libc::EINTR));
        assert!(interrupted);
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn conservative_early_timeout_reinspects_only_until_the_original_deadline() {
        let deadline = Instant::now() + Duration::from_secs(1);
        let early = deadline.checked_sub(Duration::from_nanos(1)).unwrap();
        assert!(wait_result(-1, libc::ETIMEDOUT, deadline, || early));
        assert!(!wait_result(-1, libc::ETIMEDOUT, deadline, || deadline));
        assert!(!wait_result(-1, libc::ETIMEDOUT, deadline, ||
            deadline + Duration::from_nanos(1)));

        let signal = Signal::new();
        let observed = signal.snapshot();
        let past = Instant::now().checked_sub(Duration::from_nanos(1)).unwrap();
        let mut entered = false;
        assert!(!signal.wait_with(observed, past,
            |_address, _operation, _value, _timeout, _address2, _bitset| {
                entered = true;
                (0, 0)
            }));
        assert!(!entered);
    }
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
