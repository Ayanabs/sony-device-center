use crate::error::Result;

/// Byte transport to a headset (RFCOMM on Linux). Methods take `&self` so the
/// session's reader thread can `receive` while another thread `send`s.
pub trait Transport: Send + Sync {
    fn connect(&self, address: &str) -> Result<()>;
    fn disconnect(&self);
    fn is_connected(&self) -> bool;
    fn send(&self, data: &[u8]) -> Result<usize>;
    /// Blocks up to a short timeout. `Ok(0)` means EOF; a timeout is `Err(Timeout)`.
    fn receive(&self, buf: &mut [u8]) -> Result<usize>;
}
