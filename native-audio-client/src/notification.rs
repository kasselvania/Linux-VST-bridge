//! Session-paired wake hints. Payloads and execution authority stay in the mailbox.
//! Only the native transport worker performs I/O on this connection.
use crate::{mapping::random, need, Frame, HELLO};
use std::{io::{self, Read, Write}, net::{Shutdown, TcpListener, TcpStream}, os::unix::io::AsRawFd,
    time::{Duration, Instant}};

pub const SCHEMA: u32 = 1;
pub const OFFER_BYTES: usize = 44;
pub const WAKE: u8 = 1;
const PUMP_ROLE: u32 = 1;
pub const SERVICE_INTERVAL: Duration = Duration::from_millis(4);

pub struct Prepared {
    listener: TcpListener,
    capability: [u8; 32],
}
impl Prepared {
    pub fn create() -> io::Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener, capability: random()? })
    }
    pub fn offer(&self) -> io::Result<Vec<u8>> {
        let mut bytes = vec![0; OFFER_BYTES];
        bytes[..4].copy_from_slice(b"LVBW");
        bytes[4..8].copy_from_slice(&SCHEMA.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.listener.local_addr()?.port().to_le_bytes());
        bytes[12..].copy_from_slice(&self.capability);
        Ok(bytes)
    }
    pub fn accept_while(self, session: [u8; 16], minor: u64,
        mut alive: impl FnMut() -> io::Result<()>) -> io::Result<Channel> {
        need(minor == 15, "notification protocol generation")?;
        let end = Instant::now() + Duration::from_secs(10);
        let (mut socket, address) = loop {
            alive()?;
            need(Instant::now() < end, "notification pairing deadline")?;
            match self.listener.accept() {
                Ok(peer) => break peer,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(4)),
                Err(e) => return Err(e),
            }
        };
        need(address.ip().is_loopback(), "nonlocal notification peer")?;
        socket.set_nodelay(true)?;
        let hello = crate::read_frame_version(&mut crate::Deadline { stream: &mut socket, end }, minor)?;
        need(hello.kind == HELLO && hello.session == session && hello.sequence == 0
            && hello.payload.len() == 40, "notification Hello shape")?;
        need(hello.payload[..4] == SCHEMA.to_le_bytes()
            && hello.payload[4..8] == PUMP_ROLE.to_le_bytes(), "notification schema/role")?;
        let difference = hello.payload[8..].iter().zip(self.capability)
            .fold(0u8, |difference, (received, expected)| difference | (*received ^ expected));
        need(difference == 0, "notification channel authentication")?;
        alive()?;
        let reply = Frame { kind: HELLO, session, sequence: 0, payload: vec![] }.encode_version(minor)?;
        crate::Deadline { stream: &mut socket, end }.write_all(&reply)?;
        Channel::new(socket)
    }
}

pub struct Channel {
    socket: TcpStream,
    pending: bool,
}
#[repr(C)]
struct PollFd { fd: i32, events: i16, revents: i16 }
#[cfg(target_os = "macos")] type PollCount = u32;
#[cfg(not(target_os = "macos"))] type PollCount = usize;
unsafe extern "C" { fn poll(descriptors: *mut PollFd, count: PollCount, milliseconds: i32) -> i32; }
impl Channel {
    pub fn new(socket: TcpStream) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        socket.set_nodelay(true)?;
        Ok(Self { socket, pending: false })
    }
    pub fn raw_fd(&self) -> i32 { self.socket.as_raw_fd() }
    pub fn pending_wake(&self) -> bool { self.pending }
    /// A one-byte record cannot be partially written. Backpressure retains the
    /// pending wake until a later writable turn; it never drops the last hint.
    pub fn notify(&mut self) -> io::Result<()> {
        self.pending = true;
        self.flush()
    }
    fn flush(&mut self) -> io::Result<()> {
        if !self.pending { return Ok(()); }
        match (&self.socket).write(&[WAKE]) {
            Ok(1) => { self.pending = false; Ok(()) },
            Ok(_) => Err(crate::invalid("notification endpoint disconnected")),
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => Ok(()),
            Err(e) => Err(e),
        }
    }
    /// A wake, signal or the 4-ms service interval requests a new authoritative
    /// state/health inspection. Neither activity nor backpressure renews `end`.
    pub fn service(&mut self, end: Instant) -> io::Result<()> {
        self.flush()?;
        let remaining = end.checked_duration_since(Instant::now()).unwrap_or_default().min(SERVICE_INTERVAL);
        let milliseconds = if remaining.is_zero() { 0 } else { remaining.as_millis().max(1) as i32 };
        let mut descriptor = PollFd { fd: self.raw_fd(), events: 1 | if self.pending { 4 } else { 0 }, revents: 0 };
        let result = unsafe { poll(&mut descriptor, 1, milliseconds) };
        if result < 0 {
            let e = io::Error::last_os_error();
            return if e.kind() == io::ErrorKind::Interrupted { Ok(()) } else { Err(e) };
        }
        need(descriptor.revents & (8 | 16 | 32) == 0, "notification endpoint disconnected")?;
        if descriptor.revents & 4 != 0 { self.flush()?; }
        if descriptor.revents & 1 != 0 {
            // Bound draining even under a peer sending repeated hints.
            let mut bytes = [0; 64];
            for _ in 0..4 {
                match (&self.socket).read(&mut bytes) {
                    Ok(0) => return Err(crate::invalid("notification endpoint disconnected")),
                    Ok(n) => need(bytes[..n].iter().all(|byte| *byte == WAKE), "notification hint value")?,
                    Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => break,
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }
}
impl Drop for Channel {
    fn drop(&mut self) { let _ = self.socket.shutdown(Shutdown::Both); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::{receive_version, send_version};
    fn pair() -> (Channel, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let socket = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (peer, _) = listener.accept().unwrap();
        (Channel::new(socket).unwrap(), peer)
    }
    #[test]
    fn duplicate_hints_are_bounded_and_bad_data_or_disconnect_refuses() {
        let (mut channel, mut peer) = pair();
        peer.write_all(&[WAKE; 128]).unwrap();
        channel.service(Instant::now() + Duration::from_secs(1)).unwrap();
        peer.write_all(&[0]).unwrap();
        assert!(channel.service(Instant::now() + Duration::from_secs(1)).is_err());
        let (mut channel, peer) = pair(); drop(peer);
        assert!(channel.service(Instant::now() + Duration::from_secs(1)).is_err());
    }
    #[test]
    fn pairing_rejects_crossed_session_capability_and_schema() {
        for field in [0, 1, 2, 3] {
            let prepared = Prepared::create().unwrap();
            let offer = prepared.offer().unwrap();
            let address = (std::net::Ipv4Addr::LOCALHOST, u16::from_le_bytes(offer[8..10].try_into().unwrap()));
            let receiver = std::thread::spawn(move || prepared.accept_while([19; 16], 15, || Ok(())));
            let mut peer = TcpStream::connect(address).unwrap();
            let mut payload = [SCHEMA.to_le_bytes().as_slice(), PUMP_ROLE.to_le_bytes().as_slice(), &offer[12..]].concat();
            let mut session = [19; 16];
            match field { 0 => session[0] ^= 1, 1 => payload[8] ^= 1, 2 => payload[0] ^= 1, _ => payload[4] ^= 1 }
            send_version(&mut peer, &Frame { kind: HELLO, session, sequence: 0, payload }, 1, 15).unwrap();
            assert!(receiver.join().unwrap().is_err());
        }
    }
    #[test]
    fn paired_channel_uses_fresh_capability_and_retires_peer() {
        let prepared = Prepared::create().unwrap(); let offer = prepared.offer().unwrap();
        assert!(offer[12..] != Prepared::create().unwrap().offer().unwrap()[12..]);
        let receiver = std::thread::spawn(move || prepared.accept_while([19;16], 15, || Ok(())));
        let mut peer = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, u16::from_le_bytes(offer[8..10].try_into().unwrap()))).unwrap();
        let payload = [SCHEMA.to_le_bytes().as_slice(), PUMP_ROLE.to_le_bytes().as_slice(), &offer[12..]].concat();
        send_version(&mut peer, &Frame { kind: HELLO, session:[19;16], sequence:0, payload }, 1, 15).unwrap();
        assert!(receive_version(&mut peer, 1, 15).unwrap().payload.is_empty());
        let mut channel = receiver.join().unwrap().unwrap();
        channel.notify().unwrap(); let mut byte = [0]; peer.read_exact(&mut byte).unwrap(); assert_eq!(byte, [WAKE]);
        drop(channel); assert_eq!(peer.read(&mut byte).unwrap(), 0);
    }
    #[test]
    fn backpressure_retains_the_last_wake_until_writable() {
        let (mut channel, mut peer) = pair();
        unsafe extern "C" { fn setsockopt(fd:i32, level:i32, option:i32, value:*const i32, length:u32)->i32; }
        #[cfg(target_os="macos")] let (level, send_buffer, receive_buffer)=(0xffff,0x1001,0x1002);
        #[cfg(not(target_os="macos"))] let (level, send_buffer, receive_buffer)=(1,7,8);
        let size=4096;
        assert_eq!(unsafe { setsockopt(channel.raw_fd(),level,send_buffer,&size,4) },0);
        assert_eq!(unsafe { setsockopt(peer.as_raw_fd(),level,receive_buffer,&size,4) },0);
        let bytes=[WAKE;16384]; let mut written=0;
        let end=Instant::now()+Duration::from_secs(2);
        loop {
            assert!(Instant::now()<end);
            match (&channel.socket).write(&bytes) {
                Ok(n)=>written+=n,
                Err(e) if e.kind()==io::ErrorKind::WouldBlock=>break,
                result=>panic!("unexpected prepared fill result: {result:?}"),
            }
        }
        loop {
            assert!(Instant::now()<end);
            channel.notify().unwrap();
            if channel.pending_wake() { break; }
            written+=1;
        }
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let draining=std::thread::spawn(move || {
            let mut left=written+1;let mut bytes=[0;16384];
            while left>0 { let n=peer.read(&mut bytes[..left.min(16384)]).unwrap();assert!(n>0);assert!(bytes[..n].iter().all(|byte|*byte==WAKE));left-=n; }
            peer
        });
        while channel.pending_wake() { assert!(Instant::now()<end);channel.service(end).unwrap(); }
        let _peer=draining.join().unwrap();
    }
}
