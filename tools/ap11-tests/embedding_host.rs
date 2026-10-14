//! Owned first-party Wine/X11 feasibility test, independent of production DAWs.
#[allow(dead_code)]
#[path = "../../native-vst3-proxy/backend/src/editor_surface.rs"]
mod editor_surface;
use editor_surface::x11::{Connection, Surface};
use editor_surface::*;
use std::{
    ffi::c_void,
    io::{self, Read, Write},
    mem::size_of,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};
const IO_BOUND: Duration = Duration::from_secs(20);
const FIRST_PREFIX_BOUND: Duration = Duration::from_secs(90);
#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}
extern "C" {
    fn poll(fds: *mut PollFd, count: usize, timeout: i32) -> i32;
}
#[link(name = "X11")]
extern "C" {
    fn XOpenDisplay(name: *const u8) -> *mut c_void;
    fn XFlush(display: *mut c_void) -> i32;
    fn XCloseDisplay(display: *mut c_void) -> i32;
    fn XGetImage(
        display: *mut c_void,
        window: u64,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        planes: u64,
        format: i32,
    ) -> *mut c_void;
    fn XGetPixel(image: *mut c_void, x: i32, y: i32) -> u64;
    fn XDestroyImage(image: *mut c_void) -> i32;
}
#[link(name = "Xtst")]
extern "C" {
    fn XTestFakeMotionEvent(display: *mut c_void, screen: i32, x: i32, y: i32, delay: u64) -> i32;
    fn XTestFakeButtonEvent(display: *mut c_void, button: u32, press: i32, delay: u64) -> i32;
}
fn click(x: i32, y: i32) {
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        assert!(!display.is_null());
        assert_ne!(XTestFakeMotionEvent(display, -1, x, y, 0), 0);
        assert_ne!(XTestFakeButtonEvent(display, 1, 1, 0), 0);
        assert_ne!(XTestFakeButtonEvent(display, 1, 0, 0), 0);
        XFlush(display);
        XCloseDisplay(display);
    }
}
fn trace_pointer(connection: &Connection, label: &str) {
    let mut parent = connection.root;
    let mut path = Vec::new();
    for _ in 0..8 {
        let child = connection.pointer_child(parent).unwrap();
        if child == 0 {
            break;
        }
        path.push(child);
        parent = child;
    }
    eprintln!("{label} X11 pointer path: {path:?}");
}
// Raw captures stay in the new private case directory, never in Git. This
// diagnostic is for the explicitly owned 24-bit Xvfb fixture, not product UI.
fn snapshot(connection: &Connection, path: &Path) -> io::Result<Vec<u8>> {
    let size = connection.geometry(connection.root).unwrap();
    assert!(size.width <= 2048 && size.height <= 2048);
    let mut pixels = Vec::with_capacity(usize::from(size.width) * usize::from(size.height) * 3);
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        assert!(!display.is_null());
        let image = XGetImage(
            display,
            u64::from(connection.root),
            0,
            0,
            u32::from(size.width),
            u32::from(size.height),
            u64::MAX,
            2,
        );
        assert!(!image.is_null());
        for y in 0..i32::from(size.height) {
            for x in 0..i32::from(size.width) {
                let pixel = XGetPixel(image, x, y);
                pixels.extend([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8]);
            }
        }
        XDestroyImage(image);
        XCloseDisplay(display);
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    write!(file, "P6\n{} {}\n255\n", size.width, size.height)?;
    file.write_all(&pixels)?;
    Ok(pixels)
}
struct Editor {
    child: Child,
    input: Option<ChildStdin>,
    output: ChildStdout,
    identity: PresentationV1,
}
impl Editor {
    fn launch(
        wine: &str,
        fixture: &str,
        prefix: &PathBuf,
        owner: u64,
        epoch: u32,
        mode: u32,
    ) -> io::Result<Self> {
        let startup_bound = if prefix.exists() {
            IO_BOUND
        } else {
            FIRST_PREFIX_BOUND
        };
        let mut child = Command::new(wine)
            .args([
                fixture,
                &owner.to_string(),
                &epoch.to_string(),
                &mode.to_string(),
            ])
            .env("WINEPREFIX", prefix)
            .env("WINEDEBUG", "-all")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        let mut editor = Self {
            child,
            input,
            output,
            identity: PresentationV1::new(owner, epoch, WINDOW_READY),
        };
        let ready = editor.read_with_bound(WINDOW_READY, startup_bound)?;
        assert!(ready.valid_size() && ready.window != 0 && ready.presentation == mode);
        editor.identity = ready;
        eprintln!(
            "first-party editor Ready: epoch={epoch}, mode={mode}, xid={}",
            ready.window
        );
        Ok(editor)
    }
    fn read(&mut self, kind: u32) -> io::Result<PresentationV1> {
        self.read_with_bound(kind, IO_BOUND)
    }
    fn read_with_bound(&mut self, kind: u32, bound: Duration) -> io::Result<PresentationV1> {
        let deadline = Instant::now() + bound;
        let mut bytes = [0u8; size_of::<PresentationV1>()];
        let mut offset = 0;
        while offset < bytes.len() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "first-party editor record {kind} deadline, epoch={}, bound={bound:?}",
                        self.identity.epoch
                    ),
                ));
            }
            let mut fd = PollFd {
                fd: self.output.as_raw_fd(),
                events: 1,
                revents: 0,
            };
            if unsafe { poll(&mut fd, 1, remaining.as_millis().max(1) as i32) } <= 0 {
                continue;
            }
            let count = self.output.read(&mut bytes[offset..])?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "first-party editor record incomplete",
                ));
            }
            offset += count;
        }
        let record = unsafe { bytes.as_ptr().cast::<PresentationV1>().read_unaligned() };
        assert!(
            record.valid()
                && record.kind == kind
                && record.owner == self.identity.owner
                && record.epoch == self.identity.epoch
        );
        if self.identity.window != 0 {
            assert_eq!(record.window, self.identity.window);
        }
        Ok(record)
    }
    fn send(&mut self, kind: u32, width: u32, height: u32) -> io::Result<()> {
        let record = PresentationV1 {
            kind,
            width,
            height,
            ..self.identity
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (&record as *const PresentationV1).cast::<u8>(),
                size_of::<PresentationV1>(),
            )
        };
        self.input.as_mut().unwrap().write_all(bytes)?;
        self.input.as_mut().unwrap().flush()
    }
    fn close(&mut self) -> io::Result<()> {
        self.send(CLOSE, 0, 0)?;
        self.read(WINDOW_CLOSED)?;
        self.input.take();
        let deadline = Instant::now() + IO_BOUND;
        loop {
            if let Some(status) = self.child.try_wait()? {
                assert!(status.success());
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "first-party editor exit deadline",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Editor {
    fn drop(&mut self) {
        // EOF asks this exact first-party helper to close normally, including
        // when a test assertion fails. Never kill unrelated Wine processes.
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => {
                    eprintln!(
                        "owned first-party helper exit unconfirmed: pid={}",
                        self.child.id()
                    );
                    return;
                }
            }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(
        args.len(),
        4,
        "embedding-host WINE_PROGRAM FIRST_PARTY_EXE NEW_PRIVATE_DIRECTORY"
    );
    let root = PathBuf::from(&args[3]);
    assert!(!root.exists(), "Refuse an existing fixture directory");
    std::fs::create_dir(&root)?;
    let prefix = root.join("prefix");
    let connection = Connection::open().expect("isolated X11 display");
    let parent = connection
        .child(connection.root, 20, 20, 660, 480, 0x505050)
        .unwrap();
    let chrome = connection.child(parent, 0, 0, 80, 28, 0xe0a020).unwrap();
    connection.button_input(chrome).unwrap();
    connection.map(parent).unwrap();
    connection.map(chrome).unwrap();
    let area = connection.child(parent, 0, 32, 640, 400, 0x303030).unwrap();
    connection.map(area).unwrap();
    let owner = 1297499478u64;
    let mut surface = Surface::create(area, owner, 640, 400).unwrap();
    let mut detached = Editor::launch(&args[1], &args[2], &prefix, owner, 1, DETACHED)?;
    assert_eq!(surface.bind(detached.identity), Err(Error::Identity));
    assert_eq!(
        surface.parent(detached.identity.window).unwrap(),
        connection.root
    );
    click(45, 32);
    assert!(connection.button_received(chrome).unwrap());
    click(180, 140);
    trace_pointer(&connection, "detached");
    assert_eq!(detached.read(INPUT_OBSERVED)?.result, 1);
    eprintln!("detached Win32 pointer observed");
    detached.close()?;

    let mut stale = None;
    for epoch in [2, 3] {
        let mut editor = Editor::launch(&args[1], &args[2], &prefix, owner, epoch, EMBEDDED_X11)?;
        let ready = editor.identity;
        surface.bind(ready).unwrap();
        assert_eq!(surface.parent(ready.window).unwrap(), surface.window);
        assert_eq!(connection.parent(area).unwrap(), parent);
        assert_eq!(connection.parent(chrome).unwrap(), parent);
        click(45, 32);
        assert!(connection.button_received(chrome).unwrap());
        // Real X11 pointer events must arrive at the Wine/Win32 editor.
        click(180, 140);
        trace_pointer(&connection, "embedded");
        assert_eq!(editor.read(INPUT_OBSERVED)?.result, 1);
        editor.send(RESIZE, 512, 280)?;
        let resized = editor.read(WINDOW_RESIZED)?;
        surface.resize(resized).unwrap();
        let size = surface.geometry(ready.window).unwrap();
        assert_eq!((size.width, size.height), (512, 280));
        assert_eq!(surface.parent(ready.window).unwrap(), surface.window);
        surface.focus().unwrap();
        assert_eq!(connection.focused_window().unwrap(), ready.window);
        editor.send(FOCUS, 0, 0)?;
        assert_eq!(editor.read(FOCUS_RESULT)?.result, 0);
        assert_eq!(surface.parent(ready.window).unwrap(), surface.window);
        let before = snapshot(&connection, &root.join(format!("popup-before-{epoch}.ppm")))?;
        editor.send(POPUP, 0, 0)?;
        assert_eq!(editor.read(POPUP_VISIBLE)?.result, 1);
        let during = snapshot(
            &connection,
            &root.join(format!("popup-visible-{epoch}.ppm")),
        )?;
        assert_ne!(before, during, "popup must change visible root pixels");
        editor.send(CANCEL_POPUP, 0, 0)?;
        editor.read(POPUP_RESULT)?;
        assert_eq!(surface.parent(ready.window).unwrap(), surface.window);
        click(45, 32);
        assert!(connection.button_received(chrome).unwrap());
        if let Some(old) = stale {
            assert_eq!(surface.resize(old), Err(Error::Identity));
        }
        editor.close()?;
        surface.detach().unwrap();
        stale = Some(resized);
    }
    surface.close().unwrap();
    assert_eq!(connection.parent(chrome).unwrap(), parent);
    click(45, 32);
    assert!(connection.button_received(chrome).unwrap());
    connection.destroy(parent).unwrap();
    println!("{{\"scope\":\"first_party_x11_wine_fixture\",\"detached\":true,\"embedded_generations\":2,\"native_chrome_clicks\":6,\"wine_pointer_input\":true,\"resize\":true,\"focus\":true,\"popup\":true,\"close_reopen\":true,\"stale_epoch_refused\":true}}");
    Ok(())
}
