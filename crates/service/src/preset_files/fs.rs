//! Descriptor-relative file custody. No followed links, recursive path reopen,
//! overwrite, or acceptance of a file that changed during a read.
use super::*;
use std::ffi::{CStr, CString};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;

pub(super) fn component(name: &str) -> Result<()> {
    require(
        !name.is_empty()
            && name.len() <= 240
            && name != "."
            && name != ".."
            && !name
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')),
        "preset_file_name_invalid",
    )
}
pub(super) fn relative(path: &str) -> Result<Vec<&str>> {
    let parts: Vec<_> = path.split('/').collect();
    require(parts.len() <= 16, "preset_path_depth")?;
    for part in &parts {
        component(part)?;
    }
    Ok(parts)
}
fn c(name: &str) -> Result<CString> {
    component(name)?;
    Ok(CString::new(name)?)
}
pub(super) fn at(dir: &File, name: &str, flags: i32, mode: u32) -> Result<File> {
    let name = c(name)?;
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            mode as libc::mode_t,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
pub(super) fn directory(path: &Path) -> Result<File> {
    require(path.is_absolute(), "preset_root_absolute")?;
    let mut dir = File::open("/")?;
    for part in path.components() {
        match part {
            std::path::Component::RootDir => (),
            std::path::Component::Normal(name) => {
                dir = at(
                    &dir,
                    name.to_str().ok_or("preset_path_encoding")?,
                    libc::O_RDONLY | libc::O_DIRECTORY,
                    0,
                )?;
            }
            _ => return Err("preset_path_invalid".into()),
        }
    }
    Ok(dir)
}
pub(super) fn identity(file: &File) -> Result<(u64, u64)> {
    let meta = file.metadata()?;
    require(
        meta.is_dir() && meta.uid() == unsafe { libc::geteuid() },
        "preset_directory_owner",
    )?;
    Ok((meta.dev(), meta.ino()))
}
pub(super) fn names(dir: &File) -> Result<Vec<String>> {
    // A separate open file description avoids sharing directory offsets.
    let dot = CString::new(".")?;
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            dot.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        unsafe { libc::close(fd) };
        return Err("preset_directory_read".into());
    }
    struct Stream(*mut libc::DIR);
    impl Drop for Stream {
        fn drop(&mut self) {
            unsafe { libc::closedir(self.0) };
        }
    }
    let stream = Stream(stream);
    let mut names = Vec::new();
    loop {
        #[cfg(target_os = "linux")]
        let errno = unsafe { libc::__errno_location() };
        #[cfg(target_os = "macos")]
        let errno = unsafe { libc::__error() };
        unsafe {
            *errno = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if unsafe { *errno } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            break;
        }
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        require(names.len() < MAX_ENTRIES, "preset_directory_entry_bound")?;
        let name = std::str::from_utf8(bytes)?.to_owned();
        component(&name)?;
        names.push(name);
    }
    names.sort();
    Ok(names)
}
pub(super) fn stamp(meta: &std::fs::Metadata) -> (u64, u64, u64, i64, i64, i64, i64, u32, u64) {
    (
        meta.dev(),
        meta.ino(),
        meta.len(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec(),
        meta.mode(),
        meta.nlink(),
    )
}
pub(super) fn bytes(file: &File) -> Result<Vec<u8>> {
    use std::os::unix::fs::FileExt;
    let before = file.metadata()?;
    require(
        unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) } & libc::O_ACCMODE == libc::O_RDONLY,
        "preset_readonly_descriptor_required",
    )?;
    require(
        before.is_file()
            && before.nlink() == 1
            && before.uid() == unsafe { libc::geteuid() }
            && before.len() <= MAX_FILE,
        "preset_file_kind_or_extent",
    )?;
    let mut data = vec![0; before.len() as usize];
    file.read_exact_at(&mut data, 0)?;
    require(
        stamp(&before) == stamp(&file.metadata()?),
        "preset_file_changed",
    )?;
    Ok(data)
}
pub(super) fn read(dir: &File, relative_path: &str) -> Result<Vec<u8>> {
    let parts = relative(relative_path)?;
    let mut parent = dir.try_clone()?;
    for part in &parts[..parts.len() - 1] {
        parent = at(&parent, part, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
    }
    bytes(&at(&parent, parts[parts.len() - 1], libc::O_RDONLY, 0)?)
}
pub(super) fn read_observed(dir: &File, path: &str) -> Result<(Vec<u8>, String)> {
    let parts = relative(path)?;
    let mut parent = dir.try_clone()?;
    for part in &parts[..parts.len() - 1] {
        parent = at(&parent, part, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
    }
    let source = at(&parent, parts[parts.len() - 1], libc::O_RDONLY, 0)?;
    let before = stamp(&source.metadata()?);
    let data = bytes(&source)?;
    require(
        before == stamp(&source.metadata()?),
        "preset_source_changed",
    )?;
    let again = at(&parent, parts[parts.len() - 1], libc::O_RDONLY, 0)?;
    require(
        before == stamp(&again.metadata()?),
        "preset_source_replaced",
    )?;
    Ok((data, hash(&serde_json::to_vec(&before)?)))
}
pub(super) fn mkdir(dir: &File, name: &str) -> Result<File> {
    let name_c = c(name)?;
    if unsafe { libc::mkdirat(dir.as_raw_fd(), name_c.as_ptr(), 0o700) } != 0
        && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let child = at(dir, name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
    identity(&child)?;
    Ok(child)
}
pub(super) fn write_new(dir: &File, path: &str, data: &[u8]) -> Result<()> {
    write_new_checked(dir, path, data, || Ok(()))
}
pub(super) fn write_new_checked<G>(
    dir: &File,
    path: &str,
    data: &[u8],
    before_publish: impl FnOnce() -> Result<G>,
) -> Result<()> {
    let parts = relative(path)?;
    let mut parent = dir.try_clone()?;
    for part in &parts[..parts.len() - 1] {
        parent = mkdir(&parent, part)?;
    }
    let temporary = format!(".bridge-preset-{}", crate::random_id()?);
    let mut out = at(
        &parent,
        &temporary,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        0o600,
    )?;
    let old = c(&temporary)?;
    let new = c(parts[parts.len() - 1])?;
    let result = (|| -> Result<()> {
        out.write_all(data)?;
        out.sync_all()?;
        #[cfg(test)]
        publication_checkpoint(false, PublicationBoundary::Prepared);
        let admission = before_publish()?;
        let mut current_parent = dir.try_clone()?;
        for part in &parts[..parts.len() - 1] {
            current_parent = at(&current_parent, part, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        }
        require(
            identity(&current_parent)? == identity(&parent)?,
            "preset_destination_parent_changed",
        )?;
        #[cfg(test)]
        publication_checkpoint(false, PublicationBoundary::Admitted);
        // Atomic missing-only publication. linkat fails if any object already
        // owns the destination; interruption cannot expose a partially written preset.
        require(
            unsafe {
                libc::linkat(
                    parent.as_raw_fd(),
                    old.as_ptr(),
                    parent.as_raw_fd(),
                    new.as_ptr(),
                    0,
                )
            } == 0,
            "preset_destination_exists_or_unavailable",
        )?;
        drop(admission);
        #[cfg(test)]
        publication_checkpoint(false, PublicationBoundary::Published);
        Ok(())
    })();
    require(
        unsafe { libc::unlinkat(parent.as_raw_fd(), old.as_ptr(), 0) } == 0,
        "preset_temporary_cleanup",
    )?;
    parent.sync_all()?;
    result
}
pub(super) fn link<G>(
    dir: &File,
    name: &str,
    target: &Path,
    admitted: impl FnOnce() -> Result<G>,
) -> Result<()> {
    let target = CString::new(target.as_os_str().as_bytes())?;
    let name = c(name)?;
    #[cfg(test)]
    publication_checkpoint(true, PublicationBoundary::Prepared);
    let admission = admitted()?;
    #[cfg(test)]
    publication_checkpoint(true, PublicationBoundary::Admitted);
    require(
        unsafe { libc::symlinkat(target.as_ptr(), dir.as_raw_fd(), name.as_ptr()) } == 0,
        "preset_link_path_already_used_or_unavailable",
    )?;
    drop(admission);
    #[cfg(test)]
    publication_checkpoint(true, PublicationBoundary::Published);
    dir.sync_all()?;
    Ok(())
}

pub(super) fn seal(dir: &File) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for name in names(dir)? {
        let child = at(dir, &name, libc::O_RDONLY, 0)?;
        if child.metadata()?.is_dir() {
            seal(&child)?;
        } else {
            child.set_permissions(std::fs::Permissions::from_mode(0o400))?;
            child.sync_all()?;
        }
    }
    dir.set_permissions(std::fs::Permissions::from_mode(0o700))?;
    dir.sync_all()?;
    Ok(())
}
