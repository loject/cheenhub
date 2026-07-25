//! Заглушка системного voice-call lifecycle для платформ без отдельного владельца звонка.

use std::convert::Infallible;

use futures_channel::mpsc;

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

/// Возвращает пустую подписку, потому что платформа не сообщает audio focus.
pub(crate) fn subscribe_voice_audio_focus() -> mpsc::UnboundedReceiver<VoiceAudioFocusEvent> {
    let (_sender, receiver) = mpsc::unbounded();
    receiver
}
