//! Equalizer preset ids and names (matches C++ `EqualizerPresets`).

pub const PRESETS: &[(i32, &str, &str)] = &[
    (0x00, "off", "Off"),
    (0x10, "bright", "Bright"),
    (0x11, "excited", "Excited"),
    (0x12, "mellow", "Mellow"),
    (0x13, "relaxed", "Relaxed"),
    (0x14, "vocal", "Vocal"),
    (0x15, "treble-boost", "Treble Boost"),
    (0x16, "bass-boost", "Bass Boost"),
    (0x17, "speech", "Speech"),
    (0xa0, "manual", "Custom"),
];

pub fn preset_name(preset: i32) -> String {
    PRESETS
        .iter()
        .find(|(id, _, _)| *id == preset)
        .map(|(_, _, n)| (*n).to_string())
        .unwrap_or_else(|| format!("Preset ({preset})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_known_and_unknown_presets() {
        assert_eq!(preset_name(0x16), "Bass Boost");
        assert_eq!(preset_name(22), "Bass Boost"); // what the ULT WEAR reported
        assert_eq!(preset_name(0xa0), "Custom");
        assert_eq!(preset_name(99), "Preset (99)");
    }
}
