//! A setup's own Windows "Uninstall" registrations, read from its registry
//! hives. Only a command found here may be run to uninstall, and only as one
//! of two closed shapes: an uninstaller program inside the setup's drive C, or
//! Windows Installer removal of one exact product code.
use crate::*;

const HIVE_BOUND: u64 = 8 * 1024 * 1024;
const ENTRY_BOUND: usize = 512;
const ARGUMENT_BOUND: usize = 16;

/// One `...\Uninstall\<name>` key. `id` matches the supervisor's installer
/// witness key: SHA-256 of `<hive>:<key path>`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub hive: String,
    pub key: String,
    pub display_name: Option<String>,
    pub display_version: Option<String>,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    pub system_component: bool,
    pub windows_installer: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    /// A program inside the setup's drive C (never under `windows`), with the
    /// arguments the registration itself names.
    Executable { relative: String, arguments: Vec<String> },
    /// `msiexec /x` for exactly this product code.
    Msi { product_code: String },
}

pub fn command_sha256(value: &str) -> String {
    hex(&Sha256::digest(value.as_bytes()))
}

fn unescape(value: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' { out.push(c); continue; }
        match chars.next()? {
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            'x' => {
                let mut digits = String::new();
                while digits.len() < 4 && chars.peek().is_some_and(char::is_ascii_hexdigit) {
                    digits.push(chars.next()?);
                }
                out.push(char::from_u32(u32::from_str_radix(&digits, 16).ok()?)?);
            }
            other => { out.push('\\'); out.push(other); }
        }
    }
    Some(out)
}

/// A quoted string value (`"..."` or `str(2):"..."`), unescaped.
fn string_value(raw: &str) -> Option<String> {
    let raw = raw.strip_prefix("str(2):").or_else(|| raw.strip_prefix("str(7):")).unwrap_or(raw);
    unescape(raw.strip_prefix('"')?.strip_suffix('"')?)
}

/// Only `...\CurrentVersion\Uninstall\<name>` itself (machine, user or
/// 32-bit view), never a subkey of it or another key mentioning Uninstall.
fn standard_key(key: &str) -> bool {
    let low = key.to_lowercase();
    ["software\\microsoft\\windows\\currentversion\\uninstall\\",
        "software\\wow6432node\\microsoft\\windows\\currentversion\\uninstall\\"].iter()
        .any(|prefix| low.strip_prefix(prefix).is_some_and(|name| !name.is_empty() && !name.contains('\\')))
}
/// Windows Installer names a product's Uninstall key by its product code.
pub fn key_name(entry: &Entry) -> &str {
    entry.key.rsplit('\\').next().unwrap_or_default()
}

/// Parse one hive's Uninstall keys. Unreadable or unexpected lines are skipped;
/// a key's values are only those Wine wrote under it.
pub fn parse(hive: &str, bytes: &[u8]) -> Vec<Entry> {
    let text = String::from_utf8_lossy(bytes);
    let mut out: Vec<Entry> = vec![];
    let mut current: Option<Entry> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(entry) = current.take() { out.push(entry); }
            let Some(end) = rest.find(']') else { continue };
            let key = rest[..end].replace("\\\\", "\\");
            if standard_key(&key) && out.len() < ENTRY_BOUND {
                current = Some(Entry { id: hex(&Sha256::digest(format!("{hive}:{key}").as_bytes())),
                    hive: hive.into(), key, ..Default::default() });
            }
            continue;
        }
        let Some(entry) = current.as_mut() else { continue };
        let Some(rest) = line.strip_prefix('"') else { continue };
        let Some((name, value)) = rest.split_once("\"=") else { continue };
        match name {
            "DisplayName" => entry.display_name = string_value(value),
            "DisplayVersion" => entry.display_version = string_value(value),
            "InstallLocation" => entry.install_location = string_value(value),
            "UninstallString" => entry.uninstall_string = string_value(value),
            "SystemComponent" => entry.system_component = value == "dword:00000001",
            "WindowsInstaller" => entry.windows_installer = value == "dword:00000001",
            _ => {}
        }
    }
    if let Some(entry) = current { out.push(entry); }
    out
}

/// Every Uninstall key in the setup's prefix (machine and user hives).
pub fn read(environment: &Environment) -> Result<Vec<Entry>> {
    let prefix = environment.root.join("compatdata/pfx");
    let mut out = vec![];
    for hive in ["system.reg", "user.reg"] {
        let path = prefix.join(hive);
        if !path.try_exists()? { continue; }
        let mut bytes = vec![];
        file(&path)?.take(HIVE_BOUND + 1).read_to_end(&mut bytes)?;
        require(bytes.len() as u64 <= HIVE_BOUND, "uninstall_registry_bound")?;
        out.extend(parse(hive, &bytes));
    }
    Ok(out)
}

fn product_code(value: &str) -> Option<String> {
    let inner = value.strip_prefix('{')?.strip_suffix('}')?;
    let groups: Vec<_> = inner.split('-').collect();
    (groups.iter().map(|g| g.len()).collect::<Vec<_>>() == [8, 4, 4, 4, 12]
        && inner.chars().all(|c| c == '-' || c.is_ascii_hexdigit()))
        .then(|| format!("{{{}}}", inner.to_uppercase()))
}

/// `MsiExec.exe /X{code}` (or `/I{code}`, Windows Installer's maintenance
/// entry) with nothing else. The removal always runs as `/x`.
fn msi(value: &str) -> Option<String> {
    let mut words = value.split_whitespace();
    let program = words.next()?.trim_matches('"').to_lowercase().replace('/', "\\");
    let program = program.rsplit('\\').next()?;
    if !matches!(program, "msiexec" | "msiexec.exe") { return None; }
    let switch = words.next()?;
    // Character-safe: a switch may start with any character.
    let flag: String = switch.chars().take(2).collect();
    let rest = &switch[flag.len()..];
    if !matches!(flag.to_lowercase().as_str(), "/x" | "/i") { return None; }
    let code = if rest.is_empty() { words.next()? } else { rest };
    if words.next().is_some() { return None; }
    product_code(code)
}

/// Windows command-line words: whitespace splits, double quotes group.
fn words(value: &str) -> Option<Vec<String>> {
    let mut out = vec![];
    let mut current = String::new();
    let (mut quoted, mut started) = (false, false);
    for c in value.chars() {
        match c {
            '"' => { quoted = !quoted; started = true; }
            c if c.is_whitespace() && !quoted => {
                if started { out.push(std::mem::take(&mut current)); started = false; }
            }
            c => { current.push(c); started = true; }
        }
    }
    if quoted { return None; }
    if started { out.push(current); }
    Some(out)
}

/// Resolve a `C:\...` path inside drive C, case-insensitively as Wine does,
/// refusing links, `..`, the Windows directory and anything but a file.
fn resolve(drive_c: &Path, windows_path: &str) -> Option<String> {
    let rest = windows_path.strip_prefix("C:\\").or_else(|| windows_path.strip_prefix("c:\\"))?;
    let parts: Vec<_> = rest.split('\\').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() || parts.len() > 32 || parts.iter().any(|p| matches!(*p, "." | "..") || p.contains('/'))
        || parts[0].eq_ignore_ascii_case("windows")
        || !parts.last()?.to_lowercase().ends_with(".exe") {
        return None;
    }
    let mut path = drive_c.to_path_buf();
    let mut relative = vec![];
    for part in &parts {
        let exact = path.join(part);
        let found = if fs::symlink_metadata(&exact).is_ok() { part.to_string() } else {
            fs::read_dir(&path).ok()?.take(4096).filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .find(|name| name.eq_ignore_ascii_case(part))?
        };
        path = path.join(&found);
        let meta = fs::symlink_metadata(&path).ok()?;
        if meta.file_type().is_symlink() { return None; }
        relative.push(found);
    }
    fs::symlink_metadata(&path).ok().filter(|m| m.is_file())?;
    Some(relative.join("/"))
}

/// The closed command for one registration's `UninstallString`.
pub fn command(value: &str, drive_c: &Path) -> Result<Command> {
    require(!value.is_empty() && value.len() <= 2048 && !value.chars().any(|c| c.is_control()),
        "uninstall_command_unsupported")?;
    if let Some(product_code) = msi(value) { return Ok(Command::Msi { product_code }); }
    let words = words(value).ok_or("uninstall_command_unsupported")?;
    // A quoted program is one word; an unquoted one may contain spaces, as
    // Windows resolves it: the shortest prefix that names an existing program.
    for take in 1..=words.len().min(32) {
        let program = words[..take].join(" ");
        if let Some(relative) = resolve(drive_c, &program) {
            let arguments = words[take..].to_vec();
            require(arguments.len() <= ARGUMENT_BOUND && arguments.iter().all(|a| a.len() <= 512),
                "uninstall_command_unsupported")?;
            return Ok(Command::Executable { relative, arguments });
        }
        if value.trim_start().starts_with('"') { break; }
    }
    Err("uninstall_command_unsupported".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wine_hive_entries_match_supervisor_identity_and_escapes() {
        let hive = br#"WINE REGISTRY Version 2
[Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ValhallaSpaceModulator_is1] 1729030000
#time=1db1f0c8a4b2c3e
"DisplayName"="ValhallaSpaceModulator version 1.1.6v3"
"InstallLocation"="C:\\Program Files\\Valhalla DSP, LLC\\"
"UninstallString"="\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe\""
[Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{00000000-1111-2222-3333-444444444444}] 1729030000
"DisplayName"=str(2):"Caf\x00e9 Runtime"
"SystemComponent"=dword:00000001
"WindowsInstaller"=dword:00000001
"UninstallString"="MsiExec.exe /X{00000000-1111-2222-3333-444444444444}"
[Software\\Wine\\Other] 1
"UninstallString"="ignored"
"#;
        let entries = parse("system.reg", hive);
        assert_eq!(entries.len(), 2);
        assert_eq!(key_name(&entries[1]), "{00000000-1111-2222-3333-444444444444}");
        // Only the standard key itself; never a subkey or an unrelated key.
        assert!(parse("system.reg", br#"[Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\A\\Sub] 1
"UninstallString"="x.exe"
[Software\\Vendor\\Uninstall\\B] 1
"UninstallString"="x.exe"
"#).is_empty());
        let key = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ValhallaSpaceModulator_is1";
        assert_eq!(entries[0].key, key);
        assert_eq!(entries[0].id, hex(&Sha256::digest(format!("system.reg:{key}").as_bytes())));
        assert_eq!(entries[0].uninstall_string.as_deref(), Some("\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe\""));
        assert_eq!(entries[0].install_location.as_deref(), Some("C:\\Program Files\\Valhalla DSP, LLC\\"));
        assert_eq!(entries[1].display_name.as_deref(), Some("Café Runtime"));
        assert!(entries[1].system_component && entries[1].windows_installer);
    }
    #[test]
    fn only_programs_inside_drive_c_or_exact_msi_codes_are_commands() {
        let f = crate::test_fixture::Fixture::new();
        let drive_c = f.outer.join("drive_c");
        let folder = drive_c.join("Program Files/Valhalla DSP, LLC");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("unins000.exe"), b"MZ").unwrap();
        fs::create_dir_all(drive_c.join("windows/system32")).unwrap();
        fs::write(drive_c.join("windows/system32/cmd.exe"), b"MZ").unwrap();
        std::os::unix::fs::symlink(f.outer.join("elsewhere.exe"), folder.join("link.exe")).unwrap();
        fs::write(f.outer.join("elsewhere.exe"), b"MZ").unwrap();
        let program = |relative: &str, arguments: &[&str]| Command::Executable {
            relative: relative.into(), arguments: arguments.iter().map(|a| a.to_string()).collect() };
        assert_eq!(command("\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe\"", &drive_c).unwrap(),
            program("Program Files/Valhalla DSP, LLC/unins000.exe", &[]));
        // Unquoted with spaces resolves as Windows does; case follows the disk.
        assert_eq!(command("c:\\program files\\valhalla dsp, llc\\UNINS000.EXE /uninstall \"two words\"", &drive_c).unwrap(),
            program("Program Files/Valhalla DSP, LLC/unins000.exe", &["/uninstall", "two words"]));
        assert_eq!(command("MsiExec.exe /X{00000000-1111-2222-3333-44444444444a}", &drive_c).unwrap(),
            Command::Msi { product_code: "{00000000-1111-2222-3333-44444444444A}".into() });
        assert_eq!(command("C:\\Windows\\System32\\msiexec.exe /I {00000000-1111-2222-3333-444444444444}", &drive_c).unwrap(),
            Command::Msi { product_code: "{00000000-1111-2222-3333-444444444444}".into() });
        for refused in [
            "MsiExec.exe /X{00000000-1111-2222-3333-44444444444} /qn",
            "MsiExec.exe /X{not-a-code}",
            "MsiExec.exe /qn /X{00000000-1111-2222-3333-444444444444}",
            "C:\\windows\\system32\\cmd.exe /c del",
            "\"C:\\Program Files\\Valhalla DSP, LLC\\link.exe\"",
            "\"C:\\Program Files\\Valhalla DSP, LLC\\..\\..\\windows\\system32\\cmd.exe\"",
            "\"C:\\Program Files\\Valhalla DSP, LLC\\missing.exe\"",
            "D:\\other\\unins000.exe",
            "/home/user/uninstall.sh",
            "\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe",
            "MsiExec.exe \u{e9}\u{e9}{00000000-1111-2222-3333-444444444444}",
            "MsiExec.exe /\u{e9}",
        ] {
            assert!(command(refused, &drive_c).is_err(), "{refused}");
        }
    }
}
