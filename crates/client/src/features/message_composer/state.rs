//! Состояние формы сообщения, ограниченное одним диалогом.

use dioxus::prelude::*;

use super::pending_attachment::PendingImageAttachment;

/// Общее состояние представлений формы одной комнаты или личного диалога.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct MessageComposeState {
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

/// Создаёт состояние формы, живущее в keyed-экземпляре комнаты или личного диалога.
pub(crate) fn use_message_compose_state() -> MessageComposeState {
    MessageComposeState {
        draft: use_signal(String::new),
        status: use_signal(String::new),
        is_sending: use_signal(|| false),
        is_selecting_image: use_signal(|| false),
        is_reading_clipboard: use_signal(|| false),
        pending_attachment: use_signal(|| None),
    }
}
