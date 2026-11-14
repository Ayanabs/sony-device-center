use crate::codec::decode_equalizer;
use crate::semantic::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceState {
    pub battery: BatteryState,
    pub noise_control: NoiseControlState,
    pub equalizer: EqualizerState,
}

/// What an unsolicited notification changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Changed {
    Battery,
    NoiseControl,
    Equalizer,
}

/// Applies a push notification to `state` (port of `parseNotificationPayload`).
/// Returns `None` when the payload isn't a recognised state change.
pub fn apply_notification(payload: &[u8], state: &mut DeviceState, ult_bass: bool) -> Option<Changed> {
    let p = payload;
    let op = *p.first()?;
    match op {
        // Battery: 0x25 / 0x23
        0x25 | 0x23 => {
            let b = &mut state.battery;
            if p.len() >= 4 && p[1] == 0x00 {
                b.main = Some(p[2] as i32);
                b.charging = p[3] == 1;
                return Some(Changed::Battery);
            }
            if p.len() >= 6 && p[1] == 0x09 {
                b.left = Some(p[2] as i32);
                b.right = Some(p[4] as i32);
                b.main = Some((p[2] as i32).min(p[4] as i32));
                b.charging = p[3] == 1 || p[5] == 1;
                return Some(Changed::Battery);
            }
            if p.len() >= 4 && p[1] == 0x0a {
                b.case_battery = Some(p[2] as i32);
                return Some(Changed::Battery);
            }
            None
        }
        // Noise control / ambient sound: 0x67 / 0x69 (V2 layout)
        0x67 | 0x69 if p.len() >= 7 && p[1] == 0x17 && p[2] == 0x01 => {
            let (on, ambient) = (p[3] != 0, p[4] != 0);
            let n = &mut state.noise_control;
            n.mode = if !on {
                NoiseControlMode::Off
            } else if ambient {
                NoiseControlMode::Ambient
            } else {
                NoiseControlMode::NoiseCancelling
            };
            n.ambient_level = if ambient { p[6] as i32 } else { 0 };
            n.focus_on_voice = p[5] != 0;
            Some(Changed::NoiseControl)
        }
        0x57 | 0x59 => {
            state.equalizer = decode_equalizer(p, ult_bass).ok()?;
            Some(Changed::Equalizer)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_and_nc_notifications() {
        let mut s = DeviceState::default();
        assert_eq!(apply_notification(&[0x23, 0, 55, 1], &mut s, false), Some(Changed::Battery));
        assert_eq!((s.battery.main, s.battery.charging), (Some(55), true));
        assert_eq!(
            apply_notification(&[0x67, 0x17, 1, 1, 0, 1, 3], &mut s, false),
            Some(Changed::NoiseControl)
        );
        assert_eq!(s.noise_control.mode, NoiseControlMode::NoiseCancelling);
        assert!(s.noise_control.focus_on_voice);
    }

    #[test]
    fn tws_battery_takes_the_lower_side() {
        let mut s = DeviceState::default();
        apply_notification(&[0x25, 0x09, 80, 0, 60, 1], &mut s, false).unwrap();
        assert_eq!((s.battery.main, s.battery.charging), (Some(60), true));
    }

    #[test]
    fn unknown_and_short_payloads_are_ignored() {
        let mut s = DeviceState::default();
        assert_eq!(apply_notification(&[], &mut s, false), None);
        assert_eq!(apply_notification(&[0x23, 0, 5], &mut s, false), None);
        assert_eq!(apply_notification(&[0x99, 0, 0, 0], &mut s, false), None);
    }
}
