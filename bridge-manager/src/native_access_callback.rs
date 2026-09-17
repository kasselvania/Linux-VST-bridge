//! Opaque browser return transport. Never an operator request or launch authority.
use crate::*;
#[cfg(any(target_os = "linux", test))]
use std::io::{Read, Write};
#[cfg(any(target_os = "linux", test))]
use std::os::unix::net::UnixStream;
#[cfg(any(target_os = "linux", test))]
use std::time::Duration;
pub const LIMIT: usize = 2048;
/// Keep the URL opaque. Admit a single ASCII URI argument, never shell syntax.
pub fn valid_uri(value: &[u8]) -> bool {
    if value.len() <= 14 || value.len() > LIMIT || !value.starts_with(b"native-access:") {
        return false;
    }
    let mut i = 14;
    while i < value.len() {
        let c = value[i];
        if !(0x21..=0x7e).contains(&c) || matches!(c, b'"' | b'\'' | b'\\' | b'`' | b'<' | b'>') {
            return false;
        }
        if c == b'%' {
            if i + 2 >= value.len() {
                return false;
            }
            let hex = |c: u8| match c {
                b'0'..=b'9' => Some(c - b'0'),
                b'a'..=b'f' => Some(c - b'a' + 10),
                b'A'..=b'F' => Some(c - b'A' + 10),
                _ => None,
            };
            let (Some(a), Some(b)) = (hex(value[i + 1]), hex(value[i + 2])) else {
                return false;
            };
            if a * 16 + b < 0x20 || a * 16 + b == 0x7f {
                return false;
            }
            i += 2;
        }
        i += 1;
    }
    true
}
#[cfg(any(target_os = "linux", test))]
fn exchange(peer: &mut UnixStream, operation: &str, url: &[u8]) -> Result<()> {
    require(
        valid_hex(operation, 32) && valid_uri(url),
        "native_access_callback_invalid",
    )?;
    peer.set_read_timeout(Some(Duration::from_secs(25)))?;
    peer.set_write_timeout(Some(Duration::from_secs(3)))?;
    peer.write_all(operation.as_bytes())?;
    let mut ready = [0; 4];
    peer.read_exact(&mut ready)?;
    require(&ready == b"NAC1", "native_access_callback_not_ready")?;
    peer.write_all(&(url.len() as u32).to_le_bytes())?;
    peer.write_all(url)?;
    let mut result = [0; 1];
    peer.read_exact(&mut result)?;
    require(result == [0], "native_access_callback_delivery_unconfirmed")
}
#[cfg(target_os = "linux")]
pub fn deliver(m: &Manager, url: &str) -> Result<()> {
    use std::os::unix::net::SocketAddr;
    use std::os::{fd::AsRawFd, linux::net::SocketAddrExt};
    require(valid_uri(url.as_bytes()), "native_access_callback_invalid")?;
    let c = renderer_session::current(m)?;
    let op = c["operation"]
        .as_str()
        .ok_or("native_access_callback_no_operation")?;
    require(valid_hex(op, 32), "native_access_callback_operation")?;
    let d = renderer_session::operation_dir(m, op)?;
    let spec: serde_json::Value = read_json(&d.join("spec.json"))?;
    let sw: catalogue::Software = read_json(&m.root.join("software.json"))?;
    sw.manager.verify()?;
    sw.supervisor.verify()?;
    sw.ownership.verify()?;
    sw.installer_launch
        .as_ref()
        .ok_or("native_access_callback_adapter")?
        .verify()?;
    require(
        spec["software"] == serde_json::to_value(&sw)?
            && spec["operation"] == op
            && spec["kind"] == "renderer_application"
            && spec["application_identity"] == c["application"],
        "native_access_callback_generation",
    )?;
    let app: renderer_application::Application =
        serde_json::from_value(spec["application"].clone())?;
    app.verify(&m.root)?;
    require(
        app.identity()? == c["application"],
        "native_access_callback_application",
    )?;
    let result = renderer_session::result(m, op)?;
    require(
        result["state"] == "running"
            && result["effective"]["operation"] == op
            && result["renderer"]["launch_binding"]["status"] == "bound",
        "native_access_callback_not_live",
    )?;
    let uid = unsafe { libc::getuid() };
    let address = SocketAddr::from_abstract_name(format!("lvb-native-access-{uid}-{op}"))?;
    let mut peer = UnixStream::connect_addr(&address)?;
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of_val(&cred) as libc::socklen_t;
    require(
        unsafe {
            libc::getsockopt(
                peer.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut cred as *mut libc::ucred).cast(),
                &mut length,
            )
        } == 0
            && cred.uid == uid
            && cred.pid > 0,
        "native_access_callback_peer",
    )?;
    // The receiver must be a member of this exact renderer operation, not merely
    // another same-user socket. No Linux/Windows PID equivalence is involved.
    let mut bytes = Vec::new();
    fs::File::open(format!("/proc/{}/cgroup", cred.pid))?
        .take(16385)
        .read_to_end(&mut bytes)?;
    require(bytes.len() <= 16384, "native_access_callback_cgroup_extent")?;
    let suffix = format!("/{}", renderer_session::unit(op)?);
    require(
        std::str::from_utf8(&bytes)?
            .lines()
            .any(|l| l.starts_with("0::") && l.ends_with(&suffix)),
        "native_access_callback_owner",
    )?;
    renderer_session::exact(m, op)?;
    exchange(&mut peer, op, url.as_bytes())
}
#[cfg(not(target_os = "linux"))]
pub fn deliver(_m: &Manager, _url: &str) -> Result<()> {
    Err("native_access_callback_linux_only".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_uri_has_closed_extent_and_no_argument_injection() {
        for good in [
            "native-access://auth?code=fixture&state=test",
            "native-access:fixture%2Btest",
        ] {
            assert!(valid_uri(good.as_bytes()));
        }
        for bad in [
            "https://example.test/",
            "native-access:",
            "native-access:x --no-sandbox",
            "native-access:\"x",
            "native-access:x\\",
            "native-access:x\n",
            "native-access:%00",
            "native-access:%7f",
            "native-access:%a",
            "native-access:%xz",
        ] {
            assert!(!valid_uri(bad.as_bytes()), "{bad}");
        }
        assert!(!valid_uri(
            format!("native-access:{}", "a".repeat(LIMIT)).as_bytes()
        ));
    }
    #[test]
    fn only_live_owner_acknowledgment_allows_payload_transfer() {
        for accepted in [false, true] {
            let (mut a, mut b) = UnixStream::pair().unwrap();
            let t = std::thread::spawn(move || {
                let mut operation = [0; 32];
                b.read_exact(&mut operation).unwrap();
                assert_eq!(&operation, b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
                b.write_all(if accepted { b"NAC1" } else { b"NOPE" })
                    .unwrap();
                if accepted {
                    let mut n = [0; 4];
                    b.read_exact(&mut n).unwrap();
                    let mut value = vec![0; u32::from_le_bytes(n) as usize];
                    b.read_exact(&mut value).unwrap();
                    assert_eq!(value, b"native-access:fixture");
                    b.write_all(&[0]).unwrap();
                }
            });
            assert_eq!(
                exchange(&mut a, &"a".repeat(32), b"native-access:fixture").is_ok(),
                accepted
            );
            t.join().unwrap();
        }
    }
}
