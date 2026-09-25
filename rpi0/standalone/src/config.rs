use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

pub const BOX64_SOURCE_COMMIT: &str = "2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a";
pub const WINE_SOURCE_COMMIT: &str = "db11d0fe6a169c457e23d007e20404643d067aa8";

#[derive(Clone, Debug)]
pub struct PinnedFile {
    pub path: PathBuf,
    pub sha256: [u8; 32],
}

impl PinnedFile {
    pub fn verify(&self, label: &str) -> io::Result<()> {
        let metadata = fs::symlink_metadata(&self.path)?;
        if !self.path.is_absolute() || metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(invalid(format!("{label} is not an exact regular file")));
        }
        let mut file = fs::File::open(&self.path)?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        if hash.finalize().as_slice() != self.sha256 {
            return Err(invalid(format!("{label} SHA-256 differs")));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub box64: PinnedFile,
    pub wine: PinnedFile,
    pub linux_probe: PinnedFile,
    pub windows_probe: PinnedFile,
    pub windows_host: PinnedFile,
    pub plugin: PinnedFile,
    pub prefix: PathBuf,
    pub box64_rc: PinnedFile,
    pub source_manifest_sha256: [u8; 32],
    pub bridge_frames: u32,
    pub jack_client: String,
    pub evidence_path: PathBuf,
    pub windows_evidence_path: PathBuf,
}

impl Config {
    pub fn load(path: &Path) -> io::Result<Self> {
        let metadata = fs::symlink_metadata(path)?;
        if !path.is_absolute()
            || metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > 65536
        {
            return Err(invalid(
                "configuration must be an absolute, regular, bounded file",
            ));
        }
        let text = fs::read_to_string(path)?;
        let mut values = BTreeMap::new();
        for (line_number, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| invalid(format!("configuration line {}", line_number + 1)))?;
            let key = key.trim();
            let value = value.trim();
            if key.is_empty()
                || value.is_empty()
                || values.insert(key.to_owned(), value.to_owned()).is_some()
            {
                return Err(invalid(format!(
                    "duplicate/empty configuration line {}",
                    line_number + 1
                )));
            }
        }
        const KEYS: [&str; 24] = [
            "box64_source_commit",
            "box64_path",
            "box64_sha256",
            "wine_source_commit",
            "wine_path",
            "wine_sha256",
            "linux_probe_path",
            "linux_probe_sha256",
            "windows_probe_path",
            "windows_probe_sha256",
            "windows_host_path",
            "windows_host_sha256",
            "plugin_path",
            "plugin_sha256",
            "prefix_path",
            "box64_rc_path",
            "box64_rc_sha256",
            "source_manifest_sha256",
            "bridge_frames",
            "jack_client",
            "evidence_path",
            "windows_evidence_path",
            "protocol_minor",
            "sample_rate",
        ];
        if values.len() != KEYS.len() || KEYS.iter().any(|key| !values.contains_key(*key)) {
            return Err(invalid("configuration key set differs"));
        }
        if values["box64_source_commit"] != BOX64_SOURCE_COMMIT
            || values["wine_source_commit"] != WINE_SOURCE_COMMIT
            || values["protocol_minor"] != "12"
            || values["sample_rate"] != "48000"
        {
            return Err(invalid("translation/protocol pin differs"));
        }
        let pinned = |path_key: &str, hash_key: &str| -> io::Result<PinnedFile> {
            Ok(PinnedFile {
                path: PathBuf::from(&values[path_key]),
                sha256: hex32(&values[hash_key])?,
            })
        };
        let bridge_frames = values["bridge_frames"]
            .parse::<u32>()
            .map_err(|_| invalid("bridge frames syntax"))?;
        if !matches!(bridge_frames, 512 | 1024 | 2048) {
            return Err(invalid("bridge frames outside RPI0 set"));
        }
        let prefix = PathBuf::from(&values["prefix_path"]);
        let prefix_meta = fs::symlink_metadata(&prefix)?;
        if !prefix.is_absolute() || prefix_meta.file_type().is_symlink() || !prefix_meta.is_dir() {
            return Err(invalid("prefix is not an exact directory"));
        }
        let jack_client = values["jack_client"].clone();
        if jack_client != "lvb-arm-standalone" {
            return Err(invalid("JACK client name differs"));
        }
        let evidence_path = PathBuf::from(&values["evidence_path"]);
        let evidence_parent = evidence_path
            .parent()
            .ok_or_else(|| invalid("evidence path has no parent"))?;
        let parent_meta = fs::symlink_metadata(evidence_parent)?;
        if !evidence_path.is_absolute()
            || parent_meta.file_type().is_symlink()
            || !parent_meta.is_dir()
        {
            return Err(invalid("evidence path is not in an exact directory"));
        }
        let windows_evidence_path = PathBuf::from(&values["windows_evidence_path"]);
        let windows_parent = windows_evidence_path
            .parent()
            .ok_or_else(|| invalid("Windows evidence path has no parent"))?;
        let windows_parent_meta = fs::symlink_metadata(windows_parent)?;
        if !windows_evidence_path.is_absolute()
            || windows_parent_meta.file_type().is_symlink()
            || !windows_parent_meta.is_dir()
            || windows_evidence_path == evidence_path
        {
            return Err(invalid(
                "Windows evidence path is not a distinct exact path",
            ));
        }
        let config = Self {
            box64: pinned("box64_path", "box64_sha256")?,
            wine: pinned("wine_path", "wine_sha256")?,
            linux_probe: pinned("linux_probe_path", "linux_probe_sha256")?,
            windows_probe: pinned("windows_probe_path", "windows_probe_sha256")?,
            windows_host: pinned("windows_host_path", "windows_host_sha256")?,
            plugin: pinned("plugin_path", "plugin_sha256")?,
            prefix,
            box64_rc: pinned("box64_rc_path", "box64_rc_sha256")?,
            source_manifest_sha256: hex32(&values["source_manifest_sha256"])?,
            bridge_frames,
            jack_client,
            evidence_path,
            windows_evidence_path,
        };
        config.verify_files()?;
        Ok(config)
    }

    pub fn verify_files(&self) -> io::Result<()> {
        self.box64.verify("Box64")?;
        self.wine.verify("Wine")?;
        self.linux_probe.verify("x86-64 Linux probe")?;
        self.windows_probe.verify("x86-64 Windows probe")?;
        self.windows_host.verify("Windows VST3 host")?;
        self.plugin.verify("source-owned Windows instrument")?;
        self.box64_rc.verify("Box64 configuration")
    }
}

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }
    output
}

pub fn hex32(value: &str) -> io::Result<[u8; 32]> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid("SHA-256 syntax"));
    }
    let mut output = [0; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| invalid("SHA-256 syntax"))?;
    }
    Ok(output)
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_runtime_commits_and_hash_syntax_are_closed() {
        assert_eq!(BOX64_SOURCE_COMMIT.len(), 40);
        assert_eq!(WINE_SOURCE_COMMIT.len(), 40);
        assert_eq!(hex32(&"ab".repeat(32)).unwrap(), [0xab; 32]);
        assert!(hex32("00").is_err());
    }
}
