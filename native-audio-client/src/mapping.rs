//! Shared file mapping primitives; borrowed copies only, no shared references escape.
use crate::*;
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io::{self, Read},
    os::unix::{fs::OpenOptionsExt, io::AsRawFd},
    path::Path,
    ptr::NonNull,
};
extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
}
pub struct Mapping {
    pointer: NonNull<u8>,
    _file: File,
    unmapped: bool,
    pub bytes: usize,
    pub capacity: usize,
    pub stride: usize,
    pub output: usize,
    pub version: u32,
    pub output_channels: usize,
    pub extra: Vec<[f32; BLOCK_CAP]>,
}
impl Mapping {
    pub fn new(path: &Path) -> io::Result<Self> {
        Self::with_channels(path, 2)
    }
    pub fn with_channels(path: &Path, channels: usize) -> io::Result<Self> {
        Self::with_layout(path, channels, false)
    }
    pub fn with_layout(path: &Path, channels: usize, whole_block: bool) -> io::Result<Self> {
        need(matches!(channels, 2 | MULTI_CHANNELS), "mapping channel capacity")?;
        need(!whole_block || channels == MULTI_CHANNELS, "whole block mapping channel layout")?;
        let (capacity, stride, output, version) = if whole_block {
            (BLOCK_CAP, BLOCK_STRIDE, BLOCK_OUTPUT, 3)
        } else { (CAP, STRIDE, OUTPUT, if channels == 2 { 1 } else { 2 }) };
        let bytes = output + channels * stride;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.set_len(bytes as u64)?;
        // MAP_SHARED=1, PROT_READ|PROT_WRITE=3 on this Linux/x86-64 target.
        let p = unsafe { mmap(std::ptr::null_mut(), bytes, 3, 1, file.as_raw_fd(), 0) };
        need(p as isize != -1, "mapping failed")?;
        let mut mapping = Self {
            pointer: NonNull::new(p as *mut u8).ok_or_else(|| invalid("null mapping"))?,
            _file: file,
            unmapped: false, bytes, capacity, stride, output, version,
            output_channels: 2, extra: vec![[0.; BLOCK_CAP]; channels - 2],
        };
        // The mapping is still exclusively native-owned here. Establish the
        // persistent plane guards once so an older compatible peer may validate
        // a first zero-frame operation without requiring per-operation sample
        // preparation. Interiors remain zero-backed until a nonzero request.
        let guard = GUARD.to_le_bytes();
        for (base, planes) in [(INPUT, 2), (output, channels)] {
            for ch in 0..planes {
                let plane = base + ch * stride;
                mapping.write(plane, &guard)?;
                mapping.write(plane + (capacity + 1) * 4, &guard)?;
            }
        }
        barrier();
        Ok(mapping)
    }
    pub fn close(mut self) -> io::Result<()> {
        let result = unsafe { munmap(self.pointer.as_ptr() as *mut c_void, self.bytes) };
        need(result == 0, "native mapping unmap failed")?;
        self.unmapped = true;
        Ok(())
    }
    pub fn write(&mut self, offset: usize, b: &[u8]) -> io::Result<()> {
        need(
            offset.checked_add(b.len()).is_some_and(|n| n <= self.bytes),
            "mapping write bounds",
        )?;
        // No references into the shared view escape. Called only while Linux owns it.
        unsafe {
            std::ptr::copy_nonoverlapping(b.as_ptr(), self.pointer.as_ptr().add(offset), b.len())
        };
        Ok(())
    }
    pub fn read(&self, offset: usize, n: usize) -> io::Result<Vec<u8>> {
        let mut b = vec![0; n];
        self.read_into(offset, &mut b)?;
        Ok(b)
    }
    pub fn read_into(&self, offset: usize, b: &mut [u8]) -> io::Result<()> {
        need(
            offset.checked_add(b.len()).is_some_and(|end| end <= self.bytes),
            "mapping read bounds",
        )?;
        unsafe {
            std::ptr::copy_nonoverlapping(self.pointer.as_ptr().add(offset), b.as_mut_ptr(), b.len())
        };
        Ok(())
    }
    pub fn write_plane<const N: usize>(
        &mut self,
        base: usize,
        ch: usize,
        words: &[u32; N],
    ) -> io::Result<()> {
        need(N >= self.capacity + 2 && N <= BLOCK_CAP + 2, "plane storage extent")?;
        let mut bytes = [0; BLOCK_STRIDE];
        for (word, chunk) in words.iter().zip(bytes[..self.stride].chunks_exact_mut(4)) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        self.write(base + ch * self.stride, &bytes[..self.stride])
    }
    pub fn plane<const N: usize>(&self, base: usize, ch: usize) -> io::Result<[u32; N]> {
        need(N >= self.capacity + 2 && N <= BLOCK_CAP + 2, "plane storage extent")?;
        let mut bytes = [0; BLOCK_STRIDE];
        self.read_into(base + ch * self.stride, &mut bytes[..self.stride])?;
        Ok(std::array::from_fn(|i| {
            u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap())
        }))
    }
}
impl Drop for Mapping {
    fn drop(&mut self) {
        if !self.unmapped {
            unsafe { munmap(self.pointer.as_ptr() as *mut c_void, self.bytes) };
        }
    }
}
pub fn barrier() {
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    #[cfg(target_arch = "x86_64")]
    unsafe {
        std::arch::asm!("mfence", options(nostack, preserves_flags));
    }
    #[cfg(not(target_arch = "x86_64"))]
    std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}
pub fn random<const N: usize>() -> io::Result<[u8; N]> {
    let mut b = [0; N];
    File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b)
}
