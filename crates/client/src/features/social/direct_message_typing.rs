//! Индикатор набора сообщения выбранного личного диалога.

use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;
use crate::features::typing::{
    TyperEntry, TypingIndicator, TypingNotifier, TypingTarget, TypingTransport,
    use_typing_notifier, use_typing_participants,
};

/// Состояние набора одного личного диалога.
///
/// Собирается в один хук, чтобы рабочая область диалога не знала про модуль
/// realtime social и не дублировала настройку цели для комнат.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct DirectMessageTyping {
    /// Отправитель набора для формы сообщения.
    pub(super) notifier: TypingNotifier,
    /// Участники, которые печатают в диалоге.
    participants: Signal<Vec<TyperEntry>>,
}

impl DirectMessageTyping {
    /// Рендерит индикатор набора над формой сообщения.
    ///
    /// Компонент-обертка хранится в фиче `typing` и не знает про диалог, поэтому
    /// подписка на события остается единственной и создается в хуке ниже.
    pub(super) fn indicator(&self) -> Element {
        rsx! {
            TypingIndicator { typers: (self.participants)() }
        }
    }
}

/// Создает набор для личного диалога с указанным идентификатором.
///
/// Хук живет в keyed-экземпляре диалога, поэтому при переходе к другому
/// диалогу предыдущий набор снимается размонтированием компонента.
pub(super) fn use_direct_message_typing(
    realtime: &RealtimeHandle,
    conversation_id: String,
    current_user_id: String,
) -> DirectMessageTyping {
    let target = TypingTarget::DirectMessage { conversation_id };
    let notifier = use_typing_notifier(TypingTransport::new(realtime.clone(), target.clone()));
    let participants = use_typing_participants(realtime, target, current_user_id);

    DirectMessageTyping {
        notifier,
        participants,
    }
}
