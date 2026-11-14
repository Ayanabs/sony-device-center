//! Request/response session over a byte transport (port of `SonyProtocolSession`).
//! Plain threads and condvars: no async runtime, to keep the daemon's RAM tiny.

use crate::error::{Result, SonyError, SonyErrorCode};
use crate::frame::{DataType, FrameCodec, SonyFrame};
use crate::transport::Transport;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub type NotificationCallback = Arc<dyn Fn(&SonyFrame) + Send + Sync>;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(2000);

struct Pending {
    expected_type: DataType,
    expected_opcode: u8,
    expected_subtype: i32,
    response: Option<SonyFrame>,
}

#[derive(Default)]
struct State {
    stream: Vec<u8>,
    last_seen_seq: Option<u8>,
    has_ack: bool,
    pending: Option<Pending>,
}

struct Inner {
    transport: Arc<dyn Transport>,
    running: AtomicBool,
    sequence: AtomicU8,
    state: Mutex<State>,
    ack_cv: Condvar,
    response_cv: Condvar,
    send_lock: Mutex<()>,
    write_lock: Mutex<()>,
    callbacks: Mutex<Vec<NotificationCallback>>,
}

pub struct Session {
    inner: Arc<Inner>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Session {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            inner: Arc::new(Inner {
                transport,
                running: AtomicBool::new(false),
                sequence: AtomicU8::new(0),
                state: Mutex::new(State::default()),
                ack_cv: Condvar::new(),
                response_cv: Condvar::new(),
                send_lock: Mutex::new(()),
                write_lock: Mutex::new(()),
                callbacks: Mutex::new(Vec::new()),
            }),
            reader: Mutex::new(None),
        }
    }

    pub fn connect(&self, address: &str) -> Result<()> {
        if self.is_connected() || self.inner.running.load(Ordering::SeqCst) {
            self.disconnect();
        }
        self.inner.transport.connect(address)?;
        self.start()
    }

    /// Starts the reader thread on an already-connected transport.
    pub fn start(&self) -> Result<()> {
        if !self.inner.transport.is_connected() {
            return Err(SonyError::new(SonyErrorCode::Disconnected, "Transport is not connected"));
        }
        if self.inner.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        {
            let mut st = lock(&self.inner.state);
            *st = State::default();
        }
        self.stop_reader();
        self.inner.running.store(true, Ordering::SeqCst);
        let inner = Arc::clone(&self.inner);
        *lock(&self.reader) = Some(thread::spawn(move || inner.reader_loop()));
        Ok(())
    }

    pub fn disconnect(&self) {
        self.stop_reader();
        self.inner.transport.disconnect();
        let mut st = lock(&self.inner.state);
        *st = State::default();
        self.inner.sequence.store(0, Ordering::SeqCst);
        self.inner.ack_cv.notify_all();
        self.inner.response_cv.notify_all();
    }

    pub fn is_connected(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst) && self.inner.transport.is_connected()
    }

    fn stop_reader(&self) {
        self.inner.running.store(false, Ordering::SeqCst);
        {
            let _g = lock(&self.inner.state);
            self.inner.ack_cv.notify_all();
            self.inner.response_cv.notify_all();
        }
        let handle = lock(&self.reader).take();
        if let Some(h) = handle {
            if h.thread().id() != thread::current().id() {
                let _ = h.join();
            }
        }
    }

    pub fn on_notification(&self, cb: NotificationCallback) {
        lock(&self.inner.callbacks).push(cb);
    }

    /// Send a frame and wait for the device ACK.
    pub fn send(&self, frame: &SonyFrame, timeout: Duration) -> Result<()> {
        self.require_connected("Transport not connected")?;
        let _send = lock(&self.inner.send_lock);
        let mut to_send = frame.clone();
        if is_data_mdr(to_send.data_type) {
            to_send.sequence = self.next_sequence();
        }
        lock(&self.inner.state).has_ack = false;
        self.inner.write_frame(&to_send)?;

        let deadline = Instant::now() + timeout;
        let mut st = lock(&self.inner.state);
        loop {
            if st.has_ack {
                st.has_ack = false;
                return Ok(());
            }
            if !self.inner.running.load(Ordering::SeqCst) || !self.inner.transport.is_connected() {
                return Err(SonyError::new(
                    SonyErrorCode::Disconnected,
                    "Transport disconnected while waiting for ACK",
                ));
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(SonyError::new(SonyErrorCode::Timeout, "Timeout waiting for ACK"));
            }
            st = self
                .inner
                .ack_cv
                .wait_timeout(st, deadline - now)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// Send a request and await the matching response (`ret_opcode`, and
    /// `ret_subtype` if >= 0), skipping unrelated notifications and ACKs.
    pub fn send_and_await(
        &self,
        request: &SonyFrame,
        ret_opcode: u8,
        ret_subtype: i32,
        timeout: Duration,
    ) -> Result<SonyFrame> {
        self.require_connected("Transport not connected")?;
        let _send = lock(&self.inner.send_lock);
        let mut to_send = request.clone();
        if is_data_mdr(to_send.data_type) {
            to_send.sequence = self.next_sequence();
        }
        {
            let mut st = lock(&self.inner.state);
            st.has_ack = false;
            // Frames received earlier belong to older inquiries; never reuse them.
            st.pending = Some(Pending {
                expected_type: to_send.data_type,
                expected_opcode: ret_opcode,
                expected_subtype: ret_subtype,
                response: None,
            });
        }
        if let Err(e) = self.inner.write_frame(&to_send) {
            lock(&self.inner.state).pending = None;
            return Err(e);
        }

        let deadline = Instant::now() + timeout;
        let mut st = lock(&self.inner.state);
        loop {
            if let Some(resp) = st.pending.as_mut().and_then(|p| p.response.take()) {
                st.pending = None;
                st.has_ack = false;
                return Ok(resp);
            }
            if !self.inner.running.load(Ordering::SeqCst) || !self.inner.transport.is_connected() {
                st.pending = None;
                return Err(SonyError::new(
                    SonyErrorCode::Disconnected,
                    "Transport disconnected while waiting for response",
                ));
            }
            let now = Instant::now();
            if now >= deadline {
                st.pending = None;
                return Err(SonyError::new(
                    SonyErrorCode::Timeout,
                    "Timeout waiting for response from device",
                ));
            }
            st = self
                .inner
                .response_cv
                .wait_timeout(st, deadline - now)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    fn require_connected(&self, msg: &str) -> Result<()> {
        if self.is_connected() {
            Ok(())
        } else {
            Err(SonyError::new(SonyErrorCode::Disconnected, msg))
        }
    }

    pub fn next_sequence(&self) -> u8 {
        let mut cur = self.inner.sequence.load(Ordering::SeqCst);
        if cur > 1 && cur < 254 {
            cur &= 1;
            self.inner.sequence.store(cur, Ordering::SeqCst);
        }
        self.inner.sequence.fetch_add(1, Ordering::SeqCst)
    }

    pub fn current_sequence(&self) -> u8 {
        self.inner.sequence.load(Ordering::SeqCst)
    }

    pub fn set_sequence(&self, seq: u8) {
        self.inner.sequence.store(seq, Ordering::SeqCst);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.disconnect();
    }
}

fn is_data_mdr(t: DataType) -> bool {
    matches!(t, DataType::DataMdr | DataType::DataMdrNo2)
}

impl Inner {
    fn notify_all(&self) {
        let _g = lock(&self.state);
        self.ack_cv.notify_all();
        self.response_cv.notify_all();
    }

    fn reader_loop(&self) {
        let mut buf = [0u8; 1024];
        while self.running.load(Ordering::SeqCst) {
            match self.transport.receive(&mut buf) {
                Ok(0) => break,
                Ok(n) => self.handle_bytes(&buf[..n]),
                Err(e) if e.code == SonyErrorCode::Timeout => {
                    if !self.running.load(Ordering::SeqCst) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(2));
                }
                Err(_) => break,
            }
        }
        self.running.store(false, Ordering::SeqCst);
        self.notify_all();
    }

    fn handle_bytes(&self, bytes: &[u8]) {
        let mut frames = Vec::new();
        {
            let mut st = lock(&self.state);
            st.stream.extend_from_slice(bytes);
            parse_stream(&mut st.stream, &mut frames);
        }
        for f in &frames {
            self.handle_frame(f);
        }
    }

    fn handle_frame(&self, frame: &SonyFrame) {
        if frame.data_type == DataType::Ack {
            let mut st = lock(&self.state);
            st.has_ack = true;
            if frame.sequence <= 1 {
                self.sequence.store(frame.sequence, Ordering::SeqCst);
            }
            self.ack_cv.notify_all();
            self.response_cv.notify_all();
            return;
        }
        if !is_data_mdr(frame.data_type) {
            return;
        }

        // ACK with the toggled 1-bit sequence.
        let _ = self.write_frame(&SonyFrame {
            data_type: DataType::Ack,
            sequence: 1 - (frame.sequence & 1),
            payload: vec![],
        });

        {
            let mut st = lock(&self.state);
            if st.last_seen_seq == Some(frame.sequence) {
                return; // duplicate
            }
            st.last_seen_seq = Some(frame.sequence);

            if let Some(p) = st.pending.as_mut() {
                if p.response.is_none()
                    && frame.data_type == p.expected_type
                    && frame.payload.first() == Some(&p.expected_opcode)
                    && (p.expected_subtype < 0
                        || frame.payload.get(1) == Some(&(p.expected_subtype as u8)))
                {
                    p.response = Some(frame.clone());
                    self.response_cv.notify_all();
                    return;
                }
            }
        }

        let cbs: Vec<_> = lock(&self.callbacks).clone();
        for cb in cbs {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cb(frame)));
        }
    }

    fn write_frame(&self, frame: &SonyFrame) -> Result<()> {
        let _w = lock(&self.write_lock);
        if !self.transport.is_connected() {
            return Err(SonyError::new(SonyErrorCode::Disconnected, "Transport not connected"));
        }
        let encoded = FrameCodec::encode(frame)?;
        let mut sent = 0;
        while sent < encoded.len() {
            match self.transport.send(&encoded[sent..]) {
                Ok(0) => {
                    self.running.store(false, Ordering::SeqCst);
                    self.notify_all();
                    return Err(SonyError::new(
                        SonyErrorCode::Disconnected,
                        "Bluetooth send returned EOF",
                    ));
                }
                Ok(n) => sent += n,
                Err(e) => {
                    if e.code != SonyErrorCode::Timeout {
                        self.running.store(false, Ordering::SeqCst);
                        self.notify_all();
                    }
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

/// Extracts complete frames from `buf`, resynchronising on corrupt data.
fn parse_stream(buf: &mut Vec<u8>, out: &mut Vec<SonyFrame>) {
    loop {
        match buf.iter().position(|&b| b == FrameCodec::START_MARKER) {
            None => {
                buf.clear();
                return;
            }
            Some(0) => {}
            Some(i) => {
                buf.drain(..i);
            }
        }
        let mut end = None;
        let mut restart = None;
        for (i, &b) in buf.iter().enumerate().skip(1) {
            if b == FrameCodec::START_MARKER {
                restart = Some(i);
                break;
            }
            if b == FrameCodec::END_MARKER {
                end = Some(i);
                break;
            }
        }
        if let Some(i) = restart {
            buf.drain(..i);
            continue;
        }
        let Some(end) = end else {
            if buf.len() > FrameCodec::MAX_FRAME_SIZE * 2 {
                buf.remove(0);
            }
            return;
        };
        let slice: Vec<u8> = buf.drain(..=end).collect();
        if let Ok(f) = FrameCodec::decode(&slice) {
            out.push(f);
        } // corrupt frames are dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stream_resyncs_and_buffers_partials() {
        let good = FrameCodec::encode(&SonyFrame {
            data_type: DataType::DataMdr,
            sequence: 1,
            payload: vec![1, 2, 3],
        })
        .unwrap();
        let mut buf = vec![0x00, 0x11];
        buf.extend(&good[..4]); // partial
        let mut out = vec![];
        parse_stream(&mut buf, &mut out);
        assert!(out.is_empty());
        buf.extend(&good[4..]);
        parse_stream(&mut buf, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].payload, vec![1, 2, 3]);
        assert!(buf.is_empty());
    }

    #[test]
    fn parse_stream_drops_corrupt_frame() {
        let mut bad = FrameCodec::encode(&SonyFrame::default()).unwrap();
        let n = bad.len();
        bad[n - 2] ^= 0xff;
        let mut out = vec![];
        let mut buf = bad;
        parse_stream(&mut buf, &mut out);
        assert!(out.is_empty());
    }
}
