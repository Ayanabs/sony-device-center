use crate::codec::*;
use crate::error::{Result, SonyError, SonyErrorCode};
use crate::frame::{DataType, SonyFrame};
use crate::protocol::{Protocol, ProtocolGeneration};
use crate::semantic::*;
use crate::session::{Session, DEFAULT_TIMEOUT};
use std::sync::Arc;
use std::time::Duration;

const T: Duration = Duration::from_millis(1000);

/// Auto-power-off codes: 0=Off, 1=5min, 2=30min, 3=1h, 4=3h, 5=when-taken-off.
const APO_CODES: [(u8, u8); 6] =
    [(0x11, 0x00), (0x00, 0x00), (0x01, 0x01), (0x02, 0x02), (0x03, 0x03), (0x10, 0x00)];

fn apo_index(c0: u8, c1: u8) -> i32 {
    APO_CODES.iter().position(|&(a, b)| a == c0 && b == c1).unwrap_or(0) as i32
}

fn invalid(msg: &str) -> SonyError {
    SonyError::new(SonyErrorCode::InvalidResponse, msg)
}

pub struct ProtocolV2 {
    session: Arc<Session>,
    /// Newer 10-band equalizer wire format (inquired type 0x04), e.g. WH-1000XM6.
    ten_band: bool,
    /// ULT WEAR: extended equalizer layout (inquired type 0x03).
    ult_bass: bool,
}

impl ProtocolV2 {
    pub fn new(session: Arc<Session>, ten_band: bool, ult_bass: bool) -> Self {
        Self { session, ten_band, ult_bass }
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
}

impl Protocol for ProtocolV2 {
    fn generation(&self) -> ProtocolGeneration {
        ProtocolGeneration::V2
    }

    fn init_device(&self) -> Result<()> {
        // Handshake: 00 00 -> RET 01. Failure is tolerated.
        let _ = self.request(&[0x00, 0x00], 0x01, -1);
        Ok(())
    }

    fn get_connected_devices(&self) -> Result<Vec<ConnectedDevice>> {
        // Support changes when multipoint is switched on/off; inquire each refresh.
        let table2 = |p: Vec<u8>| SonyFrame { data_type: DataType::DataMdrNo2, sequence: 0, payload: p };
        let support = self.session.send_and_await(&table2(vec![0x06, 0]), 0x07, 0, T)?;
        let p = &support.payload;
        if p.len() < 3 || p.len() != 3 + 2 * p[2] as usize {
            return Err(invalid("Malformed table-2 support list"));
        }
        let mut subtype: i32 = -1;
        for i in (3..p.len()).step_by(2) {
            if p[i] == 0x30 && subtype < 0 {
                subtype = 0;
            }
            if p[i] == 0x32 || p[i] == 0x33 {
                subtype = 2;
            }
        }
        if subtype < 0 {
            return Err(SonyError::new(
                SonyErrorCode::Unsupported,
                "Headset device list unavailable; enable multipoint to use this feature",
            ));
        }
        let reply =
            self.session.send_and_await(&table2(vec![0x36, subtype as u8]), 0x37, subtype, T)?;
        decode_connected_devices(&reply.payload)
    }

    fn get_battery(&self) -> Result<BatteryState> {
        let mut st = BatteryState::default();
        // Single battery: 22 00 -> 23 00 <level> <charging>
        if let Ok(r) = self.request(&[0x22, 0x00], 0x23, 0x00) {
            if r.payload.len() >= 4 {
                st.main = Some(r.payload[2] as i32);
                st.charging = r.payload[3] == 1;
                return Ok(st);
            }
        }
        // TWS L/R: 22 09 -> 23 09 <Llvl> <Lchg> <Rlvl> <Rchg>
        if let Ok(r) = self.request(&[0x22, 0x09], 0x23, 0x09) {
            if r.payload.len() >= 6 {
                let (l, rt) = (r.payload[2] as i32, r.payload[4] as i32);
                st.left = Some(l);
                st.right = Some(rt);
                st.main = Some(l.min(rt));
                st.charging = r.payload[3] == 1 || r.payload[5] == 1;
            }
        }
        // Case: 22 0a -> 23 0a <level> <chg>
        if let Ok(r) = self.request(&[0x22, 0x0a], 0x23, 0x0a) {
            if r.payload.len() >= 4 {
                st.case_battery = Some(r.payload[2] as i32);
            }
        }
        Ok(st)
    }

    fn get_noise_control(&self) -> Result<NoiseControlState> {
        // 66 17 -> 67 17 01 <effect> <settingType 0=NC/1=Ambient> <voice> <level>
        let r = self.request(&[0x66, 0x17], 0x67, -1)?;
        let p = &r.payload;
        if p.len() < 7 || p[1] != 0x17 || p[2] != 1 {
            return Err(invalid("Malformed noise-control response"));
        }
        let (on, ambient) = (p[3] != 0, p[4] != 0);
        Ok(NoiseControlState {
            mode: if !on {
                NoiseControlMode::Off
            } else if ambient {
                NoiseControlMode::Ambient
            } else {
                NoiseControlMode::NoiseCancelling
            },
            ambient_level: if ambient { p[6] as i32 } else { 0 },
            focus_on_voice: p[5] != 0,
        })
    }

    fn set_noise_control(&self, s: &NoiseControlState) -> Result<()> {
        let effect = (s.mode != NoiseControlMode::Off) as u8;
        let setting = (s.mode == NoiseControlMode::Ambient) as u8;
        let level = if s.ambient_level > 0 { s.ambient_level as u8 } else { 1 };
        self.send(vec![0x68, 0x17, 0x01, effect, setting, s.focus_on_voice as u8, level])
    }

    fn get_equalizer(&self) -> Result<EqualizerState> {
        // Legacy 56 00 / ten-band 56 04 / ULT 56 03
        let inq = if self.ult_bass { 0x03 } else if self.ten_band { 0x04 } else { 0x00 };
        let r = self.request(&[0x56, inq], 0x57, inq as i32)?;
        if r.payload.len() < 4 || r.payload[1] != inq {
            return Err(invalid("Incomplete equalizer response"));
        }
        decode_equalizer(&r.payload, self.ult_bass)
    }

    fn set_equalizer_preset(&self, preset: i32) -> Result<()> {
        if self.ult_bass {
            // Preserve the current ULT mode so selecting an EQ does not change it.
            let eq = self.get_equalizer()?;
            return self.send(vec![0x58, 0x03, preset as u8, eq.ult_mode as u8, 0x00]);
        }
        self.send(vec![0x58, if self.ten_band { 0x04 } else { 0x00 }, preset as u8, 0x00])
    }

    fn set_equalizer_custom(&self, clear_bass: i32, bands: &[i32]) -> Result<()> {
        if self.ult_bass {
            if bands.len() != 5 {
                return Err(invalid("ULT WEAR requires five equalizer bands"));
            }
            let eq = self.get_equalizer()?;
            let mut p = vec![0x58, 0x03, 0xa0, eq.ult_mode as u8, 0x06, clamp_eq_value(clear_bass)];
            p.extend(bands.iter().map(|&b| clamp_eq_value(b)));
            return self.send(p);
        }
        if self.ten_band {
            let mut p = vec![0x58, 0x04, 0xa0, bands.len() as u8];
            p.extend(bands.iter().map(|&b| clamp_eq_value_raw(b)));
            return self.send(p);
        }
        let mut p = vec![0x58, 0x00, 0xa0, (1 + bands.len()) as u8, clamp_eq_value(clear_bass)];
        p.extend(bands.iter().map(|&b| clamp_eq_value(b)));
        self.send(p)
    }

    fn get_pause_when_removed(&self) -> Result<bool> {
        let r = self.request(&[0xf6, 0x01], 0xf7, 0x01)?;
        if r.payload.len() != 3 || r.payload[2] > 1 {
            return Err(invalid("Invalid pause-on-removal response"));
        }
        Ok(r.payload[2] == 0) // disabled flag: zero enables automatic pause
    }

    fn set_pause_when_removed(&self, enabled: bool) -> Result<()> {
        self.send(vec![0xf8, 0x01, if enabled { 0 } else { 1 }])
    }

    fn get_dsee(&self) -> Result<bool> {
        let r = self.request(&[0xe6, 0x01], 0xe7, 0x01)?;
        r.payload.get(2).map(|&v| v != 0).ok_or_else(|| invalid("Incomplete Dsee response"))
    }

    fn set_dsee(&self, enabled: bool) -> Result<()> {
        self.send(vec![0xe8, 0x01, enabled as u8])
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
        let r = self.request(&[0x12, 0x02], 0x13, -1)?;
        Ok(r.payload.get(2).map(|&c| codec_name(c).to_string()).unwrap_or_default())
    }

    fn get_auto_power_off(&self) -> Result<i32> {
        let r = self.request(&[0x26, 0x05], 0x27, -1)?;
        if r.payload.len() >= 4 {
            Ok(apo_index(r.payload[2], r.payload[3]))
        } else {
            Err(invalid("Incomplete AutoPowerOff response"))
        }
    }

    fn set_auto_power_off(&self, index: i32) -> Result<()> {
        if !(0..6).contains(&index) {
            return Ok(());
        }
        let (a, b) = APO_CODES[index as usize];
        self.send(vec![0x28, 0x05, a, b])
    }

    fn get_speak_to_chat(&self) -> Result<bool> {
        let r = self.request(&[0xf6, 0x0c], 0xf7, 0x0c)?;
        r.payload.get(2).map(|&v| v == 0).ok_or_else(|| invalid("Incomplete SpeakToChat response"))
    }

    fn set_speak_to_chat(&self, enabled: bool) -> Result<()> {
        self.send(vec![0xf8, 0x0c, if enabled { 0 } else { 1 }, 0x01])
    }

    fn get_adaptive_volume(&self) -> Result<bool> {
        let r = self.request(&[0xf6, 0x0a], 0xf7, 0x0a)?;
        r.payload
            .get(2)
            .map(|&v| v == 0)
            .ok_or_else(|| invalid("Incomplete AdaptiveVolume response"))
    }

    fn set_adaptive_volume(&self, enabled: bool) -> Result<()> {
        self.send(vec![0xf8, 0x0a, if enabled { 0 } else { 1 }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeDevice;

    fn setup(
        f: impl Fn(DataType, &[u8]) -> Vec<(DataType, Vec<u8>)> + Send + Sync + 'static,
    ) -> (Arc<FakeDevice>, Arc<Session>) {
        let dev = Arc::new(FakeDevice::new(Box::new(f)));
        let session = Arc::new(Session::new(dev.clone()));
        session.connect("00:00:00:00:00:00").unwrap();
        (dev, session)
    }

    #[test]
    fn battery_and_noise_control_roundtrip() {
        let (dev, s) = setup(|_, p| match p {
            [0x22, 0x00] => vec![(DataType::DataMdr, vec![0x23, 0x00, 80, 1])],
            [0x66, 0x17] => vec![(DataType::DataMdr, vec![0x67, 0x17, 1, 1, 1, 0, 12])],
            _ => vec![],
        });
        let proto = ProtocolV2::new(s, false, false);
        let b = proto.get_battery().unwrap();
        assert_eq!((b.main, b.charging), (Some(80), true));
        let nc = proto.get_noise_control().unwrap();
        assert_eq!(nc.mode, NoiseControlMode::Ambient);
        assert_eq!(nc.ambient_level, 12);

        proto
            .set_noise_control(&NoiseControlState {
                mode: NoiseControlMode::NoiseCancelling,
                ambient_level: 0,
                focus_on_voice: true,
            })
            .unwrap();
        let last = dev.sent_frames().pop().unwrap();
        assert_eq!(last.payload, vec![0x68, 0x17, 1, 1, 0, 1, 1]);
    }

    #[test]
    fn ult_equalizer_reads_ult_mode_and_preserves_it_on_preset_change() {
        let (dev, s) = setup(|_, p| match p {
            // preset=0x10, ult=2, 6 values: bass+10, five bands+10
            [0x56, 0x03] => vec![(
                DataType::DataMdr,
                vec![0x57, 0x03, 0x10, 2, 6, 17, 10, 10, 10, 10, 10],
            )],
            _ => vec![],
        });
        let proto = ProtocolV2::new(s, false, true);
        let eq = proto.get_equalizer().unwrap();
        assert_eq!((eq.preset, eq.ult_mode, eq.clear_bass), (0x10, 2, 7));
        proto.set_equalizer_preset(0x11).unwrap();
        assert_eq!(dev.sent_frames().pop().unwrap().payload, vec![0x58, 0x03, 0x11, 2, 0]);
    }

    #[test]
    fn pause_on_removal_uses_inverted_flag() {
        let (dev, s) = setup(|_, p| match p {
            [0xf6, 0x01] => vec![(DataType::DataMdr, vec![0xf7, 0x01, 0x00])],
            _ => vec![],
        });
        let proto = ProtocolV2::new(s, false, true);
        assert!(proto.get_pause_when_removed().unwrap());
        proto.set_pause_when_removed(false).unwrap();
        assert_eq!(dev.sent_frames().pop().unwrap().payload, vec![0xf8, 0x01, 0x01]);
    }

    #[test]
    fn timeout_when_headset_is_silent() {
        let (_d, s) = setup(|_, _| vec![]);
        let r = s.send_and_await(
            &SonyFrame { data_type: DataType::DataMdr, sequence: 0, payload: vec![0x04, 0x02] },
            0x05,
            -1,
            Duration::from_millis(100),
        );
        assert_eq!(r.unwrap_err().code, SonyErrorCode::Timeout);
    }

    #[test]
    fn notifications_reach_callbacks() {
        let (dev, s) = setup(|_, _| vec![]);
        let (tx, rx) = std::sync::mpsc::channel();
        s.on_notification(Arc::new(move |f| {
            let _ = tx.send(f.payload.clone());
        }));
        dev.push_notification(DataType::DataMdr, vec![0x67, 0x17, 1]);
        let got = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(got, vec![0x67, 0x17, 1]);
    }
}
