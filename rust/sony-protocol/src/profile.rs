//! Static device profiles (generated from the C++ `DeviceProfileRegistry`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolVersion {
    V1,
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SonyModel {
    Unknown,
    WH1000XM3,
    WH1000XM4,
    WH1000XM5,
    WH1000XM6,
    WF1000XM4,
    WF1000XM5,
    WF1000XM6,
    WHCH720N,
    ULTWear,
    LinkBudsS,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeviceCapabilities {
    pub battery: bool,
    pub dual_battery: bool,
    pub noise_cancelling: bool,
    pub ambient_sound: bool,
    pub focus_on_voice: bool,
    pub equalizer: bool,
    pub clear_bass: bool,
    pub ten_band_equalizer: bool,
    pub dsee: bool,
    pub speak_to_chat: bool,
    pub adaptive_volume: bool,
    pub auto_power_off: bool,
    pub firmware_info: bool,
    pub codec_info: bool,
    pub wear_sensor: bool,
    pub multipoint: bool,
    pub ult_bass: bool,
    pub pause_when_removed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceProfile {
    pub model: SonyModel,
    pub protocol: ProtocolVersion,
    pub capabilities: DeviceCapabilities,
}

impl SonyModel {
    pub fn name(self) -> &'static str {
        match self {
            Self::WH1000XM3 => "WH-1000XM3",
            Self::WH1000XM4 => "WH-1000XM4",
            Self::WH1000XM5 => "WH-1000XM5",
            Self::WH1000XM6 => "WH-1000XM6",
            Self::WF1000XM4 => "WF-1000XM4",
            Self::WF1000XM5 => "WF-1000XM5",
            Self::WF1000XM6 => "WF-1000XM6",
            Self::WHCH720N => "WH-CH720N",
            Self::ULTWear => "ULT WEAR",
            Self::LinkBudsS => "LinkBuds S",
            Self::Unknown => "Unknown",
        }
    }
}

static PROFILES: &[DeviceProfile] = &[
    DeviceProfile { model: SonyModel::WH1000XM3, protocol: ProtocolVersion::V1, capabilities: DeviceCapabilities { battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, ..NO_CAPS } },
    DeviceProfile { model: SonyModel::WH1000XM4, protocol: ProtocolVersion::V1, capabilities: DeviceCapabilities { battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, equalizer: true, clear_bass: true, speak_to_chat: true, firmware_info: true, codec_info: true, wear_sensor: true, multipoint: true, ..NO_CAPS } },
    DeviceProfile { model: SonyModel::WH1000XM5, protocol: ProtocolVersion::V2, capabilities: DeviceCapabilities { battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, equalizer: true, clear_bass: true, dsee: true, speak_to_chat: true, adaptive_volume: true, auto_power_off: true, firmware_info: true, codec_info: true, wear_sensor: true, multipoint: true, ..NO_CAPS } },
    DeviceProfile { model: SonyModel::WH1000XM6, protocol: ProtocolVersion::V2, capabilities: DeviceCapabilities { battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, equalizer: true, ten_band_equalizer: true, dsee: true, speak_to_chat: true, adaptive_volume: true, auto_power_off: true, firmware_info: true, codec_info: true, wear_sensor: true, multipoint: true, ..NO_CAPS } },
    DeviceProfile { model: SonyModel::WF1000XM4, protocol: ProtocolVersion::V2, capabilities: DeviceCapabilities { battery: true, dual_battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, equalizer: true, clear_bass: true, dsee: true, speak_to_chat: true, auto_power_off: true, firmware_info: true, codec_info: true, wear_sensor: true, multipoint: true, ..NO_CAPS } },
    DeviceProfile { model: SonyModel::WF1000XM5, protocol: ProtocolVersion::V2, capabilities: DeviceCapabilities { battery: true, dual_battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, equalizer: true, clear_bass: true, dsee: true, speak_to_chat: true, adaptive_volume: true, auto_power_off: true, firmware_info: true, codec_info: true, wear_sensor: true, multipoint: true, ..NO_CAPS } },
    DeviceProfile { model: SonyModel::WF1000XM6, protocol: ProtocolVersion::V2, capabilities: DeviceCapabilities { battery: true, dual_battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, equalizer: true, clear_bass: true, dsee: true, speak_to_chat: true, adaptive_volume: true, auto_power_off: true, firmware_info: true, codec_info: true, wear_sensor: true, multipoint: true, ..NO_CAPS } },
    DeviceProfile { model: SonyModel::WHCH720N, protocol: ProtocolVersion::V2, capabilities: DeviceCapabilities { battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, equalizer: true, clear_bass: true, dsee: true, auto_power_off: true, firmware_info: true, codec_info: true, multipoint: true, ..NO_CAPS } },
    DeviceProfile { model: SonyModel::ULTWear, protocol: ProtocolVersion::V2, capabilities: DeviceCapabilities { battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, equalizer: true, clear_bass: true, dsee: true, auto_power_off: true, firmware_info: true, codec_info: true, wear_sensor: true, multipoint: true, ult_bass: true, pause_when_removed: true, ..NO_CAPS } },
    DeviceProfile { model: SonyModel::LinkBudsS, protocol: ProtocolVersion::V2, capabilities: DeviceCapabilities { battery: true, dual_battery: true, noise_cancelling: true, ambient_sound: true, focus_on_voice: true, equalizer: true, clear_bass: true, dsee: true, speak_to_chat: true, auto_power_off: true, firmware_info: true, codec_info: true, wear_sensor: true, multipoint: true, ..NO_CAPS } },
 ];

const NO_CAPS: DeviceCapabilities = DeviceCapabilities {
    battery: false,
    dual_battery: false,
    noise_cancelling: false,
    ambient_sound: false,
    focus_on_voice: false,
    equalizer: false,
    clear_bass: false,
    ten_band_equalizer: false,
    dsee: false,
    speak_to_chat: false,
    adaptive_volume: false,
    auto_power_off: false,
    firmware_info: false,
    codec_info: false,
    wear_sensor: false,
    multipoint: false,
    ult_bass: false,
    pause_when_removed: false,
};

/// Identify the model from the Bluetooth device name (e.g. "LE_WH-1000XM5").
pub fn identify_model(device_name: &str) -> SonyModel {
    let u = device_name.to_uppercase();
    let any = |keys: &[&str]| keys.iter().any(|k| u.contains(k));
    if any(&["WH-1000XM3", "WH1000XM3"]) { return SonyModel::WH1000XM3; }
    if any(&["WH-1000XM4", "WH1000XM4"]) { return SonyModel::WH1000XM4; }
    if any(&["WH-1000XM5", "WH1000XM5"]) { return SonyModel::WH1000XM5; }
    if any(&["WH-1000XM6", "WH1000XM6"]) { return SonyModel::WH1000XM6; }
    if any(&["WF-1000XM4", "WF1000XM4"]) { return SonyModel::WF1000XM4; }
    if any(&["WF-1000XM5", "WF1000XM5"]) { return SonyModel::WF1000XM5; }
    if any(&["WF-1000XM6", "WF1000XM6"]) { return SonyModel::WF1000XM6; }
    if any(&["WH-CH720N", "CH720N"]) { return SonyModel::WHCH720N; }
    if any(&["ULT WEAR", "WH-ULT900N", "ULT900N"]) { return SonyModel::ULTWear; }
    if any(&["LINKBUDS S", "WF-LS900N", "LS900N"]) { return SonyModel::LinkBudsS; }
    SonyModel::Unknown
}

pub fn get_profile(model: SonyModel) -> Option<DeviceProfile> {
    PROFILES.iter().find(|p| p.model == model).copied()
}

/// Known profile for the device name, or an empty V1 fallback (probed dynamically).
pub fn profile_for_device(device_name: &str) -> DeviceProfile {
    get_profile(identify_model(device_name)).unwrap_or(DeviceProfile {
        model: SonyModel::Unknown,
        protocol: ProtocolVersion::V1,
        capabilities: NO_CAPS,
    })
}

pub fn all_profiles() -> &'static [DeviceProfile] {
    PROFILES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_models_from_advertised_names() {
        assert_eq!(identify_model("LE_WH-1000XM5"), SonyModel::WH1000XM5);
        assert_eq!(identify_model("ULT WEAR"), SonyModel::ULTWear);
        assert_eq!(identify_model("my headphones"), SonyModel::Unknown);
        assert_eq!(identify_model(""), SonyModel::Unknown);
    }

    #[test]
    fn ult_wear_profile_has_ult_bass_and_pause_on_removal() {
        let p = profile_for_device("ULT WEAR");
        assert_eq!(p.protocol, ProtocolVersion::V2);
        assert!(p.capabilities.ult_bass && p.capabilities.pause_when_removed);
        assert!(!p.capabilities.speak_to_chat);
    }

    #[test]
    fn xm4_is_v1_and_unknown_has_no_capabilities() {
        assert_eq!(profile_for_device("WH-1000XM4").protocol, ProtocolVersion::V1);
        let u = profile_for_device("something");
        assert_eq!(u.model, SonyModel::Unknown);
        assert!(!u.capabilities.battery);
    }
}
