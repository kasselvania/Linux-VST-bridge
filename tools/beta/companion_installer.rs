//! Separate first-party dependency installer; no bridge or environment bypass.
use std::{ffi::c_void, io::Write, path::Path, ptr};

const PAYLOAD: &[u8] = include_bytes!(env!("LVB_BETA_COMPANION"));
const GENERATION: &str = env!("LVB_BETA_GENERATION");
const FILE: &str = "companion.dll";
const TITLE: &str = "LVB Unfamiliar Companion";

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(window: *mut c_void, text: *const u16, title: *const u16, flags: u32) -> i32;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegCreateKeyExW(key: *mut c_void, subkey: *const u16, reserved: u32,
        class: *mut u16, options: u32, access: u32, security: *mut c_void,
        result: *mut *mut c_void, disposition: *mut u32) -> i32;
    fn RegSetValueExW(key: *mut c_void, name: *const u16, reserved: u32,
        kind: u32, data: *const u8, length: u32) -> i32;
    fn RegCloseKey(key: *mut c_void) -> i32;
}
fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() }
fn message(text: &str, flags: u32) -> i32 {
    unsafe { MessageBoxW(ptr::null_mut(), wide(text).as_ptr(), wide(TITLE).as_ptr(), flags) }
}
struct Key(*mut c_void);
impl Drop for Key { fn drop(&mut self) { unsafe { RegCloseKey(self.0); } } }
fn registration(directory: &Path) -> Result<(), String> {
    let mut handle = ptr::null_mut();
    let hkcu = (0x80000001_u32 as i32 as isize) as *mut c_void;
    let subkey = format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\LVBUnfamiliarCompanion{GENERATION}");
    let result = unsafe { RegCreateKeyExW(hkcu, wide(&subkey).as_ptr(),
        0, ptr::null_mut(), 0, 0x0002, ptr::null_mut(), &mut handle, ptr::null_mut()) };
    if result != 0 { return Err(format!("Companion registration failed ({result})")); }
    let key = Key(handle);
    let location = directory.to_str().ok_or("Companion installation path unavailable")?;
    for (name, value) in [("DisplayName", TITLE), ("DisplayVersion", "1.0.0"),
        ("Publisher", "Linux VST Bridge"), ("InstallLocation", location)] {
        let bytes = wide(value);
        let result = unsafe { RegSetValueExW(key.0, wide(name).as_ptr(), 0, 1,
            bytes.as_ptr().cast(), (bytes.len() * 2) as u32) };
        if result != 0 { return Err(format!("Companion registration failed ({result})")); }
    }
    Ok(())
}
fn install_payload(directory: &Path) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|_| "Could not create the companion directory")?;
    let path = directory.join(FILE);
    if path.exists() {
        if std::fs::read(path).map_err(|_| "Could not read the installed companion")? != PAYLOAD {
            return Err("A different companion build is installed; no file was overwritten".into());
        }
        return Ok(());
    }
    let staged = directory.join("companion.dll.installing");
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&staged)
        .map_err(|_| "Could not stage the companion; partial files were retained")?;
    file.write_all(PAYLOAD).and_then(|_| file.sync_all())
        .map_err(|_| "Companion write failed; review the partial installation before retry")?;
    drop(file);
    std::fs::rename(staged, path).map_err(|_| "Companion file completion failed")?;
    Ok(())
}
fn install() -> Result<(), String> {
    let common = std::env::var_os("CommonProgramFiles")
        .ok_or("Windows common program files are unavailable")?;
    let directory = Path::new(&common).join("LVBUnfamiliar").join(GENERATION);
    if !directory.is_absolute() { return Err("Companion location is not absolute".into()); }
    install_payload(&directory)?;
    if cfg!(beta_partial_hold) {
        message("Partial-installation test: the required DSP library is installed, but installer registration is unfinished. Use Stop in the manager. OK completes registration.", 0);
    }
    registration(&directory)
}
fn self_test() {
    assert!(GENERATION.len() == 8 && GENERATION.bytes().all(|c| c.is_ascii_hexdigit()));
    assert!(PAYLOAD.starts_with(b"MZ") && PAYLOAD.len() > 4096);
    let directory = std::env::temp_dir().join(format!("lvb-companion-self-test-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("self-test requires a fresh owned directory");
    std::fs::write(directory.join("unrelated.vst3"), b"retained sibling").unwrap();
    install_payload(&directory).unwrap();
    install_payload(&directory).unwrap();
    assert_eq!(std::fs::read(directory.join(FILE)).unwrap(), PAYLOAD);
    assert_eq!(std::fs::read(directory.join("unrelated.vst3")).unwrap(), b"retained sibling");
    std::fs::write(directory.join(FILE), b"different dependency").unwrap();
    assert!(install_payload(&directory).is_err());
    assert_eq!(std::fs::read(directory.join(FILE)).unwrap(), b"different dependency");
    std::fs::remove_dir_all(directory).unwrap();
    println!("LVB_COMPANION_INSTALLER_V1 generation={GENERATION} retry=passed overwrite=refused sibling=retained");
}
fn main() {
    if std::env::args().any(|arg| arg == "--self-test") { self_test(); return; }
    println!("LVB_COMPANION_INSTALLER_V1 generation={GENERATION} ready");
    if cfg!(beta_hold) {
        message("Recovery test: no companion file has been installed. Use Focus and Stop in the manager. OK continues to the ordinary installation screen.", 0);
    }
    if message("Install the separate first-party DSP companion required by the unfamiliar instrument and effect? No account or activation is required.", 1) != 1 {
        std::process::exit(1);
    }
    match install() {
        Ok(()) => { message("The DSP companion is installed. Return to the manager to rescan and prepare the plug-ins.", 0); }
        Err(error) => { message(&error, 0x10); std::process::exit(2); }
    }
}
