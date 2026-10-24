fn main() {
    // Before Makepad or the package worker starts any threads.
    lvb_client::staging::prepare_process();
    if std::env::args_os().len() != 1 {
        eprintln!("This installer accepts no command, path or operation arguments.");
        std::process::exit(64);
    }
    lvb_window::ui::run_installer();
}
