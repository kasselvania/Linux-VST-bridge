//! First-party Windows installer for the delivered instrument/effect journey.
//! Build with the exact two module files in LVB_BETA_INSTRUMENT/LVB_BETA_EFFECT.
//! No bridge helper, runtime acquisition, authorization or publication bypass.
use std::{ffi::c_void, path::Path, ptr};
const INSTRUMENT: &[u8] = include_bytes!(env!("LVB_BETA_INSTRUMENT"));
const EFFECT: &[u8] = include_bytes!(env!("LVB_BETA_EFFECT"));
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
    unsafe { MessageBoxW(ptr::null_mut(), wide(text).as_ptr(),
        wide("LVB Reference Plug-ins 1.0.0").as_ptr(), flags) }
}
struct Key(*mut c_void);
impl Drop for Key { fn drop(&mut self) { unsafe { RegCloseKey(self.0); } } }
fn registration(directory: &Path) -> Result<(), String> {
    let mut handle = ptr::null_mut();
    // HKEY_CURRENT_USER is a sign-extended predefined handle on Win64.
    let hkcu = (0x80000001_u32 as i32 as isize) as *mut c_void;
    let result = unsafe { RegCreateKeyExW(hkcu,
        wide("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\LVBReferencePlugins").as_ptr(),
        0, ptr::null_mut(), 0, 0x0002, ptr::null_mut(), &mut handle, ptr::null_mut()) };
    if result != 0 { return Err(format!("Reference registration failed ({result})")); }
    let key = Key(handle);
    let location = directory.to_str().ok_or("Reference installation path unavailable")?;
    for (name, value) in [("DisplayName", "LVB Reference Plug-ins"),
        ("DisplayVersion", "1.0.0"), ("Publisher", "Linux VST Bridge"),
        ("InstallLocation", location)] {
        let bytes = wide(value);
        let result = unsafe { RegSetValueExW(key.0, wide(name).as_ptr(), 0, 1,
            bytes.as_ptr().cast(), (bytes.len() * 2) as u32) };
        if result != 0 { return Err(format!("Reference registration failed ({result})")); }
    }
    Ok(())
}
fn install() -> Result<(), String> {
    let common = std::env::var_os("CommonProgramFiles")
        .ok_or("Windows common program files are unavailable")?;
    let directory = Path::new(&common).join("VST3");
    if !directory.is_absolute() { return Err("Windows installation location is not absolute".into()); }
    std::fs::create_dir_all(&directory).map_err(|_| "Could not create the reference plug-in directory")?;
    // Each module is replaced atomically. A deliberately interrupted installation
    // can leave one exact installed module; the manager must discover that fact.
    for (name, bytes) in [("LVBReferenceInstrument.vst3", INSTRUMENT),
        ("LVBReferenceEffect.vst3", EFFECT)] {
        let path = directory.join(name);
        if path.exists() {
            if std::fs::read(&path).map_err(|_| "Could not read the existing reference module")? != bytes {
                return Err("A different reference module is already installed; no file was overwritten".into());
            }
        } else {
            let staged = directory.join(format!("{name}.installing"));
            let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&staged)
                .map_err(|_| "Could not stage the reference module; existing files were retained")?;
            use std::io::Write;
            file.write_all(bytes).and_then(|_| file.sync_all())
                .map_err(|_| "Reference module write failed; retry requires reviewing the partial installation")?;
            drop(file);
            std::fs::rename(staged, path).map_err(|_| "Reference module completion failed")?;
        }
        if cfg!(beta_partial_hold) && name == "LVBReferenceInstrument.vst3" {
            message("Partial-installation test: the instrument file is installed. Leave this window open and use Stop in the manager. OK continues installing the effect.", 0);
        }
    }
    registration(&directory)
}
fn main() {
    if std::env::args().any(|arg| arg == "--self-test") {
        assert!(INSTRUMENT.starts_with(b"MZ") && EFFECT.starts_with(b"MZ"));
        assert!(INSTRUMENT.len() > 4096 && EFFECT.len() > 4096);
        println!("LVB_REFERENCE_INSTALLER_V1 payloads=2 role=source_owned");
        return;
    }
    println!("LVB_REFERENCE_INSTALLER_V1 ready");
    if cfg!(beta_hold) {
        message("Recovery test: this installer is deliberately waiting. Use Focus and Stop on its Setup card. OK continues to the ordinary installation screen.", 0);
    }
    if message("Install the first-party reference instrument and effect? These fixtures test sound, automation and saved state. No account or activation is required.", 1) != 1 {
        std::process::exit(1);
    }
    match install() {
        Ok(()) => { message("The reference instrument and effect are installed. Continue from the manager's Setup card to find the installed plug-ins.", 0); }
        Err(error) => { message(&error, 0x10); std::process::exit(2); }
    }
}
