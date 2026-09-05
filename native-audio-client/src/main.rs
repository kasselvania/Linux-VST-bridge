use ap1_native_client::*;
use std::ffi::c_void;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::{fs::OpenOptionsExt, io::AsRawFd},
    path::{Path, PathBuf},
    ptr::NonNull,
    time::{Duration, Instant},
};
extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
}
struct Mapping {
    pointer: NonNull<u8>,
    _file: File,
    unmapped: bool,
}
impl Mapping {
    fn new(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.set_len(MAP_BYTES as u64)?;
        // MAP_SHARED=1, PROT_READ|PROT_WRITE=3 on this Linux/x86-64 target.
        let p = unsafe { mmap(std::ptr::null_mut(), MAP_BYTES, 3, 1, file.as_raw_fd(), 0) };
        need(p as isize != -1, "mapping failed")?;
        Ok(Self {
            pointer: NonNull::new(p as *mut u8).ok_or_else(|| invalid("null mapping"))?,
            _file: file,
            unmapped: false,
        })
    }
    fn close(mut self) -> io::Result<()> {
        let result = unsafe { munmap(self.pointer.as_ptr() as *mut c_void, MAP_BYTES) };
        need(result == 0, "native mapping unmap failed")?;
        self.unmapped = true;
        Ok(())
    }
    fn write(&mut self, offset: usize, b: &[u8]) -> io::Result<()> {
        need(
            offset.checked_add(b.len()).is_some_and(|n| n <= MAP_BYTES),
            "mapping write bounds",
        )?;
        // No references into the shared view escape. Called only while Linux owns it.
        unsafe {
            std::ptr::copy_nonoverlapping(b.as_ptr(), self.pointer.as_ptr().add(offset), b.len())
        };
        Ok(())
    }
    fn read(&self, offset: usize, n: usize) -> io::Result<Vec<u8>> {
        need(
            offset.checked_add(n).is_some_and(|end| end <= MAP_BYTES),
            "mapping read bounds",
        )?;
        let mut b = vec![0; n];
        unsafe {
            std::ptr::copy_nonoverlapping(self.pointer.as_ptr().add(offset), b.as_mut_ptr(), n)
        };
        Ok(b)
    }
    fn write_plane(&mut self, base: usize, ch: usize, words: &[u32; CAP + 2]) -> io::Result<()> {
        let bytes: Vec<_> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        self.write(base + ch * STRIDE, &bytes)
    }
    fn plane(&self, base: usize, ch: usize) -> io::Result<[u32; CAP + 2]> {
        let bytes = self.read(base + ch * STRIDE, STRIDE)?;
        Ok(std::array::from_fn(|i| {
            u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap())
        }))
    }
}
impl Drop for Mapping {
    fn drop(&mut self) {
        if !self.unmapped {
            unsafe { munmap(self.pointer.as_ptr() as *mut c_void, MAP_BYTES) };
        }
    }
}
fn barrier() {
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    #[cfg(target_arch = "x86_64")]
    unsafe {
        std::arch::asm!("mfence", options(nostack, preserves_flags));
    }
    #[cfg(not(target_arch = "x86_64"))]
    std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}
fn random<const N: usize>() -> io::Result<[u8; N]> {
    let mut b = [0; N];
    File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b)
}
fn record(text: String) -> io::Result<()> {
    let mut out = io::stdout().lock();
    out.write_all(text.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()
}
fn words(a: &[[u32; CAP + 2]; 2]) -> String {
    format!("[{:?},{:?}]", a[0], a[1])
}
fn run(stage: &mut &'static str) -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    need(
        args.len() == 5 && args[1] == "--session-dir" && args[3] == "--session",
        "arguments",
    )?;
    let path = PathBuf::from(&args[2]);
    let id = &args[4];
    need(
        id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()),
        "session syntax",
    )?;
    let session: [u8; 16] =
        std::array::from_fn(|i| u8::from_str_radix(&id[2 * i..2 * i + 2], 16).unwrap());
    *stage = "mapping_setup";
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
    *stage = "hello";
    let until = Instant::now() + Duration::from_secs(180);
    let mut socket: TcpStream = loop {
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
    let hello = receive(&mut socket, 10)?;
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
    send(
        &mut socket,
        &Frame {
            kind: HELLO,
            session,
            sequence: 0,
            payload: vec![],
        },
        10,
    )?;
    *stage = "ready";
    let ready = receive(&mut socket, 15)?;
    need(
        ready.kind == READY
            && ready.session == session
            && ready.sequence == 0
            && ready.payload.is_empty(),
        "Ready differs",
    )?;
    // Chosen only after the real endpoint is activated/ready. Never sent to Windows.
    let seed = u64::from_le_bytes(random::<8>()?);
    record(format!("{{\"event\":\"ap1_client_ready\",\"seed\":{seed},\"mapping_count\":1,\"connection_count\":1,\"mapping_witness\":true,\"seed_chosen_after_ready\":true}}"))?;
    let mut state = ClientState {
        session,
        next: 1,
        slot: Slot::Writable,
    };
    let mut count = 0;
    for b in 0..LENGTHS.len() {
        *stage = "process";
        need(state.slot == Slot::Writable, "slot not writable")?;
        let frames = LENGTHS[b];
        let input = [recipe(seed, b, 0), recipe(seed, b, 1)];
        let mut poison = [POISON; CAP + 2];
        poison[0] = GUARD;
        poison[CAP + 1] = GUARD;
        for ch in 0..2 {
            mapping.write_plane(INPUT, ch, &input[ch])?;
            mapping.write_plane(OUTPUT, ch, &poison)?;
        }
        barrier();
        let request = state.process(frames, GAINS[b], b == 5)?;
        // Failure latches ownership before any read/reuse. There is no resend path.
        let exchange = send(&mut socket, &request, 5)
            .and_then(|_| receive(&mut socket, 5))
            .and_then(|f| state.done(&f));
        if let Err(error) = exchange {
            state.failed();
            return Err(error);
        }
        barrier();
        let returned_input = [mapping.plane(INPUT, 0)?, mapping.plane(INPUT, 1)?];
        let output = [mapping.plane(OUTPUT, 0)?, mapping.plane(OUTPUT, 1)?];
        let comparison = compare(&input, &output, frames, GAINS[b]);
        let error = comparison
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or("null".into());
        count += frames * 2;
        record(format!("{{\"event\":\"ap1_client_block\",\"sequence\":{},\"frames\":{frames},\"gain\":{},\"silent\":{},\"input_bits\":{},\"output_bits\":{},\"maximum_absolute_error\":{error}}}",request.sequence,GAINS[b],b==5,words(&returned_input),words(&output)))?;
        // Retain actual mapped words before either validation can fail.
        need(returned_input == input, "input/guard modified")?;
        comparison?;
    }
    *stage = "close";
    send(&mut socket, &state.close()?, 5)?;
    let closed = receive(&mut socket, 10)?;
    state.closed(&closed)?;
    drop(socket);
    mapping.close()?;
    *stage = "report";
    record(format!("{{\"event\":\"ap1_client_closed\",\"blocks\":8,\"samples_compared\":{count},\"maximum_absolute_error\":0.0,\"mapping_unmapped\":true,\"closed_received\":true,\"replays\":0}}"))?;
    Ok(())
}
fn main() {
    let mut stage = "setup";
    if let Err(error) = run(&mut stage) {
        let text = format!(
            "{{\"event\":\"ap1_client_error\",\"stage\":{stage:?},\"detail\":{:?}}}",
            error.to_string()
        );
        if record(text).is_err() {
            eprintln!("AP1 diagnostic persistence failed; earlier checkpoints retained");
        }
        std::process::exit(1);
    }
}
