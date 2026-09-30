//! Контракт транспортных операций общей формы сообщения.

use dioxus::prelude::*;
use futures_util::future::LocalBoxFuture;

use super::pending_attachment::PendingImageAttachment;

/// Результат асинхронной транспортной операции.
pub(crate) type MessageOperation<T> = LocalBoxFuture<'static, Result<T, String>>;

/// Две операции адаптера без владения состоянием формы.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct MessageOperations {
    /// Загружает изображение и возвращает его идентификатор.
    pub(crate) upload: Callback<PendingImageAttachment, MessageOperation<String>>,
    /// Создаёт сообщение и уведомляет владельца списка сообщений.
    pub(crate) send: Callback<(String, Option<String>), MessageOperation<()>>,
}
