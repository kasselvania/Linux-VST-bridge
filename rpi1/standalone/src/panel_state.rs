//! Private, identity-scoped state slot. Immutable payloads, atomic current pointer.
use sha2::{Digest, Sha256};
use std::{
    fs::{self, DirBuilder, OpenOptions},
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::Path,
};
fn regular(path: &Path, limit: u64) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() > limit {
        return Err(io::Error::other("invalid panel state file"));
    }
    Ok(())
}
pub fn load(directory: &Path, capacity: usize) -> io::Result<Option<Vec<u8>>> {
    let pointer = directory.join("current");
    match fs::symlink_metadata(&pointer) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    regular(&pointer, 64)?;
    let hash = fs::read_to_string(pointer)?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(io::Error::other("invalid panel state pointer"));
    }
    let payload = directory.join(format!("{hash}.state"));
    regular(&payload, capacity as u64)?;
    let bytes = fs::read(payload)?;
    if format!("{:x}", Sha256::digest(&bytes)) != hash {
        return Err(io::Error::other("panel state digest differs"));
    }
    Ok(Some(bytes))
}
pub fn save(directory: &Path, bytes: &[u8]) -> io::Result<()> {
    if !directory.exists() {
        DirBuilder::new().mode(0o700).create(directory)?;
    }
    let metadata = fs::symlink_metadata(directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::other("panel state directory differs"));
    }
    let hash = format!("{:x}", Sha256::digest(bytes));
    let payload = directory.join(format!("{hash}.state"));
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&payload)
    {
        Ok(mut file) => {
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            regular(&payload, bytes.len() as u64)?;
            if fs::read(&payload)? != bytes {
                return Err(io::Error::other("existing panel payload differs"));
            }
        }
        Err(e) => return Err(e),
    }
    let pointer = directory.join("current");
    if fs::symlink_metadata(&pointer).is_ok() {
        regular(&pointer, 64)?;
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let temporary = directory.join(format!(".current-{}-{nonce}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(hash.as_bytes())?;
    file.sync_all()?;
    fs::rename(&temporary, &pointer)?;
    fs::File::open(directory)?.sync_all()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_pointer_survives_reopen_and_corrupt_payload_is_refused() {
        let dir = std::env::temp_dir().join(format!(
            "panel-slot-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(load(&dir, 100).unwrap().is_none());
        save(&dir, b"first").unwrap();
        save(&dir, b"second").unwrap();
        assert_eq!(load(&dir, 100).unwrap().unwrap(), b"second");
        let hash = fs::read_to_string(dir.join("current")).unwrap();
        fs::write(dir.join(format!("{hash}.state")), b"broken").unwrap();
        assert!(load(&dir, 100).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
