//! Factory-time descriptor ownership. No entry point is called from processing.
use lvb_plugin_descriptor::{Descriptor, FILE_NAME, LIMIT};
use sha2::{Digest, Sha256};
use std::{
    ffi::{c_char, c_void, CStr},
    fs::{File, OpenOptions},
    io::Read,
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
fn open(path: &Path) -> Result<File, ()> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| ())?;
    let meta = file.metadata().map_err(|_| ())?;
    if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } {
        return Err(());
    }
    Ok(file)
}
fn load(path: &Path) -> Result<(Owned, View), ()> {
    let mut bytes = Vec::new();
    open(&path.with_file_name(FILE_NAME))?
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    let descriptor = Descriptor::parse(&bytes).map_err(|_| ())?;
    let mut engine = open(path)?;
    if engine.metadata().map_err(|_| ())?.len() > 128 * 1024 * 1024 {
        return Err(());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    let mut total = 0;
    loop {
        let n = engine.read(&mut buffer).map_err(|_| ())?;
        if n == 0 {
            break;
        }
        total += n;
        if total > 128 * 1024 * 1024 {
            return Err(());
        }
        hash.update(&buffer[..n]);
    }
    if format!("{:x}", hash.finalize()) != descriptor.engine_sha256 {
        return Err(());
    }
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
        abi: 1,
        size: std::mem::size_of::<View>() as u32,
        identity: descriptor.identity().map_err(|_| ())?,
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
pub unsafe extern "C" fn lvb_descriptor_open_v1(
    path: *const c_char,
    owner: *mut *mut c_void,
    out: *mut View,
) -> i32 {
    if path.is_null() || owner.is_null() || out.is_null() {
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
        match load(path) {
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
