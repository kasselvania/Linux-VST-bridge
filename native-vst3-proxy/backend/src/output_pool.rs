//! Extra output planes owned by the worker until publication, then by the
//! callback until its final sample is consumed or discarded. Slot reuse is
//! explicitly released; the realtime side never allocates or frees storage.
use crate::queue::preallocated;
use ap1_native_client::CAP;
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
}
