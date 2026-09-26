//! Offline, opt-in fixture binding. Never invoked by the product runtime.
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

fn digest(value: &str) -> io::Result<[u8; 32]> {
    if value.len() != 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "module digest extent",
        ));
    }
    let mut out = [0; 32];
    for (i, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let pair = std::str::from_utf8(pair)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "module digest encoding"))?;
        out[i] = u8::from_str_radix(pair, 16)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "module digest syntax"))?;
    }
    Ok(out)
}

fn run() -> io::Result<()> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 5 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
        "usage: lvb-private-state-bind REFERENCE_STATE MODULE_SHA256 INPUT_PAYLOAD OUTPUT_STATE"));
    }
    let reference = fs::read(Path::new(&args[1]))?;
    let module =
        digest(args[2].to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "module digest encoding")
        })?)?;
    let payload = fs::read(Path::new(&args[3]))?;
    let result = ap2_backend::bind_private_state(&reference, module, &payload)?;
    let path = Path::new(&args[4]);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    if let Err(e) = output.write_all(&result).and_then(|_| output.sync_all()) {
        drop(output);
        let _ = fs::remove_file(path);
        return Err(e);
    }
    println!(
        "bytes={} sha256={:x}",
        result.len(),
        Sha256::digest(&result)
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("private state binding refused: {e}");
        std::process::exit(1);
    }
}
