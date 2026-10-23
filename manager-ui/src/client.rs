//! egui worker delivery and wakeup; product requests live in `lvb-client`.
pub use lvb_client::{Query, Reply};
#[cfg(test)]
pub use lvb_client::SupportReport;
use lvb_client::{Client, native::RfdSelection};
use std::{path::PathBuf, sync::{mpsc::Sender, Arc, OnceLock}};

static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
fn call(query: Query) -> Result<Reply, String> {
    let client = CLIENT.get_or_init(|| {
        let home = std::env::var_os("HOME").ok_or("Home directory unavailable")?;
        let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
        Client::new(PathBuf::from(home), executable)
    }).as_ref().map_err(Clone::clone)?;
    client.call(query, &RfdSelection)
}

pub type WindowWake = Arc<dyn Fn() + Send + Sync>;

fn deliver(reply: Reply, sender: &Sender<Reply>, ctx: &eframe::egui::Context,
    window_wake: Option<&WindowWake>) {
    if sender.send(reply).is_ok() {
        ctx.request_repaint();
        // A completed request must wake the actual native window as well as
        // egui's pass-based repaint queue. Input must not be needed to consume
        // a reply that is already waiting in the channel.
        if let Some(wake) = window_wake { wake(); }
    }
}

pub fn send(query: Query, sender: Sender<Reply>, ctx: eframe::egui::Context,
    window_wake: Option<WindowWake>) {
    std::thread::spawn(move || {
        let result = call(query).unwrap_or_else(Reply::Error);
        deliver(result, &sender, &ctx, window_wake.as_ref());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_wakes_window_when_egui_repaint_is_already_queued() {
        use std::sync::{atomic::{AtomicUsize, Ordering}, mpsc, Mutex};
        let ctx = eframe::egui::Context::default();
        let repaint_count = Arc::new(AtomicUsize::new(0));
        let observed = repaint_count.clone();
        ctx.set_request_repaint_callback(move |_| { observed.fetch_add(1, Ordering::SeqCst); });
        ctx.request_repaint();
        let already_queued = repaint_count.load(Ordering::SeqCst);
        let (sender, receiver) = mpsc::channel();
        let receiver = Mutex::new(receiver);
        let wakes = Arc::new(AtomicUsize::new(0));
        let observed = wakes.clone();
        let wake: WindowWake = Arc::new(move || {
            // The native wake must see the completed result already queued.
            assert!(matches!(receiver.lock().unwrap().try_recv(), Ok(Reply::Cancelled)));
            observed.fetch_add(1, Ordering::SeqCst);
        });
        deliver(Reply::Cancelled, &sender, &ctx, Some(&wake));
        assert_eq!(repaint_count.load(Ordering::SeqCst), already_queued);
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn completed_request_does_not_wake_a_closed_client() {
        use std::sync::{atomic::{AtomicUsize, Ordering}, mpsc};
        let ctx = eframe::egui::Context::default();
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        let wakes = Arc::new(AtomicUsize::new(0));
        let observed = wakes.clone();
        let wake: WindowWake = Arc::new(move || { observed.fetch_add(1, Ordering::SeqCst); });
        deliver(Reply::Cancelled, &sender, &ctx, Some(&wake));
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
    }
}
