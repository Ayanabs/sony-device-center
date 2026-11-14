use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SonyErrorCode {
    Timeout,
    Disconnected,
    Unsupported,
    InvalidFrame,
    InvalidChecksum,
    InvalidResponse,
    TransportFailure,
    ProtocolViolation,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code:?}: {message}")]
pub struct SonyError {
    pub code: SonyErrorCode,
    pub message: String,
}

impl SonyError {
    pub fn new(code: SonyErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

pub type Result<T> = std::result::Result<T, SonyError>;
