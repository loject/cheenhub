//! Общее состояние голосового значка для браузера и системного трея.

/// Взаимоисключающие состояния значка голосового подключения.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum VoiceIconStatus {
    /// Голосовое соединение отсутствует.
    #[default]
    Inactive,
    /// Подключён, микрофон не передаёт речь.
    Connected,
    /// Подключён, микрофон передаёт речь.
    Speaking,
    /// Микрофон выключен.
    MicrophoneMuted,
    /// Входящий звук выключен.
    OutputMuted,
    /// Выключены микрофон и входящий звук.
    BothMuted,
}

impl VoiceIconStatus {
    /// Вычисляет состояние с приоритетом отключённых аудиоустройств над VAD.
    pub(super) fn from_voice(
        connected: bool,
        microphone_muted: bool,
        output_muted: bool,
        speaking: bool,
    ) -> Self {
        if !connected {
            return Self::Inactive;
        }
        match (microphone_muted, output_muted) {
            (true, true) => Self::BothMuted,
            (true, false) => Self::MicrophoneMuted,
            (false, true) => Self::OutputMuted,
            (false, false) if speaking => Self::Speaking,
            (false, false) => Self::Connected,
        }
    }
}

#[cfg(test)]
mod tests;
