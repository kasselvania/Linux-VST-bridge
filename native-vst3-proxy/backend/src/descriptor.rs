//! Factory-time descriptor ownership. No entry point is called from processing.
use lvb_plugin_descriptor::{Descriptor, FILE_NAME, LIMIT};
use sha2::{Digest, Sha256};
use std::{
    ffi::{c_char, c_void, CStr},
    fs::{File, OpenOptions},
    io::Read,
    os::fd::{AsRawFd, FromRawFd},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
};
#[repr(C)]
pub struct Bus {
    media: u32,
    direction: u32,
    index: u32,
    channels: u32,
    kind: u32,
    flags: u32,
    arrangement: u64,
    name: [u16; 128],
}
#[repr(C)]
pub struct Parameter {
    id: u32,
    title: [u16; 128],
    units: [u16; 128],
    steps: i32,
    flags: i32,
    initial: f64,
    available: u8,
}
#[repr(C)]
pub struct View {
    abi: u32,
    size: u32,
    identity: [u8; 48],
    engine_sha256: [u8; 32],
    descriptor_sha256: [u8; 32],
    processor: [u8; 16],
    controller: [u8; 16],
    class_name: [u8; 64],
    vendor: [u8; 64],
    version: [u8; 64],
    subcategories: [u8; 128],
    bus_count: u32,
    parameter_count: u32,
    buses: *const Bus,
    parameters: *const Parameter,
}
struct Owned {
    buses: Vec<Bus>,
    parameters: Vec<Parameter>,
}
fn wide(s: &str) -> [u16; 128] {
    let mut out = [0; 128];
    for (i, v) in s.encode_utf16().enumerate() {
        out[i] = v;
    }
    out
}
fn narrow<const N: usize>(s: &str) -> [u8; N] {
    let mut out = [0; N];
    out[..s.len()].copy_from_slice(s.as_bytes());
    out
}
fn open_at(directory: &File, name: &std::ffi::OsStr) -> Result<File, ()> {
    let name = std::ffi::CString::new(name.as_bytes()).map_err(|_| ())?;
    if name.as_bytes().contains(&b'/') { return Err(()); }
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(),
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if fd < 0 { return Err(()); }
    let file = unsafe { File::from_raw_fd(fd) };
    let meta = file.metadata().map_err(|_| ())?;
    if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } {
        return Err(());
    }
    Ok(file)
}
fn mapping_identity(bytes: &[u8], address: usize) -> Result<(u64, u64), ()> {
    for line in std::str::from_utf8(bytes).map_err(|_| ())?.lines() {
        let fields: Vec<_> = line.split_whitespace().take(5).collect();
        if fields.len() != 5 { return Err(()); }
        let (begin, end) = fields[0].split_once('-').ok_or(())?;
        let begin = usize::from_str_radix(begin, 16).map_err(|_| ())?;
        let end = usize::from_str_radix(end, 16).map_err(|_| ())?;
        if !(begin..end).contains(&address) { continue; }
        if fields[1].as_bytes().get(2).is_none_or(|permission| *permission != b'x') {
            return Err(());
        }
        let (major, minor) = fields[3].split_once(':').ok_or(())?;
        let major = u64::from_str_radix(major, 16).map_err(|_| ())?;
        let minor = u64::from_str_radix(minor, 16).map_err(|_| ())?;
        let inode = fields[4].parse::<u64>().map_err(|_| ())?;
        if inode == 0 { return Err(()); }
        return Ok((libc::makedev(major as _, minor as _) as u64, inode));
    }
    Err(())
}
fn loaded_mapping(address: usize) -> Result<(u64, u64), ()> {
    let mut bytes = Vec::new();
    File::open("/proc/self/maps").map_err(|_| ())?.take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes).map_err(|_| ())?;
    if bytes.len() > 8 * 1024 * 1024 { return Err(()); }
    mapping_identity(&bytes, address)
}
fn hash(mut file: &File, maximum: usize) -> Result<[u8; 32], ()> {
    if file.metadata().map_err(|_| ())?.len() > maximum as u64 { return Err(()); }
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    let mut total = 0;
    loop {
        let n = file.read(&mut buffer).map_err(|_| ())?;
        if n == 0 { break; }
        total += n;
        if total > maximum { return Err(()); }
        hash.update(&buffer[..n]);
    }
    Ok(hash.finalize().into())
}
fn digest_text(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn load(path: &Path, image_address: usize) -> Result<(Owned, View), ()> {
    let parent = path.parent().ok_or(())?;
    let name = path.file_name().ok_or(())?;
    let directory = OpenOptions::new().read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent).map_err(|_| ())?;
    let engine = open_at(&directory, name)?;
    let engine_meta = engine.metadata().map_err(|_| ())?;
    if loaded_mapping(image_address)? != (engine_meta.dev(), engine_meta.ino()) { return Err(()); }
    let mut descriptor_file = open_at(&directory, std::ffi::OsStr::new(FILE_NAME))?;
    let mut bytes = Vec::new();
    descriptor_file.by_ref().take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > LIMIT { return Err(()); }
    let descriptor_sha256: [u8; 32] = Sha256::digest(&bytes).into();
    let descriptor = Descriptor::parse(&bytes).map_err(|_| ())?;
    let engine_sha256 = hash(&engine, 128 * 1024 * 1024)?;
    if digest_text(&engine_sha256) != descriptor.engine_sha256 { return Err(()); }
    let owned = Owned {
        buses: descriptor
            .buses
            .iter()
            .map(|b| Bus {
                media: b.media,
                direction: b.direction,
                index: b.index,
                channels: b.channels,
                kind: b.r#type,
                flags: b.flags,
                arrangement: b.arrangement,
                name: wide(&b.name),
            })
            .collect(),
        parameters: descriptor
            .parameters
            .iter()
            .map(|p| Parameter {
                id: p.id,
                title: wide(&p.title),
                units: wide(&p.units),
                steps: p.steps,
                flags: p.flags,
                initial: p.initial,
                available: u8::from(p.available),
            })
            .collect(),
    };
    let ids = descriptor.external_ids();
    let view = View {
        abi: 2,
        size: std::mem::size_of::<View>() as u32,
        identity: descriptor.identity().map_err(|_| ())?,
        engine_sha256,
        descriptor_sha256,
        processor: ids[0],
        controller: ids[1],
        class_name: narrow(&descriptor.class_name),
        vendor: narrow(&descriptor.vendor),
        version: narrow(&descriptor.version),
        subcategories: narrow(&descriptor.subcategories),
        bus_count: owned.buses.len() as u32,
        parameter_count: owned.parameters.len() as u32,
        buses: owned.buses.as_ptr(),
        parameters: owned.parameters.as_ptr(),
    };
    Ok((owned, view))
}
/// Output pointers borrow the opaque owner until close. Caller owns only the view.
#[no_mangle]
pub unsafe extern "C" fn lvb_descriptor_open_v2(
    path: *const c_char,
    image_address: *const c_void,
    owner: *mut *mut c_void,
    out: *mut View,
) -> i32 {
    if path.is_null() || image_address.is_null() || owner.is_null() || out.is_null() {
        return 1;
    }
    unsafe {
        *owner = std::ptr::null_mut();
    }
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = unsafe { CStr::from_ptr(path) }.to_bytes();
        if bytes.len() > 4096 {
            return 1;
        }
        let path = Path::new(std::ffi::OsStr::from_bytes(bytes));
        match load(path, image_address as usize) {
            Ok((owned, view)) => {
                unsafe {
                    *owner = Box::into_raw(Box::new(owned)).cast();
                    *out = view;
                }
                0
            }
            Err(()) => 1,
        }
    }))
    .unwrap_or(1)
}
#[no_mangle]
pub unsafe extern "C" fn lvb_descriptor_close_v1(owner: *mut c_void) {
    if !owner.is_null() {
        drop(unsafe { Box::from_raw(owner.cast::<Owned>()) });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapped_identity_is_exact_and_malformed_or_anonymous_input_refuses() {
        let maps = b"1000-2000 r-xp 00000000 08:02 42 /owned/native.so\n2000-3000 rw-p 0 00:00 0\n";
        assert_eq!(mapping_identity(maps, 0x1800).unwrap(), (libc::makedev(8, 2) as u64, 42));
        assert!(mapping_identity(maps, 0x2800).is_err());
        assert!(mapping_identity(b"malformed\n", 0x1800).is_err());
    }
}
