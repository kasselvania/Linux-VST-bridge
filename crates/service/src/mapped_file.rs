//! Kernel mapping coordinates, derived from an open file rather than stat's
//! device number. Used only during admission, readiness and scheduling setup.
use crate::{require, Result};
use std::fs::File;
#[cfg(target_os = "linux")]
use std::{io::Read, os::{fd::AsRawFd, unix::fs::MetadataExt}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity { pub device: u64, pub inode: u64 }

fn mapping(line: &str) -> Result<(usize, usize, &str, Identity)> {
    let fields: Vec<_> = line.split_whitespace().take(5).collect();
    require(fields.len() == 5 && fields[1].len() == 4, "file_mapping_format")?;
    let (begin, end) = fields[0].split_once('-').ok_or("file_mapping_range")?;
    let (major, minor) = fields[3].split_once(':').ok_or("file_mapping_device")?;
    Ok((usize::from_str_radix(begin, 16)?, usize::from_str_radix(end, 16)?, fields[1],
        Identity { device: libc::makedev(u32::from_str_radix(major, 16)? as _,
            u32::from_str_radix(minor, 16)? as _) as u64, inode: fields[4].parse()? }))
}

/// Presence alone is not loaded-code provenance. Native admission also verifies
/// the factory's cached engine/descriptor pair after exact loader/FD binding.
pub fn present(bytes: &[u8], wanted: Identity, executable: bool) -> Result<bool> {
    require(wanted.inode != 0, "file_mapping_anonymous")?;
    for line in std::str::from_utf8(bytes)?.lines() {
        let (_, _, permissions, identity) = mapping(line)?;
        if identity == wanted && (!executable || (permissions.as_bytes()[0] == b'r' && permissions.as_bytes()[2] == b'x')) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(target_os = "linux")]
pub fn identity(file: &File) -> Result<Identity> {
    let metadata = file.metadata()?;
    require(metadata.is_file() && metadata.len() != 0 && metadata.ino() != 0,
        "file_mapping_artifact")?;
    let address = unsafe { libc::mmap(std::ptr::null_mut(), 1, libc::PROT_READ,
        libc::MAP_PRIVATE, file.as_raw_fd(), 0) };
    if address == libc::MAP_FAILED { return Err(std::io::Error::last_os_error().into()); }
    struct Mapping(*mut libc::c_void);
    impl Drop for Mapping { fn drop(&mut self) { unsafe { libc::munmap(self.0, 1); } } }
    let _mapping = Mapping(address);
    let mut bytes = Vec::new();
    File::open("/proc/self/maps")?.take(8 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    require(bytes.len() <= 8 * 1024 * 1024, "file_mapping_extent")?;
    for line in std::str::from_utf8(&bytes)?.lines() {
        let (begin, end, _, identity) = mapping(line)?;
        if (begin..end).contains(&(address as usize)) {
            require(identity.inode == metadata.ino(), "file_mapping_identity")?;
            return Ok(identity);
        }
    }
    Err("file_mapping_absent".into())
}
#[cfg(not(target_os = "linux"))]
pub fn identity(_file: &File) -> Result<Identity> { Err("file_mapping_requires_linux".into()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinates_require_the_same_device_inode_and_optional_execution() {
        let wanted = Identity {device:libc::makedev(0, 34) as u64,inode:42};
        let bytes = b"1000-2000 r--p 0 00:22 42 /arbitrary/name\n";
        assert!(present(bytes,wanted,false).unwrap());
        assert!(!present(bytes,wanted,true).unwrap());
        assert!(!present(bytes,Identity{device:wanted.device,inode:43},false).unwrap());
        assert!(!present(bytes,Identity{device:libc::makedev(0, 35) as u64,..wanted},false).unwrap());
        assert!(present(b"malformed\n",wanted,false).is_err());
        assert!(present(bytes,Identity{inode:0,..wanted},false).is_err());
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn coordinates_come_from_the_live_pinned_file_mapping() {
        let path = std::env::temp_dir().join(format!("mapped-file-{}",crate::random_id().unwrap()));
        std::fs::write(&path,b"owned mapping").unwrap();
        let file = File::open(&path).unwrap();
        let original = identity(&file).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path,b"replacement").unwrap();
        assert_eq!(identity(&file).unwrap(),original);
        assert_ne!(identity(&File::open(&path).unwrap()).unwrap(),original);
        std::fs::remove_file(path).unwrap();
    }
}
