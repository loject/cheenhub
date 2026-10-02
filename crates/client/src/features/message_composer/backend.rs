//! Контракт транспортных операций общей формы сообщения.

use dioxus::prelude::*;
use futures_util::future::LocalBoxFuture;

use super::pending_attachment::PendingImageAttachment;
use crate::features::typing::TypingNotifier;

/// Результат асинхронной транспортной операции.
pub(crate) type MessageOperation<T> = LocalBoxFuture<'static, Result<T, String>>;

/// Две операции адаптера без владения состоянием формы.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct MessageOperations {
    /// Загружает изображение и возвращает его идентификатор.
    pub(crate) upload: Callback<PendingImageAttachment, MessageOperation<String>>,
    /// Создаёт сообщение и уведомляет владельца списка сообщений.
    pub(crate) send: Callback<(String, Option<String>), MessageOperation<()>>,
    /// Сообщает чату о наборе текста для индикатора у других участников.
    ///
    /// Форма не знает про realtime: чат передает готовый обратный вызов, который
    /// уже троттлит продления и отправляет событие в нужный модуль.
    pub(crate) typing: TypingNotifier,
}
