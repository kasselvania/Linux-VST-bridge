use crate::config::{hex, Config};
use std::{ffi::OsString, io, path::Path};

pub const PIGMENTS_CLASS: [u8; 16] = [
    0x41, 0x72, 0x74, 0x75, 0x41, 0x56, 0x49, 0x53, 0x4b, 0x61, 0x74, 0x31, 0x50, 0x72, 0x6f, 0x63,
];

/// Exact Pigments 7.0.1.6772 census order. The sole stereo auxiliary input is
/// active and receives the standalone host's preallocated silence buffers.
pub fn pigments_bus_contract() -> [u8; 132] {
    let mut bytes = [0u8; 132];
    bytes[..4].copy_from_slice(&4u32.to_le_bytes());
    let mut write = |ordinal: usize, fields: [u32; 6], arrangement: u64| {
        let start = 4 + ordinal * 32;
        for (index, value) in fields.into_iter().enumerate() {
            bytes[start + index * 4..start + index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[start + 24..start + 32].copy_from_slice(&arrangement.to_le_bytes());
    };
    // media, direction, index, channels, type, active, arrangement
    write(0, [0, 0, 0, 2, 1, 1], 3); // auxiliary Sidechain input
    write(1, [0, 1, 0, 2, 0, 1], 3); // main Stereo Out
    write(2, [1, 0, 0, 16, 0, 1], 0); // Midi In
    // Pigments reports zero raw channels here. The pinned
    // `reported_zero_event_channels_unspecified` policy translates that
    // sentinel to the 16 effective VST event channels on both sides of the
    // cross-process contract.
    write(3, [1, 1, 0, 16, 0, 1], 0); // effective Midi Out
    bytes
}

pub fn windows_path(prefix: &Path, path: &Path) -> io::Result<String> {
    let root = prefix.join("drive_c");
    let relative = path
        .strip_prefix(&root)
        .map_err(|_| invalid("artifact is outside selected Wine C: drive"))?;
    let value = relative
        .to_str()
        .ok_or_else(|| invalid("Windows path encoding"))?;
    if value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(invalid("Windows path shape"));
    }
    Ok(format!(r"C:\{}", value.replace('/', r"\")))
}

pub fn host_arguments(config: &Config, session: &str) -> io::Result<Vec<OsString>> {
    validate_session(session)?;
    let prefix = config.prefix();
    let module = windows_path(&prefix, &config.plugin.path)?;
    let host = windows_path(&prefix, &config.windows_host.path)?;
    let hash = |bytes: &[u8]| OsString::from(hex(bytes));
    // Construct every item as one argv element. These strings never pass
    // through shell expansion; literal "$session" is therefore impossible.
    Ok(vec![
        host.into(),
        "--session".into(),
        session.into(),
        "--scanner-sha256".into(),
        hash(&config.windows_host.sha256),
        "--implementation-source-manifest-sha256".into(),
        hash(&config.source_manifest_sha256),
        "--module".into(),
        module.into(),
        "--module-sha256".into(),
        hash(&config.plugin.sha256),
        "--bundle-manifest-sha256".into(),
        hash(&config.plugin.sha256),
        "--ready".into(),
        format!(r"C:\bridge\sessions\{session}\{session}.ready").into(),
        "--gate".into(),
        format!(r"C:\bridge\sessions\{session}\{session}.gate").into(),
        "--max-classes".into(),
        "256".into(),
        "--stdout-cap".into(),
        "1048576".into(),
        "--mode".into(),
        "ap9-commercial".into(),
        "--component-case".into(),
        format!("class:{}", hex(&PIGMENTS_CLASS)).into(),
    ])
}

pub fn handshake(config: &Config, session: &str) -> io::Result<String> {
    validate_session(session)?;
    Ok(format!(
        "schema=linux-vst-bridge-wf0-handshake/v1\nsession={session}\nscanner_sha256={}\nmodule_sha256={}\nbundle_manifest_sha256={}\nimplementation_source_manifest_sha256={}\nmode=ap9-commercial\ncomponent_case=class:{}\nrun_ordinal=1\n",
        hex(&config.windows_host.sha256),
        hex(&config.plugin.sha256),
        hex(&config.plugin.sha256),
        hex(&config.source_manifest_sha256),
        hex(&PIGMENTS_CLASS)
    ))
}

fn validate_session(session: &str) -> io::Result<()> {
    if session.len() != 32 || !session.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Err(invalid("session identity"))
    } else {
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PinnedFile;
    use std::path::PathBuf;

    fn config() -> Config {
        let file = |path: &str| PinnedFile {
            path: PathBuf::from(path),
            sha256: [0; 32],
        };
        Config {
            box64: file("/runtime/box64"),
            adapter: file("/runtime/adapter"),
            emulator_manifest: file("/runtime/emulator.json"),
            graphics_manifest: file("/runtime/graphics.json"),
            slr_entry: file("/runtime/slr"),
            proton: file("/runtime/proton"),
            windows_host: file("/environment/compatdata/pfx/drive_c/bridge/host.exe"),
            plugin: file(
                "/environment/compatdata/pfx/drive_c/Program Files/Common Files/VST3/Pigments.vst3",
            ),
            environment_root: "/environment".into(),
            runtime_variable_dir: "/environment/runtime-var-pyhome-exact".into(),
            display: ":1".into(),
            xauthority: file("/home/user/.Xauthority"),
            source_manifest_sha256: [0; 32],
            bridge_frames: 2048,
            jack_client: "lvb-arm-pigments".into(),
            evidence_directory: "/environment/evidence".into(),
        }
    }

    #[test]
    fn pigments_identity_is_exact() {
        assert_eq!(hex(&PIGMENTS_CLASS), "41727475415649534b61743150726f63");
    }

    #[test]
    fn bus_contract_preserves_complete_observed_census() {
        let wire = pigments_bus_contract();
        assert_eq!(u32::from_le_bytes(wire[..4].try_into().unwrap()), 4);
        let row = |ordinal: usize| {
            let start = 4 + ordinal * 32;
            (
                (0..6)
                    .map(|field| {
                        u32::from_le_bytes(
                            wire[start + field * 4..start + field * 4 + 4]
                                .try_into()
                                .unwrap(),
                        )
                    })
                    .collect::<Vec<_>>(),
                u64::from_le_bytes(wire[start + 24..start + 32].try_into().unwrap()),
            )
        };
        assert_eq!(row(0), (vec![0, 0, 0, 2, 1, 1], 3));
        assert_eq!(row(1), (vec![0, 1, 0, 2, 0, 1], 3));
        assert_eq!(row(2), (vec![1, 0, 0, 16, 0, 1], 0));
        assert_eq!(row(3), (vec![1, 1, 0, 16, 0, 1], 0));
    }

    #[test]
    fn windows_session_paths_are_concrete_and_not_shell_templates() {
        let sid = "0123456789abcdef0123456789abcdef";
        let arguments = host_arguments(&config(), sid).unwrap();
        let ready_index = arguments
            .iter()
            .position(|value| value == "--ready")
            .unwrap()
            + 1;
        let gate_index = arguments
            .iter()
            .position(|value| value == "--gate")
            .unwrap()
            + 1;
        let ready = arguments[ready_index].to_str().unwrap();
        let gate = arguments[gate_index].to_str().unwrap();
        assert!(!ready.contains('$'));
        assert!(!gate.contains('$'));
        assert_eq!(
            ready,
            r"C:\bridge\sessions\0123456789abcdef0123456789abcdef\0123456789abcdef0123456789abcdef.ready"
        );
        assert_eq!(
            gate,
            r"C:\bridge\sessions\0123456789abcdef0123456789abcdef\0123456789abcdef0123456789abcdef.gate"
        );
    }
}
