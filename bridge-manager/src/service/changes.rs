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
//! A setup's Wine prefix and its other private directories are not watched as
//! trees: running applications rewrite them constantly, and no view reads
//! them, except the prefix's top two levels (its registry hives), which are
//! watched. Nor is the retired FL Studio workspace, which no view reads.
//! Files a view reads elsewhere are covered one directory at a time
//! ([`Changes::watch_directories`]); adding one is a change too, so a view read
//! from a directory before it was watched is never answered again, and one that
//! disappears is forgotten until a view names it again. If the watch cannot
//! cover everything, such as at the user's inotify limit, it closes and frees
//! every watch, every read is composed, and it tries again a minute later.
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Mutex},
    time::{Duration, Instant},
};

/// One watched directory tree.
#[derive(Clone, Debug)]
pub struct Tree {
    pub root: PathBuf,
    /// Directory levels watched below the root; `None` for the whole tree.
    pub depth: Option<usize>,
}

/// A setup's own directories beside its prefix, which no view reads:
/// pressure-vessel's runtime copy and the container's host and home views.
const SETUP_PRIVATE: [&str; 7] = ["runtime-var", "host-cache", "host-tmp", "host-data",
    "host-config", "home", "client"];
/// The retired FL Studio workspace (#504 E3) beside the bridge's records: the
/// musician's projects and its own Windows setup, left where they are.
const RETIRED_WORKSPACE: &str = "daw-workspaces";
/// More directories than this, and the watch does not cover the view.
const WATCH_LIMIT: usize = 65536;
/// Directories watched one by one, beyond which they are started afresh.
const DIRECTORY_LIMIT: usize = 4096;
/// How long an uncovered watch waits before it tries again.
const RETRY: Duration = Duration::from_secs(60);

pub struct Changes {
    state: Mutex<State>,
    /// Kept for readers that must not take reports or rebuild (`capacity`).
    covering: AtomicBool,
    watched: AtomicUsize,
}
impl Changes {
    pub fn new(trees: Vec<Tree>) -> Self {
        Self::with_limit(trees, WATCH_LIMIT)
    }
    /// As `new`, covering at most `limit` directories.
    pub fn with_limit(trees: Vec<Tree>, limit: usize) -> Self {
        let mut state = State { inotify: None, generation: 0, trees, roots: vec![],
            directories: BTreeSet::new(), tree_paths: HashSet::new(), watches: HashMap::new(),
            awaited: HashMap::new(), limit, uncovered: None, again: false, retry: None };
        state.build();
        let changes = Self { covering: AtomicBool::new(false), watched: AtomicUsize::new(0),
            state: Mutex::new(state) };
        changes.publish(&changes.state());
        changes
    }
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
    fn publish(&self, state: &State) {
        self.covering.store(state.covering(), Ordering::Relaxed);
        self.watched.store(state.watches.len(), Ordering::Relaxed);
    }
    /// The generation after every change reported so far, or `None` while
    /// the watch does not cover everything and no view may be answered again.
    pub fn generation(&self) -> Option<u64> {
        let mut state = self.state();
        match state.retry {
            Some(at) if Instant::now() >= at => state.build(),
            Some(_) => {}
            None => if let Err(error) = state.drain() {
                // Reports may have been lost; only a new watch covers again.
                state.uncovered = Some(format!("reading changes failed: {error}"));
                state.stop();
            }
        }
        self.publish(&state);
        state.covering().then_some(state.generation)
    }
    /// Also watch each directory itself (not below it), or while it is
    /// missing, its nearest existing ancestor. A directory a tree already
    /// covers needs nothing; any newly watched one is a change.
    pub fn watch_directories(&self, directories: impl IntoIterator<Item = PathBuf>) {
        let mut state = self.state();
        let mut added = false;
        for directory in directories {
            if state.directories.contains(&directory) || state.tree_paths.contains(&directory) { continue; }
            if state.directories.len() >= DIRECTORY_LIMIT {
                // Start afresh with the directories views name from now on.
                state.directories.clear();
                state.build();
            }
            state.directories.insert(directory.clone());
            if state.retry.is_none() { state.cover_directory(&directory); }
            added = true;
        }
        if added { state.generation += 1; }
        if state.again { state.build(); }
        if state.uncovered.is_some() && state.retry.is_none() { state.stop(); }
        self.publish(&state);
    }
    /// How many directories were watched at the last read.
    pub fn watched(&self) -> usize {
        self.watched.load(Ordering::Relaxed)
    }
    /// Whether the watch covered everything at the last read.
    pub fn covering(&self) -> bool {
        self.covering.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    /// A directory `depth` levels below the root of tree `tree`, with at most
    /// `levels` more watched below it.
    Tree { tree: usize, depth: usize, levels: Option<usize> },
    /// A directory watched for the files a view was read from.
    Directory,
    /// The nearest existing ancestor of a missing directory. Only the
    /// creation of the missing child matters there.
    Ancestor,
}
impl Place {
    /// A directory watched in two places keeps the one that covers more.
    fn covers(self) -> u8 {
        match self { Self::Ancestor => 0, Self::Directory => 1, Self::Tree { .. } => 2 }
    }
}
struct Watched {
    path: PathBuf,
    place: Place,
}
struct State {
    inotify: Option<inotify::Instance>,
    generation: u64,
    trees: Vec<Tree>,
    /// Each tree's root as last resolved.
    roots: Vec<PathBuf>,
    /// Directories views named outside the trees.
    directories: BTreeSet<PathBuf>,
    /// Directories watched as part of a tree.
    tree_paths: HashSet<PathBuf>,
    watches: HashMap<i32, Watched>,
    /// Missing children each watch awaits; their creation means a rebuild.
    awaited: HashMap<i32, BTreeSet<OsString>>,
    limit: usize,
    /// Why the watch does not cover everything.
    uncovered: Option<String>,
    /// A missing child appeared while its ancestor was being watched.
    again: bool,
    /// When to try again after stopping.
    retry: Option<Instant>,
}
impl State {
    fn covering(&self) -> bool {
        self.inotify.is_some() && self.uncovered.is_none() && self.retry.is_none()
    }
    /// Watch everything afresh in a new instance. Anything may have changed
    /// while it was not watched, so this is a change too.
    fn build(&mut self) {
        for _ in 0..4 {
            self.generation += 1;
            self.inotify = None;
            self.watches.clear();
            self.awaited.clear();
            self.tree_paths.clear();
            self.uncovered = None;
            self.again = false;
            match inotify::Instance::new() {
                Ok(instance) => self.inotify = Some(instance),
                Err(error) => self.uncovered = Some(format!("inotify unavailable: {error}")),
            }
            // A root that is itself a link is followed to its target, and
            // replacing the link rebuilds.
            self.roots = self.trees.iter().map(|tree| if is_link(&tree.root) {
                tree.root.canonicalize().unwrap_or_else(|_| tree.root.clone())
            } else { tree.root.clone() }).collect();
            for tree in 0..self.trees.len() {
                let root = self.roots[tree].clone();
                if is_link(&self.trees[tree].root) { self.await_entry(&self.trees[tree].root.clone()); }
                if root.is_dir() {
                    self.walk(&root, Place::Tree { tree, depth: 0, levels: self.trees[tree].depth });
                } else {
                    self.await_missing(&root);
                }
            }
            for directory in self.directories.clone() {
                self.cover_directory(&directory);
            }
            if !self.again { break; }
        }
        if self.uncovered.is_some() {
            self.stop();
        } else {
            self.retry = None;
            eprintln!("status change watch covers {} directories", self.watches.len());
        }
    }
    /// Close the instance, freeing every watch, and try again later.
    fn stop(&mut self) {
        self.inotify = None;
        self.watches.clear();
        self.awaited.clear();
        self.tree_paths.clear();
        self.retry = Some(Instant::now() + RETRY);
        eprintln!("status change watch stopped ({}); every status read is composed until it \
            tries again in {} s", self.uncovered.as_deref().unwrap_or("unknown"), RETRY.as_secs());
    }
    fn walk(&mut self, directory: &Path, place: Place) {
        let Place::Tree { tree, depth, levels } = place else { unreachable!("a tree walk") };
        if self.watch(directory, place).is_none() || levels == Some(0) { return; }
        // A directory removed meanwhile was reported by its parent.
        let Ok(entries) = std::fs::read_dir(directory) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if !entry.file_type().is_ok_and(|kind| kind.is_dir()) { continue; }
            if let Some(below) = self.below(tree, &path, levels.map(|levels| levels - 1)) {
                self.walk(&path, Place::Tree { tree, depth: depth + 1, levels: below });
            }
        }
    }
    /// How many levels to watch below a tree directory, or `None` to leave
    /// it out: another tree's root is its own, a prefix is watched only at
    /// its top two levels, and a setup's private directories and the retired
    /// workspace not at all.
    fn below(&self, tree: usize, path: &Path, levels: Option<usize>) -> Option<Option<usize>> {
        let name = path.file_name()?;
        let parent = path.parent()?;
        if self.roots.iter().enumerate().any(|(other, root)| other != tree && root == path) { return None; }
        if name == "compatdata" { return Some(Some(levels.map_or(1, |levels| levels.min(1)))); }
        let beside = |record: &str| parent.join(record).exists();
        if (SETUP_PRIVATE.iter().any(|private| name == *private) && beside("compatdata"))
            || (name == RETIRED_WORKSPACE && beside("software.json")) {
            return None;
        }
        Some(levels)
    }
    fn cover_directory(&mut self, directory: &Path) {
        match directory.canonicalize() {
            Ok(resolved) if resolved.is_dir() => {
                // A linked directory is followed to its target; replacing the link rebuilds.
                if is_link(directory) { self.await_entry(directory); }
                self.watch(&resolved, Place::Directory);
            }
            Ok(_) => self.uncovered = Some(format!("{} is not a directory", directory.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => self.await_missing(directory),
            Err(error) => self.uncovered = Some(format!("{}: {error}", directory.display())),
        }
    }
    /// Watch the parent of the link `entry` for the entry's replacement.
    fn await_entry(&mut self, entry: &Path) {
        let (Some(parent), Some(name)) = (entry.parent(), entry.file_name()) else { return };
        if let Some(descriptor) = self.watch(parent, Place::Ancestor) {
            self.awaited.entry(descriptor).or_default().insert(name.to_owned());
        }
    }
    /// Watch the nearest existing ancestor for `missing`'s first missing
    /// component. One that appears meanwhile makes the build start again.
    fn await_missing(&mut self, missing: &Path) {
        let Some(ancestor) = missing.ancestors().skip(1).find(|path| path.is_dir()) else {
            self.uncovered = Some(format!("{} has no existing ancestor", missing.display()));
            return;
        };
        let Some(child) = missing.strip_prefix(ancestor).ok().and_then(|rest| rest.iter().next()) else { return };
        let child = child.to_owned();
        if let Some(descriptor) = self.watch(ancestor, Place::Ancestor) {
            self.awaited.entry(descriptor).or_default().insert(child.clone());
            if std::fs::symlink_metadata(ancestor.join(&child)).is_ok() { self.again = true; }
        }
    }
    /// The watch descriptor, once `directory` is watched.
    fn watch(&mut self, directory: &Path, place: Place) -> Option<i32> {
        if self.uncovered.is_some() { return None; }
        if self.watches.len() >= self.limit {
            self.uncovered = Some(format!("more than {} directories to watch", self.limit));
            return None;
        }
        match self.inotify.as_ref()?.watch(directory) {
            Ok(descriptor) => {
                let watched = self.watches.entry(descriptor)
                    .or_insert_with(|| Watched { path: directory.to_owned(), place });
                if place.covers() > watched.place.covers() { watched.place = place; }
                if matches!(watched.place, Place::Tree { .. }) { self.tree_paths.insert(watched.path.clone()); }
                Some(descriptor)
            }
            // Removed or replaced meanwhile: wait for it to return.
            Err(error) if matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ENOTDIR)) => {
                if place != Place::Ancestor { self.await_missing(directory); }
                None
            }
            Err(error) => {
                self.uncovered = Some(match error.raw_os_error() {
                    Some(libc::ENOSPC) => "the user's inotify watch limit is reached".into(),
                    _ => format!("{}: {error}", directory.display()),
                });
                None
            }
        }
    }
    /// Take every report so far.
    fn drain(&mut self) -> std::io::Result<()> {
        let Some(inotify) = self.inotify.as_ref() else { return Ok(()) };
        let (mut changed, mut rebuild, mut created, mut forgotten) = (false, false, Vec::new(), Vec::new());
        let (watches, awaited, tree_paths, roots) = (&mut self.watches, &mut self.awaited,
            &mut self.tree_paths, &self.roots);
        inotify.drain(|event| {
            let entry_created = event.mask & (libc::IN_CREATE | libc::IN_MOVED_TO) != 0;
            let is_awaited = awaited.get(&event.descriptor).is_some_and(|names| names.contains(event.name));
            let place = watches.get(&event.descriptor).map(|watched| watched.place);
            // An ancestor stands in for a missing directory; the rest of it is not watched.
            changed |= place != Some(Place::Ancestor) || is_awaited;
            // Overflow lost reports, a moved directory changed every path below
            // it, a missing child appeared, or a root went away.
            rebuild |= event.mask & (libc::IN_Q_OVERFLOW | libc::IN_MOVE_SELF) != 0
                || (is_awaited && entry_created);
            if event.mask & libc::IN_IGNORED != 0 {
                let Some(watched) = watches.remove(&event.descriptor) else { return };
                awaited.remove(&event.descriptor);
                tree_paths.remove(&watched.path);
                // A root is awaited again at once; a directory a view named,
                // or the parent standing in for it, is forgotten, and the next
                // view that needs it names it again. Either is a change: a view
                // read from it is not answered after.
                if matches!(watched.place, Place::Tree { depth: 0, .. })
                    || roots.iter().any(|root| root.starts_with(&watched.path)) {
                    rebuild = true;
                } else if matches!(watched.place, Place::Directory | Place::Ancestor) {
                    changed = true;
                    forgotten.push(watched.path);
                }
            }
            if let Some(Place::Tree { tree, depth, levels }) = place {
                if entry_created && event.mask & libc::IN_ISDIR != 0 && levels != Some(0) {
                    if let Some(watched) = watches.get(&event.descriptor) {
                        created.push((tree, watched.path.join(event.name), depth + 1, levels.map(|l| l - 1)));
                    }
                }
            }
        })?;
        if changed { self.generation += 1; }
        for gone in forgotten {
            self.directories.retain(|directory| !directory.starts_with(&gone));
        }
        for (tree, path, depth, levels) in created {
            if let Some(below) = self.below(tree, &path, levels) {
                if path.is_dir() { self.walk(&path, Place::Tree { tree, depth, levels: below }); }
            }
        }
        if rebuild || self.again { self.build(); }
        if self.uncovered.is_some() && self.retry.is_none() { self.stop(); }
        Ok(())
    }
}

fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
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
        /// Hand every queued report to `each`.
        pub fn drain(&self, mut each: impl FnMut(Event<'_>)) -> std::io::Result<()> {
            const HEADER: usize = std::mem::size_of::<libc::inotify_event>();
            let mut buffer = vec![0u8; 64 * 1024];
            loop {
                let read = unsafe { libc::read(self.0.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len()) };
                if read < 0 {
                    let error = std::io::Error::last_os_error();
                    match error.raw_os_error() {
                        Some(libc::EAGAIN) => return Ok(()),
                        Some(libc::EINTR) => continue,
                        _ => return Err(error),
                    }
                }
                let read = read as usize;
                if read == 0 { return Ok(()); }
                let mut offset = 0;
                while offset + HEADER <= read {
                    let event: libc::inotify_event = unsafe {
                        std::ptr::read_unaligned(buffer[offset..].as_ptr().cast()) };
                    let name_end = (offset + HEADER + event.len as usize).min(read);
                    let name = &buffer[offset + HEADER..name_end];
                    let name = &name[..name.iter().position(|byte| *byte == 0).unwrap_or(name.len())];
                    each(Event { descriptor: event.wd, mask: event.mask, name: OsStr::from_bytes(name) });
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
        pub fn drain(&self, _: impl FnMut(Event<'_>)) -> std::io::Result<()> {
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
        root.canonicalize().unwrap()
    }
    /// Asserts each step advances the generation and leaves the watch covering.
    struct Steps<'a> {
        changes: &'a Changes,
        generation: u64,
    }
    impl<'a> Steps<'a> {
        fn new(changes: &'a Changes) -> Self {
            Self { changes, generation: changes.generation().expect("covering") }
        }
        fn changed(&mut self, what: &str, change: impl FnOnce()) {
            change();
            let next = self.changes.generation().unwrap_or_else(|| panic!("covering after {what}"));
            assert_ne!(next, self.generation, "{what} is a change");
            self.generation = next;
        }
        fn unchanged(&mut self, what: &str, change: impl FnOnce()) {
            change();
            assert_eq!(self.changes.generation(), Some(self.generation), "{what} is not a change");
        }
    }
    #[test]
    fn every_change_in_a_tree_advances_and_reads_do_not() {
        let root = root();
        let records = root.join("managed/operator/first");
        fs::create_dir_all(&records).unwrap();
        fs::write(records.join("result.json"), b"{}").unwrap();
        let changes = Changes::new(vec![Tree { root: root.join("managed"), depth: None }]);
        let mut steps = Steps::new(&changes);
        // Reading, listing and opening are not changes, nor is closing a lock
        // file opened for writing without writing it, as readers do.
        steps.unchanged("reading", || {
            fs::read(records.join("result.json")).unwrap();
            fs::read_dir(&records).unwrap().for_each(drop);
            drop(fs::OpenOptions::new().write(true).open(records.join("result.json")).unwrap());
        });
        steps.changed("a replacement", || crate::atomic_json(&records.join("result.json"),
            &serde_json::json!({"state":"completed"})).unwrap());
        steps.changed("a write", || fs::write(records.join("result.json"), b"{}").unwrap());
        steps.changed("an unlink", || fs::remove_file(records.join("result.json")).unwrap());
        let later = root.join("managed/operator/second/nested");
        steps.changed("a new directory", || fs::create_dir_all(&later).unwrap());
        steps.changed("a write in it", || fs::write(later.join("result.json"), b"{}").unwrap());
        use std::os::unix::fs::PermissionsExt;
        steps.changed("an attribute", ||
            fs::set_permissions(later.join("result.json"), fs::Permissions::from_mode(0o400)).unwrap());
        steps.changed("a moved directory", || fs::rename(root.join("managed/operator/second"),
            root.join("managed/operator/third")).unwrap());
        steps.changed("a write below it", ||
            fs::write(root.join("managed/operator/third/nested/moved.json"), b"{}").unwrap());
        // A directory replaced by a rename over it is watched as the new one.
        let staged = root.join("managed/operator/staged");
        fs::create_dir(&staged).unwrap();
        fs::write(staged.join("result.json"), b"{}").unwrap();
        steps.changed("a replacing rename", || {
            fs::remove_dir_all(&records).unwrap();
            fs::rename(&staged, &records).unwrap();
        });
        steps.changed("a write in the replacement", || fs::write(records.join("result.json"), b"[]").unwrap());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn setup_private_directories_are_left_out_and_registry_hives_are_watched() {
        let root = root();
        let setup = root.join("managed/environments/setup");
        let plugins = setup.join("compatdata/pfx/drive_c/Plugins");
        fs::create_dir_all(&plugins).unwrap();
        for private in SETUP_PRIVATE { fs::create_dir_all(setup.join(private).join("nested")).unwrap(); }
        let workspace = root.join("managed/daw-workspaces/fl-studio");
        fs::create_dir_all(workspace.join("projects/song")).unwrap();
        fs::write(workspace.join("workspace.json"), b"{}").unwrap();
        fs::write(root.join("managed/software.json"), b"{}").unwrap();
        let changes = Changes::new(vec![Tree { root: root.join("managed"), depth: None }]);
        let mut steps = Steps::new(&changes);
        steps.unchanged("a prefix's contents", || fs::write(plugins.join("wine.log"), b"rewritten").unwrap());
        for private in SETUP_PRIVATE {
            steps.unchanged(private, || fs::write(setup.join(private).join("nested/file"), b"x").unwrap());
        }
        steps.unchanged("a retired workspace project", || fs::write(workspace.join("projects/song/song.flp"), b"x").unwrap());
        steps.unchanged("the retired workspace record", || fs::write(workspace.join("workspace.json"), b"[]").unwrap());
        steps.changed("a registry hive", || fs::write(setup.join("compatdata/pfx/system.reg"), b"hive").unwrap());
        // A named directory inside the prefix is watched, and naming it is a change.
        steps.changed("naming a directory", || changes.watch_directories([plugins.clone()]));
        steps.unchanged("naming it again", || changes.watch_directories([plugins.clone()]));
        steps.unchanged("naming a directory a tree covers", ||
            changes.watch_directories([root.join("managed/environments")]));
        steps.changed("a replaced module", || fs::write(plugins.join("Plugin.vst3"), b"module").unwrap());
        // A named directory that goes away is forgotten, without a rebuild,
        // and watched again once a view names it again.
        let watched = changes.watched();
        steps.changed("its removal", || fs::remove_dir_all(&plugins).unwrap());
        assert_eq!(changes.watched(), watched - 1, "forgotten, not rebuilt");
        steps.unchanged("its return before a view names it", || fs::create_dir(&plugins).unwrap());
        steps.changed("a view naming it again", || changes.watch_directories([plugins.clone()]));
        steps.changed("a write in it", || fs::write(plugins.join("Plugin.vst3"), b"again").unwrap());
        // A view saw a named directory missing; its nearest existing parent
        // stands in for it. That parent going away is a change too: nothing
        // watches either path afterwards, until a view names the directory again.
        let vendor = setup.join("compatdata/pfx/drive_c/Vendor");
        fs::create_dir(&vendor).unwrap();
        let missing = vendor.join("Plugin.vst3");
        steps.changed("naming a missing directory", || changes.watch_directories([missing.clone()]));
        steps.changed("its standing-in parent removed", || fs::remove_dir_all(&vendor).unwrap());
        steps.changed("a view naming it again", || changes.watch_directories([missing.clone()]));
        steps.changed("it and its parent returning", || fs::create_dir_all(&missing).unwrap());
        steps.changed("a write in it", || fs::write(missing.join("module"), b"reinstalled").unwrap());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_missing_or_removed_root_is_watched_when_it_appears() {
        let root = root();
        fs::create_dir(root.join("managed")).unwrap();
        let published = root.join("published");
        let changes = Changes::new(vec![Tree { root: root.join("managed"), depth: None },
            Tree { root: published.clone(), depth: Some(0) }]);
        let mut steps = Steps::new(&changes);
        // Until then the rest of its parent, here the home directory, is not watched.
        steps.unchanged("unrelated churn", || {
            fs::write(root.join(".xsession-errors"), b"churn").unwrap();
            fs::create_dir(root.join("unrelated")).unwrap();
        });
        steps.changed("the root appearing", || fs::create_dir(&published).unwrap());
        steps.changed("a link in it", || std::os::unix::fs::symlink(&root, published.join("LVB_a.vst3")).unwrap());
        steps.changed("the root removed", || fs::remove_dir_all(&published).unwrap());
        steps.changed("the root recreated", || fs::create_dir(&published).unwrap());
        steps.changed("a link in it", || std::os::unix::fs::symlink(&root, published.join("LVB_a.vst3")).unwrap());
        steps.changed("the link removed", || fs::remove_file(published.join("LVB_a.vst3")).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_lost_queue_is_rebuilt_and_a_partial_watch_is_freed_not_held() {
        let root = root();
        let records = root.join("managed/records");
        fs::create_dir_all(&records).unwrap();
        let changes = Changes::new(vec![Tree { root: root.join("managed"), depth: None }]);
        let mut steps = Steps::new(&changes);
        // Writes alternating between two files are never merged into one
        // report, so the queue fills without one file per report.
        let queued: usize = fs::read_to_string("/proc/sys/fs/inotify/max_queued_events")
            .map_or(16384, |limit| limit.trim().parse().unwrap());
        if queued <= 1 << 18 {
            let mut files = [fs::File::create(records.join("a")).unwrap(), fs::File::create(records.join("b")).unwrap()];
            steps.changed("an overflowing queue", || for index in 0..=queued + 16 {
                std::io::Write::write_all(&mut files[index % 2], b"x").unwrap();
            });
        } else {
            eprintln!("max_queued_events is {queued}; overflow is not exercised");
        }
        steps.changed("a write after the rebuild", || fs::write(records.join("after"), b"x").unwrap());
        // Past its limit (here 1 directory, as at the user's inotify limit) the
        // watch frees what it held and stays off until a later retry.
        let partial = Changes::with_limit(vec![Tree { root: root.join("managed"), depth: None }], 1);
        assert_eq!(partial.generation(), None);
        assert_eq!(partial.watched(), 0, "no partial set of watches is kept");
        partial.watch_directories([root.clone()]);
        assert_eq!((partial.generation(), partial.watched()), (None, 0), "not retried at once");
        fs::remove_dir_all(root).unwrap();
    }
}
