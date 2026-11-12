//! Решение по маршруту сохранённого личного диалога.

use cheenhub_contracts::rest::DmConversationSummary;

/// Решение по сохранённому маршруту ЛС после анализа обзора.
pub(super) enum DirectMessageRouteAction {
    /// Дождаться полной загрузки обзора.
    Wait,
    /// Выбрать диалог по индексу в обзоре.
    Select(usize),
    /// Повторить загрузку обзора один раз.
    Refresh,
    /// Диалог не найден даже после повторной загрузки.
    Unavailable,
}

/// Решает судьбу маршрута только по успешно полученному списку диалогов.
///
/// Отсутствующий обзор означает ожидание соединения или ошибку загрузки,
/// поэтому сам по себе не доказывает отсутствие сохранённого диалога.
pub(super) fn direct_message_route_action(
    conversation_id: &str,
    overview: Option<&[DmConversationSummary]>,
    loading: bool,
    retried: bool,
) -> DirectMessageRouteAction {
    let Some(conversations) = overview else {
        return DirectMessageRouteAction::Wait;
    };
    if let Some(index) = conversations
        .iter()
        .position(|item| item.id == conversation_id)
    {
        return DirectMessageRouteAction::Select(index);
    }
    if loading {
        DirectMessageRouteAction::Wait
    } else if retried {
        DirectMessageRouteAction::Unavailable
    } else {
        DirectMessageRouteAction::Refresh
    }
}
