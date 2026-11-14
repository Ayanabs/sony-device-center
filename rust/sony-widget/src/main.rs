//! Waybar module for Sony headphones.
//!
//!   sony-widget [run] [--addr AA:BB:..]   stream Waybar JSON lines on stdout
//!   sony-widget cycle-nc                  NC -> Ambient -> Off -> NC (running instance)
//!   sony-widget set-nc off|nc|ambient     set noise control (running instance)
//!   sony-widget status                    print the current JSON line once

mod render;

use render::{render, View};
use sony_protocol::{
    apply_notification, Changed, DeviceState, NoiseControlMode, Protocol, ProtocolV1, ProtocolV2,
    Session,
};
use sony_transport::{ProtocolKind, RfcommTransport};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const REFRESH_EVERY: Duration = Duration::from_secs(300);
const RETRY_EVERY: Duration = Duration::from_secs(5);

enum Event {
    Notification(Vec<u8>),
    Command(String, Sender<String>),
}

fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| "/tmp".into());
    dir.join("sony-widget.sock")
}

fn emit(view: &View) {
    println!("{}", render(view));
    let _ = std::io::stdout().flush();
}

/// First connected Bluetooth device that exposes a Sony service.
fn find_device() -> Option<String> {
    let out = Command::new("bluetoothctl").args(["devices", "Connected"]).output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(str::to_owned))
        .find(|a| sony_transport::probe(a).is_some())
}

fn device_name(addr: &str) -> String {
    Command::new("bluetoothctl")
        .args(["info", addr])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .find_map(|l| l.trim().strip_prefix("Name: ").map(str::to_owned))
        })
        .unwrap_or_else(|| "Sony headphones".into())
}

fn client(args: &[String]) -> i32 {
    let mut s = match UnixStream::connect(socket_path()) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("sony-widget is not running");
            return 1;
        }
    };
    let _ = writeln!(s, "{}", args.join(" "));
    let mut reply = String::new();
    let _ = BufReader::new(s).read_line(&mut reply);
    print!("{reply}");
    i32::from(reply.starts_with("error"))
}

fn serve(tx: Sender<Event>) {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let Ok(listener) = UnixListener::bind(&path) else {
        eprintln!("cannot bind {}", path.display());
        return;
    };
    thread::spawn(move || {
        for conn in listener.incoming().flatten() {
            let tx = tx.clone();
            thread::spawn(move || {
                let mut line = String::new();
                if BufReader::new(&conn).read_line(&mut line).is_ok() {
                    let (rtx, rrx) = mpsc::channel();
                    if tx.send(Event::Command(line.trim().to_owned(), rtx)).is_ok() {
                        let reply = rrx.recv_timeout(Duration::from_secs(5)).unwrap_or_else(|_| "error: timeout".into());
                        let _ = writeln!(&conn, "{reply}");
                    }
                }
            });
        }
    });
}

fn next_nc(m: NoiseControlMode) -> NoiseControlMode {
    match m {
        NoiseControlMode::NoiseCancelling => NoiseControlMode::Ambient,
        NoiseControlMode::Ambient => NoiseControlMode::Off,
        NoiseControlMode::Off => NoiseControlMode::NoiseCancelling,
    }
}

fn refresh(proto: &dyn Protocol, st: &mut DeviceState) {
    if let Ok(b) = proto.get_battery() {
        st.battery = b;
    }
    if let Ok(n) = proto.get_noise_control() {
        st.noise_control = n;
    }
}

fn handle_command(cmd: &str, proto: &dyn Protocol, st: &mut DeviceState, name: &str) -> String {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let mode = match parts.as_slice() {
        ["status"] => return render(&View::connected(name, st)),
        ["cycle-nc"] => next_nc(st.noise_control.mode),
        ["set-nc", "off"] => NoiseControlMode::Off,
        ["set-nc", "nc"] => NoiseControlMode::NoiseCancelling,
        ["set-nc", "ambient"] => NoiseControlMode::Ambient,
        _ => return "error: unknown command".into(),
    };
    let mut want = st.noise_control.clone();
    want.mode = mode;
    if mode == NoiseControlMode::Ambient && want.ambient_level == 0 {
        want.ambient_level = 10;
    }
    match proto.set_noise_control(&want) {
        Ok(()) => {
            if let Ok(n) = proto.get_noise_control() {
                st.noise_control = n;
            } else {
                st.noise_control = want;
            }
            "ok".into()
        }
        Err(e) => format!("error: {}", e.message),
    }
}

/// Waits out the retry delay while still answering commands.
fn idle(rx: &mpsc::Receiver<Event>) {
    let deadline = Instant::now() + RETRY_EVERY;
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(Event::Command(_, r)) => {
                let _ = r.send("error: headset not connected".into());
            }
            Ok(Event::Notification(_)) => {}
            Err(_) => break,
        }
    }
}

fn run(addr_arg: Option<String>) {
    let (tx, rx) = mpsc::channel::<Event>();
    serve(tx.clone());
    emit(&View::disconnected());

    loop {
        let Some(addr) = addr_arg.clone().or_else(find_device) else {
            idle(&rx);
            continue;
        };
        let transport = Arc::new(RfcommTransport::new());
        let session = Arc::new(Session::new(transport.clone()));
        if session.connect(&addr).is_err() {
            idle(&rx);
            continue;
        }
        let ntx = tx.clone();
        session.on_notification(Arc::new(move |f| {
            let _ = ntx.send(Event::Notification(f.payload.clone()));
        }));
        let proto: Box<dyn Protocol> = match transport.protocol_kind() {
            Some(ProtocolKind::V1) => Box::new(ProtocolV1::new(session.clone())),
            _ => Box::new(ProtocolV2::new(session.clone(), false, false)),
        };
        let name = device_name(&addr);
        let _ = proto.init_device();
        let mut st = DeviceState::default();
        refresh(proto.as_ref(), &mut st);
        emit(&View::connected(&name, &st));

        let mut last_refresh = Instant::now();
        while session.is_connected() {
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok(Event::Notification(p)) => {
                    let changed = match apply_notification(&p, &mut st, false) {
                        Some(c) => Some(c),
                        // V1 and other layouts: re-read instead of guessing the layout.
                        None if matches!(p.first(), Some(0x67 | 0x69)) => {
                            if let Ok(n) = proto.get_noise_control() {
                                st.noise_control = n;
                            }
                            Some(Changed::NoiseControl)
                        }
                        None => None,
                    };
                    if changed.is_some() {
                        emit(&View::connected(&name, &st));
                    }
                }
                Ok(Event::Command(cmd, reply)) => {
                    let r = handle_command(&cmd, proto.as_ref(), &mut st, &name);
                    let _ = reply.send(r);
                    emit(&View::connected(&name, &st));
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            if last_refresh.elapsed() >= REFRESH_EVERY {
                refresh(proto.as_ref(), &mut st);
                emit(&View::connected(&name, &st));
                last_refresh = Instant::now();
            }
        }
        session.disconnect();
        emit(&View::disconnected());
        idle(&rx);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("cycle-nc" | "set-nc" | "status") => std::process::exit(client(&args)),
        Some("-h" | "--help") => {
            println!("usage: sony-widget [run] [--addr MAC] | cycle-nc | set-nc off|nc|ambient | status");
        }
        _ => {
            let addr = args.iter().position(|a| a == "--addr").and_then(|i| args.get(i + 1)).cloned();
            run(addr);
        }
    }
}
