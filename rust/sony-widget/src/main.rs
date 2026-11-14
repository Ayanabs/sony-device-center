//! Headphone widget daemon for bars (Waybar / Quickshell).
//!
//!   sony-widget [run]                     stream one JSON line per state change on stdout
//!   sony-widget status                    print the current JSON line once
//!   sony-widget quit                      stop the running instance
//!   sony-widget cycle-nc                  noise cancelling -> ambient -> off -> ...
//!   sony-widget set-nc off|nc|ambient
//!   sony-widget set-ambient LEVEL [voice] ambient level 1..20, optional voice focus (0|1)
//!   sony-widget set-eq PRESET             preset id (0x16) or name (bass-boost)
//!   sony-widget set-pause on|off          pause when removed (if supported)
//!   sony-widget set-dsee on|off           DSEE (if supported)

mod render;

use render::render;
use sony_app::{Backend, Command, Emit, Request, Snapshot, TransportFactory, Update};
use sony_protocol::presets::PRESETS;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

enum Event {
    Update(Update),
    Command(String, Sender<String>),
}

fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| "/tmp".into());
    dir.join("sony-widget.sock")
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
                        let reply = rrx
                            .recv_timeout(Duration::from_secs(5))
                            .unwrap_or_else(|_| "error: timeout".into());
                        let _ = writeln!(&conn, "{reply}");
                    }
                }
            });
        }
    });
}

fn on_off(s: &str) -> Option<bool> {
    match s {
        "on" | "1" | "true" => Some(true),
        "off" | "0" | "false" => Some(false),
        _ => None,
    }
}

fn parse_preset(s: &str) -> Option<i32> {
    let l = s.to_lowercase();
    if let Some(hex) = l.strip_prefix("0x") {
        return i32::from_str_radix(hex, 16).ok();
    }
    if let Ok(n) = l.parse::<i32>() {
        return Some(n);
    }
    PRESETS.iter().find(|(_, id, name)| *id == l || name.to_lowercase() == l).map(|(n, _, _)| *n)
}

/// Turns a socket command into a backend command (or a ready reply).
fn parse_command(cmd: &str, s: &Snapshot) -> Result<Command, String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let c = &s.caps;
    let need = |ok: bool, what: &str| if ok { Ok(()) } else { Err(format!("error: {what} not supported by this headset")) };
    match parts.as_slice() {
        ["cycle-nc"] => {
            need(c.anc, "noise control")?;
            Ok(match s.noise_mode.as_str() {
                "cancelling" => Command::Ambient { level: s.ambient_level, focus_on_voice: s.focus_on_voice },
                "ambient" => Command::Anc(false),
                _ => Command::Anc(true),
            })
        }
        ["set-nc", "off"] => Ok(Command::Anc(false)),
        ["set-nc", "nc"] => Ok(Command::Anc(true)),
        ["set-nc", "ambient"] => {
            Ok(Command::Ambient { level: s.ambient_level, focus_on_voice: s.focus_on_voice })
        }
        ["set-ambient", level, rest @ ..] => {
            let level: i32 = level.parse().map_err(|_| "error: bad level".to_string())?;
            if !(1..=20).contains(&level) {
                return Err("error: level must be 1..20".into());
            }
            let voice = rest.first().and_then(|v| on_off(v)).unwrap_or(s.focus_on_voice);
            Ok(Command::Ambient { level, focus_on_voice: voice })
        }
        ["set-eq", p] => {
            need(c.equalizer, "equalizer")?;
            parse_preset(p).map(Command::EqPreset).ok_or_else(|| "error: unknown preset".into())
        }
        ["set-pause", v] => {
            need(c.pause_when_removed, "pause on removal")?;
            on_off(v).map(Command::PauseWhenRemoved).ok_or_else(|| "error: expected on|off".into())
        }
        ["set-dsee", v] => {
            need(c.dsee, "DSEE")?;
            on_off(v).map(Command::Dsee).ok_or_else(|| "error: expected on|off".into())
        }
        _ => Err("error: unknown command".into()),
    }
}

/// Only one instance may hold the headset. If another is already running, ask it
/// to quit and wait for its socket to go away, so the newest instance (and the
/// front-end reading its output) always wins instead of piling up duplicates.
fn take_over() {
    let Ok(mut s) = UnixStream::connect(socket_path()) else { return };
    let _ = writeln!(s, "quit");
    let _ = BufReader::new(s).read_line(&mut String::new());
    for _ in 0..30 {
        if UnixStream::connect(socket_path()).is_err() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// Last resort for an orphan that lost its socket (e.g. its front-end was killed and
/// a newer instance unlinked the path): ask the OS to stop every other sony-widget.
fn kill_others() {
    let me = std::process::id();
    let others: Vec<u32> = std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|&pid| pid != me)
        .filter(|pid| {
            std::fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|c| c.trim() == "sony-widget")
        })
        .collect();
    for pid in &others {
        // SAFETY: plain signal to another process of the same user.
        unsafe { libc::kill(*pid as libc::pid_t, libc::SIGTERM) };
    }
    for _ in 0..30 {
        if others.iter().all(|pid| !std::path::Path::new(&format!("/proc/{pid}")).exists()) {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn run() {
    take_over();
    kill_others();
    let (tx, rx) = mpsc::channel::<Event>();
    serve(tx.clone());

    let utx = tx.clone();
    let emit: Emit = Arc::new(move |u| {
        let _ = utx.send(Event::Update(u));
    });
    let factory: TransportFactory = Arc::new(|| {
        Arc::new(sony_transport::RfcommTransport::new()) as Arc<dyn sony_protocol::Transport>
    });
    let backend = Backend::start(factory, emit, true);

    let mut snap = Snapshot::disconnected("searching");
    let mut error = String::new();
    let mut last = String::new();
    let emit_line = |snap: &Snapshot, error: &str, last: &mut String| {
        let line = render(snap, error);
        if line != *last {
            println!("{line}");
            let _ = std::io::stdout().flush();
            *last = line;
        }
    };
    emit_line(&snap, &error, &mut last);

    while let Ok(ev) = rx.recv() {
        match ev {
            Event::Update(Update::Snapshot(s)) => {
                snap = s;
                if !snap.connected || snap.connection_state != "connected" {
                    error.clear();
                }
            }
            Event::Update(Update::Error(e)) => error = e,
            Event::Update(_) => continue,
            Event::Command(cmd, reply) => {
                if cmd == "quit" {
                    let _ = reply.send("ok".into());
                    thread::sleep(Duration::from_millis(100)); // let the reply flush
                    let _ = std::fs::remove_file(socket_path());
                    std::process::exit(0);
                }
                if cmd == "status" {
                    let _ = reply.send(render(&snap, &error));
                    continue;
                }
                let r = if !snap.connected {
                    "error: headset not connected".to_string()
                } else {
                    match parse_command(&cmd, &snap) {
                        Ok(c) => {
                            error.clear();
                            backend.request(Request::Command(c));
                            "ok".into()
                        }
                        Err(e) => e,
                    }
                };
                let _ = reply.send(r);
            }
        }
        emit_line(&snap, &error, &mut last);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("quit" | "status" | "cycle-nc" | "set-nc" | "set-ambient" | "set-eq" | "set-pause" | "set-dsee") => {
            std::process::exit(client(&args))
        }
        Some("-h" | "--help") => {
            println!("usage: sony-widget [run] | status | cycle-nc | set-nc off|nc|ambient | set-ambient LEVEL [on|off] | set-eq PRESET | set-pause on|off | set-dsee on|off");
        }
        _ => run(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Snapshot {
        let mut s = Snapshot::disconnected("connected");
        s.connected = true;
        s.caps.anc = true;
        s.caps.equalizer = true;
        s.caps.pause_when_removed = true;
        s.noise_mode = "cancelling".into();
        s.ambient_level = 7;
        s
    }

    #[test]
    fn cycle_walks_nc_ambient_off() {
        let mut s = snap();
        assert_eq!(parse_command("cycle-nc", &s), Ok(Command::Ambient { level: 7, focus_on_voice: false }));
        s.noise_mode = "ambient".into();
        assert_eq!(parse_command("cycle-nc", &s), Ok(Command::Anc(false)));
        s.noise_mode = "off".into();
        assert_eq!(parse_command("cycle-nc", &s), Ok(Command::Anc(true)));
    }

    #[test]
    fn presets_parse_by_id_name_and_number() {
        let s = snap();
        assert_eq!(parse_command("set-eq bass-boost", &s), Ok(Command::EqPreset(0x16)));
        assert_eq!(parse_command("set-eq 0x11", &s), Ok(Command::EqPreset(0x11)));
        assert_eq!(parse_command("set-eq 22", &s), Ok(Command::EqPreset(22)));
        assert!(parse_command("set-eq nonsense", &s).is_err());
    }

    #[test]
    fn unsupported_features_and_bad_input_are_rejected() {
        let s = snap();
        assert!(parse_command("set-dsee on", &s).unwrap_err().contains("not supported"));
        assert!(parse_command("set-ambient 99", &s).is_err());
        assert_eq!(parse_command("set-pause on", &s), Ok(Command::PauseWhenRemoved(true)));
        assert!(parse_command("set-pause maybe", &s).is_err());
        assert!(parse_command("rm -rf", &s).is_err());
    }
}
