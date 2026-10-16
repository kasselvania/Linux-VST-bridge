//! Why the DAW could not start a bridged plug-in's audio (#403). The proxy
//! refuses setupProcessing or setActive for a few plain reasons; the DAW shows
//! only "error code 1". One small record per plug-in class, beside the
//! service's native-startup records, lets the manager's card say why.
//!
//! Written only on the DAW's owner thread while the instance is inactive,
//! never from process(). Bounded: a few hundred bytes, one file per class,
//! replaced atomically. No paths, names, settings or vendor data.
use std::{
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

pub const DIRECTORY: &str = "native-activation";
pub const STAGES: [&str; 2] = ["setup", "activate"];
// Index + 1 is the reason code the native edge passes (0 is invalid).
pub const REASONS: [&str; 7] = [
    "daw_block_size",
    "daw_sample_format",
    "daw_process_mode",
    "daw_sample_rate",
    "windows_setup_refused",
    "windows_activation_refused",
    "memory",
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn file(directory: &Path, class: &[u8; 16]) -> PathBuf {
    directory.join(format!("{}.json", hex(class)))
}

pub struct Refusal<'a> {
    pub class: &'a [u8; 16],
    pub module: &'a [u8; 32],
    pub engine: &'a [u8; 32],
    pub stage: u32,
    pub reason: u32,
    pub requested: u64,
}

pub fn record(directory: &Path, refusal: &Refusal, observed_at: u64) -> io::Result<()> {
    let stage = refusal.stage.checked_sub(1).and_then(|i| STAGES.get(i as usize))
        .ok_or_else(|| io::Error::other("activation stage"))?;
    let reason = refusal.reason.checked_sub(1).and_then(|i| REASONS.get(i as usize))
        .ok_or_else(|| io::Error::other("activation reason"))?;
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(directory)?;
    let text = format!(
        "{{\"schema\":1,\"class_id\":\"{}\",\"module_sha256\":\"{}\",\"engine_sha256\":\"{}\",\
         \"observed_at\":{observed_at},\"stage\":\"{stage}\",\"reason\":\"{reason}\",\"requested\":{}}}\n",
        hex(refusal.class).to_uppercase(), hex(refusal.module), hex(refusal.engine), refusal.requested);
    let target = file(directory, refusal.class);
    let temporary = target.with_extension(format!("json.{}.tmp", std::process::id()));
    let written = (|| {
        let mut out = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
            .custom_flags(libc_nofollow()).open(&temporary)?;
        out.write_all(text.as_bytes())?;
        out.sync_all()?;
        std::fs::rename(&temporary, &target)
    })();
    if written.is_err() { let _ = std::fs::remove_file(&temporary); }
    written
}

/// A later activation of the same instance succeeded: its refusal no longer
/// describes this plug-in.
pub fn clear(directory: &Path, class: &[u8; 16]) -> io::Result<()> {
    match std::fs::remove_file(file(directory, class)) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

fn libc_nofollow() -> i32 {
    #[cfg(target_os = "linux")]
    { 0x20000 }
    #[cfg(target_os = "macos")]
    { 0x100 }
}

fn directory() -> PathBuf {
    crate::preview::performance_root(true).join(DIRECTORY)
}
fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// identity: 16-byte class + 32-byte module digest; engine: 32-byte digest.
/// Owner thread only. Zero when recorded.
#[no_mangle]
pub unsafe extern "C" fn lvb_activation_refused(identity: *const u8, engine: *const u8,
    stage: u32, reason: u32, requested: u64) -> u32 {
    crate::ffi(|| {
        if identity.is_null() || engine.is_null() { return 1; }
        let identity = std::slice::from_raw_parts(identity, 48);
        let refusal = Refusal {
            class: identity[..16].try_into().unwrap(),
            module: identity[16..].try_into().unwrap(),
            engine: std::slice::from_raw_parts(engine, 32).try_into().unwrap(),
            stage, reason, requested,
        };
        if record(&directory(), &refusal, now()).is_err() { 2 } else { 0 }
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn lvb_activation_cleared(identity: *const u8) -> u32 {
    crate::ffi(|| {
        if identity.is_null() { return 1; }
        let class: &[u8; 16] = std::slice::from_raw_parts(identity, 16).try_into().unwrap();
        if clear(&directory(), class).is_err() { 2 } else { 0 }
    }) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_bounded_private_record_per_class_replaced_and_cleared() {
        let root = std::env::temp_dir().join(format!("lvb-activation-{}-{}", std::process::id(),
            crate::observer::monotonic_ns()));
        let directory = root.join(DIRECTORY);
        let (class, module, engine) = ([0xAB; 16], [1; 32], [2; 32]);
        let refusal = |reason, requested| Refusal { class: &class, module: &module, engine: &engine,
            stage: 1, reason, requested };
        record(&directory, &refusal(1, 65536), 1792100498).unwrap();
        record(&directory, &refusal(4, 22050), 1792100499).unwrap();
        let path = directory.join(format!("{}.json", "ab".repeat(16)));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.len() < 512);
        assert_eq!(text, format!("{{\"schema\":1,\"class_id\":\"{}\",\"module_sha256\":\"{}\",\
            \"engine_sha256\":\"{}\",\"observed_at\":1792100499,\"stage\":\"setup\",\
            \"reason\":\"daw_sample_rate\",\"requested\":22050}}\n",
            "AB".repeat(16), "01".repeat(32), "02".repeat(32)));
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(&directory).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1, "no temporary file left");
        assert!(record(&directory, &refusal(0, 0), 1).is_err());
        assert!(record(&directory, &refusal(REASONS.len() as u32 + 1, 0), 1).is_err());
        assert!(record(&directory, &Refusal { stage: 3, ..refusal(1, 0) }, 1).is_err());
        clear(&directory, &class).unwrap();
        assert!(!path.exists());
        clear(&directory, &class).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
