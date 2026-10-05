//! Optional SDK call observation export. Never invoked from process().
//! The SDK owner lends a quiescent plain span; Rust retains no caller pointers.
use std::{
    fs::OpenOptions,
    io::{self, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
const CAPACITY: u32 = 262_144;
const RECORD_BYTES: usize = 160;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Record {
    pub schema: u32,
    pub size: u32,
    pub instance: u64,
    pub sequence: u64,
    pub entry_ns: u64,
    pub return_ns: u64,
    pub backend_handle: u64,
    pub configuration: u64,
    pub sample_rate: f64,
    pub project_samples: i64,
    pub namespace_pid: u32,
    pub namespace_tid: u32,
    pub frames: i32,
    pub mode: i32,
    pub precision: i32,
    pub maximum: i32,
    pub phase: i32,
    pub sdk_result: i32,
    pub valid: u32,
    pub outcome: u32,
    pub windows_epoch: u64,
    pub windows_last_host_call: u64,
    pub windows_requests: u64,
    pub windows_process_ns: u64,
    pub windows_first_sequence: u64,
    pub windows_last_sequence: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Summary {
    pub schema: u32,
    pub size: u32,
    pub capacity: u32,
    pub namespace_pid: u32,
    pub record_size: u32,
    pub flags: u32,
    pub instance: u64,
    pub offered: u64,
    pub retained: u64,
    pub capacity_dropped: u64,
    pub contention_dropped: u64,
    pub allocation_dropped: u64,
    pub invalid_clocks: u64,
    pub invalid_identity: u64,
    pub after_seal: u64,
    pub unfinished_writers: u64,
    pub sequence_overflow: u64,
}
const _: () = assert!(std::mem::size_of::<Record>() == RECORD_BYTES);
const _: () = assert!(std::mem::size_of::<Summary>() == 112);
fn bad() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "process call observation")
}
fn validate(summary: &Summary, records: &[Record]) -> io::Result<()> {
    if summary.schema != 2
        || summary.size != 112
        || summary.capacity > CAPACITY
        || summary.record_size != RECORD_BYTES as u32
        || summary.flags & !15 != 0
        || summary.flags & 5 != 5
        || summary.unfinished_writers != 0
        || summary.retained != records.len() as u64
        || records.len() > summary.capacity as usize
    {
        return Err(bad());
    }
    if summary.flags & 8 != 0
        && (summary.flags & 2 == 0
            || summary.instance == 0
            || summary.offered != summary.retained
            || summary.capacity_dropped != 0
            || summary.contention_dropped != 0
            || summary.allocation_dropped != 0
            || summary.invalid_clocks != 0
            || summary.invalid_identity != 0
            || summary.after_seal != 0
            || summary.sequence_overflow != 0)
    {
        return Err(bad());
    }
    for record in records {
        if record.schema != 2
            || record.size != RECORD_BYTES as u32
            || record.instance != summary.instance
            || record.namespace_pid != summary.namespace_pid
            || record.valid & !511 != 0
            || !(1..=2).contains(&record.outcome)
            || (record.valid & 64 != 0) != (record.outcome == 1)
        {
            return Err(bad());
        }
        if record.valid & 256 != 0
            && (record.windows_epoch == 0
                || record.windows_last_host_call == 0
                || record.windows_requests == 0
                || record.windows_first_sequence == 0
                || record.windows_last_sequence < record.windows_first_sequence)
        {
            return Err(bad());
        }
        if summary.flags & 8 != 0
            && (record.valid & 39 != 39
                || record.namespace_tid == 0
                || record.entry_ns == 0
                || record.return_ns < record.entry_ns)
        {
            return Err(bad());
        }
    }
    Ok(())
}
// Explicit little endian; never serialize Rust/C++ padding or atomic objects.
fn encode_summary(s: &Summary) -> [u8; 112] {
    let mut out = [0; 112];
    let mut at = 0;
    for value in [
        s.schema,
        s.size,
        s.capacity,
        s.namespace_pid,
        s.record_size,
        s.flags,
    ] {
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        at += 4;
    }
    for value in [
        s.instance,
        s.offered,
        s.retained,
        s.capacity_dropped,
        s.contention_dropped,
        s.allocation_dropped,
        s.invalid_clocks,
        s.invalid_identity,
        s.after_seal,
        s.unfinished_writers,
        s.sequence_overflow,
    ] {
        out[at..at + 8].copy_from_slice(&value.to_le_bytes());
        at += 8;
    }
    out
}
fn encode_record(r: &Record) -> [u8; RECORD_BYTES] {
    let mut out = [0; RECORD_BYTES];
    let mut at = 0;
    for value in [r.schema, r.size] {
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        at += 4;
    }
    for value in [
        r.instance,
        r.sequence,
        r.entry_ns,
        r.return_ns,
        r.backend_handle,
        r.configuration,
        r.sample_rate.to_bits(),
        r.project_samples as u64,
    ] {
        out[at..at + 8].copy_from_slice(&value.to_le_bytes());
        at += 8;
    }
    for value in [
        r.namespace_pid,
        r.namespace_tid,
        r.frames as u32,
        r.mode as u32,
        r.precision as u32,
        r.maximum as u32,
        r.phase as u32,
        r.sdk_result as u32,
        r.valid,
        r.outcome,
    ] {
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        at += 4;
    }
    for value in [
        r.windows_epoch,
        r.windows_last_host_call,
        r.windows_requests,
        r.windows_process_ns,
        r.windows_first_sequence,
        r.windows_last_sequence,
    ] {
        out[at..at + 8].copy_from_slice(&value.to_le_bytes());
        at += 8;
    }
    out
}
fn export(report: &Path, summary: &Summary, records: &[Record]) -> io::Result<()> {
    validate(summary, records)?;
    let metadata = std::fs::symlink_metadata(report)?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::getuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(bad());
    }
    let mut name = report.as_os_str().to_os_string();
    name.push(".process-calls.bin");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(Path::new(&name))?;
    file.write_all(b"LVBPC002")?;
    file.write_all(&128_u32.to_le_bytes())?;
    file.write_all(&(RECORD_BYTES as u32).to_le_bytes())?;
    file.write_all(&encode_summary(summary))?;
    for record in records {
        file.write_all(&encode_record(record))?;
    }
    file.sync_all()?;
    // No replace/retry. A partial failed file stays retained and cannot look
    // complete without its exact expected extent plus successful export status.
    Ok(())
}
/// Off-thread diagnostic ABI2. The caller owns valid aligned immutable spans
/// for this call; no SDK object, allocation or ownership crosses the boundary.
///
/// # Safety
/// Non-null pointers must reference the declared aligned summary and `count`
/// immutable records until return, with no concurrent writer or ownership change.
#[no_mangle]
pub unsafe extern "C" fn lvb_process_call_export(
    id: u64,
    summary: *const Summary,
    records: *const Record,
    count: u32,
) -> u32 {
    crate::ffi(|| {
        if summary.is_null() || (count != 0 && records.is_null()) || count > CAPACITY {
            return 1;
        }
        let summary = unsafe { &*summary };
        let records = if count == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(records, count as usize) }
        };
        let Some(report) = crate::queued::process_call_report(id) else {
            return 2;
        };
        match export(&report, summary, records) {
            Ok(()) => 0,
            Err(_) => 3,
        }
    }) as u32
}
#[cfg(test)]
mod tests {
    use super::*;
    fn observation() -> (Summary, Record) {
        let record = Record {
            schema: 2,
            size: 160,
            instance: 7,
            sequence: 0,
            entry_ns: 10,
            return_ns: 20,
            namespace_pid: 12,
            namespace_tid: 13,
            frames: -1,
            mode: 0,
            precision: 0,
            sdk_result: 1,
            valid: 103,
            outcome: 1,
            ..Record::default()
        };
        let summary = Summary {
            schema: 2,
            size: 112,
            capacity: CAPACITY,
            namespace_pid: 12,
            record_size: 160,
            flags: 15,
            instance: 7,
            offered: 1,
            retained: 1,
            ..Summary::default()
        };
        (summary, record)
    }
    #[test]
    fn timing_plain_layout_and_signed_little_endian() {
        let (s, r) = observation();
        assert!(validate(&s, &[r]).is_ok());
        let bytes = encode_record(&r);
        assert_eq!(&bytes[80..84], &(-1_i32).to_le_bytes());
        assert_eq!(&bytes[24..32], &10_u64.to_le_bytes());
        assert_eq!(&encode_summary(&s)[40..48], &1_u64.to_le_bytes());
    }
    #[test]
    fn timing_refuses_false_completeness_and_unfinished_ownership() {
        let (mut s, mut r) = observation();
        s.capacity_dropped = 1;
        assert!(validate(&s, &[r]).is_err());
        s.flags &= !8;
        assert!(validate(&s, &[r]).is_ok()); // Retain explicit partial observation.
        s.unfinished_writers = 1;
        assert!(validate(&s, &[r]).is_err());
        s.unfinished_writers = 0;
        r.outcome = 2;
        assert!(validate(&s, &[r]).is_err());
        r.valid &= !64;
        assert!(validate(&s, &[r]).is_ok()); // Unwind never fabricates result.
        r.instance = 8;
        assert!(validate(&s, &[r]).is_err());
    }
    #[test]
    fn timing_export_is_private_exclusive_and_never_follows_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let root = std::env::temp_dir().join(format!("lvb-timing-export-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let report = root.join("session.ndjson");
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&report)
            .unwrap();
        let (s, r) = observation();
        export(&report, &s, &[r]).unwrap();
        let calls = root.join("session.ndjson.process-calls.bin");
        let bytes = std::fs::read(&calls).unwrap();
        assert_eq!(bytes.len(), 288);
        assert_eq!(&bytes[..8], b"LVBPC002");
        assert_eq!(std::fs::metadata(&calls).unwrap().mode() & 0o777, 0o600);
        assert!(export(&report, &s, &[r]).is_err());
        assert_eq!(std::fs::read(&calls).unwrap(), bytes);
        std::fs::remove_file(&calls).unwrap();
        symlink(&report, &calls).unwrap();
        assert!(export(&report, &s, &[r]).is_err());
        assert_eq!(std::fs::metadata(&report).unwrap().len(), 0);
        std::fs::remove_file(&calls).unwrap();
        std::fs::set_permissions(&report, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(export(&report, &s, &[r]).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
