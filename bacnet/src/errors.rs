use thiserror::Error;

#[derive(Debug, Error)]
pub enum BACnetErr {
    #[error("Rejected: code {code}")]
    Rejected { code: u8 }, // Rejected with the given reason code

    #[error("Aborted: {text} (code {code})")]
    Aborted { text: String, code: u8 },

    #[error("Error: class={class_text} ({class}) {text} ({code})")]
    Error {
        class_text: String,
        class: u32,
        text: String,
        code: u32,
    },

    #[error("Request is still ongoing")]
    RequestOngoing,

    #[error("No value was extracted")]
    NoValue,

    #[error("Invalid value was extracted")]
    InvalidValue,

    #[error("Not connected to server with Device ID {device_id}")]
    NotConnected { device_id: u32 },

    #[error("TSM Timeout")]
    TsmTimeout,

    #[error("APDU Timeout")]
    ApduTimeout,

    #[error("Decoding failed")]
    DecodeFailed,

    #[error("Encode failed")]
    EncodeFailed,

    #[error("Unhandled type tag {tag_name} ({tag:?})")]
    UnhandledTag { tag_name: String, tag: u8 },

    #[error("Couldn't get lock")]
    CouldntGetLock,
}
