#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BatteryState {
    pub main: Option<i32>,
    pub left: Option<i32>,
    pub right: Option<i32>,
    pub case_battery: Option<i32>,
    pub charging: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedDevice {
    pub address: String,
    pub name: String,
    /// Headset-selected playback device; this does not imply media is playing.
    pub playback_device: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NoiseControlMode {
    #[default]
    Off,
    NoiseCancelling,
    Ambient,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoiseControlState {
    pub mode: NoiseControlMode,
    pub ambient_level: i32,
    pub focus_on_voice: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqualizerState {
    pub preset: i32,
    pub clear_bass: i32,
    /// ULT WEAR reports this separately from its EQ preset: 0=Off, 1/2=ULT. -1 = n/a.
    pub ult_mode: i32,
    /// 5 bands (+ clear_bass) on legacy devices; 10 bands on newer ones (e.g. WH-1000XM6).
    pub bands: Vec<i32>,
}

impl Default for EqualizerState {
    fn default() -> Self {
        Self { preset: 0, clear_bass: 0, ult_mode: -1, bands: vec![0; 5] }
    }
}
