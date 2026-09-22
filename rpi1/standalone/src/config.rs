use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

pub const RUNTIME_ID: &str = "proton-11.0-2c-25118279-slr4-4.0.20260805.254769";
pub const BOX64_SOURCE_COMMIT: &str = "2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a";
pub const BOX64_SHA256: &str = "79cdd30e5480f5dfb8cd717af99d0e5a89eb55b31701de6fde7896a6a0e79ab6";
pub const ADAPTER_SHA256: &str = "99e1dc907c53fba755c1839922654753c09435fa3c474ded00f8dc2d499b7828";
pub const EMULATOR_MANIFEST_SHA256: &str =
    "9a30bd4f6cd6ec7e888a90af81f26aa9c3967abc9410e3535dea7a804f18e7c1";
pub const GRAPHICS_MANIFEST_SHA256: &str =
    "a076eea9d36c398b94b9a9b1f093d31811659a731169a42bf9ad6bae54bf871d";
pub const SLR_ENTRY_SHA256: &str =
    "caa39b5cde8ea955288b574b49b3416fd16e7be3f53a17249d673f5ecfdce2c1";
pub const PROTON_SHA256: &str = "787504a79bacf6b303984a9a846cf47463f36599248e8306b26d5faf78267aad";
pub const PIGMENTS_SHA256: &str =
    "bdc91ebef8e5b486c8f998f1eef6a99626dd5a0b46d986263eeb1980b96a3c07";

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
        let opened = file.metadata()?;
        if (opened.dev(), opened.ino()) != (metadata.dev(), metadata.ino()) {
            return Err(invalid(format!("{label} changed during open")));
        }
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

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Clone, Debug)]
pub struct Config {
    pub box64: PinnedFile,
    pub adapter: PinnedFile,
    pub emulator_manifest: PinnedFile,
    pub graphics_manifest: PinnedFile,
    pub slr_entry: PinnedFile,
    pub proton: PinnedFile,
    pub windows_host: PinnedFile,
    pub plugin: PinnedFile,
    pub environment_root: PathBuf,
    pub runtime_variable_dir: PathBuf,
    pub display: String,
    pub xauthority: PinnedFile,
    pub source_manifest_sha256: [u8; 32],
    pub bridge_frames: u32,
    pub jack_client: String,
    pub evidence_directory: PathBuf,
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
        const KEYS: [&str; 40] = [
            "runtime_id",
            "box64_source_commit",
            "box64_path",
            "box64_sha256",
            "emulator_adapter_path",
            "emulator_adapter_sha256",
            "emulator_manifest_path",
            "emulator_manifest_sha256",
            "graphics_manifest_path",
            "graphics_manifest_sha256",
            "slr_entry_path",
            "slr_entry_sha256",
            "proton_path",
            "proton_sha256",
            "windows_host_path",
            "windows_host_sha256",
            "plugin_path",
            "plugin_sha256",
            "environment_root",
            "runtime_variable_dir",
            "display",
            "xauthority_path",
            "xauthority_sha256",
            "source_manifest_sha256",
            "bridge_frames",
            "jack_client",
            "evidence_directory",
            "protocol_minor",
            "sample_rate",
            "pythonhome",
            "machine_architecture",
            "accessibility_policy",
            "event_output_policy",
            "editor_lifetime_policy",
            "vendor_retirement_policy",
            "pigments_class_id",
            "pigments_version",
            "pigments_parameter_count",
            "pigments_precision",
            "environment_family",
        ];
        if values.len() != KEYS.len() || KEYS.iter().any(|key| !values.contains_key(*key)) {
            return Err(invalid("configuration key set differs"));
        }
        for (key, expected) in [
            ("runtime_id", RUNTIME_ID),
            ("box64_source_commit", BOX64_SOURCE_COMMIT),
            ("box64_sha256", BOX64_SHA256),
            ("emulator_adapter_sha256", ADAPTER_SHA256),
            ("emulator_manifest_sha256", EMULATOR_MANIFEST_SHA256),
            ("graphics_manifest_sha256", GRAPHICS_MANIFEST_SHA256),
            ("slr_entry_sha256", SLR_ENTRY_SHA256),
            ("proton_sha256", PROTON_SHA256),
            ("plugin_sha256", PIGMENTS_SHA256),
            ("protocol_minor", "12"),
            ("sample_rate", "48000"),
            ("pythonhome", "/usr"),
            ("machine_architecture", "aarch64-linux-gnu"),
            ("accessibility_policy", "uiautomationcore="),
            (
                "event_output_policy",
                "reported_zero_event_channels_unspecified",
            ),
            (
                "editor_lifetime_policy",
                "retain_editor_view_until_instance_retirement",
            ),
            (
                "vendor_retirement_policy",
                "process_scoped_vendor_retirement",
            ),
            ("pigments_class_id", "41727475415649534B61743150726F63"),
            ("pigments_version", "7.0.1.6772"),
            ("pigments_parameter_count", "4446"),
            ("pigments_precision", "float32_only"),
            ("environment_family", "arturia_persistent_v1:1"),
        ] {
            if values[key] != expected {
                return Err(invalid(format!("{key} pin differs")));
            }
        }
        let pinned = |path_key: &str, hash_key: &str| -> io::Result<PinnedFile> {
            Ok(PinnedFile {
                path: PathBuf::from(&values[path_key]),
                sha256: hex32(&values[hash_key])?,
            })
        };
        let environment_root = exact_directory(&values["environment_root"], "environment root")?;
        let runtime_variable_dir = exact_directory(
            &values["runtime_variable_dir"],
            "runtime variable directory",
        )?;
        if runtime_variable_dir.parent() != Some(environment_root.as_path())
            || !runtime_variable_dir
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("runtime-var-pyhome-"))
            || !runtime_variable_dir.join(".ref").is_file()
        {
            return Err(invalid("runtime variable directory binding differs"));
        }
        let bridge_frames = values["bridge_frames"]
            .parse::<u32>()
            .map_err(|_| invalid("bridge frames syntax"))?;
        if !matches!(bridge_frames, 512 | 1024 | 2048) {
            return Err(invalid("bridge frames outside RPI1 set"));
        }
        if values["jack_client"] != "lvb-arm-pigments" {
            return Err(invalid("JACK client name differs"));
        }
        if values["display"].is_empty() || values["display"].bytes().any(|b| b.is_ascii_control()) {
            return Err(invalid("display identity"));
        }
        let evidence_directory =
            exact_directory(&values["evidence_directory"], "evidence directory")?;
        let config = Self {
            box64: pinned("box64_path", "box64_sha256")?,
            adapter: pinned("emulator_adapter_path", "emulator_adapter_sha256")?,
            emulator_manifest: pinned("emulator_manifest_path", "emulator_manifest_sha256")?,
            graphics_manifest: pinned("graphics_manifest_path", "graphics_manifest_sha256")?,
            slr_entry: pinned("slr_entry_path", "slr_entry_sha256")?,
            proton: pinned("proton_path", "proton_sha256")?,
            windows_host: pinned("windows_host_path", "windows_host_sha256")?,
            plugin: pinned("plugin_path", "plugin_sha256")?,
            environment_root,
            runtime_variable_dir,
            display: values["display"].clone(),
            xauthority: pinned("xauthority_path", "xauthority_sha256")?,
            source_manifest_sha256: hex32(&values["source_manifest_sha256"])?,
            bridge_frames,
            jack_client: values["jack_client"].clone(),
            evidence_directory,
        };
        config.verify_files()?;
        config.verify_layout()?;
        Ok(config)
    }

    pub fn prefix(&self) -> PathBuf {
        self.environment_root.join("compatdata/pfx")
    }

    pub fn verify_files(&self) -> io::Result<()> {
        self.box64.verify("Box64")?;
        self.adapter.verify("Box64 emulator adapter")?;
        self.emulator_manifest.verify("emulator manifest")?;
        self.graphics_manifest
            .verify("graphics provider manifest")?;
        self.slr_entry.verify("SLR4 entry point")?;
        self.proton.verify("Proton")?;
        self.windows_host.verify("Windows VST3 host")?;
        self.plugin.verify("Pigments module")?;
        self.xauthority.verify("X authority")
    }

    fn verify_layout(&self) -> io::Result<()> {
        for (file, label) in [
            (&self.box64, "Box64"),
            (&self.adapter, "adapter"),
            (&self.emulator_manifest, "emulator manifest"),
            (&self.graphics_manifest, "graphics manifest"),
            (&self.slr_entry, "SLR4 entry"),
            (&self.proton, "Proton"),
            (&self.windows_host, "Windows host"),
            (&self.plugin, "Pigments"),
        ] {
            if !file.path.starts_with(&self.environment_root) {
                return Err(invalid(format!("{label} is outside selected environment")));
            }
        }
        let drive_c = self.prefix().join("drive_c");
        if !self.windows_host.path.starts_with(&drive_c) || !self.plugin.path.starts_with(&drive_c)
        {
            return Err(invalid("Windows artifact is outside selected C: drive"));
        }
        Ok(())
    }
}

fn exact_directory(value: &str, label: &str) -> io::Result<PathBuf> {
    let path = PathBuf::from(value);
    let metadata = fs::symlink_metadata(&path)?;
    if !path.is_absolute() || metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(invalid(format!("{label} is not an exact directory")));
    }
    Ok(path)
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
    fn product_and_translation_pins_are_closed() {
        assert_eq!(BOX64_SOURCE_COMMIT.len(), 40);
        for value in [
            BOX64_SHA256,
            ADAPTER_SHA256,
            EMULATOR_MANIFEST_SHA256,
            GRAPHICS_MANIFEST_SHA256,
            SLR_ENTRY_SHA256,
            PROTON_SHA256,
            PIGMENTS_SHA256,
        ] {
            assert!(hex32(value).is_ok());
        }
        assert_eq!(
            RUNTIME_ID,
            "proton-11.0-2c-25118279-slr4-4.0.20260805.254769"
        );
    }
}
