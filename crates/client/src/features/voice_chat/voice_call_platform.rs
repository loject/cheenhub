//! Выбор платформенного владельца системного voice-call lifecycle.

/// Системный маршрут вывода звука голосового звонка.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VoiceOutputRoute {
    /// Тихий режим через разговорный динамик.
    Earpiece,
    /// Громкая связь через основной динамик.
    Speaker,
}

/// Тип активной цели в системном Android-уведомлении.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VoiceNotificationTargetKind {
    /// Серверная голосовая комната.
    ServerRoom,
    /// Личный голосовой звонок.
    DirectCall,
}

/// Состояние микрофона в системном Android-уведомлении.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VoiceNotificationMicrophoneState {
    /// Микрофон выключен.
    Off,
    /// Микрофон включается.
    Starting,
    /// Микрофон передаёт голос.
    Live,
    /// Микрофон недоступен из-за разрешения или ошибки устройства.
    Unavailable,
}

/// Актуальные данные активного голосового подключения для системного уведомления.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ActiveVoiceNotification {
    /// Тип активной цели.
    pub(crate) target_kind: VoiceNotificationTargetKind,
    /// Идентификатор цели для защиты от устаревших системных действий.
    pub(crate) target_id: String,
    /// Отображаемое имя голосовой комнаты или собеседника.
    pub(crate) target_name: String,
    /// Текущее состояние микрофона.
    pub(crate) microphone: VoiceNotificationMicrophoneState,
}

/// Команда из системного Android-уведомления активного звонка.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VoiceNotificationAction {
    /// Переключить микрофон.
    ToggleMicrophone,
    /// Выйти из голосовой комнаты или завершить личный звонок.
    Leave,
}

impl TryFrom<i32> for VoiceNotificationAction {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ToggleMicrophone),
            2 => Ok(Self::Leave),
            _ => Err(()),
        }
    }
}

#[cfg(target_os = "android")]
#[path = "voice_call/android.rs"]
mod implementation;

#[cfg(not(target_os = "android"))]
#[path = "voice_call/unsupported.rs"]
mod implementation;

pub(crate) use implementation::{
    VoiceAudioFocusEvent, load_voice_output_route, set_voice_call_participating,
    set_voice_output_route, subscribe_voice_audio_focus, subscribe_voice_notification_actions,
    supports_voice_output_route, update_active_voice_notification,
};
