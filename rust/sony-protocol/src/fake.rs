//! In-memory transport with a scripted "headset", for tests and `--simulated` runs.

use crate::error::{Result, SonyError, SonyErrorCode};
use crate::frame::{DataType, FrameCodec, SonyFrame};
use crate::transport::Transport;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// Maps a request payload (+ its data type) to the replies the headset sends.
pub type Responder = Box<dyn Fn(DataType, &[u8]) -> Vec<(DataType, Vec<u8>)> + Send + Sync>;

pub struct FakeDevice {
    connected: AtomicBool,
    inbound: Mutex<VecDeque<u8>>,
    cv: Condvar,
    out_seq: AtomicU8,
    responder: Responder,
    sent: Mutex<Vec<SonyFrame>>,
}

impl FakeDevice {
    pub fn new(responder: Responder) -> Self {
        Self {
            connected: AtomicBool::new(false),
            inbound: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
            out_seq: AtomicU8::new(0),
            responder,
            sent: Mutex::new(Vec::new()),
        }
    }

    /// Every non-ACK frame the host has sent so far.
    pub fn sent_frames(&self) -> Vec<SonyFrame> {
        self.sent.lock().unwrap().clone()
    }

    /// Inject an unsolicited frame from the headset.
    pub fn push_notification(&self, data_type: DataType, payload: Vec<u8>) {
        self.push_frame(data_type, payload);
    }

    fn push_frame(&self, data_type: DataType, payload: Vec<u8>) {
        let seq = self.out_seq.fetch_xor(1, Ordering::SeqCst);
        let wire = FrameCodec::encode(&SonyFrame { data_type, sequence: seq, payload }).unwrap();
        self.inbound.lock().unwrap().extend(wire);
        self.cv.notify_all();
    }
}

impl Transport for FakeDevice {
    fn connect(&self, _address: &str) -> Result<()> {
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn disconnect(&self) {
        self.connected.store(false, Ordering::SeqCst);
        self.cv.notify_all();
    }
    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
    fn send(&self, data: &[u8]) -> Result<usize> {
        let frame = FrameCodec::decode(data)?;
        if frame.data_type == DataType::Ack {
            return Ok(data.len());
        }
        self.sent.lock().unwrap().push(frame.clone());
        // ACKs carry the toggled host sequence and don't use the data counter.
        let ack = SonyFrame { data_type: DataType::Ack, sequence: 1 - (frame.sequence & 1), payload: vec![] };
        self.inbound.lock().unwrap().extend(FrameCodec::encode(&ack).unwrap());
        self.cv.notify_all();
        for (t, p) in (self.responder)(frame.data_type, &frame.payload) {
            self.push_frame(t, p);
        }
        Ok(data.len())
    }
    fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        let mut q = self.inbound.lock().unwrap();
        if q.is_empty() {
            q = self.cv.wait_timeout(q, Duration::from_millis(20)).unwrap().0;
        }
        if !self.is_connected() {
            return Ok(0);
        }
        if q.is_empty() {
            return Err(SonyError::new(SonyErrorCode::Timeout, "receive timeout"));
        }
        let n = buf.len().min(q.len());
        for b in buf.iter_mut().take(n) {
            *b = q.pop_front().unwrap();
        }
        Ok(n)
    }
}
