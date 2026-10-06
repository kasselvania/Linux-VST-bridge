//! First-party Windows installer for the delivered instrument/effect journey.
//! Build with the exact two module files in LVB_BETA_INSTRUMENT/LVB_BETA_EFFECT.
//! No bridge helper, runtime acquisition, authorization or publication bypass.
use std::{ffi::c_void, path::Path, ptr};
const INSTRUMENT: &[u8] = include_bytes!(env!("LVB_BETA_INSTRUMENT"));
const EFFECT: &[u8] = include_bytes!(env!("LVB_BETA_EFFECT"));
#[cfg(all(beta_completion, any(beta_hold, beta_partial_hold)))]
compile_error!("Completion instrumentation uses the ordinary installer only");
const FAMILY: &str = if cfg!(beta_completion) { "completion" } else { "reference" };
const TITLE: &str = if cfg!(beta_completion) { "LVB Completion Plug-ins 1.0.0" } else { "LVB Reference Plug-ins 1.0.0" };
const DISPLAY_NAME: &str = if cfg!(beta_completion) { "LVB Completion Plug-ins" } else { "LVB Reference Plug-ins" };
const REGISTRATION: &str = if cfg!(beta_completion) { "LVBCompletionPlugins" } else { "LVBReferencePlugins" };
const INSTRUMENT_NAME: &str = if cfg!(beta_completion) { "LVBCompletionInstrument.vst3" } else { "LVBReferenceInstrument.vst3" };
const EFFECT_NAME: &str = if cfg!(beta_completion) { "LVBCompletionEffect.vst3" } else { "LVBReferenceEffect.vst3" };
const MARKER: &str = if cfg!(beta_completion) { "LVB_COMPLETION_INSTALLER_V1" } else { "LVB_REFERENCE_INSTALLER_V1" };
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
        wide(TITLE).as_ptr(), flags) }
}
struct Key(*mut c_void);
impl Drop for Key { fn drop(&mut self) { unsafe { RegCloseKey(self.0); } } }
fn registration(directory: &Path) -> Result<(), String> {
    let mut handle = ptr::null_mut();
    // HKEY_CURRENT_USER is a sign-extended predefined handle on Win64.
    let hkcu = (0x80000001_u32 as i32 as isize) as *mut c_void;
    let subkey = format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{REGISTRATION}");
    let result = unsafe { RegCreateKeyExW(hkcu,
        wide(&subkey).as_ptr(),
        0, ptr::null_mut(), 0, 0x0002, ptr::null_mut(), &mut handle, ptr::null_mut()) };
    if result != 0 { return Err(format!("{DISPLAY_NAME} registration failed ({result})")); }
    let key = Key(handle);
    let location = directory.to_str().ok_or("Fixture installation path unavailable")?;
    for (name, value) in [("DisplayName", DISPLAY_NAME),
        ("DisplayVersion", "1.0.0"), ("Publisher", "Linux VST Bridge"),
        ("InstallLocation", location)] {
        let bytes = wide(value);
        let result = unsafe { RegSetValueExW(key.0, wide(name).as_ptr(), 0, 1,
            bytes.as_ptr().cast(), (bytes.len() * 2) as u32) };
        if result != 0 { return Err(format!("{DISPLAY_NAME} registration failed ({result})")); }
    }
    Ok(())
}
fn install() -> Result<(), String> {
    let common = std::env::var_os("CommonProgramFiles")
        .ok_or("Windows common program files are unavailable")?;
    let directory = Path::new(&common).join("VST3");
    if !directory.is_absolute() { return Err("Windows installation location is not absolute".into()); }
    install_modules(&directory, true)?;
    registration(&directory)
}
fn install_modules(directory: &Path, show_hold: bool) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|_| "Could not create the fixture plug-in directory")?;
    // Each new module is published atomically; an existing exact build is retained.
    // A deliberately interrupted installation
    // can leave one exact installed module; the manager must discover that fact.
    for (name, bytes) in [(INSTRUMENT_NAME, INSTRUMENT), (EFFECT_NAME, EFFECT)] {
        let path = directory.join(name);
        if path.exists() {
            if std::fs::read(&path).map_err(|_| "Could not read the existing fixture module")? != bytes {
                return Err("A different fixture module is already installed; no file was overwritten".into());
            }
        } else {
            let staged = directory.join(format!("{name}.installing"));
            let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&staged)
                .map_err(|_| "Could not stage the fixture module; existing files were retained")?;
            use std::io::Write;
            file.write_all(bytes).and_then(|_| file.sync_all())
                .map_err(|_| "Fixture module write failed; retry requires reviewing the partial installation")?;
            drop(file);
            std::fs::rename(staged, path).map_err(|_| "Fixture module completion failed")?;
        }
        if show_hold && cfg!(beta_partial_hold) && name == INSTRUMENT_NAME {
            message("Partial-installation test: the instrument file is installed. Leave this window open and use Stop in the manager. OK continues installing the effect.", 0);
        }
    }
    Ok(())
}
fn self_test() {
    assert!(INSTRUMENT.starts_with(b"MZ") && EFFECT.starts_with(b"MZ"));
    assert!(INSTRUMENT.len() > 4096 && EFFECT.len() > 4096);
    let directory = std::env::temp_dir().join(format!("lvb-{FAMILY}-installer-self-test-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("self-test requires a fresh owned directory");
    let other_names = if cfg!(beta_completion) { ["LVBReferenceInstrument.vst3", "LVBReferenceEffect.vst3"] }
        else { ["LVBCompletionInstrument.vst3", "LVBCompletionEffect.vst3"] };
    for name in other_names { std::fs::write(directory.join(name), b"other fixture family").unwrap(); }
    // Use the actual file-installation implementation without registry/UI or
    // touching a product environment. Both families must remain independent.
    install_modules(&directory, false).unwrap();
    install_modules(&directory, false).unwrap();
    assert_eq!(std::fs::read(directory.join(INSTRUMENT_NAME)).unwrap(), INSTRUMENT);
    assert_eq!(std::fs::read(directory.join(EFFECT_NAME)).unwrap(), EFFECT);
    for name in other_names { assert_eq!(std::fs::read(directory.join(name)).unwrap(), b"other fixture family"); }
    std::fs::write(directory.join(INSTRUMENT_NAME), b"different installed build").unwrap();
    assert!(install_modules(&directory, false).is_err());
    assert_eq!(std::fs::read(directory.join(INSTRUMENT_NAME)).unwrap(), b"different installed build");
    assert_eq!(std::fs::read(directory.join(EFFECT_NAME)).unwrap(), EFFECT);
    // Recovery variants' self-test must not display their deliberate hold UI.
    std::fs::remove_dir_all(directory).unwrap();
    println!("{MARKER} payloads=2 role=source_owned coexistence=passed overwrite=refused");
}
fn main() {
    if std::env::args().any(|arg| arg == "--self-test") {
        self_test();
        return;
    }
    println!("{MARKER} ready");
    if cfg!(beta_hold) {
        message("Recovery test: this installer is deliberately waiting. Use Focus and Stop on its Setup card. OK continues to the ordinary installation screen.", 0);
    }
    if message(&format!("Install the first-party {FAMILY} instrument and effect? These fixtures test sound, automation and saved state. No account or activation is required."), 1) != 1 {
        std::process::exit(1);
    }
    match install() {
        Ok(()) => { message(&format!("The {FAMILY} instrument and effect are installed. Continue from the manager's Setup card to find the installed plug-ins."), 0); }
        Err(error) => { message(&error, 0x10); std::process::exit(2); }
    }
}
