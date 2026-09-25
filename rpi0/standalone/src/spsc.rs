use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Fixed single-producer/single-consumer queue. Producer and consumer never
/// share a slot concurrently; publication and reclamation are release/acquire.
pub struct Queue<T: Copy, const N: usize> {
    values: UnsafeCell<[MaybeUninit<T>; N]>,
    published: AtomicUsize,
    consumed: AtomicUsize,
}

unsafe impl<T: Copy + Send, const N: usize> Sync for Queue<T, N> {}

impl<T: Copy, const N: usize> Queue<T, N> {
    pub const fn new() -> Self {
        assert!(N > 0);
        Self {
            values: UnsafeCell::new([const { MaybeUninit::uninit() }; N]),
            published: AtomicUsize::new(0),
            consumed: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, value: T) -> Result<(), T> {
        let p = self.published.load(Ordering::Relaxed);
        let c = self.consumed.load(Ordering::Acquire);
        if p.wrapping_sub(c) >= N || p == usize::MAX {
            return Err(value);
        }
        unsafe {
            (*self.values.get())[p % N].write(value);
        }
        self.published.store(p + 1, Ordering::Release);
        Ok(())
    }

    pub fn pop(&self) -> Option<T> {
        let c = self.consumed.load(Ordering::Relaxed);
        let p = self.published.load(Ordering::Acquire);
        if c == p || c == usize::MAX {
            return None;
        }
        let value = unsafe { (*self.values.get())[c % N].assume_init_read() };
        self.consumed.store(c + 1, Ordering::Release);
        Some(value)
    }
}

impl<T: Copy, const N: usize> Default for Queue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_is_bounded_and_ordered() {
        let q = Queue::<u32, 2>::new();
        assert_eq!(q.pop(), None);
        assert_eq!(q.push(3), Ok(()));
        assert_eq!(q.push(4), Ok(()));
        assert_eq!(q.push(5), Err(5));
        assert_eq!(q.pop(), Some(3));
        assert_eq!(q.pop(), Some(4));
        assert_eq!(q.pop(), None);
    }
}
