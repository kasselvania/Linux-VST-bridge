fn main() {
    // Build-side provenance is inspected in the ELF. Keep the first-run
    // executable's no-argument contract; never open a UI to inspect a build.
    std::hint::black_box(option_env!("LVB_INSTALLER_PACKAGE_INFO"));
    // Before Makepad or the package worker starts any threads.
    lvb_client::staging::prepare_process();
    if std::env::args_os().len() != 1 {
        eprintln!("This installer accepts no command, path or operation arguments.");
        std::process::exit(64);
    }
    lvb_window::ui::run_installer();
}
