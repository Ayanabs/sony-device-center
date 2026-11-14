use crate::error::{SonyError, SonyErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DataType {
    Data = 0,
    Ack = 1,
    DataMcNo1 = 2,
    DataIcd = 9,
    DataEv = 10,
    #[default]
    DataMdr = 12,
    DataCommon = 13,
    DataMdrNo2 = 14,
    Shot = 16,
    ShotMcNo1 = 18,
    ShotIcd = 25,
    ShotEv = 26,
    ShotMdr = 28,
    ShotCommon = 29,
    ShotMdrNo2 = 30,
    LargeDataCommon = 45,
    Unknown = 255,
}

impl From<u8> for DataType {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Data,
            1 => Self::Ack,
            2 => Self::DataMcNo1,
            9 => Self::DataIcd,
            10 => Self::DataEv,
            12 => Self::DataMdr,
            13 => Self::DataCommon,
            14 => Self::DataMdrNo2,
            16 => Self::Shot,
            18 => Self::ShotMcNo1,
            25 => Self::ShotIcd,
            26 => Self::ShotEv,
            28 => Self::ShotMdr,
            29 => Self::ShotCommon,
            30 => Self::ShotMdrNo2,
            45 => Self::LargeDataCommon,
            _ => Self::Unknown,
        }
    }
}

impl DataType {
    pub fn name(self) -> &'static str {
        match self {
            Self::Data => "Data",
            Self::Ack => "Ack",
            Self::DataMcNo1 => "DataMcNo1",
            Self::DataIcd => "DataIcd",
            Self::DataEv => "DataEv",
            Self::DataMdr => "DataMdr",
            Self::DataCommon => "DataCommon",
            Self::DataMdrNo2 => "DataMdrNo2",
            Self::Shot => "Shot",
            Self::ShotMcNo1 => "ShotMcNo1",
            Self::ShotIcd => "ShotIcd",
            Self::ShotEv => "ShotEv",
            Self::ShotMdr => "ShotMdr",
            Self::ShotCommon => "ShotCommon",
            Self::ShotMdrNo2 => "ShotMdrNo2",
            Self::LargeDataCommon => "LargeDataCommon",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SonyFrame {
    pub data_type: DataType,
    pub sequence: u8,
    pub payload: Vec<u8>,
}

pub struct FrameCodec;

impl FrameCodec {
    pub const START_MARKER: u8 = 0x3e;
    pub const END_MARKER: u8 = 0x3c;
    pub const ESCAPE_MARKER: u8 = 0x3d;
    pub const ESCAPED_60: u8 = 0x2c;
    pub const ESCAPED_61: u8 = 0x2d;
    pub const ESCAPED_62: u8 = 0x2e;
    pub const MAX_FRAME_SIZE: usize = 2048;
    /// type(1) + seq(1) + len(4) + checksum(1)
    pub const MIN_BODY_SIZE: usize = 7;

    pub fn escape(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len());
        for &b in data {
            match b {
                Self::END_MARKER => out.extend([Self::ESCAPE_MARKER, Self::ESCAPED_60]),
                Self::ESCAPE_MARKER => out.extend([Self::ESCAPE_MARKER, Self::ESCAPED_61]),
                Self::START_MARKER => out.extend([Self::ESCAPE_MARKER, Self::ESCAPED_62]),
                _ => out.push(b),
            }
        }
        out
    }

    pub fn unescape(data: &[u8]) -> Result<Vec<u8>, SonyError> {
        let mut out = Vec::with_capacity(data.len());
        let mut it = data.iter();
        while let Some(&b) = it.next() {
            if b != Self::ESCAPE_MARKER {
                out.push(b);
                continue;
            }
            let next = it.next().ok_or_else(|| {
                SonyError::new(SonyErrorCode::InvalidFrame, "No data left for escaped byte data")
            })?;
            out.push(match *next {
                Self::ESCAPED_60 => Self::END_MARKER,
                Self::ESCAPED_61 => Self::ESCAPE_MARKER,
                Self::ESCAPED_62 => Self::START_MARKER,
                _ => {
                    return Err(SonyError::new(SonyErrorCode::InvalidFrame, "Unexpected escaped byte"))
                }
            });
        }
        Ok(out)
    }

    /// Unsigned modulo-256 sum over unescaped bytes.
    pub fn calculate_checksum(data: &[u8]) -> u8 {
        data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
    }

    pub fn encode(frame: &SonyFrame) -> Result<Vec<u8>, SonyError> {
        let mut raw = Vec::with_capacity(frame.payload.len() + 7);
        raw.push(frame.data_type as u8);
        raw.push(frame.sequence);
        raw.extend((frame.payload.len() as u32).to_be_bytes());
        raw.extend(&frame.payload);
        raw.push(Self::calculate_checksum(&raw));

        let escaped = Self::escape(&raw);
        if escaped.len() + 2 > Self::MAX_FRAME_SIZE {
            return Err(SonyError::new(
                SonyErrorCode::InvalidFrame,
                "Exceeded the max bluetooth message size, and I can't handle chunked messages",
            ));
        }
        let mut out = Vec::with_capacity(escaped.len() + 2);
        out.push(Self::START_MARKER);
        out.extend(escaped);
        out.push(Self::END_MARKER);
        Ok(out)
    }

    /// Decode a complete wire frame (`<START> ... <END>`).
    pub fn decode(frame_data: &[u8]) -> Result<SonyFrame, SonyError> {
        if frame_data.len() < 2 {
            return Err(SonyError::new(SonyErrorCode::InvalidFrame, "Frame too small for delimiters"));
        }
        if frame_data[0] != Self::START_MARKER || frame_data[frame_data.len() - 1] != Self::END_MARKER {
            return Err(SonyError::new(SonyErrorCode::InvalidFrame, "Invalid frame delimiters"));
        }
        Self::decode_body(&frame_data[1..frame_data.len() - 1])
    }

    /// Decode un-delimited body bytes.
    pub fn decode_body(body: &[u8]) -> Result<SonyFrame, SonyError> {
        let raw = Self::unescape(body)?;
        if raw.len() < Self::MIN_BODY_SIZE {
            return Err(SonyError::new(
                SonyErrorCode::InvalidFrame,
                "Invalid message: Smaller than the minimum message size",
            ));
        }
        let size = u32::from_be_bytes([raw[2], raw[3], raw[4], raw[5]]) as usize;
        if raw.len() < 6 + size + 1 {
            return Err(SonyError::new(
                SonyErrorCode::InvalidFrame,
                "Invalid message: declared size exceeds received data",
            ));
        }
        if raw[6 + size] != Self::calculate_checksum(&raw[..6 + size]) {
            return Err(SonyError::new(SonyErrorCode::InvalidChecksum, "Invalid checksum!"));
        }
        Ok(SonyFrame {
            data_type: DataType::from(raw[0]),
            sequence: raw[1],
            payload: raw[6..6 + size].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_with_escaped_bytes() {
        let f = SonyFrame { data_type: DataType::DataMdr, sequence: 3, payload: vec![0x3c, 0x3d, 0x3e, 1, 2] };
        let wire = FrameCodec::encode(&f).unwrap();
        assert_eq!(wire[0], FrameCodec::START_MARKER);
        assert_eq!(*wire.last().unwrap(), FrameCodec::END_MARKER);
        assert_eq!(FrameCodec::decode(&wire).unwrap(), f);
    }

    #[test]
    fn rejects_bad_checksum() {
        let f = SonyFrame { data_type: DataType::Ack, sequence: 0, payload: vec![] };
        let mut wire = FrameCodec::encode(&f).unwrap();
        let n = wire.len();
        wire[n - 2] ^= 0x01;
        assert_eq!(FrameCodec::decode(&wire).unwrap_err().code, SonyErrorCode::InvalidChecksum);
    }

    #[test]
    fn rejects_bad_delimiters_and_oversize() {
        assert_eq!(FrameCodec::decode(&[1, 2, 3]).unwrap_err().code, SonyErrorCode::InvalidFrame);
        let big = SonyFrame { payload: vec![0; 3000], ..Default::default() };
        assert!(FrameCodec::encode(&big).is_err());
    }

    #[test]
    fn dangling_escape_is_error() {
        assert!(FrameCodec::unescape(&[0x3d]).is_err());
        assert!(FrameCodec::unescape(&[0x3d, 0x00]).is_err());
    }
}
