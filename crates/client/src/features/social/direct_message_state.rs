//! Состояние одного keyed-экземпляра личного диалога.

use std::rc::Rc;

use cheenhub_contracts::rest::DmMessageSummary;
use dioxus::prelude::*;

use crate::features::text_chat::ScrollCommand;

/// Сигналы истории и прокрутки, принадлежащие одному личному диалогу.
#[derive(Clone, Copy)]
pub(super) struct DirectMessageState {
    /// Загруженные сообщения выбранного диалога.
    pub(super) messages: Signal<Vec<DmMessageSummary>>,
    /// Сообщения с анимацией появления.
    pub(super) appearing_message_ids: Signal<Vec<String>>,
    /// Сообщения с анимацией удаления.
    pub(super) removing_message_ids: Signal<Vec<String>>,
    /// Выполняется первоначальная загрузка истории.
    pub(super) is_loading: Signal<bool>,
    /// На сервере есть более старые сообщения.
    pub(super) has_more: Signal<bool>,
    /// Выполняется загрузка предыдущей страницы.
    pub(super) is_loading_older: Signal<bool>,
    /// Пользовательское сообщение об ошибке истории.
    pub(super) status: Signal<String>,
    /// Список находится около нижней границы.
    pub(super) is_near_bottom: Signal<bool>,
    /// Смонтированный контейнер истории.
    pub(super) list_element: Signal<Option<Rc<MountedData>>>,
    /// Отложенная команда прокрутки.
    pub(super) pending_scroll: Signal<Option<ScrollCommand>>,
}
