//! Удаление сервера и всех связанных записей в in-memory-хранилище.

use uuid::Uuid;

use super::in_memory::InMemoryState;

/// Удаляет сервер вместе с комнатами, участниками, приглашениями и ролями.
///
/// Возвращает `false`, если сервера с таким идентификатором и владельцем нет,
/// чтобы вызывающий код отличал отсутствие сервера от успешного удаления.
pub(super) fn delete_owned_server(
    state: &mut InMemoryState,
    server_id: &Uuid,
    owner_user_id: &Uuid,
) -> bool {
    let Some(index) = state
        .servers
        .iter()
        .position(|server| server.id == *server_id && server.owner_user_id == *owner_user_id)
    else {
        return false;
    };

    state.servers.remove(index);
    // Приглашения удаляются вместе с их использованиями: без приглашения строка
    // использования не может указать, по какой ссылке пришёл участник.
    let invite_ids = state
        .invites
        .iter()
        .filter(|invite| invite.server_id == *server_id)
        .map(|invite| invite.id)
        .collect::<Vec<_>>();
    state
        .invites
        .retain(|invite| invite.server_id != *server_id);
    state
        .invite_uses
        .retain(|invite_use| !invite_ids.contains(&invite_use.invite_id));
    state
        .members
        .retain(|member| member.server_id != *server_id);
    state
        .exclusions
        .retain(|exclusion| exclusion.server_id != *server_id);
    state.rooms.retain(|room| room.server_id != *server_id);
    state.roles.retain(|role| role.server_id != *server_id);
    state
        .member_roles
        .retain(|(member_server_id, _, _, _)| *member_server_id != *server_id);

    true
}
