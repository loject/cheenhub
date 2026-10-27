//! Пакетное разрешение текущих ролей авторов текстовых сообщений.

use std::collections::{HashMap, HashSet};

use cheenhub_contracts::realtime::{AuthorRolesResolved, ResolveAuthorRoles, ServerRoleKind};
use uuid::Uuid;

use crate::features::servers::domain::Server;
use crate::state::AppState;

use super::TextChatApplicationError;

/// Адресный realtime-запрос с проверкой права чтения комнаты и жёстким лимитом пакета.
pub(crate) async fn resolve_author_roles(
    state: &AppState,
    user_id: &Uuid,
    request: ResolveAuthorRoles,
) -> Result<AuthorRolesResolved, TextChatApplicationError> {
    if request.author_user_ids.len() > 100 {
        return Err(TextChatApplicationError::BadRequest(
            "Можно запросить роли не более 100 авторов за раз.".to_owned(),
        ));
    }
    let server_id = super::parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = super::parse_id(&request.room_id, "Комната не найдена.")?;
    super::ensure_room_text_available(state, user_id, &server_id, &room_id).await?;
    let author_ids = request
        .author_user_ids
        .iter()
        .map(|id| super::parse_id(id, "Некорректный идентификатор автора."))
        .collect::<Result<Vec<_>, _>>()?;
    let server = state
        .server_store
        .find_server(&server_id)
        .await
        .map_err(TextChatApplicationError::Internal)?
        .ok_or_else(|| TextChatApplicationError::NotFound("Сервер не найден.".to_owned()))?;
    Ok(AuthorRolesResolved {
        server_id: server_id.to_string(),
        author_role_ids: author_role_ids(state, &server, &author_ids).await?,
    })
}

/// Возвращает роли только активных авторов из запрошенного набора.
/// Владелец имеет неявные роли даже при отсутствии строки участника.
pub(super) async fn author_role_ids(
    state: &AppState,
    server: &Server,
    author_ids: &[Uuid],
) -> Result<HashMap<String, Vec<String>>, TextChatApplicationError> {
    let requested = author_ids.iter().copied().collect::<HashSet<_>>();
    if requested.is_empty() {
        return Ok(HashMap::new());
    }
    let mut assignments = state
        .server_store
        .active_member_role_ids_for_users(&server.id, author_ids)
        .await
        .map_err(TextChatApplicationError::Internal)?;
    if requested.contains(&server.owner_user_id) {
        assignments.entry(server.owner_user_id).or_default();
    }
    if assignments.is_empty() {
        return Ok(HashMap::new());
    }
    let mut palette = state
        .server_store
        .list_server_roles(&server.id)
        .await
        .map_err(TextChatApplicationError::Internal)?;
    palette.sort_by_key(|role| {
        let priority = match role.kind {
            ServerRoleKind::Owner => 0,
            ServerRoleKind::Custom => 1,
            ServerRoleKind::Member => 2,
        };
        (priority, role.position)
    });
    Ok(assignments
        .into_iter()
        .map(|(user_id, explicit_ids)| {
            let role_ids = palette
                .iter()
                .filter(|role| match role.kind {
                    ServerRoleKind::Owner => user_id == server.owner_user_id,
                    ServerRoleKind::Member => true,
                    ServerRoleKind::Custom => explicit_ids.contains(&role.id),
                })
                .map(|role| role.id.to_string())
                .collect();
            (user_id.to_string(), role_ids)
        })
        .collect())
}
