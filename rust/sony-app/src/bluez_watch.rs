//! Event-driven "a Bluetooth device just connected" notifications, so the app can
//! sit completely idle while the headset is off instead of polling for it.
//!
//! It runs `dbus-monitor` on BlueZ's `PropertiesChanged` signals and parses the
//! text output. (`dbus-monitor` ships with the `dbus` package.)

use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;

const RULE: &str = "type='signal',sender='org.bluez',\
    interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',arg0='org.bluez.Device1'";

/// Incremental parser for the `dbus-monitor` text of one BlueZ device signal.
#[derive(Default)]
pub struct ConnWatcher {
    addr: Option<String>,
    device1: bool,
    key: Option<&'static str>,
}

impl ConnWatcher {
    /// Feed one output line. Returns the device address when it just became
    /// connected (`Connected` = true) or its services resolved (`ServicesResolved`
    /// = true, which is when its SDP records can be read).
    pub fn feed(&mut self, line: &str) -> Option<String> {
        let t = line.trim();
        if t.starts_with("signal ") {
            self.addr = None;
            self.device1 = false;
            self.key = None;
            if let Some(i) = t.find("/dev_") {
                let mac = t.get(i + 5..i + 5 + 17)?;
                self.addr = Some(mac.replace('_', ":"));
            }
            return None;
        }
        if t == "string \"org.bluez.Device1\"" {
            self.device1 = true;
            return None;
        }
        if !self.device1 {
            return None;
        }
        match t {
            "string \"Connected\"" => self.key = Some("Connected"),
            "string \"ServicesResolved\"" => self.key = Some("ServicesResolved"),
            _ if self.key.is_some() => {
                if let Some(v) = t.strip_prefix("variant").map(str::trim) {
                    if let Some(b) = v.strip_prefix("boolean ") {
                        self.key = None;
                        if b.trim() == "true" {
                            return self.addr.clone();
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }
}

/// Starts the watcher. Calls `on_up(address)` for each device that connects and
/// `on_end()` if the monitor stops. Returns the monitor's pid, or `None` if
/// `dbus-monitor` can't be started (the caller then falls back to slow polling).
pub fn spawn(on_up: impl Fn(String) + Send + 'static, on_end: impl Fn() + Send + 'static) -> Option<u32> {
    let mut cmd = Command::new("dbus-monitor");
    cmd.args(["--system", RULE]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    // SAFETY: only async-signal-safe prctl between fork and exec. The monitor must
    // never outlive us (a killed widget would otherwise leave it running forever).
    unsafe {
        cmd.pre_exec(|| {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            Ok(())
        });
    }
    let mut child = cmd.spawn().ok()?;
    let pid = child.id();
    let out = child.stdout.take()?;
    thread::spawn(move || {
        let mut w = ConnWatcher::default();
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(addr) = w.feed(&line) {
                on_up(addr);
            }
        }
        let _ = child.wait();
        on_end();
    });
    Some(pid)
}

pub fn stop(pid: u32) {
    // SAFETY: plain signal to the monitor process we started.
    unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONNECTED: &str = r#"signal time=1.2 sender=:1.5 -> destination=(null destination) serial=7 path=/org/bluez/hci0/dev_A8_E6_E8_EA_71_E9; interface=org.freedesktop.DBus.Properties; member=PropertiesChanged
   string "org.bluez.Device1"
   array [
      dict entry(
         string "Connected"
         variant             boolean true
      )
   ]
   array [
   ]"#;

    fn run(text: &str) -> Vec<String> {
        let mut w = ConnWatcher::default();
        text.lines().filter_map(|l| w.feed(l)).collect()
    }

    #[test]
    fn reports_a_device_that_connects() {
        assert_eq!(run(CONNECTED), vec!["A8:E6:E8:EA:71:E9"]);
    }

    #[test]
    fn ignores_disconnects_and_unrelated_properties() {
        assert!(run(&CONNECTED.replace("boolean true", "boolean false")).is_empty());
        assert!(run(&CONNECTED.replace("\"Connected\"", "\"RSSI\"")).is_empty());
        assert!(run(&CONNECTED.replace("org.bluez.Device1", "org.bluez.Adapter1")).is_empty());
    }

    #[test]
    fn services_resolved_also_triggers_and_state_resets_between_signals() {
        let both = format!(
            "{}\n{}",
            CONNECTED.replace("\"Connected\"", "\"ServicesResolved\""),
            CONNECTED.replace("A8_E6_E8_EA_71_E9", "11_22_33_44_55_66").replace("boolean true", "boolean false"),
        );
        assert_eq!(run(&both), vec!["A8:E6:E8:EA:71:E9"]);
    }
}
