//! UI-thread X11 presentation. The DAW owns its parent; Wine owns its editor.
//! This module owns only its connection and the child it creates. It is not
//! called by production editor code until the Wine fixture qualifies it.
use std::mem::size_of;

pub const PRESENTATION_VERSION: u32 = 1;
pub const DETACHED: u32 = 0;
pub const EMBEDDED_X11: u32 = 1;
pub const WINDOW_READY: u32 = 1;
pub const WINDOW_RESIZED: u32 = 2;
pub const WINDOW_CLOSED: u32 = 3;
pub const FOCUS_RESULT: u32 = 4;
pub const POPUP_RESULT: u32 = 5;
pub const INPUT_OBSERVED: u32 = 6;
pub const POPUP_VISIBLE: u32 = 7;
pub const RESIZE: u32 = 101;
pub const FOCUS: u32 = 102;
pub const POPUP: u32 = 103;
pub const CLOSE: u32 = 104;
pub const CANCEL_POPUP: u32 = 105;

/// Independent fixture boundary; never an HWND, SDK pointer or parent cast.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationV1 {
    pub version: u32,
    pub extent: u32,
    pub kind: u32,
    pub presentation: u32,
    pub owner: u64,
    pub epoch: u32,
    pub window: u32,
    pub width: u32,
    pub height: u32,
    pub result: u32,
    pub reserved: u32,
}
const _: () = assert!(size_of::<PresentationV1>() == 48);
impl PresentationV1 {
    pub fn new(owner: u64, epoch: u32, kind: u32) -> Self {
        Self {
            version: PRESENTATION_VERSION,
            extent: size_of::<Self>() as u32,
            kind,
            presentation: EMBEDDED_X11,
            owner,
            epoch,
            window: 0,
            width: 0,
            height: 0,
            result: 0,
            reserved: 0,
        }
    }
    pub fn valid(&self) -> bool {
        self.version == PRESENTATION_VERSION
            && self.extent as usize == size_of::<Self>()
            && matches!(self.presentation, DETACHED | EMBEDDED_X11)
            && self.owner != 0
            && self.epoch != 0
            && self.reserved == 0
    }
    pub fn valid_size(&self) -> bool {
        self.width > 0
            && self.height > 0
            && u16::try_from(self.width).is_ok()
            && u16::try_from(self.height).is_ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Identity,
    Busy,
    WrongThread,
    Connection,
    Timeout,
    X11(u8),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub owner: u64,
    pub epoch: u32,
    pub window: u32,
}
impl Binding {
    pub fn accepts(&self, message: &PresentationV1) -> bool {
        message.valid()
            && message.presentation == EMBEDDED_X11
            && self.owner == message.owner
            && self.epoch == message.epoch
            && self.window == message.window
    }
}

#[cfg(target_os = "linux")]
pub mod x11 {
    use super::*;
    use std::{
        ffi::c_void,
        ptr,
        thread::ThreadId,
        time::{Duration, Instant},
    };
    const REPLY_BOUND: Duration = Duration::from_millis(250);
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Cookie {
        pub sequence: u32,
    }
    #[repr(C)]
    struct Roots {
        data: *mut c_void,
        rem: i32,
        index: i32,
    }
    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }
    #[repr(C)]
    pub struct Geometry {
        response: u8,
        depth: u8,
        sequence: u16,
        length: u32,
        pub root: u32,
        pub x: i16,
        pub y: i16,
        pub width: u16,
        pub height: u16,
        border: u16,
        pad: [u8; 2],
    }
    #[repr(C)]
    struct Tree {
        response: u8,
        pad: u8,
        sequence: u16,
        length: u32,
        root: u32,
        parent: u32,
        children: u16,
        padding: [u8; 14],
    }
    #[link(name = "xcb")]
    extern "C" {
        fn xcb_connect(display: *const u8, screen: *mut i32) -> *mut c_void;
        fn xcb_connection_has_error(connection: *mut c_void) -> i32;
        fn xcb_disconnect(connection: *mut c_void);
        fn xcb_get_setup(connection: *mut c_void) -> *const c_void;
        fn xcb_setup_roots_iterator(setup: *const c_void) -> Roots;
        fn xcb_screen_next(roots: *mut Roots);
        fn xcb_get_file_descriptor(connection: *mut c_void) -> i32;
        fn xcb_generate_id(connection: *mut c_void) -> u32;
        fn xcb_flush(connection: *mut c_void) -> i32;
        fn xcb_poll_for_reply(
            connection: *mut c_void,
            sequence: u32,
            reply: *mut *mut c_void,
            error: *mut *mut c_void,
        ) -> i32;
        fn xcb_request_check(connection: *mut c_void, cookie: Cookie) -> *mut c_void;
        fn xcb_get_input_focus(connection: *mut c_void) -> Cookie;
        fn xcb_get_geometry(connection: *mut c_void, window: u32) -> Cookie;
        fn xcb_query_tree(connection: *mut c_void, window: u32) -> Cookie;
        fn xcb_create_window_checked(
            connection: *mut c_void,
            depth: u8,
            window: u32,
            parent: u32,
            x: i16,
            y: i16,
            width: u16,
            height: u16,
            border: u16,
            class: u16,
            visual: u32,
            mask: u32,
            values: *const u32,
        ) -> Cookie;
        fn xcb_map_window_checked(connection: *mut c_void, window: u32) -> Cookie;
        fn xcb_destroy_window_checked(connection: *mut c_void, window: u32) -> Cookie;
        fn xcb_reparent_window_checked(
            connection: *mut c_void,
            window: u32,
            parent: u32,
            x: i16,
            y: i16,
        ) -> Cookie;
        fn xcb_change_save_set_checked(connection: *mut c_void, mode: u8, window: u32) -> Cookie;
        fn xcb_configure_window_checked(
            connection: *mut c_void,
            window: u32,
            mask: u16,
            values: *const u32,
        ) -> Cookie;
        fn xcb_set_input_focus_checked(
            connection: *mut c_void,
            revert: u8,
            window: u32,
            time: u32,
        ) -> Cookie;
        fn xcb_change_window_attributes_checked(
            connection: *mut c_void,
            window: u32,
            mask: u32,
            values: *const u32,
        ) -> Cookie;
        fn xcb_poll_for_event(connection: *mut c_void) -> *mut c_void;
    }
    extern "C" {
        fn poll(fds: *mut PollFd, count: usize, timeout: i32) -> i32;
        fn free(pointer: *mut c_void);
    }

    pub struct Connection {
        pointer: *mut c_void,
        pub root: u32,
        owner: ThreadId,
    }
    impl Connection {
        pub fn open() -> Result<Self, Error> {
            unsafe {
                let mut screen = 0;
                let pointer = xcb_connect(ptr::null(), &mut screen);
                if pointer.is_null() || xcb_connection_has_error(pointer) != 0 {
                    if !pointer.is_null() {
                        xcb_disconnect(pointer);
                    }
                    return Err(Error::Connection);
                }
                let mut roots = xcb_setup_roots_iterator(xcb_get_setup(pointer));
                for _ in 0..screen {
                    xcb_screen_next(&mut roots);
                }
                if roots.rem == 0 {
                    xcb_disconnect(pointer);
                    return Err(Error::Connection);
                }
                let root = roots.data.cast::<u32>().read();
                Ok(Self {
                    pointer,
                    root,
                    owner: std::thread::current().id(),
                })
            }
        }
        fn owner(&self) -> Result<(), Error> {
            if self.owner != std::thread::current().id() {
                return Err(Error::WrongThread);
            }
            if unsafe { xcb_connection_has_error(self.pointer) } != 0 {
                return Err(Error::Connection);
            }
            Ok(())
        }
        fn reply(&self, cookie: Cookie) -> Result<*mut c_void, Error> {
            self.owner()?;
            if unsafe { xcb_flush(self.pointer) } <= 0 {
                return Err(Error::Connection);
            }
            let deadline = Instant::now() + REPLY_BOUND;
            loop {
                let mut reply = ptr::null_mut();
                let mut error = ptr::null_mut();
                if unsafe {
                    xcb_poll_for_reply(self.pointer, cookie.sequence, &mut reply, &mut error)
                } != 0
                {
                    if !error.is_null() {
                        let code = unsafe { error.cast::<u8>().add(1).read() };
                        unsafe {
                            free(error);
                            if !reply.is_null() {
                                free(reply);
                            }
                        }
                        return Err(Error::X11(code));
                    }
                    return if reply.is_null() {
                        Err(Error::Connection)
                    } else {
                        Ok(reply)
                    };
                }
                self.owner()?;
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(Error::Timeout);
                }
                let mut fd = PollFd {
                    fd: unsafe { xcb_get_file_descriptor(self.pointer) },
                    events: 1,
                    revents: 0,
                };
                unsafe {
                    poll(&mut fd, 1, remaining.as_millis().max(1) as i32);
                }
            }
        }
        fn checked(&self, cookie: Cookie) -> Result<(), Error> {
            // Complete one later round trip first. request_check then consumes
            // an already-resolved request; it cannot wait for another reply.
            let sync = self.reply(unsafe { xcb_get_input_focus(self.pointer) })?;
            unsafe {
                free(sync);
            }
            let error = unsafe { xcb_request_check(self.pointer, cookie) };
            if error.is_null() {
                return Ok(());
            }
            let code = unsafe { error.cast::<u8>().add(1).read() };
            unsafe {
                free(error);
            }
            Err(Error::X11(code))
        }
        pub fn geometry(&self, window: u32) -> Result<Geometry, Error> {
            let reply = self.reply(unsafe { xcb_get_geometry(self.pointer, window) })?;
            let result = unsafe { reply.cast::<Geometry>().read() };
            unsafe {
                free(reply);
            }
            Ok(result)
        }
        pub fn parent(&self, window: u32) -> Result<u32, Error> {
            let reply = self.reply(unsafe { xcb_query_tree(self.pointer, window) })?;
            let parent = unsafe { (*reply.cast::<Tree>()).parent };
            unsafe {
                free(reply);
            }
            Ok(parent)
        }
        pub fn child(
            &self,
            parent: u32,
            x: i16,
            y: i16,
            width: u16,
            height: u16,
            colour: u32,
        ) -> Result<u32, Error> {
            self.owner()?;
            if parent == 0 || width == 0 || height == 0 {
                return Err(Error::Identity);
            }
            self.geometry(parent)?;
            let window = unsafe { xcb_generate_id(self.pointer) };
            self.checked(unsafe {
                xcb_create_window_checked(
                    self.pointer,
                    0,
                    window,
                    parent,
                    x,
                    y,
                    width,
                    height,
                    0,
                    1,
                    0,
                    2,
                    &colour,
                )
            })?;
            Ok(window)
        }
        pub fn map(&self, window: u32) -> Result<(), Error> {
            self.checked(unsafe { xcb_map_window_checked(self.pointer, window) })
        }
        pub fn size(&self, window: u32, width: u16, height: u16) -> Result<(), Error> {
            let values = [u32::from(width), u32::from(height)];
            self.checked(unsafe {
                xcb_configure_window_checked(self.pointer, window, 12, values.as_ptr())
            })
        }
        pub fn focus(&self, window: u32) -> Result<(), Error> {
            self.checked(unsafe { xcb_set_input_focus_checked(self.pointer, 2, window, 0) })
        }
        pub fn destroy(&self, window: u32) -> Result<(), Error> {
            self.checked(unsafe { xcb_destroy_window_checked(self.pointer, window) })
        }
        pub fn button_input(&self, window: u32) -> Result<(), Error> {
            let mask = 1u32 << 2;
            self.checked(unsafe {
                xcb_change_window_attributes_checked(self.pointer, window, 2048, &mask)
            })
        }
        pub fn button_received(&self, window: u32) -> Result<bool, Error> {
            self.owner()?;
            let deadline = Instant::now() + REPLY_BOUND;
            loop {
                let event = unsafe { xcb_poll_for_event(self.pointer) };
                if !event.is_null() {
                    let kind = unsafe { event.cast::<u8>().read() } & 0x7f;
                    let target =
                        unsafe { event.cast::<u8>().add(12).cast::<u32>().read_unaligned() };
                    unsafe {
                        free(event);
                    }
                    if kind == 4 && target == window {
                        return Ok(true);
                    }
                }
                if Instant::now() >= deadline {
                    return Ok(false);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    impl Drop for Connection {
        fn drop(&mut self) {
            unsafe {
                xcb_disconnect(self.pointer);
            }
        }
    }

    pub struct Surface {
        connection: Connection,
        pub window: u32,
        owner: u64,
        last_epoch: u32,
        binding: Option<(Binding, u32, i16, i16)>,
    }
    impl Surface {
        pub fn create(parent: u32, owner: u64, width: u16, height: u16) -> Result<Self, Error> {
            if owner == 0 {
                return Err(Error::Identity);
            }
            let connection = Connection::open()?;
            let window = connection.child(parent, 0, 0, width, height, 0x202020)?;
            connection.map(window)?;
            Ok(Self {
                connection,
                window,
                owner,
                last_epoch: 0,
                binding: None,
            })
        }
        pub fn bind(&mut self, message: PresentationV1) -> Result<(), Error> {
            self.connection.owner()?;
            if !message.valid()
                || !message.valid_size()
                || message.kind != WINDOW_READY
                || message.presentation != EMBEDDED_X11
                || message.result != 0
                || message.owner != self.owner
                || message.epoch <= self.last_epoch
                || message.window == 0
            {
                return Err(Error::Identity);
            }
            if self.binding.is_some() {
                return Err(Error::Busy);
            }
            let original = self.connection.geometry(message.window)?;
            let parent = self.connection.parent(message.window)?;
            // Retire identity before mutation. SaveSet rescues the foreign
            // editor if this native connection dies.
            self.last_epoch = message.epoch;
            self.connection.checked(unsafe {
                xcb_change_save_set_checked(self.connection.pointer, 0, message.window)
            })?;
            let binding = Binding {
                owner: message.owner,
                epoch: message.epoch,
                window: message.window,
            };
            self.binding = Some((binding, parent, original.x, original.y));
            self.connection.checked(unsafe {
                xcb_reparent_window_checked(
                    self.connection.pointer,
                    message.window,
                    self.window,
                    0,
                    0,
                )
            })?;
            self.connection
                .size(self.window, message.width as u16, message.height as u16)?;
            self.connection.map(message.window)?;
            Ok(())
        }
        pub fn resize(&self, message: PresentationV1) -> Result<(), Error> {
            let Some((binding, _, _, _)) = self.binding else {
                return Err(Error::Identity);
            };
            if !binding.accepts(&message)
                || !message.valid_size()
                || message.kind != WINDOW_RESIZED
                || message.result != 0
            {
                return Err(Error::Identity);
            }
            if self.connection.parent(binding.window)? != self.window {
                return Err(Error::Identity);
            }
            self.connection
                .size(self.window, message.width as u16, message.height as u16)
        }
        pub fn focus(&self) -> Result<(), Error> {
            let Some((binding, _, _, _)) = self.binding else {
                return Err(Error::Identity);
            };
            if self.connection.parent(binding.window)? != self.window {
                return Err(Error::Identity);
            }
            self.connection.focus(binding.window)
        }
        pub fn detach(&mut self) -> Result<(), Error> {
            let Some((binding, parent, x, y)) = self.binding else {
                return Ok(());
            };
            match self.connection.parent(binding.window) {
                Err(Error::X11(3)) => {
                    self.binding = None;
                    return Ok(());
                } // positive BadWindow
                Err(error) => return Err(error),
                Ok(actual) if actual != self.window && actual != parent => {
                    return Err(Error::Identity)
                }
                Ok(_) => {}
            }
            self.connection.checked(unsafe {
                xcb_reparent_window_checked(self.connection.pointer, binding.window, parent, x, y)
            })?;
            self.connection.checked(unsafe {
                xcb_change_save_set_checked(self.connection.pointer, 1, binding.window)
            })?;
            self.binding = None;
            Ok(())
        }
        pub fn close(&mut self) -> Result<(), Error> {
            self.detach()?;
            self.connection.destroy(self.window)
        }
        pub fn parent(&self, window: u32) -> Result<u32, Error> {
            self.connection.parent(window)
        }
        pub fn geometry(&self, window: u32) -> Result<Geometry, Error> {
            self.connection.geometry(window)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versioned_record_rejects_foreign_shape() {
        let message = PresentationV1::new(9, 1, WINDOW_READY);
        assert!(message.valid());
        for changed in [
            PresentationV1 {
                presentation: 2,
                ..message
            },
            PresentationV1 {
                version: 2,
                ..message
            },
            PresentationV1 {
                extent: 40,
                ..message
            },
            PresentationV1 {
                owner: 0,
                ..message
            },
            PresentationV1 {
                epoch: 0,
                ..message
            },
            PresentationV1 {
                reserved: 1,
                ..message
            },
        ] {
            assert!(!changed.valid());
        }
    }
    #[test]
    fn editor_identity_does_not_follow_path_or_late_epoch() {
        let binding = Binding {
            owner: 9,
            epoch: 2,
            window: 45,
        };
        let message = PresentationV1 {
            window: 45,
            ..PresentationV1::new(9, 2, WINDOW_RESIZED)
        };
        assert!(binding.accepts(&message));
        for changed in [
            PresentationV1 {
                presentation: DETACHED,
                ..message
            },
            PresentationV1 {
                owner: 8,
                ..message
            },
            PresentationV1 {
                epoch: 1,
                ..message
            },
            PresentationV1 {
                window: 46,
                ..message
            },
        ] {
            assert!(!binding.accepts(&changed));
        }
    }
    #[test]
    fn size_is_the_x11_geometry_range() {
        let message = PresentationV1::new(9, 1, WINDOW_READY);
        assert!(!message.valid_size());
        assert!(PresentationV1 {
            width: 65535,
            height: 1,
            ..message
        }
        .valid_size());
        assert!(!PresentationV1 {
            width: 65536,
            height: 1,
            ..message
        }
        .valid_size());
    }
}
