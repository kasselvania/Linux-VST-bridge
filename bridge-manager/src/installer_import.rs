//! Fixed binary ingress. stdin is an inherited read-only regular file, never a path.
use super::*;
use std::io::{Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
pub const MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Installer {
    pub schema: u32,
    pub id: String,
    pub artifact: Artifact,
    pub byte_size: u64,
    pub format: String,
    pub name_hint: String,
    pub source_class: String,
    pub import_result: String,
    pub created_at: u64,
    pub vendor: Option<String>,
    pub product: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Presentation {
    pub schema: u32,
    pub installer_sha256: String,
    pub display_label: String,
    pub label_source: String,
    pub updated_at: u64,
}
pub type ImportResult = linux_vst_bridge::operator_model::InstallerImportResult;
fn unsafe_label_char(c: char) -> bool {
    c.is_control() || matches!(c, '/' | '\\' | ':')
        || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}
pub fn valid_label(label: &str) -> bool {
    !label.trim().is_empty()
        && label.trim() == label
        && label.len() <= 96
        && !matches!(label, "." | "..")
        && !label.chars().any(unsafe_label_char)
}
pub fn suggested_label(basename: &str, sha: &str) -> (String, &'static str) {
    let leaf = basename.rsplit(['/', '\\']).next().unwrap_or("");
    let stem = leaf.strip_suffix(".exe").or_else(|| leaf.strip_suffix(".EXE"))
        .or_else(|| leaf.strip_suffix(".msi")).or_else(|| leaf.strip_suffix(".MSI"))
        .unwrap_or(leaf);
    let cleaned: String = stem.chars().filter(|c| !unsafe_label_char(*c))
        .take(96).collect();
    let cleaned = cleaned.trim();
    if valid_label(cleaned) && !matches!(cleaned.to_ascii_lowercase().as_str(),
        "setup" | "install" | "installer" | "windows installer") {
        return (cleaned.into(), "selected_basename");
    }
    (format!("Unknown Windows installer · ID {}…{}", &sha[..8], &sha[60..]), "generated")
}
fn presentation_path(m: &Manager, sha: &str) -> PathBuf {
    m.root.join("installers/presentation").join(format!("{sha}.json"))
}
pub fn presentation(m: &Manager, installer: &Installer) -> Result<Presentation> {
    let path = presentation_path(m, &installer.id);
    if !path.try_exists()? {
        let (display_label, label_source) = suggested_label("", &installer.id);
        return Ok(Presentation { schema: 1, installer_sha256: installer.id.clone(),
            display_label, label_source: label_source.into(), updated_at: installer.created_at });
    }
    let value: Presentation = read_json(&path)?;
    require(value.schema == 1 && value.installer_sha256 == installer.id
        && valid_label(&value.display_label)
        && matches!(value.label_source.as_str(), "selected_basename" | "generated" | "operator_named"),
        "installer_presentation_binding")?;
    Ok(value)
}
pub fn rename(m: &Manager, sha: &str, label: &str) -> Result<Presentation> {
    require(valid_hex(sha, 64) && valid_label(label), "installer_presentation_label")?;
    let _guard = m.lock("installer-import.lock")?;
    let installer = load_record(m, sha)?;
    let dir = m.root.join("installers/presentation");
    private_dir(&dir)?;
    let value = Presentation { schema: 1, installer_sha256: installer.id,
        display_label: label.into(), label_source: "operator_named".into(),
        updated_at: observation::now()? };
    atomic_json(&presentation_path(m, sha), &value)?;
    Ok(value)
}
fn stamp(m: &fs::Metadata) -> (u64, u64, u64, i64, i64, i64, i64, u32, u32) {
    (
        m.dev(),
        m.ino(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
        m.uid(),
        m.mode(),
    )
}
fn kind(f: &mut fs::File, size: u64) -> Result<&'static str> {
    let mut h = [0; 512];
    f.seek(SeekFrom::Start(0))?;
    f.read_exact(&mut h)?;
    if h[..8] == [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1] {
        let major = u16::from_le_bytes([h[26], h[27]]);
        let shift = u16::from_le_bytes([h[30], h[31]]);
        require(
            h[28..30] == [0xfe, 0xff]
                && matches!((major, shift), (3, 9) | (4, 12))
                && h[32..34] == [6, 0]
                && size >= (1u64 << shift) * 2
                && size.is_multiple_of(1u64 << shift),
            "installer_compound_header",
        )?;
        return Ok("msi_compound");
    }
    require(&h[..2] == b"MZ", "installer_unsupported_bytes")?;
    let at = u32::from_le_bytes(h[60..64].try_into()?) as u64;
    require(
        at >= 64 && at.checked_add(26).is_some_and(|n| n <= size),
        "installer_pe_extent",
    )?;
    f.seek(SeekFrom::Start(at))?;
    let mut pe = [0; 26];
    f.read_exact(&mut pe)?;
    let machine = u16::from_le_bytes(pe[4..6].try_into()?);
    let flags = u16::from_le_bytes(pe[22..24].try_into()?);
    let optional = u16::from_le_bytes(pe[20..22].try_into()?) as u64;
    require(
        &pe[..4] == b"PE\0\0"
            && matches!(machine, 0x14c | 0x8664)
            && flags & 2 != 0
            && flags & 0x2000 == 0
            && optional >= 2
            && at + 24 + optional <= size
            && matches!(u16::from_le_bytes(pe[24..26].try_into()?), 0x10b | 0x20b),
        "installer_not_supported_pe_executable",
    )?;
    Ok("pe_executable")
}
fn available_space(dir: &Path) -> Result<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(dir.as_os_str().as_bytes())?;
    let mut s = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    require(
        unsafe { libc::statvfs(c.as_ptr(), s.as_mut_ptr()) } == 0,
        "installer_free_space_unavailable",
    )?;
    let s = unsafe { s.assume_init() };
    Ok((u128::from(s.f_bavail) * u128::from(s.f_frsize)).min(u128::from(u64::MAX)) as u64)
}
pub fn import(m: &Manager, source: fs::File) -> Result<Installer> {
    let dir = m.root.join("installers");
    private_dir(&dir)?;
    Ok(import_with(m, source, available_space(&dir)?, MAX_BYTES, |_| Ok(()))?.0)
}
pub fn import_named(m: &Manager, source: fs::File, basename: &str) -> Result<ImportResult> {
    require(basename.len() <= 512 && !basename.contains('\0'), "installer_basename_bound")?;
    let dir = m.root.join("installers");
    private_dir(&dir)?;
    let (installer, newly_imported) = import_with(m, source, available_space(&dir)?, MAX_BYTES, |_| Ok(()))?;
    let _guard = m.lock("installer-import.lock")?;
    let path = presentation_path(m, &installer.id);
    if !path.try_exists()? {
        let (display_label, label_source) = suggested_label(basename, &installer.id);
        let value = Presentation { schema: 1, installer_sha256: installer.id.clone(),
            display_label, label_source: label_source.into(), updated_at: observation::now()? };
        private_dir(path.parent().ok_or("installer_presentation_directory")?)?;
        atomic_json(&path, &value)?;
    }
    let presentation = presentation(m, &installer)?;
    Ok(ImportResult { schema: 2, installer_sha256: installer.id,
        byte_size: installer.byte_size, format: installer.format, newly_imported,
        display_label: presentation.display_label, created_at: installer.created_at })
}
fn import_with(
    m: &Manager,
    mut source: fs::File,
    free: u64,
    bound: u64,
    mut after_chunk: impl FnMut(u64) -> Result<()>,
) -> Result<(Installer, bool)> {
    let before = source.metadata()?;
    let flags = unsafe { libc::fcntl(source.as_raw_fd(), libc::F_GETFL) };
    require(
        before.is_file()
            && installer_source::accepted_owner(before.uid(), before.mode(), unsafe { libc::getuid() })
            && flags >= 0
            && flags & libc::O_ACCMODE == libc::O_RDONLY,
        "installer_requires_operator_or_protected_system_readonly_regular_descriptor",
    )?;
    require(
        (512..=bound).contains(&before.len()),
        "installer_size_bound",
    )?;
    require(
        free >= before.len().saturating_add(64 * 1024 * 1024),
        "installer_insufficient_space",
    )?;
    let format = kind(&mut source, before.len())?.to_owned();
    source.seek(SeekFrom::Start(0))?;
    let _lock = m.lock("installer-import.lock")?;
    let dir = m.root.join("installers");
    private_dir(&dir)?;
    let temp = dir.join(format!(".import-{}", random_id()?));
    let outcome = (|| {
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        let mut hash = sha2::Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            let n = source.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            size = size
                .checked_add(n as u64)
                .ok_or("installer_size_overflow")?;
            require(
                size <= before.len() && size <= bound,
                "installer_source_changed",
            )?;
            out.write_all(&buffer[..n])?;
            hash.update(&buffer[..n]);
            after_chunk(size)?;
        }
        require(
            size == before.len() && stamp(&before) == stamp(&source.metadata()?),
            "installer_source_changed",
        )?;
        out.sync_all()?;
        drop(out);
        let sha = hex(&hash.finalize());
        let path = dir.join(format!(
            "{sha}.{}",
            if format == "msi_compound" {
                "msi"
            } else {
                "exe"
            }
        ));
        let record = dir.join(format!("{sha}.json"));
        if record.exists() {
            let prior = load(m, &sha)?;
            fs::remove_file(&temp)?;
            return Ok((prior, false));
        }
        if path.exists() {
            require(digest(&path)? == sha, "installer_existing_artifact_changed")?;
            fs::remove_file(&temp)?;
        } else {
            fs::set_permissions(&temp, fs::Permissions::from_mode(0o400))?;
            fs::rename(&temp, &path)?;
        }
        let value = Installer {
            schema: 1,
            id: sha.clone(),
            artifact: Artifact { path, sha256: sha },
            byte_size: size,
            format,
            name_hint: "Windows installer".into(),
            source_class: "operator_selected_file".into(),
            import_result: "imported".into(),
            created_at: observation::now()?,
            vendor: None,
            product: None,
        };
        atomic_json(&record, &value)?;
        fs::File::open(&dir)?.sync_all()?;
        Ok((value, true))
    })();
    if temp.exists() {
        let _ = fs::remove_file(&temp);
    }
    outcome
}
#[cfg(test)]
thread_local! {
    pub(super) static VERIFY_BARRIER: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = std::cell::RefCell::new(None);
}
pub fn load(m: &Manager, id: &str) -> Result<Installer> {
    let r = load_record(m, id)?;
    #[cfg(test)]
    VERIFY_BARRIER.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook();
        }
    });
    r.artifact.verify()?;
    Ok(r)
}
/// Small immutable-record validation; this grants no digest verification.
pub fn load_record(m: &Manager, id: &str) -> Result<Installer> {
    require(valid_hex(id, 64), "installer_identity")?;
    let d = m.root.join("installers");
    let r: Installer = read_json(&d.join(format!("{id}.json")))?;
    let ext = match r.format.as_str() {
        "pe_executable" => "exe",
        "msi_compound" => "msi",
        _ => return Err("installer_format".into()),
    };
    require(
        r.schema == 1
            && r.id == id
            && r.artifact.sha256 == id
            && r.artifact.path == d.join(format!("{id}.{ext}"))
            && r.vendor.is_none()
            && r.product.is_none()
            && r.source_class == "operator_selected_file"
            && r.import_result == "imported",
        "installer_record_binding",
    )?;
    let meta = file(&r.artifact.path)?.metadata()?;
    require(
        meta.len() == r.byte_size && meta.mode() & 0o222 == 0,
        "installer_size_or_permissions_changed",
    )?;
    Ok(r)
}
pub fn list(m: &Manager) -> Result<Vec<Installer>> {
    list_with(m, load)
}
/// Current Setup presentation checks immutable record binding and cheap file
/// posture. The owning action still calls `load` to hash installer bytes.
pub fn list_records(m: &Manager) -> Result<Vec<Installer>> {
    list_with(m, load_record)
}
fn list_with(m: &Manager, read: fn(&Manager, &str) -> Result<Installer>) -> Result<Vec<Installer>> {
    let dir = m.root.join("installers");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    for (n, e) in fs::read_dir(dir)?.enumerate() {
        require(n < 512, "installer_inventory_bound")?;
        let p = e?.path();
        if p.extension().is_some_and(|x| x == "json") {
            out.push(read(
                m,
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("installer_id")?,
            )?);
        }
    }
    out.sort_by_key(|x| x.created_at);
    Ok(out)
}
pub fn presentation_token(m: &Manager) -> Result<Vec<(String, Option<Presentation>)>> {
    let dir = m.root.join("installers");
    if !dir.exists() { return Ok(Vec::new()); }
    let mut rows = Vec::new();
    for entry in fs::read_dir(&dir)?.take(513) {
        require(rows.len() < 512, "installer_inventory_bound")?;
        let path = entry?.path();
        let Some(sha) = path.file_stem().and_then(|v| v.to_str()) else { continue };
        if path.extension().is_some_and(|v| v == "json") && valid_hex(sha, 64) {
            let value = if presentation_path(m, sha).try_exists()? {
                let presentation: Presentation = read_json(&presentation_path(m, sha))?;
                require(presentation.schema == 1 && presentation.installer_sha256 == sha
                    && valid_label(&presentation.display_label)
                    && matches!(presentation.label_source.as_str(),
                        "selected_basename" | "generated" | "operator_named"),
                    "installer_presentation_binding")?;
                Some(presentation)
            } else { None };
            rows.push((sha.to_owned(), value));
        }
    }
    rows.sort_by(|a,b| a.0.cmp(&b.0));
    Ok(rows)
}
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn pe() -> Vec<u8> {
        let mut b = vec![0; 1024];
        b[..2].copy_from_slice(b"MZ");
        b[60] = 128;
        b[128..132].copy_from_slice(b"PE\0\0");
        b[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        b[148] = 2;
        b[150] = 2;
        b[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        b
    }
    #[test]
    fn ingress_exact_dedup_privacy_and_formats() {
        let f = test_fixture::Fixture::new();
        let p = f.outer.join("not-a-vendor.txt");
        fs::write(&p, pe()).unwrap();
        let a = import(&f.m, file(&p).unwrap()).unwrap();
        let b = import(&f.m, file(&p).unwrap()).unwrap();
        assert_eq!(a, b);
        assert!(a.vendor.is_none());
        assert!(!serde_json::to_string(&a).unwrap().contains("not-a-vendor"));
        let mut c = vec![0; 1024];
        c[..8].copy_from_slice(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]);
        c[26] = 3;
        c[28] = 0xfe;
        c[29] = 0xff;
        c[30] = 9;
        c[32] = 6;
        fs::write(&p, c).unwrap();
        assert_eq!(
            import(&f.m, file(&p).unwrap()).unwrap().format,
            "msi_compound"
        );
        let link = f.outer.join("link.exe");
        std::os::unix::fs::symlink(&p, &link).unwrap();
        assert!(file(&link).is_err());
        assert!(import(&f.m, fs::File::open(&f.outer).unwrap()).is_err());
    }
    #[test]
    fn typed_import_presentation_is_exact_deduplicated_and_legacy_rename_is_sidecar_only() {
        let f = test_fixture::Fixture::new();
        let source = f.outer.join("Lunacy Audio Installer.exe");
        fs::write(&source, pe()).unwrap();
        let first = import_named(&f.m, file(&source).unwrap(),
            source.file_name().unwrap().to_str().unwrap()).unwrap();
        assert!(first.newly_imported);
        assert_eq!(first.display_label, "Lunacy Audio Installer");
        let sha = first.installer_sha256.clone();
        let artifact = load(&f.m, &sha).unwrap();
        let record = fs::read(f.m.root.join("installers").join(format!("{sha}.json"))).unwrap();
        let before_token = presentation_token(&f.m).unwrap();
        let renamed = rename(&f.m, &sha, "Lunacy Audio").unwrap();
        assert_eq!(renamed.display_label, "Lunacy Audio");
        assert_ne!(presentation_token(&f.m).unwrap(), before_token);
        let second = import_named(&f.m, file(&source).unwrap(), "other.exe").unwrap();
        assert!(!second.newly_imported);
        assert_eq!(second.display_label, "Lunacy Audio");
        assert_eq!(list(&f.m).unwrap(), vec![artifact]);
        assert_eq!(fs::read(f.m.root.join("installers").join(format!("{sha}.json"))).unwrap(), record);
        assert!(!serde_json::to_string(&second).unwrap().contains(f.outer.to_str().unwrap()));
    }
    #[test]
    fn basename_sanitization_and_generic_ids_are_bounded() {
        let a = "aa".repeat(32);
        let b = "bb".repeat(32);
        assert_eq!(suggested_label("C:\\Downloads\\Lunacy Audio Installer.exe", &a).0,
            "Lunacy Audio Installer");
        assert_eq!(suggested_label("/tmp/hello\nworld.msi", &a).0, "helloworld");
        assert_eq!(suggested_label("C:\\Downloads\\C: Serum\u{202e} 2.exe", &a).0,
            "C Serum 2");
        assert_ne!(suggested_label("setup.exe", &a).0, suggested_label("setup.exe", &b).0);
        assert!(suggested_label(&format!("{}.exe", "a".repeat(200)), &a).0.len() <= 96);
        for bad in ["", " hello", "hello/command", "hello\\command", "hello\nworld",
            "C:program", ".", "..", "safe\u{202e}evil"] {
            assert!(!valid_label(bad));
        }
    }
    #[test]
    fn ingress_bounds_mutation_and_partial_cleanup() {
        let f = test_fixture::Fixture::new();
        let p = f.outer.join("source");
        fs::write(&p, pe()).unwrap();
        assert!(import_with(&f.m, file(&p).unwrap(), 0, MAX_BYTES, |_| Ok(())).is_err());
        assert!(import_with(&f.m, file(&p).unwrap(), u64::MAX, 512, |_| Ok(())).is_err());
        assert!(
            import_with(&f.m, file(&p).unwrap(), u64::MAX, MAX_BYTES, |_| {
                fs::write(&p, b"truncated")?;
                Ok(())
            })
            .is_err()
        );
        assert!(fs::read_dir(f.m.root.join("installers"))
            .unwrap()
            .next()
            .is_none());
        fs::write(&p, pe()).unwrap();
        assert!(import(
            &f.m,
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&p)
                .unwrap()
        )
        .is_err());
        assert!(
            import_with(&f.m, file(&p).unwrap(), u64::MAX, MAX_BYTES, |_| Err(
                "injected failure".into()
            ))
            .is_err()
        );
        assert!(fs::read_dir(f.m.root.join("installers"))
            .unwrap()
            .next()
            .is_none());
    }
    #[test]
    fn malformed_format_and_nonregular_ingress_are_refused() {
        let f = test_fixture::Fixture::new();
        let p = f.outer.join("looks-valid.exe");
        for bytes in [
            vec![0; 1024],
            {
                let mut b = pe();
                b[150] = 0;
                b
            },
            {
                let mut b = pe();
                b[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
                b
            },
        ] {
            fs::write(&p, bytes).unwrap();
            assert!(import(&f.m, file(&p).unwrap()).is_err());
        }
        use std::os::fd::OwnedFd;
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        assert!(import(&f.m, fs::File::from(OwnedFd::from(a))).is_err());
        let fifo = f.outer.join("pipe");
        let name = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let descriptor = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&fifo)
            .unwrap();
        assert!(import(&f.m, descriptor).is_err());
    }

    #[test]
    fn source_permission_change_during_copy_refuses_without_partial_artifacts() {
        let f = test_fixture::Fixture::new();
        let source = f.outer.join("installer.exe");
        fs::write(&source, pe()).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(import_with(&f.m, file(&source).unwrap(), u64::MAX, MAX_BYTES, |_| {
            fs::set_permissions(&source, fs::Permissions::from_mode(0o644))?;
            Ok(())
        }).is_err());
        assert!(fs::read_dir(f.m.root.join("installers")).unwrap().next().is_none());
    }
}
