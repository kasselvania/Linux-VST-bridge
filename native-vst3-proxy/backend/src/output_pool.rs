//! Extra output planes belong to the worker until completion publication, then
//! to the callback until its bounded copy into frame history (or discard).
//! The callback releases that slot immediately; delayed samples have a separate
//! prepared owner. The realtime side never allocates or frees storage.
use crate::queue::preallocated;
use ap1_native_client::BLOCK_CAP as CAP;
use std::{cell::UnsafeCell, sync::atomic::{AtomicBool, Ordering}};
pub(crate) const NONE: usize = usize::MAX;
pub(crate) struct Pool {
    pub channels: usize,
    samples: Box<[UnsafeCell<f32>]>,
    occupied: Box<[AtomicBool]>,
}
unsafe impl Send for Pool {}
unsafe impl Sync for Pool {}
impl Pool {
    pub fn new(channels: usize, slots: usize) -> Self {
        // The immutable transport slot extent remains CAP across inactive M
        // growth/shrink. At 62 channels and 513 slots: 130,277,376 sample bytes.
        assert!(channels > 0 && channels <= 62 && slots > 0);
        let mut samples = preallocated(channels * slots * CAP);
        samples.resize_with(channels * slots * CAP, || UnsafeCell::new(0.));
        Self { channels, samples: samples.into_boxed_slice(),
            occupied: (0..slots).map(|_| AtomicBool::new(false)).collect() }
    }
    // Only the single worker calls publish. Queue publication then hands this
    // slot to the callback. A failed publication is explicitly released.
    pub fn publish(&self, slot: usize, planes: &[[f32; CAP]], n: usize) -> bool {
        if slot >= self.occupied.len() || n > CAP || planes.len() < self.channels
            || self.occupied[slot].swap(true, Ordering::Acquire) { return false; }
        for (ch, plane) in planes.iter().take(self.channels).enumerate() {
            unsafe { std::ptr::copy_nonoverlapping(plane.as_ptr(),
                self.samples[(slot*self.channels+ch)*CAP].get(), n); }
        }
        true
    }
    pub fn publish_available(&self, planes: &[[f32; CAP]], n: usize) -> Option<usize> {
        // One producer, a bounded prepared slot census, and no retry loop.
        (0..self.occupied.len()).find(|&slot| self.publish(slot, planes, n))
    }
    // Called only with a slot received from the completion queue. Null output
    // pointers represent explicitly inactive SDK buses and are never touched.
    pub unsafe fn copy(&self, slot: usize, offset: usize, count: usize,
                       out: &[*mut f32], destination: usize) {
        debug_assert!(slot < self.occupied.len() && offset + count <= CAP);
        debug_assert_eq!(out.len(), self.channels);
        for (ch, &p) in out.iter().enumerate() {
            if !p.is_null() { std::ptr::copy_nonoverlapping(
                self.samples[(slot*self.channels+ch)*CAP+offset].get(), p.add(destination), count); }
        }
    }
    pub fn release(&self, slot: usize) {
        if slot != NONE { self.occupied[slot].store(false, Ordering::Release); }
    }
}

/// Callback-owned presentation storage, measured in frames rather than calls.
/// At most D old frames and the current N<=M frames remain useful. Late audio
/// is clipped before copying; its returned events/points retain their own owner.
/// Bytes = channels*(D+M)*4 + (D+M)*8, independent of the number of N=1 calls.
pub(crate) struct Retained {
    pub channels: usize,
    pub maximum: usize,
    samples: Box<[f32]>,
    flags: Box<[u64]>,
    start: u64,
    end: u64,
}
impl Retained {
    pub fn new(channels: usize, maximum: usize, delay: usize) -> Self {
        assert!((2..=64).contains(&channels) && (1..=CAP).contains(&maximum));
        let frames = delay.checked_add(maximum).unwrap();
        let mut samples = preallocated(channels * frames); samples.resize(channels * frames, 0.);
        let mut flags = preallocated(frames); flags.resize(frames, 0);
        Self { channels, maximum, samples: samples.into_boxed_slice(),
            flags: flags.into_boxed_slice(), start: 0, end: 0 }
    }
    pub fn clear(&mut self) { self.start = 0; self.end = 0; }
    pub fn len(&self) -> usize { (self.end - self.start) as usize }
    pub fn is_empty(&self) -> bool { self.start == self.end }
    pub fn position(&self) -> u64 { self.start }
    #[cfg(test)]
    pub fn storage_bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<f32>() + self.flags.len() * std::mem::size_of::<u64>()
    }
    pub fn discard_before(&mut self, position: u64) -> u64 {
        let count = position.saturating_sub(self.start).min(self.end - self.start);
        self.start += count; count
    }
    pub fn append(&mut self, position: u64, n: usize, flags: u64,
        main: &[[f32; CAP]; 2], extra: Option<(&Pool, usize)>, floor: u64,
    ) -> Option<u64> {
        if n > self.maximum || (self.channels > 2 && extra.is_none()) { return None; }
        let skip = floor.saturating_sub(position).min(n as u64) as usize;
        let count = n - skip;
        if count == 0 { return Some(skip as u64); }
        let first = position.checked_add(skip as u64)?;
        if self.is_empty() { self.start = first; self.end = first; }
        if first != self.end || self.len() + count > self.flags.len() { return None; }
        if let Some((pool, slot)) = extra {
            if pool.channels + 2 != self.channels || slot == NONE { return None; }
        }
        let capacity = self.flags.len();
        let mut source = skip;
        while source < n {
            let destination = self.end as usize % capacity;
            let count = (n - source).min(capacity - destination);
            for (ch, plane) in main.iter().enumerate() {
                self.samples[ch * capacity + destination..ch * capacity + destination + count]
                    .copy_from_slice(&plane[source..source + count]);
            }
            if let Some((pool, slot)) = extra {
                let mut planes = [std::ptr::null_mut(); 62];
                for (ch, pointer) in planes[..pool.channels].iter_mut().enumerate() {
                    *pointer = unsafe { self.samples.as_mut_ptr().add((ch + 2) * capacity) };
                }
                unsafe { pool.copy(slot, source, count, &planes[..pool.channels], destination); }
            }
            self.flags[destination..destination + count].fill(flags);
            self.end += count as u64; source += count;
        }
        Some(skip as u64)
    }
    pub fn copy(&mut self, count: usize, main: &mut [[f32; CAP]; 2],
        extra: &[*mut f32], destination: usize, extra_destination: usize,
    ) -> (usize, u64) {
        let capacity = self.flags.len();
        let source = self.start as usize % capacity;
        let count = count.min(self.len()).min(capacity - source);
        let flags = self.flags[source..source + count].iter().fold(u64::MAX, |mask, &f| mask & f);
        for (ch, plane) in main.iter_mut().enumerate() {
            plane[destination..destination + count]
                .copy_from_slice(&self.samples[ch * capacity + source..ch * capacity + source + count]);
        }
        debug_assert_eq!(extra.len() + 2, self.channels);
        for (ch, &pointer) in extra.iter().enumerate() {
            if !pointer.is_null() { unsafe {
                std::ptr::copy_nonoverlapping(self.samples.as_ptr().add((ch + 2) * capacity + source),
                    pointer.add(extra_destination), count);
            } }
        }
        self.start += count as u64;
        (count, flags)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinct_planes_and_no_reuse_before_consumer_release() {
        let pool=Pool::new(62,2);
        let planes: Vec<_>=(0..62).map(|ch| [ch as f32 + 0.25; CAP]).collect();
        assert!(pool.publish(0,&planes,256));
        assert!(!pool.publish(0,&planes,256));
        let mut out=vec![[0.;CAP];62];
        let pointers:Vec<_>=out.iter_mut().map(|p|p.as_mut_ptr()).collect();
        unsafe {pool.copy(0,17,100,&pointers,3);}
        for ch in 0..62 { assert_eq!(&out[ch][3..103],&planes[ch][17..117]); assert_eq!(out[ch][103],0.); }
        pool.release(0);
        assert!(pool.publish(0,&planes,128));
    }
    #[test]
    fn retained_storage_is_frames_plus_one_current_block_and_exact_silence_masks() {
        let mut retained = Retained::new(2, 1024, 1024);
        assert_eq!(retained.storage_bytes(), 2048 * (2 * 4 + 8));
        let main = [[0.25; CAP], [0.5; CAP]];
        for position in 0..1025 {
            assert_eq!(retained.append(position, 1, if position % 2 == 0 { 1 } else { 2 },
                &main, None, 0), Some(0));
        }
        let mut out = [[0.; CAP]; 2];
        for position in 0..1025 {
            let (count, flags) = retained.copy(1, &mut out, &[], 0, 0);
            assert_eq!(count, 1);
            assert_eq!(flags, if position % 2 == 0 { 1 } else { 2 });
            assert_eq!((out[0][0], out[1][0]), (0.25, 0.5));
        }
        assert!(retained.is_empty());
        let largest = Retained::new(64, 1024, 1024);
        assert_eq!(largest.storage_bytes(), 540672);
    }
}
