//! One JSON line per state change. The top-level `text`/`tooltip`/`class`/
//! `percentage` fields are what Waybar's `return-type: json` expects; `state`
//! carries everything else for richer front-ends (the bar plugin's panel).

use serde_json::{json, Value};
use sony_app::Snapshot;

pub fn render(s: &Snapshot, error: &str) -> String {
    render_value(s, error).to_string()
}

fn render_value(s: &Snapshot, error: &str) -> Value {
    if !s.connected {
        return json!({
            "text": "󰟎",
            "tooltip": "Headset not connected",
            "class": ["disconnected"],
            "percentage": 0,
            "state": { "connected": false, "connectionState": s.connection_state, "error": error }
        });
    }
    let (icon, mode_name, mode_class) = match s.noise_mode.as_str() {
        "cancelling" => ("󰋋", "Noise cancelling", "nc"),
        "ambient" => ("󰋎", "Ambient sound", "ambient"),
        "off" => ("󰟎", "Off", "off"),
        _ => ("󰋋", "Unknown", "off"),
    };
    let batt = (s.battery >= 0).then(|| format!(" {}%", s.battery)).unwrap_or_default();
    let bolt = if s.charging { "" } else { "" };
    let mut classes = vec![mode_class];
    if s.charging {
        classes.push("charging");
    } else if s.battery >= 0 && s.battery <= 15 {
        classes.push("low");
    }
    let tooltip = format!(
        "{}\nBattery: {}{}\nNoise control: {}",
        s.name,
        if s.battery >= 0 { format!("{}%", s.battery) } else { "unknown".into() },
        if s.charging { " (charging)" } else { "" },
        mode_name
    );
    json!({
        "text": format!("{icon}{batt}{bolt}"),
        "tooltip": tooltip,
        "class": classes,
        "percentage": s.battery.max(0),
        "state": {
            "connected": true,
            "connectionState": s.connection_state,
            "name": s.name,
            "address": s.address,
            "battery": s.battery,
            "charging": s.charging,
            "noise": s.noise_mode,
            "ambientLevel": s.ambient_level,
            "focusOnVoice": s.focus_on_voice,
            "eqPreset": s.eq_preset,
            "eqPresetName": s.eq_preset_name,
            "clearBass": s.clear_bass,
            "ultMode": s.ult_mode,
            "codec": s.codec,
            "firmware": s.firmware,
            "dsee": s.dsee,
            "pauseWhenRemoved": s.pause_when_removed,
            "caps": {
                "anc": s.caps.anc, "ambient": s.caps.ambient, "equalizer": s.caps.equalizer,
                "dsee": s.caps.dsee, "ult": s.caps.ult_bass,
                "pauseWhenRemoved": s.caps.pause_when_removed,
                "connectedDevices": s.caps.connected_devices,
            },
            "connectedDevices": s.connected_devices.iter().map(|d| json!({
                "name": d.name, "address": d.address, "playback": d.playback_device
            })).collect::<Vec<_>>(),
            "connectedDevicesStatus": s.connected_devices_status,
            "error": error,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected() -> Snapshot {
        let mut s = Snapshot::disconnected("connected");
        s.connected = true;
        s.name = "ULT \"WEAR\"".into();
        s.battery = 12;
        s.noise_mode = "ambient".into();
        s.eq_preset_name = "Bass Boost".into();
        s
    }

    #[test]
    fn disconnected_line() {
        let v: Value = serde_json::from_str(&render(&Snapshot::disconnected("disconnected"), "")).unwrap();
        assert_eq!(v["class"][0], "disconnected");
        assert_eq!(v["state"]["connected"], false);
    }

    #[test]
    fn connected_line_is_valid_json_with_classes_and_escaping() {
        let v: Value = serde_json::from_str(&render(&connected(), "")).unwrap();
        assert_eq!(v["percentage"], 12);
        assert_eq!(v["class"], json!(["ambient", "low"]));
        assert_eq!(v["state"]["name"], "ULT \"WEAR\"");
        assert!(v["text"].as_str().unwrap().contains("12%"));
        assert_eq!(v["state"]["eqPresetName"], "Bass Boost");
    }

    #[test]
    fn errors_are_reported_in_state() {
        let v: Value = serde_json::from_str(&render(&connected(), "boom")).unwrap();
        assert_eq!(v["state"]["error"], "boom");
    }
}
