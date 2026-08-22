//! Состояние формы сообщения, ограниченное одной комнатой.

use dioxus::prelude::*;

use super::pending_attachment::PendingImageAttachment;

/// Общее состояние всех представлений формы одной комнаты.
#[derive(Clone, Copy)]
pub(crate) struct RoomComposeState {
    /// Текст черновика.
    pub(crate) draft: Signal<String>,
    /// Сообщение об ошибке операции формы.
    pub(crate) status: Signal<String>,
    /// Признак загрузки вложения или отправки сообщения.
    pub(crate) is_sending: Signal<bool>,
    /// Признак открытия системного выбора изображения.
    pub(crate) is_selecting_image: Signal<bool>,
    /// Признак асинхронного чтения изображения из буфера обмена.
    pub(crate) is_reading_clipboard: Signal<bool>,
    /// Изображение, ожидающее отправки.
    pub(crate) pending_attachment: Signal<Option<PendingImageAttachment>>,
}

/// Создаёт состояние формы, живущее в keyed-экземпляре комнаты.
pub(crate) fn use_room_compose_state() -> RoomComposeState {
    RoomComposeState {
        draft: use_signal(String::new),
        status: use_signal(String::new),
        is_sending: use_signal(|| false),
        is_selecting_image: use_signal(|| false),
        is_reading_clipboard: use_signal(|| false),
        pending_attachment: use_signal(|| None),
    }
}
