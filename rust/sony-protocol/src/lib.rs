//! Sony headphone wire protocol (Rust port of `libs/sony-protocol`).

pub mod codec;
pub mod error;
pub mod fake;
pub mod frame;
pub mod protocol;
pub mod presets;
pub mod profile;
pub mod protocol_v1;
pub mod protocol_v2;
pub mod semantic;
pub mod session;
pub mod state;
pub mod transport;

pub use error::{Result, SonyError, SonyErrorCode};
pub use frame::{DataType, FrameCodec, SonyFrame};
pub use protocol::{Protocol, ProtocolGeneration};
pub use protocol_v1::ProtocolV1;
pub use protocol_v2::ProtocolV2;
pub use semantic::*;
pub use session::Session;
pub use state::{apply_notification, Changed, DeviceState};
pub use transport::Transport;
