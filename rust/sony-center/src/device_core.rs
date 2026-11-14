//! Qt-free application core: connects to a headset, reads its state into a
//! `Snapshot` and applies UI commands. The cxx-qt controller is a thin shell
//! over this, so all of it is unit-tested against the fake headset.

use sony_protocol::profile::{profile_for_device, DeviceCapabilities, ProtocolVersion, SonyModel};
use sony_protocol::presets::preset_name;
use sony_protocol::{
    apply_notification, ConnectedDevice, DeviceState, NoiseControlMode, NoiseControlState,
    Protocol, ProtocolGeneration, ProtocolV1, ProtocolV2, Session, SonyErrorCode, Transport,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Caps {
    pub anc: bool,
    pub ambient: bool,
    pub equalizer: bool,
    pub clear_bass: bool,
    pub ten_band_equalizer: bool,
    pub dsee: bool,
    pub speak_to_chat: bool,
    pub adaptive_volume: bool,
    pub ult_bass: bool,
    pub pause_when_removed: bool,
    pub connected_devices: bool,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub connected: bool,
    pub connection_state: String,
    pub name: String,
    pub address: String,
    /// feature name -> "valid" | "unsupported" | "error"
    pub features: BTreeMap<String, String>,
    pub caps: Caps,
    pub battery: i32,
    pub charging: bool,
    /// "cancelling" | "ambient" | "off" | "unknown"
    pub noise_mode: String,
    pub ambient_level: i32,
    pub focus_on_voice: bool,
    pub eq_preset: i32,
    pub eq_preset_name: String,
    pub clear_bass: i32,
    pub eq_bands: Vec<i32>,
    pub ult_mode: i32,
    pub dsee: bool,
    pub speak_to_chat: bool,
    pub adaptive_volume: bool,
    pub pause_when_removed: bool,
    pub auto_power_off: i32,
    pub codec: String,
    pub firmware: String,
    pub connected_devices: Vec<ConnectedDevice>,
    pub connected_devices_status: String,
}

impl Snapshot {
    pub fn disconnected(state: &str) -> Self {
        Self {
            connected: false,
            connection_state: state.into(),
            name: String::new(),
            address: String::new(),
            features: BTreeMap::new(),
            caps: Caps::default(),
            battery: -1,
            charging: false,
            noise_mode: "unknown".into(),
            ambient_level: 10,
            focus_on_voice: false,
            eq_preset: -1,
            eq_preset_name: "Unknown".into(),
            clear_bass: 0,
            eq_bands: vec![0; 5],
            ult_mode: -1,
            dsee: false,
            speak_to_chat: false,
            adaptive_volume: false,
            pause_when_removed: false,
            auto_power_off: 0,
            codec: "Unknown".into(),
            firmware: String::new(),
            connected_devices: vec![],
            connected_devices_status: "unknown".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Anc(bool),
    Ambient { level: i32, focus_on_voice: bool },
    EqPreset(i32),
    EqCustom { clear_bass: i32, bands: Vec<i32> },
    Dsee(bool),
    SpeakToChat(bool),
    AdaptiveVolume(bool),
    PauseWhenRemoved(bool),
    AutoPowerOff(i32),
}

fn caps_of(c: &DeviceCapabilities, v2: bool) -> Caps {
    Caps {
        anc: c.noise_cancelling,
        ambient: c.ambient_sound,
        equalizer: c.equalizer,
        clear_bass: c.clear_bass,
        ten_band_equalizer: c.ten_band_equalizer,
        dsee: c.dsee,
        speak_to_chat: c.speak_to_chat,
        adaptive_volume: c.adaptive_volume,
        ult_bass: c.ult_bass,
        pause_when_removed: c.pause_when_removed,
        connected_devices: c.multipoint && v2,
    }
}

/// An open connection to one headset.
pub struct Link {
    pub session: Arc<Session>,
    proto: Box<dyn Protocol>,
    caps: Caps,
    state: DeviceState,
    ult_bass: bool,
    snap: Snapshot,
}

impl Link {
    /// Connects over `transport` and reads the initial state.
    pub fn open(transport: Arc<dyn Transport>, address: &str, name: &str) -> Result<Link, String> {
        let session = Arc::new(Session::new(transport.clone()));
        session.connect(address).map_err(|e| e.message)?;
        let profile = profile_for_device(name);
        let v2 = match profile.model {
            SonyModel::Unknown => transport.generation_hint() == Some(ProtocolGeneration::V2),
            _ => profile.protocol == ProtocolVersion::V2,
        };
        let mut c = profile.capabilities;
        if profile.model == SonyModel::Unknown {
            // Unknown model: try the basics every Sony headset has.
            c.battery = true;
            c.noise_cancelling = true;
            c.ambient_sound = true;
        }
        let proto: Box<dyn Protocol> = if v2 {
            Box::new(ProtocolV2::new(session.clone(), c.ten_band_equalizer, c.ult_bass))
        } else {
            Box::new(ProtocolV1::new(session.clone()))
        };
        let _ = proto.init_device();
        let mut snap = Snapshot::disconnected("connected");
        snap.connected = true;
        snap.name = name.to_string();
        snap.address = address.to_string();
        snap.caps = caps_of(&c, v2);
        let mut link = Link {
            session,
            proto,
            caps: snap.caps.clone(),
            state: DeviceState::default(),
            ult_bass: c.ult_bass,
            snap,
        };
        link.refresh_all(&c);
        Ok(link)
    }

    pub fn is_connected(&self) -> bool {
        self.session.is_connected()
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snap
    }

    fn mark<T>(&mut self, key: &str, r: &sony_protocol::Result<T>, supported: bool) {
        let v = match r {
            _ if !supported => "unsupported",
            Ok(_) => "valid",
            Err(e) if e.code == SonyErrorCode::Unsupported => "unsupported",
            Err(_) => "error",
        };
        self.snap.features.insert(key.into(), v.into());
    }

    /// Queries a feature only when the profile says the headset has it.
    fn feature<T>(
        &mut self,
        key: &str,
        supported: bool,
        query: impl FnOnce(&dyn Protocol) -> sony_protocol::Result<T>,
        apply: impl FnOnce(&mut Self, T),
    ) {
        if !supported {
            self.snap.features.insert(key.into(), "unsupported".into());
            return;
        }
        let r = query(self.proto.as_ref());
        self.mark(key, &r, true);
        if let Ok(v) = r {
            apply(self, v);
        }
    }

    fn refresh_all(&mut self, c: &DeviceCapabilities) {
        self.feature("battery", c.battery, |p| p.get_battery(), |s, v| s.state.battery = v);
        self.feature(
            "noiseControl",
            c.noise_cancelling || c.ambient_sound,
            |p| p.get_noise_control(),
            |s, v| s.state.noise_control = v,
        );
        self.feature("equalizer", c.equalizer, |p| p.get_equalizer(), |s, v| s.state.equalizer = v);
        self.feature("codec", c.codec_info, |p| p.get_codec(), |s, v| {
            s.snap.codec = if v.is_empty() { "Unknown".into() } else { v }
        });
        self.feature("firmware", c.firmware_info, |p| p.get_firmware_version(), |s, v| s.snap.firmware = v);
        self.feature("dsee", c.dsee, |p| p.get_dsee(), |s, v| s.snap.dsee = v);
        self.feature("speakToChat", c.speak_to_chat, |p| p.get_speak_to_chat(), |s, v| {
            s.snap.speak_to_chat = v
        });
        self.feature("adaptiveVolume", c.adaptive_volume, |p| p.get_adaptive_volume(), |s, v| {
            s.snap.adaptive_volume = v
        });
        self.feature(
            "pauseWhenRemoved",
            c.pause_when_removed,
            |p| p.get_pause_when_removed(),
            |s, v| s.snap.pause_when_removed = v,
        );
        self.feature("autoPowerOff", c.auto_power_off, |p| p.get_auto_power_off(), |s, v| {
            s.snap.auto_power_off = v
        });

        let want_devices = self.caps.connected_devices;
        self.snap.connected_devices.clear();
        self.snap.connected_devices_status = "unknown".into();
        let r = if want_devices { Some(self.proto.get_connected_devices()) } else { None };
        match r {
            Some(Ok(d)) => {
                self.snap.features.insert("connectedDevices".into(), "valid".into());
                self.snap.connected_devices = d;
                self.snap.connected_devices_status = "valid".into();
            }
            Some(Err(e)) if e.code == SonyErrorCode::Unsupported => {
                self.snap.features.insert("connectedDevices".into(), "unsupported".into());
                self.snap.connected_devices_status = "unsupported".into();
            }
            Some(Err(_)) => {
                self.snap.features.insert("connectedDevices".into(), "error".into());
            }
            None => {
                self.snap.features.insert("connectedDevices".into(), "unsupported".into());
            }
        }
        self.rebuild();
    }

    /// Copies `state` into the UI-facing snapshot.
    fn rebuild(&mut self) {
        let valid = |s: &Snapshot, k: &str| s.features.get(k).map(String::as_str) == Some("valid");
        let s = &mut self.snap;
        s.battery = if valid(s, "battery") { self.state.battery.main.unwrap_or(-1) } else { -1 };
        s.charging = self.state.battery.charging;
        let nc = &self.state.noise_control;
        s.noise_mode = if !valid(s, "noiseControl") {
            "unknown".into()
        } else {
            match nc.mode {
                NoiseControlMode::Off => "off",
                NoiseControlMode::NoiseCancelling => "cancelling",
                NoiseControlMode::Ambient => "ambient",
            }
            .into()
        };
        // ANC/Off report zero, which is not a valid ambient command level;
        // keep the last confirmed ambient level for the next mode switch.
        if nc.mode == NoiseControlMode::Ambient && (1..=20).contains(&nc.ambient_level) {
            s.ambient_level = nc.ambient_level;
        }
        s.focus_on_voice = nc.focus_on_voice;
        let eq = &self.state.equalizer;
        if valid(s, "equalizer") {
            s.eq_preset = eq.preset;
            s.eq_preset_name = preset_name(eq.preset);
            s.ult_mode = eq.ult_mode;
        } else {
            s.eq_preset = -1;
            s.eq_preset_name = "Unknown".into();
            s.ult_mode = -1;
        }
        s.clear_bass = eq.clear_bass;
        s.eq_bands = eq.bands.clone();
    }

    /// Applies a push notification; returns true if the snapshot changed.
    pub fn on_notification(&mut self, payload: &[u8]) -> bool {
        if apply_notification(payload, &mut self.state, self.ult_bass).is_some() {
            self.rebuild();
            return true;
        }
        // Layouts we don't decode (e.g. V1 noise control): re-read instead of guessing.
        if matches!(payload.first(), Some(0x67 | 0x69)) {
            if let Ok(n) = self.proto.get_noise_control() {
                self.state.noise_control = n;
                self.rebuild();
                return true;
            }
        }
        false
    }

    /// Applies a command, then re-reads what it can have changed.
    pub fn apply(&mut self, cmd: Command) -> Result<(), String> {
        let e = |e: sony_protocol::SonyError| e.message;
        let sent = cmd.clone();
        match cmd {
            Command::Anc(enabled) => {
                let mut n = self.state.noise_control.clone();
                n.mode = if enabled { NoiseControlMode::NoiseCancelling } else { NoiseControlMode::Off };
                self.proto.set_noise_control(&n).map_err(e)?;
            }
            Command::Ambient { level, focus_on_voice } => {
                let level = if (1..=20).contains(&level) { level } else { self.snap.ambient_level };
                let n = NoiseControlState {
                    mode: NoiseControlMode::Ambient,
                    ambient_level: level,
                    focus_on_voice,
                };
                self.proto.set_noise_control(&n).map_err(e)?;
            }
            Command::EqPreset(p) => self.proto.set_equalizer_preset(p).map_err(e)?,
            Command::EqCustom { clear_bass, bands } => {
                self.proto.set_equalizer_custom(clear_bass, &bands).map_err(e)?
            }
            Command::Dsee(v) => self.proto.set_dsee(v).map_err(e)?,
            Command::SpeakToChat(v) => self.proto.set_speak_to_chat(v).map_err(e)?,
            Command::AdaptiveVolume(v) => self.proto.set_adaptive_volume(v).map_err(e)?,
            Command::PauseWhenRemoved(v) => self.proto.set_pause_when_removed(v).map_err(e)?,
            Command::AutoPowerOff(i) => self.proto.set_auto_power_off(i).map_err(e)?,
        }
        self.reread_after(&sent);
        Ok(())
    }

    fn reread_after(&mut self, cmd: &Command) {
        match cmd {
            Command::Anc(_) | Command::Ambient { .. } => {
                if let Ok(n) = self.proto.get_noise_control() {
                    self.state.noise_control = n;
                }
            }
            Command::EqPreset(_) | Command::EqCustom { .. } => {
                if let Ok(q) = self.proto.get_equalizer() {
                    self.state.equalizer = q;
                }
            }
            Command::Dsee(_) => {
                if let Ok(v) = self.proto.get_dsee() {
                    self.snap.dsee = v;
                }
            }
            Command::SpeakToChat(_) => {
                if let Ok(v) = self.proto.get_speak_to_chat() {
                    self.snap.speak_to_chat = v;
                }
            }
            Command::AdaptiveVolume(_) => {
                if let Ok(v) = self.proto.get_adaptive_volume() {
                    self.snap.adaptive_volume = v;
                }
            }
            Command::PauseWhenRemoved(_) => {
                if let Ok(v) = self.proto.get_pause_when_removed() {
                    self.snap.pause_when_removed = v;
                }
            }
            Command::AutoPowerOff(_) => {
                if let Ok(v) = self.proto.get_auto_power_off() {
                    self.snap.auto_power_off = v;
                }
            }
        }
        self.rebuild();
    }

    pub fn close(&self) {
        self.session.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sony_protocol::fake::FakeDevice;
    use sony_protocol::DataType;

    fn ult_device() -> Arc<FakeDevice> {
        Arc::new(FakeDevice::new(Box::new(|_, p| {
            let mdr = |v: Vec<u8>| vec![(DataType::DataMdr, v)];
            match p {
                [0x22, 0x00] => mdr(vec![0x23, 0x00, 73, 0]),
                [0x66, 0x17] => mdr(vec![0x67, 0x17, 1, 1, 0, 0, 0]),
                [0x56, 0x03] => mdr(vec![0x57, 0x03, 0x16, 2, 6, 17, 10, 10, 10, 10, 10]),
                [0x12, 0x02] => mdr(vec![0x13, 0x02, 0x02]),
                [0x04, 0x02] => mdr(vec![0x05, 0x02, 5, b'2', b'.', b'1']),
                [0xe6, 0x01] => mdr(vec![0xe7, 0x01, 1]),
                [0xf6, 0x01] => mdr(vec![0xf7, 0x01, 0]),
                [0x26, 0x05] => mdr(vec![0x27, 0x05, 0x11, 0x10]),
                _ => vec![],
            }
        })))
    }

    #[test]
    fn ult_wear_connect_reads_expected_state() {
        let link = Link::open(ult_device(), "A8:E6:E8:EA:71:E9", "ULT WEAR").unwrap();
        let s = link.snapshot();
        assert!(s.connected);
        assert_eq!((s.battery, s.noise_mode.as_str()), (73, "cancelling"));
        assert_eq!((s.eq_preset, s.eq_preset_name.as_str(), s.ult_mode), (0x16, "Bass Boost", 2));
        assert_eq!(s.clear_bass, 7);
        assert!(s.caps.ult_bass && s.caps.pause_when_removed && s.pause_when_removed);
        assert!(s.dsee && s.codec == "AAC");
        assert_eq!(s.features["battery"], "valid");
        // ULT WEAR has no speak-to-chat / adaptive volume: never even queried.
        assert_eq!(s.features["speakToChat"], "unsupported");
        assert_eq!(s.features["adaptiveVolume"], "unsupported");
    }

    #[test]
    fn unsupported_features_are_not_queried() {
        let dev = ult_device();
        let _ = Link::open(dev.clone(), "A8:E6:E8:EA:71:E9", "ULT WEAR").unwrap();
        // speak-to-chat (f6 0c) and adaptive volume (f6 0a) must never be sent to a ULT WEAR.
        assert!(!dev
            .sent_frames()
            .iter()
            .any(|f| f.payload[0] == 0xf6 && matches!(f.payload[1], 0x0a | 0x0c)));
    }

    #[test]
    fn notifications_update_the_snapshot() {
        let mut link = Link::open(ult_device(), "A8:E6:E8:EA:71:E9", "ULT WEAR").unwrap();
        assert!(link.on_notification(&[0x23, 0, 40, 1]));
        assert_eq!((link.snapshot().battery, link.snapshot().charging), (40, true));
        assert!(link.on_notification(&[0x67, 0x17, 1, 1, 1, 0, 9]));
        assert_eq!(link.snapshot().noise_mode, "ambient");
        assert_eq!(link.snapshot().ambient_level, 9);
        assert!(!link.on_notification(&[0x99, 1, 2, 3]));
    }

    #[test]
    fn set_preset_keeps_ult_mode() {
        let dev = ult_device();
        let mut link = Link::open(dev.clone(), "A8:E6:E8:EA:71:E9", "ULT WEAR").unwrap();
        link.apply(Command::EqPreset(0x11)).unwrap();
        let last_set = dev.sent_frames().into_iter().rev().find(|f| f.payload[0] == 0x58).unwrap();
        assert_eq!(last_set.payload, vec![0x58, 0x03, 0x11, 2, 0]);
    }

    #[test]
    fn ambient_level_out_of_range_reuses_last_confirmed() {
        let dev = ult_device();
        let mut link = Link::open(dev.clone(), "A8:E6:E8:EA:71:E9", "ULT WEAR").unwrap();
        link.apply(Command::Ambient { level: 0, focus_on_voice: false }).unwrap();
        let set = dev.sent_frames().into_iter().rev().find(|f| f.payload[0] == 0x68).unwrap();
        assert_eq!(set.payload[6], 10); // default ambient level, not 0
    }
}
