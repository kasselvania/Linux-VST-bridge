//! First-party vendor application fixture. The product observes cohort retirement,
//! never a fixture marker. Reuses the separately built unfamiliar/DSP installers.
use std::{ffi::c_void, ptr};
#[cfg(hub_bootstrap)]
use std::path::Path;

const TITLE: &str = "LVB Source Vendor Hub";
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(window: *mut c_void, text: *const u16, title: *const u16, flags: u32) -> i32;
}
#[cfg(any(hub_bootstrap, hub_helper))]
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
#[cfg(any(hub_bootstrap, hub_helper))]
fn registry(subkey: &str, values: &[(&str, &str)]) -> Result<(), String> {
    let mut key = ptr::null_mut();
    let hkcu = (0x80000001_u32 as i32 as isize) as *mut c_void;
    let result = unsafe { RegCreateKeyExW(hkcu, wide(subkey).as_ptr(), 0,
        ptr::null_mut(), 0, 0x0002, ptr::null_mut(), &mut key, ptr::null_mut()) };
    if result != 0 { return Err(format!("Hub registration failed ({result})")); }
    let mut outcome = Ok(());
    for (name, value) in values {
        let bytes = wide(value);
        let result = unsafe { RegSetValueExW(key, wide(name).as_ptr(), 0, 1,
            bytes.as_ptr().cast(), (bytes.len() * 2) as u32) };
        if result != 0 { outcome = Err(format!("Hub registration failed ({result})")); break; }
    }
    unsafe { RegCloseKey(key); }
    outcome
}

#[cfg(any(hub_bootstrap, hub_application))]
const PLUGINS: &[u8] = include_bytes!(env!("LVB_HUB_PLUGINS"));
#[cfg(any(hub_bootstrap, hub_application))]
const COMPANION: &[u8] = include_bytes!(env!("LVB_HUB_COMPANION"));
#[cfg(hub_bootstrap)]
const APPLICATION: &[u8] = include_bytes!(env!("LVB_HUB_APPLICATION"));
#[cfg(hub_bootstrap)]
const HELPER: &[u8] = include_bytes!(env!("LVB_HUB_HELPER"));

#[cfg(hub_bootstrap)]
fn install_files(directory: &Path) -> Result<(), String> {
    use std::io::Write;
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    for (name, bytes) in [("hub.exe", APPLICATION), ("helper.exe", HELPER),
        ("plugins.exe", PLUGINS), ("companion.exe", COMPANION)] {
        let path = directory.join(name);
        if path.exists() {
            if std::fs::read(&path).map_err(|e| e.to_string())? != bytes {
                return Err(format!("A different {name} is installed; no file was overwritten"));
            }
            continue;
        }
        let staged = directory.join(format!("{name}.installing"));
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true)
            .open(&staged).map_err(|e| e.to_string())?;
        file.write_all(bytes).and_then(|_| file.sync_all()).map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(staged, path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[cfg(hub_bootstrap)]
fn run() -> Result<(), String> {
    if std::env::args().any(|arg| arg == "--self-test") {
        let directory = std::env::temp_dir().join(format!("lvb-hub-{}", std::process::id()));
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        std::fs::write(directory.join("unrelated"), b"retained").map_err(|e| e.to_string())?;
        install_files(&directory)?;
        install_files(&directory)?;
        assert_eq!(std::fs::read(directory.join("hub.exe")).unwrap(), APPLICATION);
        std::fs::write(directory.join("hub.exe"), b"different").unwrap();
        assert!(install_files(&directory).is_err());
        assert_eq!(std::fs::read(directory.join("hub.exe")).unwrap(), b"different");
        assert_eq!(std::fs::read(directory.join("unrelated")).unwrap(), b"retained");
        std::fs::remove_dir_all(directory).map_err(|e| e.to_string())?;
        println!("LVB_HUB_INSTALLER_V1 retry=passed overwrite=refused sibling=retained");
        return Ok(());
    }
    if message("Install the first-party vendor hub? This installs the application and its own helper. Plug-ins are installed later from the hub. No account or activation is required.", 1) != 1 {
        return Err("Hub installation cancelled".into());
    }
    let programs = std::env::var_os("ProgramFiles").ok_or("Program Files unavailable")?;
    let directory = Path::new(&programs).join("LVBSourceVendorHub");
    if !directory.is_absolute() { return Err("Hub installation location is not absolute".into()); }
    install_files(&directory)?;
    registry("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\LVBSourceVendorHub",
        &[("DisplayName", TITLE), ("DisplayVersion", "1.0.0"),
            ("Publisher", "Linux VST Bridge"), ("InstallLocation", directory.to_str().ok_or("Hub path unavailable")?)])?;
    message("The first-party vendor hub is installed.", 0);
    Ok(())
}

#[cfg(hub_application)]
fn run() -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let directory = executable.parent().ok_or("Hub directory unavailable")?;
    for (name, bytes) in [("plugins.exe", PLUGINS), ("companion.exe", COMPANION)] {
        if std::fs::read(directory.join(name)).map_err(|e| e.to_string())? != bytes {
            return Err(format!("The selected {name} differs from this hub build"));
        }
    }
    let mut helper = Command::new(directory.join("helper.exe"))
        .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
    let output = helper.stdout.take().ok_or("Hub helper output unavailable")?;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(output).read_line(&mut line)
            .map(|_| line == "LVB_HUB_HELPER_READY_V1\n");
        let _ = sender.send(result);
    });
    if !matches!(receiver.recv_timeout(std::time::Duration::from_secs(10)), Ok(Ok(true))) {
        let _ = helper.kill(); let _ = helper.wait();
        return Err("The hub helper did not start within ten seconds".into());
    }
    let choice = message("Install the first-party unfamiliar instrument and effect, including their separate DSP companion? Yes installs them. No finishes this visit without changing plug-in files. Cancel interrupts the visit.", 3);
    if choice == 6 {
        for name in ["companion.exe", "plugins.exe"] {
            let status = Command::new(directory.join(name)).status().map_err(|e| e.to_string())?;
            if !status.success() { return Err(format!("The {name} installer did not finish successfully")); }
        }
    } else if choice != 7 { return Err("Hub visit cancelled".into()); }
    message("Close this hub window to finish your visit. Its helper will still finish background maintenance in a separate window. Complete that window too; the manager waits for the whole application to close.", 0);
    // The helper intentionally survives the main application. EOF only starts
    // its real final registry mutation; it is not the product completion signal.
    drop(helper.stdin.take());
    Ok(())
}

#[cfg(hub_helper)]
fn run() -> Result<(), String> {
    use std::io::{Read, Write};
    println!("LVB_HUB_HELPER_READY_V1");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut buffer = [0_u8; 1];
    while std::io::stdin().read(&mut buffer).map_err(|e| e.to_string())? != 0 {}
    message("The hub window is closed. This helper is still finishing background maintenance. OK writes the hub's session history and closes the helper.", 0);
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_secs().to_string();
    registry("Software\\LVBSourceVendorHub", &[("LastVisitTime", &timestamp)])
}

fn main() {
    if let Err(error) = run() { message(&error, 0x10); std::process::exit(2); }
}
