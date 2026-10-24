//! Status reads the service answers from views it already composed.
//!
//! A composed reply is answered again only under the same change generation
//! ([`Changes`]) and the same unwritten service state, and only when its
//! composer found nothing that could change it without a record write: no
//! operation, installer or application session was live. Any other
//! read is composed afresh, exactly as the command line composes it.
use super::{changes::Changes, contract::Read};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::{AtomicU64, Ordering}, Arc, Mutex, OnceLock},
};

/// Whether a composed reply may be answered again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    /// This read is composed for every read.
    Never,
    /// Nothing it shows can change without a record write.
    Quiet,
    /// Something live can change what views show without writing: nothing
    /// composed under this generation is answered again.
    Live,
}
/// One composed reply.
pub struct Composed {
    /// The reply as the command line writes it.
    pub reply: Vec<u8>,
    pub hold: Hold,
    /// Directories of the files it was read from, which the change watch
    /// may not cover yet.
    pub directories: Vec<PathBuf>,
}

/// Pages held at once, and their total size.
const HELD: usize = 64;
const HELD_BYTES: usize = 32 * 1024 * 1024;

#[derive(Default)]
pub struct Views {
    changes: OnceLock<Changes>,
    volatile_revision: AtomicU64,
    held: Mutex<Held>,
    answered_again: AtomicU64,
    composed: AtomicU64,
}
#[derive(Default)]
struct Held {
    generation: Option<(u64, u64)>,
    replies: HashMap<Read, Arc<Vec<u8>>>,
    bytes: usize,
}
impl Views {
    /// Until [`cover`](Self::cover) is called, every read is composed.
    pub fn new() -> Self {
        Self::default()
    }
    /// Start answering held views, once the change watch covers them.
    pub fn cover(&self, changes: Changes) {
        let _ = self.changes.set(changes);
    }
    /// Drain the same watch used by current reads. A push never grants offers.
    pub fn records_revision(&self) -> Option<u64> {
        let volatile = self.volatile_revision.load(Ordering::Acquire);
        if volatile == u64::MAX { return None; }
        self.changes.get().and_then(Changes::generation)
            .and_then(|revision| revision.checked_add(volatile))
    }
    /// An owner changed a fact held only in service memory. This invalidates
    /// both current replies and the subscription's record coverage together.
    pub fn invalidate(&self) {
        let _ = self.volatile_revision.fetch_update(Ordering::AcqRel, Ordering::Acquire,
            |revision| Some(revision.saturating_add(1)));
    }
    fn held(&self) -> std::sync::MutexGuard<'_, Held> {
        self.held.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
    /// Answer `read`. `service` stands for the service's own state that no
    /// record holds (its unconfirmed-cleanup census); another value is a change.
    pub fn answer(&self, read: &Read, service: u64,
        compose: impl FnOnce() -> crate::Result<Composed>) -> crate::Result<Arc<Vec<u8>>> {
        // Taken before composing: a change during composition is newer.
        let generation = self.records_revision()
            .map(|generation| (generation, service));
        if let Some(generation) = generation {
            let held = self.held();
            if held.generation == Some(generation) {
                if let Some(reply) = held.replies.get(read) {
                    self.answered_again.fetch_add(1, Ordering::Relaxed);
                    return Ok(reply.clone());
                }
            }
        }
        let composed = compose()?;
        self.composed.fetch_add(1, Ordering::Relaxed);
        // Watching a directory for the first time is a change, so a view
        // read from it before (this one, or one composed alongside) is not
        // answered again.
        if let Some(changes) = self.changes.get() { changes.watch_directories(composed.directories); }
        let reply = Arc::new(composed.reply);
        let Some(generation) = generation else { return Ok(reply) };
        let mut held = self.held();
        match composed.hold {
            Hold::Never => {}
            Hold::Live => if held.generation == Some(generation) { *held = Held::default(); },
            Hold::Quiet => {
                if held.generation != Some(generation) {
                    *held = Held { generation: Some(generation), ..Held::default() };
                }
                if held.replies.len() < HELD && held.bytes + reply.len() <= HELD_BYTES {
                    held.bytes += reply.len();
                    if let Some(replaced) = held.replies.insert(read.clone(), reply.clone()) {
                        held.bytes -= replaced.len();
                    }
                }
            }
        }
        Ok(reply)
    }
    /// What the service reports about its reads (`linux-vst-bridge capacity`).
    /// It takes no change reports, so reading it changes nothing.
    pub fn statistics(&self) -> serde_json::Value {
        let changes = self.changes.get();
        serde_json::json!({
            "watched_directories": changes.map_or(0, Changes::watched),
            "covering": changes.is_some_and(Changes::covering),
            "answered_again": self.answered_again.load(Ordering::Relaxed),
            "composed": self.composed.load(Ordering::Relaxed),
        })
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::service::{changes::Tree, contract::ProductKey};
    use std::{cell::Cell, fs};
    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("lvb-views-{}", crate::random_id().unwrap()));
        fs::create_dir_all(root.join("managed/operator")).unwrap();
        root.canonicalize().unwrap()
    }
    fn product() -> Read {
        Read::Product(ProductKey { environment: "e".into(), module_sha256: "m".into(), class_id: "c".into() })
    }
    #[test]
    fn a_view_is_answered_again_only_while_nothing_changed() {
        let root = root();
        let views = Views::new();
        let composed = Cell::new(0);
        let answer = |read: &Read, service: u64, hold: Hold| {
            views.answer(read, service, || {
                composed.set(composed.get() + 1);
                Ok(Composed { reply: format!("{}", composed.get()).into_bytes(), hold,
                    directories: vec![root.join("prefix")] })
            }).unwrap()
        };
        let current = Read::CurrentOverview;
        assert_eq!(*answer(&current, 0, Hold::Quiet), b"1");
        assert_eq!(*answer(&current, 0, Hold::Quiet), b"2", "not answered again before the watch covers");
        views.cover(Changes::new(vec![Tree { root: root.join("managed"), depth: None }]));
        fs::create_dir(root.join("prefix")).unwrap();
        assert_eq!(*answer(&current, 0, Hold::Quiet), b"3", "first composed");
        assert_eq!(*answer(&current, 0, Hold::Quiet), b"4", "its directory was watched only after it");
        assert_eq!(*answer(&current, 0, Hold::Quiet), b"4", "answered again");
        assert_eq!(*answer(&product(), 0, Hold::Quiet), b"5");
        assert_eq!(*answer(&product(), 0, Hold::Quiet), b"5");
        assert_eq!(views.statistics()["answered_again"], 2);
        fs::write(root.join("managed/operator/latest.json"), b"{}").unwrap();
        assert_eq!(*answer(&current, 0, Hold::Quiet), b"6", "a record changed");
        fs::write(root.join("prefix/module.vst3"), b"replaced").unwrap();
        assert_eq!(*answer(&current, 0, Hold::Quiet), b"7", "a file in a named directory changed");
        assert_eq!(*answer(&product(), 0, Hold::Quiet), b"8");
        assert_eq!(*answer(&current, 1, Hold::Quiet), b"9", "the service's census changed");
        assert_eq!(*answer(&current, 1, Hold::Quiet), b"9");
        assert_eq!(*answer(&Read::Pulse, 1, Hold::Never), b"10");
        assert_eq!(*answer(&current, 1, Hold::Quiet), b"9", "a read composed every time holds nothing back");
        assert_eq!(*answer(&product(), 1, Hold::Live), b"11", "an operation is live");
        assert_eq!(*answer(&current, 1, Hold::Quiet), b"12", "so nothing of this generation is answered again");
        assert_eq!(*answer(&Read::Snapshot, 1, Hold::Never), b"13");
        assert_eq!(*answer(&Read::Snapshot, 1, Hold::Never), b"14");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_view_read_before_its_directory_was_watched_is_not_answered_again() {
        let root = root();
        let prefix = root.join("prefix");
        fs::create_dir(&prefix).unwrap();
        let module = prefix.join("module.vst3");
        fs::write(&module, b"before").unwrap();
        let views = &Views::new();
        views.cover(Changes::new(vec![Tree { root: root.join("managed"), depth: None }]));
        let (module, prefix) = (module.as_path(), prefix.as_path());
        let (read, read_done) = std::sync::mpsc::channel();
        let (watched, watched_done) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            // One reader reads the module, then finishes only after another
            // reader named its directory.
            let first = scope.spawn(move || views.answer(&product(), 0, || {
                let reply = fs::read(module).unwrap();
                read.send(()).unwrap();
                watched_done.recv().unwrap();
                Ok(Composed { reply, hold: Hold::Quiet, directories: vec![prefix.to_owned()] })
            }).unwrap());
            read_done.recv().unwrap();
            // The module changes while nothing watches its directory.
            fs::write(module, b"after").unwrap();
            views.answer(&Read::CurrentOverview, 0, || Ok(Composed { reply: b"overview".to_vec(),
                hold: Hold::Quiet, directories: vec![prefix.to_owned()] })).unwrap();
            watched.send(()).unwrap();
            assert_eq!(*first.join().unwrap(), b"before");
        });
        let current = views.answer(&product(), 0, || Ok(Composed { reply: fs::read(module).unwrap(),
            hold: Hold::Quiet, directories: vec![prefix.to_owned()] })).unwrap();
        assert_eq!(*current, b"after", "the page read before the watch is not answered again");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn volatile_owner_changes_invalidate_held_and_in_progress_views() {
        let root = root();
        let views = Views::new();
        views.cover(Changes::new(vec![Tree { root: root.join("managed"), depth: None }]));
        let compose = |text: &[u8]| Composed { reply: text.to_vec(), hold: Hold::Quiet, directories: vec![] };
        let first_revision = views.records_revision().unwrap();
        views.answer(&product(), 0, || Ok(compose(b"before"))).unwrap();
        assert_eq!(*views.answer(&product(), 0, || panic!("held view should be reused")).unwrap(), b"before");
        views.invalidate();
        assert!(views.records_revision().unwrap() > first_revision);
        assert_eq!(*views.answer(&product(), 0, || Ok(compose(b"after"))).unwrap(), b"after");
        // An owner update during composition must not cache the older offer.
        views.invalidate();
        views.answer(&product(), 0, || {
            views.invalidate();
            Ok(compose(b"crossed an owner change"))
        }).unwrap();
        assert_eq!(*views.answer(&product(), 0, || Ok(compose(b"current"))).unwrap(), b"current");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn exhausted_volatile_revision_never_reuses_an_offer() {
        let root = root();
        let views = Views::new();
        views.cover(Changes::new(vec![Tree { root: root.join("managed"), depth: None }]));
        views.volatile_revision.store(u64::MAX - 1, Ordering::Release);
        views.invalidate();
        assert_eq!(views.records_revision(), None);
        views.invalidate();
        assert_eq!(views.records_revision(), None);
        for text in [b"first".as_slice(), b"second".as_slice()] {
            assert_eq!(views.answer(&product(), 0, || Ok(Composed {
                reply: text.to_vec(), hold: Hold::Quiet, directories: vec![],
            })).unwrap().as_slice(), text);
        }
        fs::remove_dir_all(root).unwrap();
    }
}
