//! Доступ к подтверждённой медиасессии голосовой комнаты.

use dioxus::prelude::*;
use uuid::Uuid;

use super::{VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget};

impl VoiceConnectionHandle {
    /// Возвращает UUID подтверждённого подключения только для исходной комнаты.
    pub(crate) fn media_session_id_for(&self, target: &VoiceRoomTarget) -> Option<Uuid> {
        (self.media_session)()
            .filter(|session| session.target.matches(target))
            .map(|session| session.session_id)
    }

    /// Разрешает отправку медиа только после подтверждения подключения к этой комнате.
    pub(crate) fn outbound_session_id_for(&self, server_id: &str, room_id: &str) -> Option<Uuid> {
        let VoiceConnectionState::Connected { target, .. } = self.state() else {
            return None;
        };
        if target.server_id != server_id || target.room_id != room_id {
            return None;
        }
        let session_id = self.media_session_id_for(&target)?;
        Some(session_id)
    }

    /// Сбрасывает разрешение отправки медиа после выхода или обрыва транспорта.
    pub(crate) fn clear_media_session(&self) {
        self.pending_eviction.borrow_mut().take();
        let mut media_session = self.media_session;
        media_session.set(None);
    }
}
