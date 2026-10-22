//! Status reads the service answers from views it already composed.
//!
//! A composed reply is answered again only under the same change generation
//! ([`Changes`]) and the same unwritten service state, and only when nothing
//! it shows can change without a record write: no operation, installer or
//! application session was live. A current overview or product page finds
//! that out as it is composed; a snapshot relies on one of them composed
//! under the same generation. Any other read is composed afresh, exactly as
//! the command line composes it.
use super::{changes::Changes, contract::Read};
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}};

/// One composed reply.
pub struct Composed {
    /// The reply as the command line writes it.
    pub reply: Vec<u8>,
    /// Whether nothing it shows can change without a record write; `None`
    /// when this read cannot tell and another read must have found it.
    pub quiescent: Option<bool>,
    /// Directories of the files it was read from, which the change watch
    /// may not cover yet.
    pub directories: Vec<PathBuf>,
}

/// Product pages held at once, besides the current overview.
const HELD: usize = 64;

pub struct Views {
    changes: Option<Changes>,
    held: Mutex<Held>,
}
#[derive(Default)]
struct Held {
    generation: Option<(u64, u64)>,
    /// A read under this generation found nothing live.
    quiescent: bool,
    replies: HashMap<Read, Arc<Vec<u8>>>,
}
impl Views {
    /// Without a change watch, every read is composed.
    pub fn new(changes: Option<Changes>) -> Self {
        Self { changes, held: Mutex::new(Held::default()) }
    }
    /// Answer `read`. `service` stands for the service's own state that no
    /// record holds (its unconfirmed-cleanup census); another value is a change.
    pub fn answer(&self, read: &Read, service: u64,
        compose: impl FnOnce() -> crate::Result<Composed>) -> crate::Result<Arc<Vec<u8>>> {
        // Taken before composing: a change during composition is newer.
        let generation = self.changes.as_ref().and_then(Changes::generation)
            .map(|generation| (generation, service));
        if let Some(generation) = generation {
            let held = self.held.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if held.generation == Some(generation) {
                if let Some(reply) = held.replies.get(read) { return Ok(reply.clone()); }
            }
        }
        let composed = compose()?;
        // A file read from a directory watched only now could have changed
        // unreported before; the next composition is covered.
        let covered = !self.changes.as_ref()
            .is_some_and(|changes| changes.watch_directories(composed.directories));
        let reply = Arc::new(composed.reply);
        let Some(generation) = generation.filter(|_| covered) else { return Ok(reply) };
        let mut held = self.held.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if held.generation != Some(generation) {
            *held = Held { generation: Some(generation), ..Held::default() };
        }
        held.quiescent |= composed.quiescent == Some(true);
        if composed.quiescent.unwrap_or(held.quiescent) && held.replies.len() <= HELD {
            held.replies.insert(read.clone(), reply.clone());
        }
        Ok(reply)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::service::{changes::Tree, contract::ProductKey};
    use std::{cell::Cell, fs};
    #[test]
    fn a_view_is_answered_again_only_while_nothing_changed() {
        let root = std::env::temp_dir().join(format!("lvb-views-{}", crate::random_id().unwrap()));
        fs::create_dir_all(root.join("operator")).unwrap();
        let views = Views::new(Some(Changes::new(vec![Tree { root: root.clone(), depth: None }]).unwrap()));
        let composed = Cell::new(0);
        let answer = |read: &Read, service: u64, quiescent: Option<bool>| {
            views.answer(read, service, || {
                composed.set(composed.get() + 1);
                Ok(Composed { reply: format!("{}", composed.get()).into_bytes(), quiescent,
                    directories: vec![root.join("operator")] })
            }).unwrap()
        };
        let (current, snapshot) = (Read::CurrentOverview, Read::Snapshot);
        assert_eq!(*answer(&current, 0, Some(true)), b"1", "first composed");
        assert_eq!(*answer(&current, 0, Some(true)), b"2", "its directories were watched only after it");
        assert_eq!(*answer(&current, 0, Some(true)), b"2", "answered again");
        let product = Read::Product(ProductKey { environment: "e".into(), module_sha256: "m".into(), class_id: "c".into() });
        assert_eq!(*answer(&product, 0, Some(true)), b"3");
        assert_eq!(*answer(&product, 0, Some(true)), b"3");
        assert_eq!(*answer(&snapshot, 0, None), b"4", "nothing was live under this generation");
        assert_eq!(*answer(&snapshot, 0, None), b"4");
        fs::write(root.join("operator/latest.json"), b"{}").unwrap();
        assert_eq!(*answer(&snapshot, 0, None), b"5", "a record changed and no read has found it quiet");
        assert_eq!(*answer(&snapshot, 0, None), b"6");
        assert_eq!(*answer(&current, 0, Some(false)), b"7", "an operation is live");
        assert_eq!(*answer(&current, 0, Some(false)), b"8");
        assert_eq!(*answer(&snapshot, 0, None), b"9");
        assert_eq!(*answer(&current, 1, Some(true)), b"10", "the service's census changed");
        assert_eq!(*answer(&current, 1, Some(false)), b"10", "still current");
        assert_eq!(*answer(&product, 1, Some(true)), b"11");
        assert_eq!(*answer(&Read::Pulse, 1, Some(false)), b"12");
        assert_eq!(*answer(&Read::Pulse, 1, Some(false)), b"13", "a live read is always composed");
        assert_eq!(*Views::new(None).answer(&current, 0, || Ok(Composed { reply: b"fresh".to_vec(),
            quiescent: Some(true), directories: vec![] })).unwrap(), b"fresh");
        fs::remove_dir_all(root).unwrap();
    }
}
