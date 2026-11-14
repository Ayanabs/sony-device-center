//! Linux Bluetooth RFCOMM transport (BlueZ), for Sony's SPP-style service.

mod sdp;

use sony_protocol::{Result, SonyError, SonyErrorCode, Transport};
use std::sync::atomic::{AtomicI32, Ordering};

pub use sdp::{ProtocolKind, SERVICE_UUID_V1, SERVICE_UUID_V2};

const SOL_RFCOMM: libc::c_int = 18;
const RFCOMM_LM: libc::c_int = 3;
const RFCOMM_LM_AUTH: u32 = 0x0002;
const RFCOMM_LM_ENCRYPT: u32 = 0x0004;
const POLL_TIMEOUT_MS: libc::c_int = 200;

#[repr(C)]
struct SockaddrRc {
    rc_family: libc::sa_family_t,
    rc_bdaddr: [u8; 6],
    rc_channel: u8,
}

fn fail(code: SonyErrorCode, msg: impl Into<String>) -> SonyError {
    SonyError::new(code, msg)
}

/// Parses "AA:BB:CC:DD:EE:FF" into BlueZ's little-endian `bdaddr_t` byte order.
pub(crate) fn parse_bdaddr(s: &str) -> Option<[u8; 6]> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return None;
    }
    let mut out = [0u8; 6];
    for (i, p) in parts.iter().enumerate() {
        out[5 - i] = u8::from_str_radix(p, 16).ok()?;
    }
    Some(out)
}

pub struct RfcommTransport {
    fd: AtomicI32,
    kind: std::sync::Mutex<Option<ProtocolKind>>,
}

impl Default for RfcommTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl RfcommTransport {
    pub fn new() -> Self {
        Self { fd: AtomicI32::new(-1), kind: std::sync::Mutex::new(None) }
    }

    /// Which Sony service generation the last `connect` found.
    pub fn protocol_kind(&self) -> Option<ProtocolKind> {
        *self.kind.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Transport for RfcommTransport {
    fn connect(&self, address: &str) -> Result<()> {
        self.disconnect();
        let bdaddr = parse_bdaddr(address)
            .ok_or_else(|| fail(SonyErrorCode::TransportFailure, "Invalid Bluetooth address"))?;
        let (kind, channel) = sdp::find_sony_channel(address).ok_or_else(|| {
            fail(
                SonyErrorCode::TransportFailure,
                "Couldn't find the Sony service record on this device - is this a supported headset?",
            )
        })?;

        // SAFETY: plain libc socket calls on a descriptor we own.
        unsafe {
            let fd = libc::socket(31 /* AF_BLUETOOTH */, libc::SOCK_STREAM, 3 /* BTPROTO_RFCOMM */);
            if fd < 0 {
                return Err(fail(SonyErrorCode::TransportFailure, "Could not create Bluetooth socket"));
            }
            let lm: u32 = RFCOMM_LM_AUTH | RFCOMM_LM_ENCRYPT;
            libc::setsockopt(
                fd,
                SOL_RFCOMM,
                RFCOMM_LM,
                &lm as *const _ as *const libc::c_void,
                std::mem::size_of::<u32>() as libc::socklen_t,
            );
            let addr = SockaddrRc { rc_family: 31, rc_bdaddr: bdaddr, rc_channel: channel };
            let rc = libc::connect(
                fd,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of::<SockaddrRc>() as libc::socklen_t,
            );
            if rc < 0 {
                libc::close(fd);
                return Err(fail(
                    SonyErrorCode::TransportFailure,
                    format!("Could not connect Bluetooth socket: {}", std::io::Error::last_os_error()),
                ));
            }
            self.fd.store(fd, Ordering::SeqCst);
        }
        *self.kind.lock().unwrap_or_else(|e| e.into_inner()) = Some(kind);
        Ok(())
    }

    /// Callers must stop concurrent `receive` first (`Session::disconnect` does).
    fn disconnect(&self) {
        let fd = self.fd.swap(-1, Ordering::SeqCst);
        if fd >= 0 {
            // SAFETY: fd was a valid socket owned by this transport.
            unsafe {
                libc::shutdown(fd, libc::SHUT_RDWR);
                libc::close(fd);
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.fd.load(Ordering::SeqCst) >= 0
    }

    fn send(&self, data: &[u8]) -> Result<usize> {
        let fd = self.fd.load(Ordering::SeqCst);
        if fd < 0 {
            return Err(fail(SonyErrorCode::Disconnected, "Not connected"));
        }
        // SAFETY: data is a valid slice for the call's duration.
        let n = unsafe { libc::send(fd, data.as_ptr().cast(), data.len(), libc::MSG_NOSIGNAL) };
        if n < 0 {
            let e = std::io::Error::last_os_error();
            return Err(fail(SonyErrorCode::TransportFailure, format!("Bluetooth send failed: {e}")));
        }
        Ok(n as usize)
    }

    fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        let fd = self.fd.load(Ordering::SeqCst);
        if fd < 0 {
            return Err(fail(SonyErrorCode::Disconnected, "Not connected"));
        }
        let mut pfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
        // SAFETY: pfd/buf are valid for the duration of each call.
        unsafe {
            let r = libc::poll(&mut pfd, 1, POLL_TIMEOUT_MS);
            if r < 0 {
                let e = std::io::Error::last_os_error();
                if e.kind() == std::io::ErrorKind::Interrupted {
                    return Err(fail(SonyErrorCode::Timeout, "interrupted"));
                }
                return Err(fail(SonyErrorCode::TransportFailure, format!("poll failed: {e}")));
            }
            if r == 0 {
                return Err(fail(SonyErrorCode::Timeout, "receive timeout"));
            }
            if pfd.revents & libc::POLLIN == 0 {
                return Ok(0); // HUP / ERR / NVAL: treat as EOF
            }
            let n = libc::recv(fd, buf.as_mut_ptr().cast(), buf.len(), 0);
            if n < 0 {
                let e = std::io::Error::last_os_error();
                return Err(fail(SonyErrorCode::TransportFailure, format!("Bluetooth recv failed: {e}")));
            }
            Ok(n as usize)
        }
    }
}

impl Drop for RfcommTransport {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bdaddr_is_little_endian() {
        assert_eq!(parse_bdaddr("A8:E6:E8:EA:71:E9"), Some([0xe9, 0x71, 0xea, 0xe8, 0xe6, 0xa8]));
        assert_eq!(parse_bdaddr("nope"), None);
        assert_eq!(parse_bdaddr("A8:E6:E8:EA:71:ZZ"), None);
    }

    #[test]
    fn disconnected_transport_reports_so() {
        let t = RfcommTransport::new();
        assert!(!t.is_connected());
        assert_eq!(t.send(&[1]).unwrap_err().code, SonyErrorCode::Disconnected);
        t.disconnect(); // idempotent
    }
}

/// Cheap check: does this device expose a Sony service (SDP only, no connect)?
pub fn probe(address: &str) -> Option<ProtocolKind> {
    sdp::find_sony_channel(address).map(|(k, _)| k)
}
