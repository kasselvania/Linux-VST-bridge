//! The view's change watch.
//!
//! The service answers a status read from a view it already composed only
//! while nothing that view was read from has changed. Those records live in a
//! few directory trees, and the kernel reports every change in them (a write,
//! rename, unlink, attribute or link change) to this watch before the writing
//! call returns, whichever process made it: the service itself, an operation
//! worker, a package or older command, or a supervisor. A read takes every
//! report so far first, and any report advances the generation, so a view
//! composed before a change is never answered after it.
//!
//! Wine prefixes (`compatdata`) are never watched as trees: running
//! applications rewrite them constantly, and a view reads only declared files
//! inside them, whose directories are watched one by one
//! ([`Changes::watch_directories`]). A directory the watch cannot cover, such
//! as one past the user's inotify limit, makes the watch incomplete, and the
//! service then composes every read until it covers everything again.
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

/// One watched directory tree.
#[derive(Clone, Debug)]
pub struct Tree {
    pub root: PathBuf,
    /// Directory levels watched below the root; `None` for the whole tree.
    pub depth: Option<usize>,
}

/// More directories than this makes the watch incomplete.
const WATCH_LIMIT: usize = 65536;
/// Directories watched one by one for the files views were read from.
const DIRECTORY_LIMIT: usize = 4096;
/// How often an incomplete watch tries to cover everything again.
const RETRY: Duration = Duration::from_secs(10);

pub struct Changes {
    state: Mutex<State>,
}
impl Changes {
    pub fn new(trees: Vec<Tree>) -> crate::Result<Self> {
        let mut state = State::new(trees)?;
        state.build();
        Ok(Self { state: Mutex::new(state) })
    }
    /// The generation after every change reported so far, or `None` while
    /// some directory is not covered and no view may be answered again.
    pub fn generation(&self) -> Option<u64> {
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.drain().is_err() {
            // Reports may be lost; only a new watch is complete again.
            state.uncovered = true;
        }
        if state.uncovered && state.attempted.elapsed() >= RETRY {
            state.build();
        }
        (!state.uncovered).then_some(state.generation)
    }
    /// Also watch each directory itself (not below it), or while it is
    /// missing, its nearest existing ancestor. Returns whether any of them
    /// was not watched before, so a view read from it is not yet covered.
    pub fn watch_directories(&self, directories: impl IntoIterator<Item = PathBuf>) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut added = false;
        for directory in directories {
            if state.directories.contains(&directory) { continue; }
            if state.directories.len() >= DIRECTORY_LIMIT {
                state.uncovered = true;
                return true;
            }
            state.directories.insert(directory.clone());
            state.add_directory(&directory);
            added = true;
        }
        added
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    /// A directory `depth` levels below the root of tree `tree`.
    Tree { tree: usize, depth: usize },
    /// A directory watched for the files a view was read from.
    Directory,
    /// The nearest existing ancestor of a missing tree root or directory.
    Ancestor,
}
struct Watched {
    path: PathBuf,
    place: Place,
}
struct State {
    inotify: inotify::Instance,
    generation: u64,
    trees: Vec<Tree>,
    directories: BTreeSet<PathBuf>,
    watches: HashMap<i32, Watched>,
    uncovered: bool,
    attempted: Instant,
}
impl State {
    fn new(trees: Vec<Tree>) -> crate::Result<Self> {
        Ok(Self { inotify: inotify::Instance::new()?, generation: 0, trees,
            directories: BTreeSet::new(), watches: HashMap::new(), uncovered: false,
            attempted: Instant::now() })
    }
    /// Watch everything afresh in a new instance. Anything may have changed
    /// while it was not watched, so this is a change too.
    fn build(&mut self) {
        self.attempted = Instant::now();
        self.generation += 1;
        self.watches.clear();
        self.uncovered = false;
        match inotify::Instance::new() {
            Ok(instance) => self.inotify = instance,
            Err(_) => {
                self.uncovered = true;
                return;
            }
        }
        for tree in 0..self.trees.len() {
            let root = self.trees[tree].root.clone();
            if root.is_dir() { self.add_tree(tree, &root, 0); } else { self.add_ancestor(&root); }
        }
        for directory in self.directories.clone() {
            self.add_directory(&directory);
        }
    }
    fn add_tree(&mut self, tree: usize, directory: &Path, depth: usize) {
        if !self.add(directory, Place::Tree { tree, depth })
            || self.trees[tree].depth.is_some_and(|levels| depth >= levels) {
            return;
        }
        // A directory removed meanwhile was reported by its parent.
        let Ok(entries) = std::fs::read_dir(directory) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) && self.descends(tree, &path) {
                self.add_tree(tree, &path, depth + 1);
            }
        }
    }
    /// Prefixes are not watched as trees, and another tree's root is its own.
    fn descends(&self, tree: usize, path: &Path) -> bool {
        path.file_name().is_some_and(|name| name != "compatdata")
            && !self.trees.iter().enumerate().any(|(other, t)| other != tree && t.root == path)
    }
    fn add_directory(&mut self, directory: &Path) {
        if directory.is_dir() {
            self.add(directory, Place::Directory);
        } else {
            self.add_ancestor(directory);
        }
    }
    fn add_ancestor(&mut self, missing: &Path) {
        if let Some(ancestor) = missing.ancestors().skip(1).find(|path| path.is_dir()) {
            self.add(ancestor, Place::Ancestor);
        }
    }
    /// Whether `directory` is watched. One watched before keeps its place.
    fn add(&mut self, directory: &Path, place: Place) -> bool {
        if self.watches.len() >= WATCH_LIMIT {
            self.uncovered = true;
            return false;
        }
        match self.inotify.watch(directory) {
            Ok(descriptor) => {
                self.watches.entry(descriptor)
                    .or_insert_with(|| Watched { path: directory.to_owned(), place });
                true
            }
            // Removed or replaced meanwhile: its parent reported that.
            Err(error) if matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ENOTDIR)) => false,
            Err(_) => {
                self.uncovered = true;
                false
            }
        }
    }
    /// Take every report so far.
    fn drain(&mut self) -> std::io::Result<()> {
        let mut rebuild = false;
        let mut created = Vec::new();
        let changed = self.inotify.drain(|event| {
            if event.mask & libc::IN_Q_OVERFLOW != 0 || event.mask & libc::IN_MOVE_SELF != 0 {
                rebuild = true;
            }
            if event.mask & libc::IN_IGNORED != 0 {
                self.watches.remove(&event.descriptor);
            }
            if event.mask & libc::IN_ISDIR != 0 && event.mask & (libc::IN_CREATE | libc::IN_MOVED_TO) != 0 {
                match self.watches.get(&event.descriptor) {
                    Some(Watched { path, place: Place::Tree { tree, depth } }) =>
                        created.push((*tree, path.join(event.name), depth + 1)),
                    Some(Watched { place: Place::Ancestor, .. }) => rebuild = true,
                    _ => {}
                }
            }
        })?;
        if changed { self.generation += 1; }
        for (tree, path, depth) in created {
            if self.trees[tree].depth.is_none_or(|levels| depth <= levels) && self.descends(tree, &path) {
                self.add_tree(tree, &path, depth);
            }
        }
        if rebuild { self.build(); }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod inotify {
    use std::{
        ffi::{CString, OsStr},
        os::{fd::{AsRawFd, FromRawFd, OwnedFd}, unix::ffi::OsStrExt},
        path::Path,
    };
    /// Every change a directory's entries or the directory itself can undergo.
    /// Closing a file opened for writing is not one: lock files are opened
    /// so by readers, and a write is reported as it happens.
    const CHANGES: u32 = libc::IN_MODIFY | libc::IN_ATTRIB
        | libc::IN_MOVED_FROM | libc::IN_MOVED_TO | libc::IN_CREATE | libc::IN_DELETE
        | libc::IN_DELETE_SELF | libc::IN_MOVE_SELF
        | libc::IN_ONLYDIR | libc::IN_DONT_FOLLOW | libc::IN_EXCL_UNLINK;
    pub struct Instance(OwnedFd);
    pub struct Event<'a> {
        pub descriptor: i32,
        pub mask: u32,
        pub name: &'a OsStr,
    }
    impl Instance {
        pub fn new() -> std::io::Result<Self> {
            let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
            if fd < 0 { return Err(std::io::Error::last_os_error()); }
            Ok(Self(unsafe { OwnedFd::from_raw_fd(fd) }))
        }
        pub fn watch(&self, directory: &Path) -> std::io::Result<i32> {
            let path = CString::new(directory.as_os_str().as_bytes())
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
            let descriptor = unsafe { libc::inotify_add_watch(self.0.as_raw_fd(), path.as_ptr(), CHANGES) };
            if descriptor < 0 { Err(std::io::Error::last_os_error()) } else { Ok(descriptor) }
        }
        /// Hand every queued report to `each`; whether there was any.
        pub fn drain(&self, mut each: impl FnMut(Event<'_>)) -> std::io::Result<bool> {
            const HEADER: usize = std::mem::size_of::<libc::inotify_event>();
            let mut buffer = vec![0u8; 64 * 1024];
            let mut any = false;
            loop {
                let read = unsafe { libc::read(self.0.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len()) };
                if read < 0 {
                    let error = std::io::Error::last_os_error();
                    match error.raw_os_error() {
                        Some(libc::EAGAIN) => return Ok(any),
                        Some(libc::EINTR) => continue,
                        _ => return Err(error),
                    }
                }
                let read = read as usize;
                if read == 0 { return Ok(any); }
                let mut offset = 0;
                while offset + HEADER <= read {
                    let event: libc::inotify_event = unsafe {
                        std::ptr::read_unaligned(buffer[offset..].as_ptr().cast()) };
                    let name_end = (offset + HEADER + event.len as usize).min(read);
                    let name = &buffer[offset + HEADER..name_end];
                    let name = &name[..name.iter().position(|byte| *byte == 0).unwrap_or(name.len())];
                    each(Event { descriptor: event.wd, mask: event.mask, name: OsStr::from_bytes(name) });
                    any = true;
                    offset = name_end;
                }
            }
        }
    }
}
#[cfg(not(target_os = "linux"))]
mod inotify {
    use std::{ffi::OsStr, path::Path};
    pub struct Instance;
    pub struct Event<'a> {
        pub descriptor: i32,
        pub mask: u32,
        pub name: &'a OsStr,
    }
    impl Instance {
        pub fn new() -> std::io::Result<Self> {
            Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
        }
        pub fn watch(&self, _: &Path) -> std::io::Result<i32> {
            Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
        }
        pub fn drain(&self, _: impl FnMut(Event<'_>)) -> std::io::Result<bool> {
            Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::fs;
    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("lvb-changes-{}", crate::random_id().unwrap()));
        fs::create_dir_all(&root).unwrap();
        root
    }
    #[test]
    fn every_change_in_a_tree_advances_and_reads_do_not() {
        let root = root();
        let records = root.join("managed/operator/first");
        fs::create_dir_all(&records).unwrap();
        fs::write(records.join("result.json"), b"{}").unwrap();
        let changes = Changes::new(vec![Tree { root: root.join("managed"), depth: None }]).unwrap();
        let mut generation = changes.generation().unwrap();
        // Reading, listing and opening are not changes, nor is closing a lock
        // file opened for writing without writing it, as readers do.
        fs::read(records.join("result.json")).unwrap();
        fs::read_dir(&records).unwrap().for_each(drop);
        drop(fs::OpenOptions::new().write(true).open(records.join("result.json")).unwrap());
        assert_eq!(changes.generation(), Some(generation));
        let mut changed = |change: &dyn Fn()| {
            change();
            let next = changes.generation().unwrap();
            assert_ne!(next, generation);
            generation = next;
        };
        changed(&|| crate::atomic_json(&records.join("result.json"), &serde_json::json!({"state":"completed"})).unwrap());
        changed(&|| fs::write(records.join("result.json"), b"{}").unwrap());
        changed(&|| fs::remove_file(records.join("result.json")).unwrap());
        // A directory created later is watched from then on.
        let later = root.join("managed/operator/second/nested");
        changed(&|| fs::create_dir_all(&later).unwrap());
        changed(&|| fs::write(later.join("result.json"), b"{}").unwrap());
        use std::os::unix::fs::PermissionsExt;
        changed(&|| fs::set_permissions(later.join("result.json"), fs::Permissions::from_mode(0o400)).unwrap());
        changed(&|| fs::rename(root.join("managed/operator/second"), root.join("managed/operator/third")).unwrap());
        changed(&|| fs::write(root.join("managed/operator/third/nested/moved.json"), b"{}").unwrap());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn prefixes_are_left_out_and_named_directories_and_missing_roots_are_watched() {
        let root = root();
        let prefix = root.join("managed/environments/setup/compatdata/pfx/drive_c/Plugins");
        fs::create_dir_all(&prefix).unwrap();
        let changes = Changes::new(vec![Tree { root: root.join("managed"), depth: None },
            Tree { root: root.join("published"), depth: Some(0) }]).unwrap();
        let generation = changes.generation().unwrap();
        fs::write(prefix.join("wine.log"), b"constantly rewritten").unwrap();
        assert_eq!(changes.generation(), Some(generation), "a prefix is not a watched tree");
        assert!(changes.watch_directories([prefix.clone()]));
        assert!(!changes.watch_directories([prefix.clone()]), "already watched");
        let generation = changes.generation().unwrap();
        fs::write(prefix.join("Plugin.vst3"), b"replaced module").unwrap();
        assert_ne!(changes.generation(), Some(generation));
        // A missing tree root is watched once it appears.
        let generation = changes.generation().unwrap();
        fs::create_dir(root.join("published")).unwrap();
        let created = changes.generation().unwrap();
        assert_ne!(created, generation);
        fs::write(root.join("published/LVB_plugin.vst3"), b"link").unwrap();
        assert_ne!(changes.generation(), Some(created));
        fs::remove_dir_all(root).unwrap();
    }
}
