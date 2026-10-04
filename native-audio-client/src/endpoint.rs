//! Reusable AP1 mapping and authenticated endpoint, independent of a sample recipe.
use crate::mapping::{barrier, random, Mapping};
use crate::*;
use std::{
    fs::OpenOptions,
    io::{self, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::{Duration, Instant},
};
pub fn receive_version(socket: &mut TcpStream, seconds: u64, minor: u64) -> io::Result<Frame> {
    read_frame_version(
        &mut Deadline {
            stream: socket,
            end: Instant::now() + Duration::from_secs(seconds),
        },
        minor,
    )
}
pub fn receive_version_into(socket: &mut TcpStream, seconds: u64, minor: u64, frame: &mut Frame) -> io::Result<()> {
    let mut reader = Deadline { stream: socket, end: Instant::now() + Duration::from_secs(seconds) };
    let mut header = [0; HEADER];
    reader.read_exact(&mut header)?;
    let n = payload_length_version(&header, minor)?;
    need(n <= frame.payload.capacity(), "audio reply scratch extent")?;
    frame.payload.clear();
    frame.payload.resize(n, 0);
    reader.read_exact(&mut frame.payload)?;
    frame.kind = get(&header[8..10]) as u16;
    frame.session.copy_from_slice(&header[16..32]);
    frame.sequence = get(&header[40..48]);
    Ok(())
}
pub fn send_version_with(socket: &mut TcpStream, f: &Frame, seconds: u64, minor: u64, bytes: &mut Vec<u8>) -> io::Result<()> {
    f.encode_version_into(minor, bytes)?;
    Deadline { stream: socket, end: Instant::now() + Duration::from_secs(seconds) }.write_all(bytes)
}
pub fn send_version(socket: &mut TcpStream, f: &Frame, seconds: u64, minor: u64) -> io::Result<()> {
    Deadline {
        stream: socket,
        end: Instant::now() + Duration::from_secs(seconds),
    }
    .write_all(&f.encode_version(minor)?)
}
pub struct Prepared {
    mapping: Mapping,
    listener: TcpListener,
    capability: [u8; 32],
    witness: u64,
    session: [u8; 16],
    declared_minor: Option<u64>,
}
impl Prepared {
    pub fn create(path: &Path, session: [u8; 16]) -> io::Result<Self> {
        Self::with_channels(path, session, 2)
    }
    pub fn with_channels(path: &Path, session: [u8; 16], channels: usize) -> io::Result<Self> {
        Self::with_layout(path, session, channels, false)
    }
    pub fn with_layout(path: &Path, session: [u8; 16], channels: usize, whole_block: bool) -> io::Result<Self> {
        Self::prepare(path, session, channels, whole_block, None)
    }
    pub fn with_protocol(path: &Path, session: [u8; 16], channels: usize, minor: u64) -> io::Result<Self> {
        need((1..=15).contains(&minor), "unsupported prepared protocol")?;
        Self::prepare(path, session, channels, minor >= 14, Some(minor))
    }
    fn prepare(path: &Path, session: [u8; 16], channels: usize, whole_block: bool, declared_minor: Option<u64>) -> io::Result<Self> {
        let mut mapping = Mapping::with_layout(&path.join("ap1.audio"), channels, whole_block)?;
        let capability = random::<32>()?;
        let witness = u64::from_le_bytes(random::<8>()?);
        let mut header = [0; 64];
        for (o, v) in [
            (0, 0x4d315041),
            (4, mapping.version as u64),
            (8, mapping.capacity as u64),
            (12, channels as u64),
            (16, mapping.bytes as u64),
            (20, INPUT as u64),
            (24, mapping.output as u64),
            (28, mapping.stride as u64),
        ] {
            put(&mut header[o..o + 4], v);
        }
        put(&mut header[32..40], witness);
        mapping.write(0, &header)?;
        barrier();
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let mut config = vec![0; if declared_minor == Some(15) { 60 } else { 52 }];
        put(&mut config[0..2], listener.local_addr()?.port() as u64);
        config[4..20].copy_from_slice(&session);
        config[20..52].copy_from_slice(&capability);
        if declared_minor == Some(15) {
            put(&mut config[2..4], 1); // bootstrap schema, independent of IPC
            put(&mut config[52..54], 15);
            put(&mut config[56..60], 60);
        }
        let temp = path.join("ap1.control.tmp");
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temp)?;
        f.write_all(&config)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(temp, path.join("ap1.control"))?;
        Ok(Self {
            mapping,
            listener,
            capability,
            witness,
            session,
            declared_minor,
        })
    }
    pub fn accept(self, minor: u64) -> io::Result<(Mapping, TcpStream)> {
        self.accept_while(minor, || Ok(()))
    }
    pub fn accept_while(
        self,
        minor: u64,
        alive: impl FnMut() -> io::Result<()>,
    ) -> io::Result<(Mapping, TcpStream)> {
        need(minor < 15, "paired notification endpoint required")?;
        let (mapping, socket, _) = self.accept_notified_while(minor, alive)?;
        Ok((mapping, socket))
    }
    pub fn accept_notified_while(
        self,
        minor: u64,
        mut alive: impl FnMut() -> io::Result<()>,
    ) -> io::Result<(Mapping, TcpStream, Option<crate::notification::Channel>)> {
        let Self {
            mut mapping,
            listener,
            capability,
            witness,
            session,
            declared_minor,
        } = self;
        need((minor >= 14) == (mapping.version == 3), "protocol/mapping generation differs")?;
        need(declared_minor.is_none_or(|declared| declared == minor)
            && (minor < 15 || declared_minor == Some(15)), "bootstrap protocol differs")?;
        let notifications = if minor == 15 { Some(crate::notification::Prepared::create()?) } else { None };
        let until = Instant::now() + Duration::from_secs(180);
        let mut socket: TcpStream = loop {
            alive()?;
            match listener.accept() {
                Ok((s, a)) => {
                    need(a.ip().is_loopback(), "nonlocal peer")?;
                    break s;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    need(Instant::now() < until, "setup timeout")?;
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(e),
            }
        };
        drop(listener);
        socket.set_nonblocking(false)?;
        socket.set_nodelay(true)?;
        let hello = receive_version(&mut socket, 10, minor)?;
        need(
            hello.kind == HELLO
                && hello.session == session
                && hello.sequence == 0
                && hello.payload.len() == 40,
            "Hello shape",
        )?;
        let mut difference = 0u8;
        for (a, b) in hello.payload[..32].iter().zip(capability) {
            difference |= *a ^ b;
        }
        need(
            difference == 0
                && get(&hello.payload[32..36]) == mapping.capacity as u64
                && get(&hello.payload[36..40]) == mapping.bytes as u64,
            "Hello authentication/layout",
        )?;
        barrier();
        need(
            get(&mapping.read(40, 8)?) == witness ^ WITNESS,
            "Windows mapping witness",
        )?;
        mapping.write(56, &(witness ^ WITNESS ^ 1).to_le_bytes())?;
        barrier();
        send_version(
            &mut socket,
            &Frame {
                kind: HELLO,
                session,
                sequence: 0,
                payload: if let Some(notification) = &notifications { notification.offer()? } else { vec![] },
            },
            10,
            minor,
        )?;
        let channel = notifications.map(|prepared| prepared.accept_while(session, minor, &mut alive)).transpose()?;
        Ok((mapping, socket, channel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::FileExt;
    #[test]
    fn explicit_bootstrap_pairs_notification_without_changing_mapping_generation() {
        let directory=std::env::temp_dir().join(format!("lvb-notification-bootstrap-{}-{}",std::process::id(),u64::from_le_bytes(random().unwrap())));
        std::fs::create_dir(&directory).unwrap();
        let session=[21;16];
        let endpoint=Prepared::with_protocol(&directory,session,MULTI_CHANNELS,15).unwrap();
        let config=std::fs::read(directory.join("ap1.control")).unwrap();
        assert_eq!(config.len(),60);assert_eq!(get(&config[2..4]),1);assert_eq!(get(&config[52..54]),15);assert_eq!(get(&config[54..56]),0);assert_eq!(get(&config[56..60]),60);
        assert_eq!(endpoint.mapping.version,3);
        let peer_directory=directory.clone();
        let peer=std::thread::spawn(move || {
            let file=OpenOptions::new().read(true).write(true).open(peer_directory.join("ap1.audio")).unwrap();
            let mut header=[0;64];file.read_exact_at(&mut header,0).unwrap();
            file.write_all_at(&(get(&header[32..40])^WITNESS).to_le_bytes(),40).unwrap();barrier();
            let mut socket=TcpStream::connect((std::net::Ipv4Addr::LOCALHOST,get(&config[0..2]) as u16)).unwrap();
            let mut payload=config[20..52].to_vec();payload.extend_from_slice(&(BLOCK_CAP as u32).to_le_bytes());payload.extend_from_slice(&(get(&header[16..20]) as u32).to_le_bytes());
            send_version(&mut socket,&Frame{kind:HELLO,session,sequence:0,payload},1,15).unwrap();
            let offer=receive_version(&mut socket,1,15).unwrap();
            assert_eq!((offer.kind,offer.session,offer.sequence),(HELLO,session,0));assert_eq!(offer.payload.len(),crate::notification::OFFER_BYTES);
            assert!(offer.payload[..4]==*b"LVBW"&&get(&offer.payload[4..8])==1&&get(&offer.payload[10..12])==0);
            let mut notification=TcpStream::connect((std::net::Ipv4Addr::LOCALHOST,get(&offer.payload[8..10]) as u16)).unwrap();
            let mut payload=vec![0;40];put(&mut payload[..4],1);put(&mut payload[4..8],1);payload[8..].copy_from_slice(&offer.payload[12..]);
            send_version(&mut notification,&Frame{kind:HELLO,session,sequence:0,payload},1,15).unwrap();
            let ack=receive_version(&mut notification,1,15).unwrap();assert_eq!((ack.kind,ack.session,ack.sequence),(HELLO,session,0));assert!(ack.payload.is_empty());
            let mut wake=[0];notification.set_read_timeout(Some(Duration::from_secs(2))).unwrap();notification.read_exact(&mut wake).unwrap();assert_eq!(wake,[crate::notification::WAKE]);
        });
        let (mapping,_socket,mut notifications)=endpoint.accept_notified_while(15,||Ok(())).unwrap();
        notifications.as_mut().unwrap().notify().unwrap();peer.join().unwrap();
        drop(notifications);mapping.close().unwrap();std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn product_bootstrap_refuses_legacy_or_unpaired_selection() {
        for unpaired in [false,true] {
            let directory=std::env::temp_dir().join(format!("lvb-notification-refusal-{}-{}",std::process::id(),u64::from_le_bytes(random().unwrap())));
            std::fs::create_dir(&directory).unwrap();
            let endpoint=Prepared::with_protocol(&directory,[22;16],MULTI_CHANNELS,15).unwrap();
            let result=if unpaired { endpoint.accept(15).map(|_|()) } else { endpoint.accept_notified_while(14,||Ok(())).map(|_|()) };
            assert!(result.is_err());std::fs::remove_dir_all(directory).unwrap();
        }
    }
}
