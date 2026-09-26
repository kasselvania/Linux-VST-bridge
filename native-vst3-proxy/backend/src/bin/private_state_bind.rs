//! Offline, opt-in fixture binding. Never invoked by the product runtime.
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

fn refused(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, reason)
}

// Both inputs remain open and are read only after their private identity and
// extent have been checked. A changing file fails rather than growing a Vec.
fn read_private_file(path: &Path, limit: usize) -> io::Result<(Vec<u8>, (u64, u64))> {
    if !path.is_absolute() {
        return Err(refused("input path must be absolute"));
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let before = file.metadata()?;
    if !before.is_file() {
        return Err(refused("input must be a regular file"));
    }
    if before.uid() != unsafe { libc::geteuid() } || before.mode() & 0o077 != 0 {
        return Err(refused(
            "input must be owned by the effective user and private",
        ));
    }
    let size = usize::try_from(before.len()).map_err(|_| refused("input exceeds limit"))?;
    if size > limit {
        return Err(refused("input exceeds limit"));
    }
    let mut bytes = vec![0; size];
    file.read_exact(&mut bytes)?;
    let mut extra = [0];
    if file.read(&mut extra)? != 0 {
        return Err(refused("input changed while reading"));
    }
    let after = file.metadata()?;
    if after.len() != before.len()
        || after.dev() != before.dev()
        || after.ino() != before.ino()
        || after.mtime() != before.mtime()
        || after.mtime_nsec() != before.mtime_nsec()
        || after.ctime() != before.ctime()
        || after.ctime_nsec() != before.ctime_nsec()
    {
        return Err(refused("input changed while reading"));
    }
    Ok((bytes, (before.dev(), before.ino())))
}

fn digest(value: &str) -> io::Result<[u8; 32]> {
    if value.len() != 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "module digest extent",
        ));
    }
    let mut out = [0; 32];
    for (i, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let pair = std::str::from_utf8(pair)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "module digest encoding"))?;
        out[i] = u8::from_str_radix(pair, 16)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "module digest syntax"))?;
    }
    Ok(out)
}

fn bind_files(
    reference_path: &Path,
    module_hex: &str,
    payload_path: &Path,
    output_path: &Path,
) -> io::Result<(usize, sha2::digest::Output<Sha256>)> {
    if !output_path.is_absolute() {
        return Err(refused("output path must be absolute"));
    }
    if output_path == reference_path || output_path == payload_path {
        return Err(refused("input/output alias"));
    }
    let (reference, reference_id) =
        read_private_file(reference_path, ap2_backend::PRIVATE_STATE_CAPACITY)?;
    let module = digest(module_hex)?;
    let (payload, payload_id) =
        read_private_file(payload_path, ap2_backend::PRIVATE_STATE_PAYLOAD_LIMIT)?;
    if let Ok(existing) = fs::metadata(output_path) {
        let output_id = (existing.dev(), existing.ino());
        if output_id == reference_id || output_id == payload_id {
            return Err(refused("input/output alias"));
        }
    }
    let result = ap2_backend::bind_private_state(&reference, module, &payload)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(output_path)?;
    if let Err(e) = output.write_all(&result).and_then(|_| output.sync_all()) {
        drop(output);
        let _ = fs::remove_file(output_path);
        return Err(e);
    }
    Ok((result.len(), Sha256::digest(&result)))
}

fn run() -> io::Result<()> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 5 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
        "usage: lvb-private-state-bind REFERENCE_STATE MODULE_SHA256 INPUT_PAYLOAD OUTPUT_STATE"));
    }
    let (bytes, sha256) = bind_files(
        Path::new(&args[1]),
        args[2]
            .to_str()
            .ok_or_else(|| refused("module digest encoding"))?,
        Path::new(&args[3]),
        Path::new(&args[4]),
    )?;
    println!("bytes={bytes} sha256={sha256:x}");
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("private state binding refused: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::fs::{symlink, PermissionsExt},
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct PrivateDir(PathBuf);
    impl PrivateDir {
        fn new() -> Self {
            let path = env::temp_dir().join(format!(
                "lvb-private-bind-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            path
        }
        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for PrivateDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    const MODULE: [u8; 32] = [9; 32];
    const MODULE_HEX: &str = "0909090909090909090909090909090909090909090909090909090909090909";
    fn payload() -> Vec<u8> {
        let mut p = vec![0; 16];
        p[12] = 2;
        p
    }
    fn reference(payload: &[u8]) -> Vec<u8> {
        let mut b = vec![0; 104];
        b[..8].copy_from_slice(b"LVBSTATE");
        b[8..12].copy_from_slice(&3u32.to_le_bytes());
        b[12..16].copy_from_slice(&104u32.to_le_bytes());
        b[16..32].copy_from_slice(&[7; 16]);
        b[32..64].copy_from_slice(&MODULE);
        b[64..68].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        b[72..104].copy_from_slice(&Sha256::digest(payload));
        b.extend_from_slice(payload);
        b
    }

    #[test]
    fn private_reader_refuses_symlink_loose_mode_and_oversize() {
        let dir = PrivateDir::new();
        let input = dir.file("input", &[1, 2, 3]);
        let link = dir.path("link");
        symlink(&input, &link).unwrap();
        assert!(read_private_file(&link, 3).is_err());
        assert!(read_private_file(Path::new("input"), 3).is_err());
        fs::set_permissions(&input, fs::Permissions::from_mode(0o640)).unwrap();
        assert!(read_private_file(&input, 3).is_err());
        fs::set_permissions(&input, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(read_private_file(&dir.0, 3).is_err());
        assert!(read_private_file(&input, 2).is_err());
        assert_eq!(read_private_file(&input, 3).unwrap().0, [1, 2, 3]);
    }

    #[test]
    fn binder_refuses_oversized_inputs_without_output() {
        let dir = PrivateDir::new();
        let p = payload();
        let reference = dir.file(
            "reference",
            &vec![0; ap2_backend::PRIVATE_STATE_CAPACITY + 1],
        );
        let input = dir.file("payload", &p);
        let output = dir.path("output");
        assert!(bind_files(&reference, MODULE_HEX, &input, &output).is_err());
        assert!(!output.exists());
        fs::write(&reference, self::reference(&p)).unwrap();
        let oversize = dir.file(
            "oversize",
            &vec![0; ap2_backend::PRIVATE_STATE_PAYLOAD_LIMIT + 1],
        );
        assert!(bind_files(&reference, MODULE_HEX, &oversize, &output).is_err());
        assert!(!output.exists());
    }

    #[test]
    fn binder_refuses_wrong_identity_corruption_and_alias_then_writes_once() {
        let dir = PrivateDir::new();
        let p = payload();
        let reference = dir.file("reference", &self::reference(&p));
        let input = dir.file("payload", &p);
        let output = dir.path("output");
        assert!(bind_files(&reference, &"08".repeat(32), &input, &output).is_err());
        assert!(!output.exists());
        let mut corrupt = self::reference(&p);
        corrupt[72] ^= 1;
        fs::write(&reference, &corrupt).unwrap();
        assert!(bind_files(&reference, MODULE_HEX, &input, &output).is_err());
        assert!(!output.exists());
        fs::write(&reference, self::reference(&p)).unwrap();
        assert!(bind_files(&reference, MODULE_HEX, &input, &reference).is_err());
        let alias = dir.path("alias");
        fs::hard_link(&input, &alias).unwrap();
        assert!(bind_files(&reference, MODULE_HEX, &input, &alias).is_err());
        let (size, sha) = bind_files(&reference, MODULE_HEX, &input, &output).unwrap();
        let result = fs::read(&output).unwrap();
        assert_eq!(size, result.len());
        assert_eq!(sha.as_slice(), Sha256::digest(&result).as_slice());
        assert_eq!(fs::metadata(&output).unwrap().mode() & 0o777, 0o600);
        assert_eq!(&result[16..32], &[7; 16]);
        assert_eq!(&result[32..64], &MODULE);
        assert_eq!(&result[104..], p.as_slice());
        assert_eq!(
            bind_files(&reference, MODULE_HEX, &input, &output)
                .unwrap_err()
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&output).unwrap(), result);
    }
}
