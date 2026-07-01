//! Типы цели голосового подключения.

/// Цель комнаты с поддержкой голоса.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoiceRoomTarget {
    /// Тип цели.
    pub(crate) kind: VoiceRoomTargetKind,
    /// Идентификатор сервера.
    pub(crate) server_id: String,
    /// Идентификатор комнаты.
    pub(crate) room_id: String,
    /// Человекочитаемое имя комнаты.
    pub(crate) room_name: String,
}

/// Тип цели голосового подключения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VoiceRoomTargetKind {
    /// Серверная голосовая комната.
    Server,
    /// Голосовой звонок личного диалога.
    DirectMessage,
}

impl VoiceRoomTarget {
    /// Возвращает цель серверной голосовой комнаты.
    pub(crate) fn server(server_id: String, room_id: String, room_name: String) -> Self {
        Self {
            kind: VoiceRoomTargetKind::Server,
            server_id,
            room_id,
            room_name,
        }
    }

    /// Возвращает цель голосового звонка личного диалога.
    pub(crate) fn direct_message(conversation_id: String, room_name: String) -> Self {
        Self {
            kind: VoiceRoomTargetKind::DirectMessage,
            server_id: conversation_id.clone(),
            room_id: conversation_id,
            room_name,
        }
    }

    /// Возвращает, указывают ли две цели на одну голосовую комнату.
    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.server_id == other.server_id
            && self.room_id == other.room_id
    }
}
