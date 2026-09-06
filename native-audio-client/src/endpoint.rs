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
}
impl Prepared {
    pub fn create(path: &Path, session: [u8; 16]) -> io::Result<Self> {
        let mut mapping = Mapping::new(&path.join("ap1.audio"))?;
        let capability = random::<32>()?;
        let witness = u64::from_le_bytes(random::<8>()?);
        let mut header = [0; 64];
        for (o, v) in [
            (0, 0x4d315041),
            (4, 1),
            (8, CAP as u64),
            (12, 2),
            (16, MAP_BYTES as u64),
            (20, INPUT as u64),
            (24, OUTPUT as u64),
            (28, STRIDE as u64),
        ] {
            put(&mut header[o..o + 4], v);
        }
        put(&mut header[32..40], witness);
        mapping.write(0, &header)?;
        barrier();
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let mut config = vec![0; 52];
        put(&mut config[0..2], listener.local_addr()?.port() as u64);
        config[4..20].copy_from_slice(&session);
        config[20..52].copy_from_slice(&capability);
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
        })
    }
    pub fn accept(self, minor: u64) -> io::Result<(Mapping, TcpStream)> {
        self.accept_while(minor, || Ok(()))
    }
    pub fn accept_while(
        self,
        minor: u64,
        mut alive: impl FnMut() -> io::Result<()>,
    ) -> io::Result<(Mapping, TcpStream)> {
        let Self {
            mut mapping,
            listener,
            capability,
            witness,
            session,
        } = self;
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
                && get(&hello.payload[32..36]) == CAP as u64
                && get(&hello.payload[36..40]) == MAP_BYTES as u64,
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
                payload: vec![],
            },
            10,
            minor,
        )?;
        Ok((mapping, socket))
    }
}
