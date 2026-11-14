use crate::error::{Result, SonyError, SonyErrorCode};
use crate::semantic::{ConnectedDevice, EqualizerState};

fn invalid(msg: &str) -> SonyError {
    SonyError::new(SonyErrorCode::InvalidResponse, msg)
}

/// ULT WEAR: 57/59 03 <preset> <ULT mode> <count> <bass+10> <five bands+10>.
/// The device also answers 56 00 with this extended layout, so the profile may
/// identify that alias via `ult_bass`.
pub fn decode_equalizer(p: &[u8], ult_bass: bool) -> Result<EqualizerState> {
    if p.len() < 4 || (p[0] != 0x57 && p[0] != 0x59) || !matches!(p[1], 0 | 1 | 3 | 4) {
        return Err(invalid("Incomplete equalizer response"));
    }
    let ult = p[1] == 0x03 || (ult_bass && p[1] == 0x00);
    let count_index = if ult { 4 } else { 3 };
    if p.len() <= count_index || (ult && p[3] > 2) {
        return Err(invalid("Invalid ULT equalizer response"));
    }
    let count = p[count_index] as usize;
    let start = count_index + 1;
    let ten_band = p[1] == 0x04;
    if (count != 0 && count != if ten_band { 10 } else { 6 }) || p.len() < start + count {
        return Err(invalid("Truncated or unexpected equalizer bands"));
    }
    let mut st = EqualizerState {
        preset: p[2] as i32,
        ult_mode: if ult { p[3] as i32 } else { -1 },
        bands: vec![0; if ten_band { 10 } else { 5 }],
        clear_bass: 0,
    };
    if count != 0 {
        if !ten_band {
            st.clear_bass = p[start] as i32 - 10;
        }
        for i in 0..st.bands.len() {
            let raw = p[start + i + if ten_band { 0 } else { 1 }] as i32;
            st.bands[i] = raw - if ten_band { 0 } else { 10 };
        }
    }
    Ok(st)
}

/// V2 table-2 PERI RET/NTFY PARAM. Subtype 0 adds a length-prefixed name after
/// the address and connection slot; subtype 2 also carries a 3-byte CoD.
pub fn decode_connected_devices(p: &[u8]) -> Result<Vec<ConnectedDevice>> {
    let bad = || invalid("Malformed headset device list");
    if p.len() < 4 || (p[0] != 0x37 && p[0] != 0x39) || (p[1] != 0 && p[1] != 2) {
        return Err(bad());
    }
    let mut devices = Vec::new();
    let mut slots = Vec::new();
    let mut pos = 3usize;
    for _ in 0..p[2] {
        let fixed = if p[1] == 2 { 22 } else { 19 };
        if p.len() - pos < fixed {
            return Err(bad());
        }
        let raw = &p[pos..pos + 17];
        let mut address = String::with_capacity(17);
        for (j, &ch) in raw.iter().enumerate() {
            let ok = if j % 3 == 2 { ch == b':' } else { ch.is_ascii_hexdigit() };
            if !ok {
                return Err(bad());
            }
            address.push(ch.to_ascii_uppercase() as char);
        }
        pos += 17;
        let slot = p[pos];
        pos += 1;
        if p[1] == 2 {
            pos += 3;
        }
        let length = p[pos] as usize;
        pos += 1;
        if p.len() - pos < length {
            return Err(bad());
        }
        let name = String::from_utf8_lossy(&p[pos..pos + length]).into_owned();
        pos += length;
        if slot != 0 {
            devices.push(ConnectedDevice { address, name, playback_device: false });
            slots.push(slot);
        }
    }
    if p.len() - pos != 1 {
        return Err(bad());
    }
    let playback = p[pos];
    for (d, s) in devices.iter_mut().zip(&slots) {
        d.playback_device = playback != 0 && *s == playback;
    }
    Ok(devices)
}

pub fn codec_name(code: u8) -> &'static str {
    match code {
        0x01 => "SBC",
        0x02 => "AAC",
        0x10 => "LDAC",
        0x20 => "aptX",
        0x21 => "aptX HD",
        _ => "",
    }
}

/// Legacy equalizer values are encoded as value+10.
pub fn clamp_eq_value(v: i32) -> u8 {
    (v.clamp(-10, 10) + 10) as u8
}

/// The 10-band layout uses raw unsigned device values 0..=12 with no bias.
pub fn clamp_eq_value_raw(v: i32) -> u8 {
    v.clamp(0, 12) as u8
}
