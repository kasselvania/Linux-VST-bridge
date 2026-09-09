//! Native-only SPSC storage. Exactly one producer and one consumer per queue.
//! Slots are published/released with Release and observed with Acquire. Neither
//! endpoint touches a slot while the other owns it. T is Copy: no callback Drop.
use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicU64, Ordering},
};
/// Reserve and touch callback storage while inactive. Allocation alone leaves
/// anonymous pages to fault on their first callback write. Touching bytes in
/// spare capacity never constructs a T or changes a queue's logical contents.
pub(crate) fn preallocated<T>(capacity: usize) -> Vec<T> {
    let mut values = Vec::<T>::with_capacity(capacity);
    let bytes = values.capacity().checked_mul(std::mem::size_of::<T>()).unwrap();
    if bytes != 0 {
        let pointer = values.as_mut_ptr().cast::<u8>();
        for offset in (0..bytes).step_by(4096).chain(std::iter::once(bytes - 1)) {
            unsafe { std::ptr::write_volatile(pointer.add(offset), 0); }
        }
    }
    values
}
pub struct Queue<T: Copy> {
    slots: Box<[UnsafeCell<MaybeUninit<T>>]>,
    read: AtomicU64,
    write: AtomicU64,
    high: AtomicU64,
}
unsafe impl<T: Copy + Send> Send for Queue<T> {}
unsafe impl<T: Copy + Send> Sync for Queue<T> {}
impl<T: Copy> Queue<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        let mut slots = preallocated(capacity);
        slots.resize_with(capacity, || UnsafeCell::new(MaybeUninit::uninit()));
        Self {
            slots: slots.into_boxed_slice(),
            read: AtomicU64::new(0),
            write: AtomicU64::new(0),
            high: AtomicU64::new(0),
        }
    }
    pub fn push(&self, value: T) -> bool {
        let w = self.write.load(Ordering::Relaxed);
        let r = self.read.load(Ordering::Acquire);
        if w == u64::MAX || w - r == self.slots.len() as u64 {
            return false;
        }
        unsafe {
            (*self.slots[w as usize % self.slots.len()].get()).write(value);
        }
        // Only this producer writes high; no retrying atomic RMW is needed.
        self.high.store(
            self.high.load(Ordering::Relaxed).max(w - r + 1),
            Ordering::Relaxed,
        );
        self.write.store(w + 1, Ordering::Release);
        true
    }
    pub fn pop(&self) -> Option<T> {
        let r = self.read.load(Ordering::Relaxed);
        if r == self.write.load(Ordering::Acquire) {
            return None;
        }
        let value =
            unsafe { (*self.slots[r as usize % self.slots.len()].get()).assume_init_read() };
        self.read.store(r + 1, Ordering::Release);
        Some(value)
    }
    // Consumer only. Copy slots have no destructor. An unpublished producer slot
    // stays owned by that producer and is subsequently checked by epoch.
    pub fn discard_published(&self) {
        self.read
            .store(self.write.load(Ordering::Acquire), Ordering::Release);
    }
    pub fn published(&self) -> u64 {
        self.write.load(Ordering::Acquire)
    }
    pub fn consumed(&self) -> u64 {
        self.read.load(Ordering::Acquire)
    }
    pub fn high_water(&self) -> u64 {
        self.high.load(Ordering::Relaxed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capacity_wrap_and_discard() {
        let q = Queue::new(3);
        for base in 0..100 {
            for i in 0..3 {
                assert!(q.push(base * 3 + i));
            }
            assert!(!q.push(999));
            for i in 0..3 {
                assert_eq!(q.pop(), Some(base * 3 + i));
            }
            assert_eq!(q.pop(), None);
        }
        assert_eq!(q.high_water(), 3);
        assert!(q.push(1));
        q.discard_published();
        assert_eq!(q.pop(), None);
    }
    #[test]
    fn two_threads_preserve_values() {
        let q = std::sync::Arc::new(Queue::new(31));
        let peer = q.clone();
        let t = std::thread::spawn(move || {
            for n in 0..100_000 {
                while !peer.push(n) {
                    std::thread::yield_now();
                }
            }
        });
        for n in 0..100_000 {
            loop {
                if let Some(v) = q.pop() {
                    assert_eq!(v, n);
                    break;
                }
                std::thread::yield_now();
            }
        }
        t.join().unwrap();
    }
}
