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

/// Короткий системный звук уведомления.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NotificationSound {
    /// Получено новое личное сообщение.
    MessageReceived,
    /// Текущий пользователь вошёл в голосовую комнату.
    CurrentUserJoined,
    /// Текущий пользователь вышел из голосовой комнаты.
    CurrentUserLeft,
    /// Другой пользователь вошёл в голосовую комнату.
    OtherUserJoined,
    /// Другой пользователь вышел из голосовой комнаты.
    OtherUserLeft,
    /// Камера включена.
    CameraEnabled,
    /// Камера выключена.
    CameraDisabled,
    /// Демонстрация экрана включена.
    ScreenShareEnabled,
    /// Демонстрация экрана выключена.
    ScreenShareDisabled,
    /// Realtime-соединение потеряно.
    ConnectionLost,
    /// Realtime-соединение восстановлено.
    ConnectionRestored,
    /// Повторяющийся сигнал потери соединения во время голосового чата.
    ConnectionSignalLoop,
}

impl NotificationSound {
    /// Возвращает путь к ассету внутри публичной директории клиента.
    pub(crate) fn asset_path(self) -> &'static str {
        match self {
            Self::MessageReceived => "/audio/notifications/message_received.wav",
            Self::CurrentUserJoined => "/audio/notifications/current-user-joined.wav",
            Self::CurrentUserLeft => "/audio/notifications/current-user-left.wav",
            Self::OtherUserJoined => "/audio/notifications/other-user-joined.wav",
            Self::OtherUserLeft => "/audio/notifications/other-user-left.wav",
            Self::CameraEnabled => "/audio/notifications/camera-enabled.wav",
            Self::CameraDisabled => "/audio/notifications/camera-disabled.wav",
            Self::ScreenShareEnabled => "/audio/notifications/screen-share-enabled.wav",
            Self::ScreenShareDisabled => "/audio/notifications/screen-share-disabled.wav",
            Self::ConnectionLost => "/audio/notifications/cheenhub_signal_lost.wav",
            Self::ConnectionRestored => "/audio/notifications/cheenhub_signal_restored.wav",
            Self::ConnectionSignalLoop => "/audio/notifications/cheenhub_signal_loop.wav",
        }
    }

    /// Возвращает стабильное имя события для логов и native mixer.
    pub(crate) fn event_name(self) -> &'static str {
        match self {
            Self::MessageReceived => "message-received",
            Self::CurrentUserJoined => "current-user-joined",
            Self::CurrentUserLeft => "current-user-left",
            Self::OtherUserJoined => "other-user-joined",
            Self::OtherUserLeft => "other-user-left",
            Self::CameraEnabled => "camera-enabled",
            Self::CameraDisabled => "camera-disabled",
            Self::ScreenShareEnabled => "screen-share-enabled",
            Self::ScreenShareDisabled => "screen-share-disabled",
            Self::ConnectionLost => "connection-lost",
            Self::ConnectionRestored => "connection-restored",
            Self::ConnectionSignalLoop => "connection-signal-loop",
        }
    }
}
