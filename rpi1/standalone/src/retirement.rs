use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::Path,
    ptr::NonNull,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

pub struct RetirementStatus {
    mapping: NonNull<u8>,
    extent: usize,
}

impl RetirementStatus {
    pub fn create(directory: &Path, session: &[u8; 16]) -> io::Result<Self> {
        let path = directory.join("ap18.retirement");
        let mut bytes = [0u8; 256];
        bytes[..4].copy_from_slice(b"LVRT");
        bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&256u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&8u32.to_le_bytes());
        bytes[16..32].copy_from_slice(session);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Self::map(file, session)
    }

    fn map(file: File, session: &[u8; 16]) -> io::Result<Self> {
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() != 256 {
            return Err(invalid("retirement status extent"));
        }
        let raw = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                256,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if raw == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let mapping = NonNull::new(raw.cast::<u8>()).ok_or_else(|| invalid("retirement mmap"))?;
        let result = Self {
            mapping,
            extent: 256,
        };
        let bytes = unsafe { std::slice::from_raw_parts(result.mapping.as_ptr(), result.extent) };
        if bytes[..4] != *b"LVRT"
            || u32::from_le_bytes(bytes[4..8].try_into().unwrap()) != 1
            || u32::from_le_bytes(bytes[8..12].try_into().unwrap()) != 256
            || u32::from_le_bytes(bytes[12..16].try_into().unwrap()) != 8
            || bytes[16..32] != *session
        {
            return Err(invalid("retirement status identity/version"));
        }
        Ok(result)
    }

    fn word(&self, offset: usize) -> u64 {
        debug_assert_eq!(offset % 8, 0);
        unsafe { (&*self.mapping.as_ptr().add(offset).cast::<AtomicU64>()).load(Ordering::Acquire) }
    }

    pub fn snapshot(&self) -> io::Result<Option<RetirementSnapshot>> {
        let commit = self.word(64);
        if commit == 0 {
            return Ok(None);
        }
        if commit != 1 {
            return Err(invalid("retirement status commit"));
        }
        let snapshot = RetirementSnapshot {
            milestones: self.word(192),
            epoch: self.word(200),
            sequence: self.word(208),
            position: self.word(216),
            generation: self.word(224),
        };
        let reserved = [self.word(232), self.word(240), self.word(248)];
        if self.word(64) != commit || snapshot.milestones != 127 || reserved != [0; 3] {
            return Err(invalid("retirement status incomplete"));
        }
        Ok(Some(snapshot))
    }

    pub fn await_ready(&self, timeout: Duration) -> io::Result<RetirementSnapshot> {
        let end = Instant::now() + timeout;
        loop {
            if let Some(snapshot) = self.snapshot()? {
                return Ok(snapshot);
            }
            if Instant::now() >= end {
                return Err(invalid("Pigments process-scoped retirement status absent"));
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for RetirementStatus {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mapping.as_ptr().cast(), self.extent);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetirementSnapshot {
    pub milestones: u64,
    pub epoch: u64,
    pub sequence: u64,
    pub position: u64,
    pub generation: u64,
}

unsafe impl Send for RetirementStatus {}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_retirement_never_authorizes_cleanup() {
        let root = std::env::temp_dir().join(format!("lvb-rpi1-lvrt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let session = [0x5au8; 16];
        let status = RetirementStatus::create(&root, &session).unwrap();
        assert_eq!(status.snapshot().unwrap(), None);
        unsafe {
            (&*status.mapping.as_ptr().add(192).cast::<AtomicU64>()).store(126, Ordering::Release);
            (&*status.mapping.as_ptr().add(64).cast::<AtomicU64>()).store(1, Ordering::Release);
        }
        assert!(status.snapshot().is_err());
        drop(status);
        std::fs::remove_file(root.join("ap18.retirement")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
