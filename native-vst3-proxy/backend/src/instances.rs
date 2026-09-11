//! Fixed-capacity, generation-checked instance ownership. No global callback lock.
use std::{
    ops::Deref,
    ptr,
    sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

pub const CAPACITY: usize = 4;
/// Non-RT insertion outcomes. A held owner cannot establish permanent fullness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertError {
    Full,
    Busy,
    GenerationExhausted,
}
impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Full => ap1_native_client::admission::Refusal::NativeImageCapacity.code(),
            Self::Busy => "admission_native_image_busy",
            Self::GenerationExhausted => "admission_native_generation_exhausted",
        })
    }
}
impl std::error::Error for InsertError {}

const CLOSED: u64 = 1 << 63;
struct Entry<T> {
    id: u64,
    value: T,
}
struct Slot<T> {
    entry: AtomicPtr<Entry<T>>,
    users: AtomicU64,
    owner: AtomicBool,
}
impl<T> Slot<T> {
    const fn new() -> Self {
        Self {
            entry: AtomicPtr::new(ptr::null_mut()),
            users: AtomicU64::new(CLOSED),
            owner: AtomicBool::new(false),
        }
    }
}
struct Owner<'a>(&'a AtomicBool);
impl<'a> Owner<'a> {
    fn acquire(flag: &'a AtomicBool) -> Option<Self> {
        flag.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Self(flag))
    }
}
impl Drop for Owner<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
pub struct Registry<T> {
    slots: [Slot<T>; CAPACITY],
    next: AtomicU64,
}
// Entries are published immutable; exclusive removal waits for all leases.
// Any interior mutation in T must supply its own synchronization.
unsafe impl<T: Send + Sync> Sync for Registry<T> {}
impl<T> Registry<T> {
    pub const fn new() -> Self {
        Self {
            slots: [const { Slot::new() }; CAPACITY],
            next: AtomicU64::new(0),
        }
    }
    /// Startup can block only this reserved slot, never a sibling callback.
    pub fn insert<E>(
        &self,
        create: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<u64, E>, InsertError> {
        let mut contended = false;
        for (index, slot) in self.slots.iter().enumerate() {
            let Some(_owner) = Owner::acquire(&slot.owner) else {
                contended = true;
                continue;
            };
            if !slot.entry.load(Ordering::Acquire).is_null() {
                continue;
            }
            let serial = self
                .next
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                    (n < u64::MAX / CAPACITY as u64 - 1).then(|| n + 1)
                })
                .map_err(|_| InsertError::GenerationExhausted)?;
            let id = serial * CAPACITY as u64 + index as u64 + 1;
            return Ok(create().map(|value| {
                slot.entry.store(
                    Box::into_raw(Box::new(Entry { id, value })),
                    Ordering::Release,
                );
                // Preserve temporary counts from refused readers of a closed slot.
                slot.users.fetch_and(!CLOSED, Ordering::Release);
                id
            }));
        }
        Err(if contended {
            InsertError::Busy
        } else {
            InsertError::Full
        })
    }
    /// Exactly one increment/decrement: no allocation, waiting or CAS retry loop.
    pub fn lease(&self, id: u64) -> Option<Lease<'_, T>> {
        let slot = self.slots.get(id.checked_sub(1)? as usize % CAPACITY)?;
        let previous = slot.users.fetch_add(1, Ordering::Acquire);
        let mut lease = Lease {
            slot,
            entry: ptr::null(),
        };
        if previous & CLOSED != 0 {
            return None;
        }
        let entry = slot.entry.load(Ordering::Acquire);
        if entry.is_null() || unsafe { (*entry).id } != id {
            return None;
        }
        lease.entry = entry;
        Some(lease)
    }
    /// Only the owner thread removes. Stop admission first; outstanding leases
    /// protect memory until they return. A stuck caller retains the slot on timeout.
    pub fn remove<R>(&self, id: u64, finish: impl FnOnce(&mut T) -> R) -> Result<R, u32> {
        self.exclusive(id, true, finish)
    }
    /// Non-RT replacement of one transport. Its logical instance and saved state
    /// survive; callbacks cannot lease the old transport during replacement.
    pub fn update<R>(&self, id: u64, replace: impl FnOnce(&mut T) -> R) -> Result<R, u32> {
        self.exclusive(id, false, replace)
    }
    fn exclusive<R>(
        &self,
        id: u64,
        remove: bool,
        finish: impl FnOnce(&mut T) -> R,
    ) -> Result<R, u32> {
        let index = id.checked_sub(1).ok_or(1u32)? as usize % CAPACITY;
        let slot = &self.slots[index];
        let _owner = Owner::acquire(&slot.owner).ok_or(3u32)?;
        let p = slot.entry.load(Ordering::Acquire);
        if p.is_null() || unsafe { (*p).id } != id {
            return Err(1);
        }
        slot.users.fetch_or(CLOSED, Ordering::AcqRel);
        let until = Instant::now() + Duration::from_secs(20);
        while slot.users.load(Ordering::Acquire) & !CLOSED != 0 {
            if Instant::now() >= until {
                return Err(2);
            }
            thread::sleep(Duration::from_micros(50));
        }
        let result = finish(unsafe { &mut (*p).value });
        if remove {
            slot.entry.store(ptr::null_mut(), Ordering::Release);
            unsafe {
                drop(Box::from_raw(p));
            }
        } else {
            slot.users.fetch_and(!CLOSED, Ordering::Release);
        }
        Ok(result)
    }
}
pub struct Lease<'a, T> {
    slot: &'a Slot<T>,
    entry: *const Entry<T>,
}
impl<T> Deref for Lease<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &(*self.entry).value }
    }
}
impl<T> Drop for Lease<'_, T> {
    fn drop(&mut self) {
        self.slot.users.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reusable_slot_owner_contention_is_temporary_and_preserves_stale_handles() {
        let r = Registry::new();
        let ids: Vec<_> = (0..CAPACITY)
            .map(|i| r.insert(|| Ok::<_, ()>(i)).unwrap().unwrap())
            .collect();
        let retired = ids[CAPACITY - 1];
        r.remove(retired, |_| ()).unwrap();
        let barrier = std::sync::Barrier::new(2);
        thread::scope(|scope| {
            let t = scope.spawn(|| {
                let _owner = Owner::acquire(&r.slots[CAPACITY - 1].owner).unwrap();
                barrier.wait();
                barrier.wait();
            });
            barrier.wait();
            let result = r.insert(|| -> Result<usize, ()> {
                panic!("contended insertion must not start or acquire external ownership")
            });
            // Release before assertions so a failing regression cannot strand the thread.
            barrier.wait();
            t.join().unwrap();
            assert_eq!(
                result.unwrap_err().to_string(),
                "admission_native_image_busy"
            );
        });
        let fresh = r.insert(|| Ok::<_, ()>(99)).unwrap().unwrap();
        assert_ne!(fresh, retired);
        assert!(r.lease(retired).is_none());
        assert!(r.lease(0).is_none());
        assert!(r.insert(|| Ok::<_, ()>(100)).is_err());
        for (i, id) in ids.into_iter().enumerate().take(CAPACITY - 1) {
            assert_eq!(*r.lease(id).unwrap(), i);
            r.remove(id, |_| ()).unwrap();
            assert!(r.lease(id).is_none());
        }
        assert_eq!(r.remove(fresh, |v| *v), Ok(99));
        assert!(r.lease(fresh).is_none());
    }

    #[test]
    fn generation_exhaustion_is_terminal_without_creating_or_reusing_handles() {
        let r = Registry::new();
        let old = r.insert(|| Ok::<_, ()>(1)).unwrap().unwrap();
        r.remove(old, |_| ()).unwrap();
        r.next
            .store(u64::MAX / CAPACITY as u64 - 1, Ordering::Relaxed);
        for _ in 0..2 {
            let e = r.insert(|| -> Result<usize, ()> { panic!("no exhausted startup") });
            assert_eq!(e, Err(InsertError::GenerationExhausted));
            assert_eq!(
                e.unwrap_err().to_string(),
                "admission_native_generation_exhausted"
            );
            assert!(r.lease(old).is_none());
        }
    }

    #[test]
    fn transport_replacement_excludes_old_readers_and_keeps_sibling_live() {
        let r = Registry::new();
        let a = r.insert(|| Ok::<_, ()>(vec![1])).unwrap().unwrap();
        let b = r.insert(|| Ok::<_, ()>(vec![2])).unwrap().unwrap();
        r.update(a, |transport| {
            assert!(r.lease(a).is_none());
            assert_eq!(*r.lease(b).unwrap(), vec![2]);
            *transport = vec![3];
        })
        .unwrap();
        assert_eq!(*r.lease(a).unwrap(), vec![3]);
        assert_eq!(*r.lease(b).unwrap(), vec![2]);
        r.remove(a, |_| ()).unwrap();
        r.remove(b, |_| ()).unwrap();
    }
    #[test]
    fn capacity_stale_handles_and_sibling_removal() {
        let r = Registry::new();
        let ids: Vec<_> = (0..CAPACITY)
            .map(|i| r.insert(|| Ok::<_, ()>(i)).unwrap().unwrap())
            .collect();
        assert_eq!(r.insert(|| Ok::<_, ()>(99)), Err(InsertError::Full));
        assert_eq!(
            InsertError::Full.to_string(),
            "admission_native_image_capacity"
        );
        assert!(r.lease(0).is_none());
        assert!(r.lease(ids[0] + CAPACITY as u64).is_none());
        assert_eq!(r.remove(ids[0], |v| *v), Ok(0));
        let replacement = r.insert(|| Ok::<_, ()>(99)).unwrap().unwrap();
        assert_ne!(replacement, ids[0]);
        assert!(r.lease(ids[0]).is_none());
        for (i, id) in ids.into_iter().enumerate().skip(1) {
            assert_eq!(*r.lease(id).unwrap(), i);
            r.remove(id, |_| ()).unwrap();
        }
        r.remove(replacement, |_| ()).unwrap();
    }
    #[test]
    fn blocked_startup_and_teardown_do_not_block_sibling_leases() {
        let r = Registry::new();
        let a = r.insert(|| Ok::<_, ()>(1)).unwrap().unwrap();
        let barrier = std::sync::Barrier::new(2);
        thread::scope(|scope| {
            let t = scope.spawn(|| {
                r.insert(|| {
                    barrier.wait();
                    barrier.wait();
                    Ok::<_, ()>(2)
                })
                .unwrap()
                .unwrap()
            });
            barrier.wait();
            for _ in 0..10000 {
                assert_eq!(*r.lease(a).unwrap(), 1);
            }
            barrier.wait();
            let b = t.join().unwrap();
            let registry = &r;
            let sync = &barrier;
            let t = scope.spawn(move || {
                registry
                    .remove(b, |_| {
                        sync.wait();
                        sync.wait();
                    })
                    .unwrap()
            });
            barrier.wait();
            assert!(r.lease(b).is_none());
            for _ in 0..10000 {
                assert_eq!(*r.lease(a).unwrap(), 1);
            }
            barrier.wait();
            t.join().unwrap();
        });
        r.remove(a, |_| ()).unwrap();
    }
    #[test]
    fn close_waits_for_its_readers_and_failed_startup_releases_capacity() {
        let r = Registry::new();
        assert_eq!(r.insert(|| Err::<usize, _>("startup")), Ok(Err("startup")));
        let id = r.insert(|| Ok::<_, ()>(7)).unwrap().unwrap();
        let lease = r.lease(id).unwrap();
        thread::scope(|scope| {
            let t = scope.spawn(|| r.remove(id, |v| *v).unwrap());
            while r.slots[(id - 1) as usize % CAPACITY]
                .users
                .load(Ordering::Acquire)
                & CLOSED
                == 0
            {
                thread::yield_now();
            }
            assert_eq!(*lease, 7);
            assert!(r.lease(id).is_none());
            drop(lease);
            assert_eq!(t.join().unwrap(), 7);
        });
    }
}
