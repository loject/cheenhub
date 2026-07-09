//! Контекст активного workspace приложения.
//!
//! Отслеживает, какую комнату или личный диалог просматривает пользователь
//! в данный момент, чтобы другие модули могли подавлять уведомления
//! для активного workspace.

use dioxus::prelude::*;

/// Контекст, предоставляющий идентификатор активного workspace.
///
/// Поддерживает серверные комнаты и личные диалоги (DM).
#[derive(Clone, Copy)]
pub(crate) struct ActiveRoomContext {
    room_id: Signal<Option<String>>,
    conversation_id: Signal<Option<String>>,
}

impl ActiveRoomContext {
    /// Создаёт контекст активного workspace из сигналов уровня приложения.
    pub(crate) fn new(
        room_id: Signal<Option<String>>,
        conversation_id: Signal<Option<String>>,
    ) -> Self {
        Self {
            room_id,
            conversation_id,
        }
    }

    /// Возвращает идентификатор активной серверной комнаты, если он установлен.
    pub(crate) fn get(&self) -> Option<String> {
        (self.room_id)()
    }

    /// Устанавливает идентификатор активной серверной комнаты.
    pub(crate) fn set(&self, room_id: Option<String>) {
        let mut current = self.room_id;
        current.set(room_id);
    }

    /// Возвращает идентификатор активного личного диалога, если он установлен.
    pub(crate) fn conversation_id(&self) -> Option<String> {
        (self.conversation_id)()
    }

    /// Устанавливает идентификатор активного личного диалога.
    pub(crate) fn set_conversation_id(&self, conversation_id: Option<String>) {
        let mut current = self.conversation_id;
        current.set(conversation_id);
    }
}
