//! The existing signed ELF intake, retained entirely on the installer worker.
use super::{Intake, Package};
use linux_vst_bridge::portable_package::{self, Bundle, Trust};
use std::{path::PathBuf, process::Command};

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
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    // Bundle keeps the exact opened file and identity through stage; it is not
    // reopened from a filename after the musician approves installation.
    let bundle = Trust::compiled()
        .and_then(|trust| Bundle::open(&executable, trust))
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
        // Revalidate the exact signed staged frontend after any time the user
        // spent on the completion screen. Never substitute the selected route.
        let verified = portable_package::input_executable(
            frontend,
            &self.home,
            "linux-audio-compatibility-manager",
        )
        .map_err(|error| error.to_string())?;
        if verified.as_ref() != Some(frontend) {
            return Err(
                "The staged Setup application changed. Reopen the installer to verify its files."
                    .into(),
            );
        }
        Command::new(frontend)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
