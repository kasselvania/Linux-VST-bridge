//! Closed K8I1 authority for one Kontakt 8 Player package.
//!
//! Package paths and deployment data are not operator input.  The installed
//! immutable adapter contains the independently verified source-owned tools;
//! the runtime repeats its exact archive validation before arming anything in
//! the prefix.

use crate::{catalogue::Software, renderer_application::Application, *};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;

pub const PRODUCT: &str = "Kontakt 8 Player";
pub const VERSION: &str = "8.13.1";
pub const SETUP_BASENAME: &str = "Kontakt 8 Setup PC.exe";
pub const SETUP_SHA256: &str = "5f7f26389337f3cd282319589549ade23f860023e6cb9a99f0f45baf67f7215d";
pub const SETUP_SIZE: u64 = 1_188_804_208;
pub const MSI_BASENAME: &str = "Kontakt 8 Setup PC.msi";
pub const PRISTINE_MSI_SHA256: &str =
    "0f1caaac78c5ae718d849c3a2f5b9c7f1161b4326edf3f21dd7af2f01d95c929";
pub const PRISTINE_MSI_SIZE: u64 = 4_104_192;
pub const WINE_BASE: &str = "dc26e61847081a1b5cb0733dc30feba6ee575482";
pub const WINE_PATCHES: [&str; 2] = [
    "24bbf46c6f055077df428341595873fe23475a60",
    "e0130972d5ffe578a4a25d75f4c1d0229c880a8c",
];

fn software_sha256(software: &Software) -> Result<String> {
    Ok(hex(&Sha256::digest(serde_json::to_vec(
        &serde_json::to_value(software)?,
    )?)))
}

/// No adapter is a valid legacy software generation and produces no K8I1
/// authority.  A present adapter must be a same-generation immutable file.
pub fn session_authority(application: &Application, software: &Software) -> Result<Option<Value>> {
    let Some(adapter) = software.kontakt8_adapter.as_ref() else {
        require(
            software.kontakt8_runtime.is_none(),
            "kontakt8_software_pair",
        )?;
        return Ok(None);
    };
    let runtime = software
        .kontakt8_runtime
        .as_ref()
        .ok_or("kontakt8_software_pair")?;
    adapter.verify()?;
    runtime.verify()?;
    require(
        adapter.path.parent() == software.manager.path.parent()
            && runtime.path.parent() == software.manager.path.parent()
            && adapter
                .path
                .file_name()
                .is_some_and(|n| n == "kontakt8-adapter.zip")
            && runtime.path.file_name().is_some_and(|n| n == "kontakt8.py")
            && adapter.path.canonicalize()? == adapter.path,
        "kontakt8_adapter_software_binding",
    )?;
    require(
        runtime.sha256 == hex(&Sha256::digest(include_bytes!("../runtime/kontakt8.py"))),
        "kontakt8_runtime_identity",
    )?;
    let md = file(&adapter.path)?.metadata()?;
    require(
        md.nlink() == 1 && md.len() > 0 && md.len() <= 64 * 1024 * 1024,
        "kontakt8_adapter_extent",
    )?;
    let prefix = application.environment.root.join("compatdata/pfx");
    let pmd = fs::symlink_metadata(&prefix)?;
    require(
        pmd.is_dir() && prefix.canonicalize()? == prefix,
        "kontakt8_prefix_identity",
    )?;
    Ok(Some(json!({
        "schema": 1,
        "authority": "exact_kontakt8_8_13_1_msi_diversion_and_verified_deployment",
        "application": application.identity()?,
        "software_sha256": software_sha256(software)?,
        "environment": application.environment.id,
        "prefix": {
            "path_sha256": hex(&Sha256::digest(prefix.as_os_str().as_encoded_bytes())),
            "device": pmd.dev(),
            "inode": pmd.ino()
        },
        "adapter": adapter,
        "product": {"name": PRODUCT, "version": VERSION},
        "setup": {"basename": SETUP_BASENAME, "sha256": SETUP_SHA256, "size": SETUP_SIZE},
        "pristine_msi": {"basename": MSI_BASENAME, "sha256": PRISTINE_MSI_SHA256,
            "size": PRISTINE_MSI_SIZE},
        "intercepts": ["MsiInstallProductA", "MsiInstallProductW"],
        "request_limit": 1,
        "timeout_seconds": 7200,
        "wine": {"base": WINE_BASE, "patches": WINE_PATCHES},
        "selected_features": ["standalone", "vst3", "native_instruments_common"],
        "omitted_features": ["aax"],
        "licensing_state": "never_manufactured"
    })))
}

/// Extend the already-bound application spec without giving the operator a
/// separate product-install action or any caller-selected hook parameter.
pub fn bind(spec: &mut Value, application: &Application, software: &Software) -> Result<()> {
    if let Some(authority) = session_authority(application, software)? {
        spec["kontakt8_session"] = authority;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_fixture_constants_are_closed() {
        assert_eq!(PRODUCT, "Kontakt 8 Player");
        assert_eq!(VERSION, "8.13.1");
        assert!(valid_hex(SETUP_SHA256, 64));
        assert!(valid_hex(PRISTINE_MSI_SHA256, 64));
        assert_eq!(WINE_PATCHES.len(), 2);
        assert_ne!(WINE_PATCHES[0], WINE_PATCHES[1]);
    }
}
