//! Waybar `return-type: json` rendering.

use sony_protocol::{DeviceState, NoiseControlMode};

pub struct View {
    pub connected: bool,
    pub name: String,
    pub battery: Option<i32>,
    pub charging: bool,
    pub mode: NoiseControlMode,
}

impl View {
    pub fn disconnected() -> Self {
        Self { connected: false, name: String::new(), battery: None, charging: false, mode: NoiseControlMode::Off }
    }

    pub fn connected(name: &str, st: &DeviceState) -> Self {
        Self {
            connected: true,
            name: name.to_owned(),
            battery: st.battery.main,
            charging: st.battery.charging,
            mode: st.noise_control.mode,
        }
    }
}

fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

pub fn render(v: &View) -> String {
    if !v.connected {
        return r#"{"text":"󰟎","tooltip":"Headset not connected","class":"disconnected","percentage":0}"#.into();
    }
    let (icon, mode_name, mode_class) = match v.mode {
        NoiseControlMode::NoiseCancelling => ("󰋋", "Noise cancelling", "nc"),
        NoiseControlMode::Ambient => ("󰋎", "Ambient sound", "ambient"),
        NoiseControlMode::Off => ("󰟎", "Off", "off"),
    };
    let level = v.battery.unwrap_or(0);
    let batt = v.battery.map(|b| format!(" {b}%")).unwrap_or_default();
    let bolt = if v.charging { "" } else { "" };
    let mut classes = vec![mode_class];
    if v.charging {
        classes.push("charging");
    } else if v.battery.is_some_and(|b| b <= 15) {
        classes.push("low");
    }
    let tooltip = format!(
        "{}\nBattery: {}{}\nNoise control: {}",
        v.name,
        v.battery.map(|b| format!("{b}%")).unwrap_or_else(|| "unknown".into()),
        if v.charging { " (charging)" } else { "" },
        mode_name
    );
    format!(
        r#"{{"text":"{icon}{batt}{bolt}","tooltip":"{}","class":[{}],"percentage":{level}}}"#,
        json_escape(&tooltip),
        classes.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnected_line_is_valid_shape() {
        let s = render(&View::disconnected());
        assert!(s.contains("\"class\":\"disconnected\""));
    }

    #[test]
    fn connected_line_has_battery_and_classes() {
        let mut st = DeviceState::default();
        st.battery.main = Some(12);
        st.noise_control.mode = NoiseControlMode::Ambient;
        let s = render(&View::connected("ULT \"WEAR\"", &st));
        assert!(s.contains("12%"));
        assert!(s.contains("\"ambient\",\"low\""));
        assert!(s.contains("ULT \\\"WEAR\\\""), "name must be JSON-escaped: {s}");
    }
}
