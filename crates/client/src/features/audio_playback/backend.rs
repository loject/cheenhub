//! Общие контракты audio playback.

/// Кодек закодированного воспроизведения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaybackCodec {
    /// Opus audio.
    Opus,
}

/// Закодированный голосовой фрейм, подготовленный к воспроизведению.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoiceFrame {
    /// Authenticated sender identifier.
    pub(crate) sender_user_id: String,
    /// Sender-local packet sequence.
    #[allow(dead_code)]
    pub(crate) sequence: u64,
    /// Capture or encode timestamp in microseconds.
    pub(crate) timestamp_us: u64,
    /// Frame duration in microseconds.
    pub(crate) duration_us: u32,
    /// Encoded codec.
    pub(crate) codec: PlaybackCodec,
    /// Encoded frame bytes.
    pub(crate) bytes: Vec<u8>,
}
