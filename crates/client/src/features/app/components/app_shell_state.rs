//! Сохранение сводок серверов и состояния их рабочих областей.

use super::app_shell::ServerShellState;
use cheenhub_contracts::rest::ServerRoomKind;

/// Возвращает начальное состояние рабочей области сервера.
pub(super) fn default_server_shell_state() -> ServerShellState {
    ServerShellState {
        chat_open: false,
        room_kind: "text_and_voice",
    }
}

/// Сопоставляет тип комнаты с атрибутом раскладки рабочей области.
pub(crate) fn room_kind_attr(kind: ServerRoomKind) -> &'static str {
    match kind {
        ServerRoomKind::Text => "text",
        ServerRoomKind::Voice => "voice",
        ServerRoomKind::TextAndVoice => "text_and_voice",
    }
}

/// Восстанавливает состояние рабочей области указанного сервера.
pub(super) fn saved_server_shell_state(
    states: &[(String, ServerShellState)],
    server_id: &str,
) -> Option<ServerShellState> {
    states
        .iter()
        .find_map(|(saved_id, state)| (saved_id == server_id).then_some(*state))
}

/// Сохраняет состояние рабочей области, не меняя состояния других серверов.
pub(super) fn upsert_server_shell_state(
    states: &mut Vec<(String, ServerShellState)>,
    server_id: String,
    state: ServerShellState,
) {
    if let Some((_, saved_state)) = states
        .iter_mut()
        .find(|(saved_id, _)| saved_id == &server_id)
    {
        *saved_state = state;
        return;
    }

    states.push((server_id, state));
}
