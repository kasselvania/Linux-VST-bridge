use linux_vst_bridge::{owner, Result};
use lvb_model::contract::{self, Request, Transport};
use std::{io::Write, os::fd::{AsFd, FromRawFd, IntoRawFd}, path::Path};

fn main() -> Result<()> {
    unsafe { libc::umask(0o077); }
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "native-access-callback") {
        unsafe {
            let limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            linux_vst_bridge::require(libc::setrlimit(libc::RLIMIT_CORE, &limit) == 0, "callback_privacy")?;
            #[cfg(target_os = "linux")]
            linux_vst_bridge::require(libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) == 0, "callback_privacy")?;
        }
    }
    if let Some(request) = Request::from_command(&args, std::io::stdin()) {
        let request = request?;
        owner::require_current_frontend_caller()?;
        let input = if matches!(request, Request::ImportInstaller { .. }) {
            Some(unsafe { std::fs::File::from_raw_fd(std::io::stdin().as_fd().try_clone_to_owned()?.into_raw_fd()) })
        } else { None };
        let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
        let bytes = contract::Service::installed(Path::new(&home)).exchange(&request, input)
            .map_err(|failure| match failure {
                contract::Failure::FrontendSuperseded => contract::FRONTEND_SUPERSEDED.to_owned(),
                contract::Failure::Unavailable => "Bridge service is unavailable".into(),
                contract::Failure::Error(reason) => reason,
            })?;
        std::io::stdout().write_all(&bytes)?;
        return Ok(());
    }
    match args.first().map(String::as_str) {
        Some("serve") if args.len() == 1 => owner::serve(linux_vst_bridge::Manager::installed()?),
        Some("capacity") if args.len() == 1 => owner::capacity_read(&linux_vst_bridge::Manager::installed()?),
        Some("owned-audio-scheduling") if args.len() == 1 => owner::run_audio_scheduling(),
        Some(command) if command.starts_with("package-") => owner::package_command(&args),
        _ => Err("Usage: linux-vst-bridge serve | capacity | operator ... | import-installer NAME | native-access-callback URI | package-...".into()),
    }
}
