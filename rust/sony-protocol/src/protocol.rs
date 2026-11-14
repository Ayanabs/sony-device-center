use crate::error::{Result, SonyError, SonyErrorCode};
use crate::semantic::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolGeneration {
    V1,
    V2,
}

fn unsupported(what: &str) -> SonyError {
    SonyError::new(SonyErrorCode::Unsupported, format!("{what} is not supported by this protocol"))
}

pub trait Protocol: Send + Sync {
    fn generation(&self) -> ProtocolGeneration;
    fn init_device(&self) -> Result<()>;
    fn get_battery(&self) -> Result<BatteryState>;
    fn get_noise_control(&self) -> Result<NoiseControlState>;
    fn set_noise_control(&self, state: &NoiseControlState) -> Result<()>;
    fn get_equalizer(&self) -> Result<EqualizerState>;
    fn set_equalizer_preset(&self, preset: i32) -> Result<()>;
    /// `bands.len()` is 5 for legacy devices, 10 for ten-band devices
    /// (`clear_bass` is ignored in the latter case).
    fn set_equalizer_custom(&self, clear_bass: i32, bands: &[i32]) -> Result<()>;
    fn get_dsee(&self) -> Result<bool>;
    fn set_dsee(&self, enabled: bool) -> Result<()>;
    fn get_firmware_version(&self) -> Result<String>;
    fn get_codec(&self) -> Result<String>;
    fn get_auto_power_off(&self) -> Result<i32>;
    fn set_auto_power_off(&self, index: i32) -> Result<()>;
    fn get_speak_to_chat(&self) -> Result<bool>;
    fn set_speak_to_chat(&self, enabled: bool) -> Result<()>;
    fn get_adaptive_volume(&self) -> Result<bool>;
    fn set_adaptive_volume(&self, enabled: bool) -> Result<()>;

    fn get_pause_when_removed(&self) -> Result<bool> {
        Err(unsupported("Pause on removal"))
    }
    fn set_pause_when_removed(&self, _enabled: bool) -> Result<()> {
        Err(unsupported("Pause on removal"))
    }
    fn get_connected_devices(&self) -> Result<Vec<ConnectedDevice>> {
        Err(unsupported("Headset device list"))
    }
}
