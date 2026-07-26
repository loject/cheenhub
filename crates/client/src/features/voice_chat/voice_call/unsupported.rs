//! Заглушка системного voice-call lifecycle для платформ без отдельного владельца звонка.

use std::convert::Infallible;

use futures_channel::mpsc;

use super::VoiceOutputRoute;

/// Изменение доступности системного audio focus.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VoiceAudioFocusEvent {
    /// Audio focus снова доступен.
    Gained,
    /// Audio focus временно потерян.
    LostTransient,
    /// Audio focus потерян без гарантии автоматического возврата.
    Lost,
}

/// На этой платформе отдельный системный lifecycle звонка не требуется.
pub(crate) fn set_voice_call_participating(_participating: bool) -> Result<(), Infallible> {
    Ok(())
}

/// На этой платформе системный переключатель маршрута звонка отсутствует.
pub(crate) fn supports_voice_output_route() -> bool {
    false
}

/// На этой платформе системный маршрут звонка недоступен.
pub(crate) async fn load_voice_output_route() -> Result<Option<VoiceOutputRoute>, String> {
    Ok(None)
}

/// На этой платформе системный маршрут звонка не изменяется.
pub(crate) async fn set_voice_output_route(_route: VoiceOutputRoute) -> Result<(), String> {
    Ok(())
}

/// Возвращает пустую подписку, потому что платформа не сообщает audio focus.
pub(crate) fn subscribe_voice_audio_focus() -> mpsc::UnboundedReceiver<VoiceAudioFocusEvent> {
    let (_sender, receiver) = mpsc::unbounded();
    receiver
}
