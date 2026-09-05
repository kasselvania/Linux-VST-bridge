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
}
impl Mapping {
    pub fn new(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.set_len(MAP_BYTES as u64)?;
        // MAP_SHARED=1, PROT_READ|PROT_WRITE=3 on this Linux/x86-64 target.
        let p = unsafe { mmap(std::ptr::null_mut(), MAP_BYTES, 3, 1, file.as_raw_fd(), 0) };
        need(p as isize != -1, "mapping failed")?;
        Ok(Self {
            pointer: NonNull::new(p as *mut u8).ok_or_else(|| invalid("null mapping"))?,
            _file: file,
            unmapped: false,
        })
    }
    pub fn close(mut self) -> io::Result<()> {
        let result = unsafe { munmap(self.pointer.as_ptr() as *mut c_void, MAP_BYTES) };
        need(result == 0, "native mapping unmap failed")?;
        self.unmapped = true;
        Ok(())
    }
    pub fn write(&mut self, offset: usize, b: &[u8]) -> io::Result<()> {
        need(
            offset.checked_add(b.len()).is_some_and(|n| n <= MAP_BYTES),
            "mapping write bounds",
        )?;
        // No references into the shared view escape. Called only while Linux owns it.
        unsafe {
            std::ptr::copy_nonoverlapping(b.as_ptr(), self.pointer.as_ptr().add(offset), b.len())
        };
        Ok(())
    }
    pub fn read(&self, offset: usize, n: usize) -> io::Result<Vec<u8>> {
        need(
            offset.checked_add(n).is_some_and(|end| end <= MAP_BYTES),
            "mapping read bounds",
        )?;
        let mut b = vec![0; n];
        unsafe {
            std::ptr::copy_nonoverlapping(self.pointer.as_ptr().add(offset), b.as_mut_ptr(), n)
        };
        Ok(b)
    }
    pub fn write_plane(
        &mut self,
        base: usize,
        ch: usize,
        words: &[u32; CAP + 2],
    ) -> io::Result<()> {
        let bytes: Vec<_> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        self.write(base + ch * STRIDE, &bytes)
    }
    pub fn plane(&self, base: usize, ch: usize) -> io::Result<[u32; CAP + 2]> {
        let bytes = self.read(base + ch * STRIDE, STRIDE)?;
        Ok(std::array::from_fn(|i| {
            u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap())
        }))
    }
}
impl Drop for Mapping {
    fn drop(&mut self) {
        if !self.unmapped {
            unsafe { munmap(self.pointer.as_ptr() as *mut c_void, MAP_BYTES) };
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
