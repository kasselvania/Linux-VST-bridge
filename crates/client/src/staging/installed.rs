//! The existing signed ELF intake, retained entirely on the installer worker.
use super::{Intake, Package};
use linux_vst_bridge::portable_package::{self, Bundle, Trust};
use std::path::PathBuf;

pub(super) struct Installed {
    bundle: Bundle,
    home: PathBuf,
    frontend: Option<PathBuf>,
}

pub(super) fn inspect() -> Result<Installed, String> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        "Your home directory is unavailable. Open this installer from your desktop session."
            .to_owned()
    })?;
    // /proc/self/exe opens the running kernel object even if its old name was
    // replaced. Bundle retains that object throughout staging.
    let bundle = Trust::compiled()
        .and_then(Bundle::open_running)
        .map_err(|error| error.to_string())?;
    Ok(Installed {
        bundle,
        home,
        frontend: None,
    })
}

impl Intake for Installed {
    fn package(&self) -> Package {
        Package {
            version: self.bundle.release().version.clone(),
            internal: self.bundle.key_class() == "internal_test",
        }
    }

    fn stage(&mut self) -> Result<(), String> {
        self.frontend = Some(
            self.bundle
                .stage(&self.home)
                .map_err(|error| error.to_string())?,
        );
        Ok(())
    }

    fn expose_setup(&mut self) -> Result<(), String> {
        let frontend = self
            .frontend
            .as_ref()
            .ok_or("Application files have not been staged.")?;
        portable_package::expose_setup(frontend, &self.home).map_err(|error| error.to_string())
    }

    fn open_setup(&mut self) -> Result<(), String> {
        let frontend = self
            .frontend
            .as_ref()
            .ok_or("Application files have not been staged.")?;
        portable_package::retained_input_executable(
            frontend,
            &self.home,
            "linux-audio-compatibility-manager",
        )
        .and_then(|verified| verified.spawn())
        .map_err(|error| error.to_string())
    }
}
