//! V1 command set (WH-1000XM3/XM4 era). Byte layouts match the C++ port.
//!
//! CRITICAL: opcode 0x22 is POWER OFF on this generation. It must never be
//! sent from this file; a test pins that for the battery path.

use crate::codec::*;
use crate::error::{Result, SonyError, SonyErrorCode};
use crate::frame::{DataType, SonyFrame};
use crate::protocol::{Protocol, ProtocolGeneration};
use crate::semantic::*;
use crate::session::{Session, DEFAULT_TIMEOUT};
use std::sync::Arc;
use std::time::Duration;

const NC_ASM: u8 = 0x02;
const EQ: u8 = 0x01;
const STC: u8 = 0x05; // SMART_TALKING_MODE (not V2's 0x0c)
const STC_ON_OFF: u8 = 0x01;
const STC_CONFIG: u8 = 0x00;
const STC_SESSION_ACTIVE: u8 = 0x02;
const STC_CONFIG_SET: u8 = 0xfc;
const T: Duration = Duration::from_millis(1000);

fn invalid(msg: &str) -> SonyError {
    SonyError::new(SonyErrorCode::InvalidResponse, msg)
}

fn unsupported(msg: &str) -> SonyError {
    SonyError::new(SonyErrorCode::Unsupported, msg)
}

pub struct ProtocolV1 {
    session: Arc<Session>,
}

impl ProtocolV1 {
    pub fn new(session: Arc<Session>) -> Self {
        Self { session }
    }

    fn mdr(payload: Vec<u8>) -> SonyFrame {
        SonyFrame { data_type: DataType::DataMdr, sequence: 0, payload }
    }

    fn request(&self, payload: &[u8], ret: u8, sub: i32) -> Result<SonyFrame> {
        self.session.send_and_await(&Self::mdr(payload.to_vec()), ret, sub, T)
    }

    fn send(&self, payload: Vec<u8>) -> Result<()> {
        self.session.send(&Self::mdr(payload), DEFAULT_TIMEOUT)
    }

    pub fn set_vpt(&self, preset: i32) -> Result<()> {
        self.send(vec![0x48, 0x01, preset as u8])
    }

    pub fn set_sound_position(&self, preset: i32) -> Result<()> {
        self.send(vec![0x48, 0x02, preset as u8])
    }
}

impl Protocol for ProtocolV1 {
    fn generation(&self) -> ProtocolGeneration {
        ProtocolGeneration::V1
    }

    fn init_device(&self) -> Result<()> {
        // Best-effort poll; the first reply on an XM4 is slow, so wait longer.
        let _ = self.session.send_and_await(
            &Self::mdr(vec![0x66, NC_ASM]),
            0x67,
            NC_ASM as i32,
            Duration::from_millis(1500),
        );
        Ok(())
    }

    fn get_battery(&self) -> Result<BatteryState> {
        let mut st = BatteryState::default();
        // Single: 10 00 -> 11 00 <level> <charging>
        if let Ok(r) = self.request(&[0x10, 0x00], 0x11, 0x00) {
            if r.payload.len() >= 4 {
                st.main = Some(r.payload[2] as i32);
                st.charging = r.payload[3] == 1;
                return Ok(st);
            }
        }
        // Dual: 10 01 -> 11 01 <Llvl> <Lchg> <Rlvl> <Rchg>
        if let Ok(r) = self.request(&[0x10, 0x01], 0x11, 0x01) {
            if r.payload.len() >= 6 {
                let (l, rt) = (r.payload[2] as i32, r.payload[4] as i32);
                st.charging = r.payload[3] == 1 || r.payload[5] == 1;
                if rt == 0 && l > 0 {
                    // Over-ear echoes its single cell into the left slot.
                    st.main = Some(l);
                } else {
                    st.left = Some(l);
                    st.right = Some(rt);
                    st.main = Some(l.min(rt));
                }
            }
        }
        // Case: 10 02 -> 11 02 <level> <charging>
        if let Ok(r) = self.request(&[0x10, 0x02], 0x11, 0x02) {
            if r.payload.len() >= 4 {
                st.case_battery = Some(r.payload[2] as i32);
            }
        }
        Ok(st)
    }

    fn get_noise_control(&self) -> Result<NoiseControlState> {
        // 66 02 -> 67 02 <effect> <ncType> <dualSingle> <asmType> <asmId> <asmLevel>
        let r = self.request(&[0x66, NC_ASM], 0x67, NC_ASM as i32)?;
        let p = &r.payload;
        if p.len() < 8 || p[1] != NC_ASM {
            return Err(invalid("Malformed noise-control response"));
        }
        let on = p[2] != 0;
        let ambient = p[4] == 0;
        let mode = if !on {
            NoiseControlMode::Off
        } else if ambient {
            NoiseControlMode::Ambient
        } else {
            NoiseControlMode::NoiseCancelling
        };
        Ok(NoiseControlState {
            mode,
            ambient_level: if mode == NoiseControlMode::Ambient { p[7] as i32 } else { 0 },
            focus_on_voice: p[6] == 1,
        })
    }

    fn set_noise_control(&self, s: &NoiseControlState) -> Result<()> {
        let off = s.mode == NoiseControlMode::Off;
        let ambient = s.mode == NoiseControlMode::Ambient;
        let level = if ambient { s.ambient_level.clamp(1, 20) as u8 } else { 0 };
        self.send(vec![
            0x68,
            NC_ASM,
            if off { 0x00 } else { 0x11 },
            0x01,
            if ambient || off { 0x00 } else { 0x02 },
            0x01,
            s.focus_on_voice as u8,
            level,
        ])
    }

    fn get_equalizer(&self) -> Result<EqualizerState> {
        // 56 01 -> 57 01 <preset> 06 <bass+10> <b1..b5 +10>
        let r = self.request(&[0x56, EQ], 0x57, EQ as i32)?;
        let p = &r.payload;
        if p.len() < 10 || p[1] != EQ {
            return Err(invalid("Incomplete equalizer response"));
        }
        Ok(EqualizerState {
            preset: p[2] as i32,
            clear_bass: p[4] as i32 - 10,
            ult_mode: -1,
            bands: (0..5).map(|i| p[5 + i] as i32 - 10).collect(),
        })
    }

    fn set_equalizer_preset(&self, preset: i32) -> Result<()> {
        self.send(vec![0x58, EQ, preset as u8, 0x00])
    }

    fn set_equalizer_custom(&self, clear_bass: i32, bands: &[i32]) -> Result<()> {
        let mut p = vec![0x58, EQ, 0xa0, (1 + bands.len()) as u8, clamp_eq_value(clear_bass)];
        p.extend(bands.iter().map(|&b| clamp_eq_value(b)));
        self.send(p)
    }

    fn get_dsee(&self) -> Result<bool> {
        Err(unsupported("DSEE is not supported on Protocol V1"))
    }
    fn set_dsee(&self, _: bool) -> Result<()> {
        Err(unsupported("DSEE is not supported on Protocol V1"))
    }

    fn get_firmware_version(&self) -> Result<String> {
        let r = self.request(&[0x04, 0x02], 0x05, -1)?;
        Ok(if r.payload.len() > 3 {
            String::from_utf8_lossy(&r.payload[3..]).into_owned()
        } else {
            String::new()
        })
    }

    fn get_codec(&self) -> Result<String> {
        let r = self.request(&[0x18, 0x00], 0x19, -1)?;
        Ok(r.payload.get(2).map(|&c| codec_name(c).to_string()).unwrap_or_default())
    }

    fn get_auto_power_off(&self) -> Result<i32> {
        Err(unsupported("Auto Power Off is not supported on Protocol V1"))
    }
    fn set_auto_power_off(&self, _: i32) -> Result<()> {
        Err(unsupported("Auto Power Off is not supported on Protocol V1"))
    }

    fn get_speak_to_chat(&self) -> Result<bool> {
        // f6 05 -> f7 05 <kind> <onOff>. Kind 0x02 is an active session: still "on".
        let r = self.request(&[0xf6, STC], 0xf7, STC as i32)?;
        let p = &r.payload;
        if p.len() < 4 || p[1] != STC {
            return Err(invalid("Incomplete SpeakToChat response"));
        }
        Ok(p[2] == STC_SESSION_ACTIVE || p[3] != 0)
    }

    fn set_speak_to_chat(&self, enabled: bool) -> Result<()> {
        // Config (auto sensitivity, no voice focus, ~30 s timeout) first, so the
        // timer exists before the feature arms.
        if enabled {
            self.send(vec![STC_CONFIG_SET, STC, STC_CONFIG, 0x00, 0x00, 0x01])?;
        }
        self.send(vec![0xf8, STC, STC_ON_OFF, enabled as u8])
    }

    fn get_adaptive_volume(&self) -> Result<bool> {
        Err(unsupported("Adaptive Volume is not supported on Protocol V1"))
    }
    fn set_adaptive_volume(&self, _: bool) -> Result<()> {
        Err(unsupported("Adaptive Volume is not supported on Protocol V1"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeDevice;

    fn setup(
        f: impl Fn(DataType, &[u8]) -> Vec<(DataType, Vec<u8>)> + Send + Sync + 'static,
    ) -> (Arc<FakeDevice>, ProtocolV1) {
        let dev = Arc::new(FakeDevice::new(Box::new(f)));
        let session = Arc::new(Session::new(dev.clone()));
        session.connect("00:00:00:00:00:00").unwrap();
        (dev, ProtocolV1::new(session))
    }

    #[test]
    fn battery_never_sends_power_off_opcode() {
        let (dev, p) = setup(|_, req| match req {
            [0x10, 0x01] => vec![(DataType::DataMdr, vec![0x11, 0x01, 60, 0, 0, 0])],
            _ => vec![],
        });
        let b = p.get_battery().unwrap();
        // Over-ear echo (right == 0): single reading, not min(left, right) == 0.
        assert_eq!(b.main, Some(60));
        assert!(dev.sent_frames().iter().all(|f| f.payload[0] != 0x22));
    }

    #[test]
    fn noise_control_ambient_roundtrip() {
        let (dev, p) = setup(|_, req| match req {
            [0x66, 0x02] => vec![(DataType::DataMdr, vec![0x67, 0x02, 0x11, 1, 0, 1, 1, 7])],
            _ => vec![],
        });
        let nc = p.get_noise_control().unwrap();
        assert_eq!((nc.mode, nc.ambient_level, nc.focus_on_voice), (NoiseControlMode::Ambient, 7, true));
        p.set_noise_control(&nc).unwrap();
        assert_eq!(dev.sent_frames().pop().unwrap().payload, vec![0x68, 2, 0x11, 1, 0, 1, 1, 7]);
    }

    #[test]
    fn speak_to_chat_enable_sends_config_then_enable() {
        let (dev, p) = setup(|_, _| vec![]);
        p.set_speak_to_chat(true).unwrap();
        let sent: Vec<_> = dev.sent_frames().into_iter().map(|f| f.payload).collect();
        assert_eq!(sent, vec![vec![0xfc, 5, 0, 0, 0, 1], vec![0xf8, 5, 1, 1]]);
    }

    #[test]
    fn v2_only_features_are_unsupported() {
        let (_d, p) = setup(|_, _| vec![]);
        assert_eq!(p.get_dsee().unwrap_err().code, SonyErrorCode::Unsupported);
        assert_eq!(p.get_auto_power_off().unwrap_err().code, SonyErrorCode::Unsupported);
        assert_eq!(p.get_pause_when_removed().unwrap_err().code, SonyErrorCode::Unsupported);
    }
}
