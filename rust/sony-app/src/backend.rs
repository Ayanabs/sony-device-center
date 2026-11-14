//! Worker thread that owns the headset connection. The UI sends `Request`s and
//! receives `Update`s through a callback, so this file has no Qt in it.

use crate::bluez_watch;
use crate::device_core::{Command, Link, Snapshot};
use sony_protocol::profile::{identify_model, SonyModel};
use sony_protocol::Transport;
use std::process::Command as Proc;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct PairedDevice {
    pub name: String,
    pub address: String,
}

pub enum Request {
    Command(Command),
    Connect { address: String, name: String },
    Disconnect,
    Discover,
    Shutdown,
}

pub enum Update {
    Snapshot(Snapshot),
    Devices(Vec<PairedDevice>),
    Error(String),
    /// A request finished (successfully or not): the UI can stop showing "busy".
    Completed,
}

enum Msg {
    Req(Request),
    Notify(Vec<u8>),
    /// BlueZ says this device just connected: time to look for a Sony headset.
    BluezUp,
    /// The BlueZ watcher stopped; fall back to slow polling.
    WatchEnded,
}

pub type TransportFactory = Arc<dyn Fn() -> Arc<dyn Transport> + Send + Sync>;
pub type Emit = Arc<dyn Fn(Update) + Send + Sync>;

pub struct Backend {
    tx: Sender<Msg>,
    thread: Option<JoinHandle<()>>,
}

/// Used only when the BlueZ watcher is unavailable.
const FALLBACK_POLL: Duration = Duration::from_secs(30);
/// After a device connects its SDP records may not be readable yet: retry a few
/// times with growing delays, then give up until the next connect event.
const RETRY_DELAYS_MS: [u64; 6] = [400, 1000, 2000, 3000, 5000, 8000];

impl Backend {
    /// `auto_connect` looks for an already-connected Sony headset on start-up
    /// and whenever the link drops.
    pub fn start(factory: TransportFactory, emit: Emit, auto_connect: bool) -> Backend {
        let (tx, rx) = mpsc::channel::<Msg>();
        let ntx = tx.clone();
        let thread = thread::spawn(move || Worker { factory, emit, ntx, link: None, auto_connect, watch_pid: None, retry_at: None, attempt: 0 }.run(rx));
        Backend { tx, thread: Some(thread) }
    }

    pub fn request(&self, r: Request) {
        let _ = self.tx.send(Msg::Req(r));
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Req(Request::Shutdown));
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct Worker {
    factory: TransportFactory,
    emit: Emit,
    ntx: Sender<Msg>,
    link: Option<Link>,
    auto_connect: bool,
    watch_pid: Option<u32>,
    /// Next connect attempt after a BlueZ event, and how many attempts were made.
    retry_at: Option<Instant>,
    attempt: usize,
}

impl Worker {
    fn publish(&self) {
        let snap = match &self.link {
            Some(l) => l.snapshot().clone(),
            None => Snapshot::disconnected("disconnected"),
        };
        (self.emit)(Update::Snapshot(snap));
    }

    fn connect(&mut self, address: &str, name: &str) {
        self.drop_link();
        (self.emit)(Update::Snapshot(Snapshot::disconnected("connecting")));
        match Link::open((self.factory)(), address, name) {
            Ok(link) => {
                let ntx = self.ntx.clone();
                link.session.on_notification(Arc::new(move |f| {
                    let _ = ntx.send(Msg::Notify(f.payload.clone()));
                }));
                self.link = Some(link);
            }
            Err(e) => (self.emit)(Update::Error(e)),
        }
        self.publish();
    }

    fn drop_link(&mut self) {
        if let Some(l) = self.link.take() {
            l.close();
        }
    }

    fn try_auto_connect(&mut self) {
        if let Some((address, name)) = find_connected_sony() {
            self.connect(&address, &name);
        }
    }

    fn start_watch(&mut self) {
        let (up, end) = (self.ntx.clone(), self.ntx.clone());
        self.watch_pid = bluez_watch::spawn(
            move |_addr| {
                let _ = up.send(Msg::BluezUp);
            },
            move || {
                let _ = end.send(Msg::WatchEnded);
            },
        );
    }

    fn schedule_attempt(&mut self) {
        let d = RETRY_DELAYS_MS.get(self.attempt).copied().unwrap_or(0);
        self.retry_at = (self.attempt < RETRY_DELAYS_MS.len())
            .then(|| Instant::now() + Duration::from_millis(d));
    }

    fn run(mut self, rx: mpsc::Receiver<Msg>) {
        (self.emit)(Update::Snapshot(Snapshot::disconnected("searching")));
        (self.emit)(Update::Devices(list_paired()));
        if self.auto_connect {
            self.start_watch();
            // One look now, in case the headset is already connected.
            self.try_auto_connect();
        }
        (self.emit)(Update::Completed);
        let mut last_poll = Instant::now();
        loop {
            let now = Instant::now();
            let wait = self
                .retry_at
                .map(|t| t.saturating_duration_since(now))
                .unwrap_or(Duration::from_secs(2))
                .min(Duration::from_secs(2));
            match rx.recv_timeout(wait) {
                Ok(Msg::Req(Request::Shutdown)) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(Msg::Req(Request::Discover)) => {
                    (self.emit)(Update::Devices(list_paired()));
                }
                Ok(Msg::Req(Request::Connect { address, name })) => {
                    self.connect(&address, &name);
                    (self.emit)(Update::Completed);
                }
                Ok(Msg::Req(Request::Disconnect)) => {
                    self.drop_link();
                    self.auto_connect = false; // user chose to disconnect
                    self.retry_at = None;
                    self.publish();
                    (self.emit)(Update::Completed);
                }
                Ok(Msg::Req(Request::Command(cmd))) => {
                    match self.link.as_mut() {
                        Some(l) => {
                            if let Err(e) = l.apply(cmd) {
                                (self.emit)(Update::Error(e));
                            }
                        }
                        None => (self.emit)(Update::Error("Headset not connected".into())),
                    }
                    self.publish();
                    (self.emit)(Update::Completed);
                }
                Ok(Msg::Notify(p)) => {
                    if self.link.as_mut().is_some_and(|l| l.on_notification(&p)) {
                        self.publish();
                    }
                }
                Ok(Msg::BluezUp) => {
                    if self.link.is_none() && self.auto_connect && self.retry_at.is_none() {
                        self.attempt = 0;
                        self.schedule_attempt();
                    }
                }
                Ok(Msg::WatchEnded) => self.watch_pid = None,
                Err(RecvTimeoutError::Timeout) => {}
            }
            // Link dropped (headset off / out of range)? Just show it; the next
            // BlueZ "connected" event brings it back, so there is no polling.
            if self.link.as_ref().is_some_and(|l| !l.is_connected()) {
                self.drop_link();
                self.publish();
            }
            if self.link.is_some() {
                self.retry_at = None;
            } else if self.auto_connect {
                if self.retry_at.is_some_and(|t| Instant::now() >= t) {
                    self.retry_at = None;
                    self.try_auto_connect();
                    if self.link.is_none() {
                        self.attempt += 1;
                        self.schedule_attempt();
                    }
                } else if self.watch_pid.is_none() && last_poll.elapsed() >= FALLBACK_POLL {
                    // No watcher (dbus-monitor missing): poll slowly instead.
                    last_poll = Instant::now();
                    self.try_auto_connect();
                }
            }
        }
        self.drop_link();
        if let Some(pid) = self.watch_pid.take() {
            bluez_watch::stop(pid);
        }
    }
}

fn bluetoothctl(args: &[&str]) -> Vec<(String, String)> {
    let Ok(out) = Proc::new("bluetoothctl").args(args).output() else { return vec![] };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.splitn(3, ' ');
            (it.next()? == "Device").then_some(())?;
            let addr = it.next()?.to_string();
            let name = it.next().unwrap_or("").trim().to_string();
            Some((addr, name))
        })
        .collect()
}

/// Paired devices that look like Sony headphones, for the device switcher.
pub fn list_paired() -> Vec<PairedDevice> {
    bluetoothctl(&["devices", "Paired"])
        .into_iter()
        .filter(|(_, n)| identify_model(n) != SonyModel::Unknown)
        .map(|(address, name)| PairedDevice { name, address })
        .collect()
}

/// First currently-connected device that exposes a Sony service.
fn find_connected_sony() -> Option<(String, String)> {
    bluetoothctl(&["devices", "Connected"])
        .into_iter()
        .find(|(a, _)| sony_transport::probe(a).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sony_protocol::fake::FakeDevice;
    use sony_protocol::DataType;
    use std::sync::Mutex;

    fn factory() -> TransportFactory {
        Arc::new(|| {
            Arc::new(FakeDevice::new(Box::new(|_, p| match p {
                // Unknown model => V1 command set, basic features only.
                [0x10, 0x00] => vec![(DataType::DataMdr, vec![0x11, 0x00, 61, 0])],
                [0x66, 0x02] => vec![(DataType::DataMdr, vec![0x67, 0x02, 0x11, 1, 2, 1, 0, 0])],
                _ => vec![],
            }))) as Arc<dyn Transport>
        })
    }

    fn wait_for(updates: &Arc<Mutex<Vec<Snapshot>>>, f: impl Fn(&Snapshot) -> bool) -> bool {
        for _ in 0..100 {
            if updates.lock().unwrap().iter().any(&f) {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        false
    }

    #[test]
    fn connect_command_and_disconnect_flow() {
        let snaps = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::<String>::new()));
        let (s2, e2) = (snaps.clone(), errors.clone());
        let emit: Emit = Arc::new(move |u| match u {
            Update::Snapshot(s) => s2.lock().unwrap().push(s),
            Update::Error(e) => e2.lock().unwrap().push(e),
            _ => {}
        });
        let b = Backend::start(factory(), emit, false);
        b.request(Request::Connect { address: "A8:E6:E8:EA:71:E9".into(), name: "Test Headset".into() });
        assert!(wait_for(&snaps, |s| s.connected && s.battery == 61));
        b.request(Request::Command(Command::Anc(false)));
        b.request(Request::Disconnect);
        assert!(wait_for(&snaps, |s| !s.connected && s.connection_state == "disconnected"));
        // A command with no headset connected reports an error instead of hanging.
        b.request(Request::Command(Command::Dsee(true)));
        for _ in 0..100 {
            if !errors.lock().unwrap().is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(errors.lock().unwrap().last().unwrap(), "Headset not connected");
    }
}
