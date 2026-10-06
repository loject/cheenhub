//! Управление ссылками-приглашениями, созданными текущим пользователем на сервере.

use std::collections::HashMap;

use cheenhub_contracts::realtime::ServerRolePermission;
use cheenhub_contracts::rest::{
    CreateServerInviteRequest, CreateServerInviteResponse, DeleteServerInviteResponse,
    ServerInviteLinksResponse, ServerOwnInviteLink,
};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::features::servers::domain::{Server, ServerInvite, invite_links_limit};
use crate::features::servers::error::ServerError;
use crate::features::servers::validation;
use crate::state::AppState;

use super::support::{
    current_user_id, parse_server_id, server_for_member_or_owner, user_has_server_permission,
};

/// Создает приглашение для сервера, где текущий пользователь имеет нужное право.
pub(crate) async fn create_invite(
    state: &AppState,
    access_token: &str,
    server_id: String,
    request: CreateServerInviteRequest,
) -> Result<CreateServerInviteResponse, ServerError> {
    let user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let valid = validation::create_server_invite(request.max_uses, request.expires_in_days)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let server = invite_links_server(state, &server_id, &user_id).await?;
    let limit = invite_links_limit(&server, &user_id);
    let links_before = own_invite_links(state, &server.id, &user_id).await?;
    let active_count = active_link_count(&links_before);

    // Владелец сервера не ограничен лимитом, поэтому проверка выполняется только когда он есть.
    if let Some(limit) = limit
        && active_count >= limit
    {
        tracing::warn!(
            server_id = %server.id,
            user_id = %user_id,
            active_links = active_count,
            limit = limit,
            "rejected server invite creation because the per-user link limit was reached"
        );
        return Err(ServerError::BadRequest(
            "Ты использовал все ссылки приглашения для этого сервера. Удали ненужную ссылку, чтобы создать новую."
                .to_owned(),
        ));
    }

    let expires_at = valid
        .expires_in_days
        .map(|days| Utc::now() + Duration::days(days.into()));
    let invite = state
        .server_store
        .insert_server_invite(&server.id, &user_id, valid.max_uses, expires_at)
        .await
        .map_err(ServerError::Internal)?;

    tracing::info!(
        server_id = %server.id,
        invite_code = %invite.id,
        user_id = %user_id,
        active_links = active_count + 1,
        limit = ?limit,
        "created server invite"
    );

    Ok(CreateServerInviteResponse {
        code: invite.id.to_string(),
        links: own_invite_links(state, &server.id, &user_id).await?,
        limit,
    })
}

/// Возвращает ссылки-приглашения, созданные текущим пользователем на сервере.
pub(crate) async fn list_own_invite_links(
    state: &AppState,
    access_token: &str,
    server_id: String,
) -> Result<ServerInviteLinksResponse, ServerError> {
    let user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let server = invite_links_server(state, &server_id, &user_id).await?;
    let limit = invite_links_limit(&server, &user_id);
    let links = own_invite_links(state, &server.id, &user_id).await?;

    tracing::debug!(
        server_id = %server.id,
        user_id = %user_id,
        invite_count = links.len(),
        active_count = active_link_count(&links),
        limit = ?limit,
        "listed own server invite links"
    );

    Ok(ServerInviteLinksResponse {
        server_id: server.id.to_string(),
        links,
        limit,
    })
}

/// Удаляет ссылку-приглашение, созданную текущим пользователем на сервере.
///
/// Ссылка перестаёт действовать, но сохраняется вместе с историей входов, чтобы по ней
/// по-прежнему можно было определить, откуда пришёл участник сервера.
pub(crate) async fn delete_own_invite(
    state: &AppState,
    access_token: &str,
    server_id: String,
    code: String,
) -> Result<DeleteServerInviteResponse, ServerError> {
    let user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let invite_id = Uuid::parse_str(&code)
        .map_err(|_| ServerError::BadRequest("Ссылка приглашения не найдена.".to_owned()))?;
    let server = invite_links_server(state, &server_id, &user_id).await?;
    let Some(invite) = state
        .server_store
        .soft_delete_server_invite_created_by(&server.id, &invite_id, &user_id, Utc::now())
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound(
            "Ссылка приглашения не найдена.".to_owned(),
        ));
    };

    tracing::info!(
        server_id = %server.id,
        invite_code = %invite.id,
        user_id = %user_id,
        deleted_at = %invite.deleted_at.unwrap_or_else(Utc::now),
        "soft deleted own server invite link"
    );

    Ok(DeleteServerInviteResponse {
        code: invite.id.to_string(),
        links: own_invite_links(state, &server.id, &user_id).await?,
        limit: invite_links_limit(&server, &user_id),
    })
}

/// Находит сервер, доступный текущему пользователю, и проверяет право создавать ссылки приглашения.
async fn invite_links_server(
    state: &AppState,
    server_id: &Uuid,
    user_id: &Uuid,
) -> Result<Server, ServerError> {
    let server = server_for_member_or_owner(state, server_id, user_id).await?;
    if !user_has_server_permission(
        state,
        &server,
        user_id,
        ServerRolePermission::CreateInviteLinks,
    )
    .await?
    {
        tracing::warn!(
            server_id = %server.id,
            user_id = %user_id,
            "rejected server invite link access without permission"
        );
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    }

    Ok(server)
}

/// Собирает ссылки-приглашения текущего пользователя с их состоянием на текущий момент.
async fn own_invite_links(
    state: &AppState,
    server_id: &Uuid,
    user_id: &Uuid,
) -> Result<Vec<ServerOwnInviteLink>, ServerError> {
    let invites = state
        .server_store
        .list_server_invites_by_creator(server_id, user_id)
        .await
        .map_err(ServerError::Internal)?;
    let invite_ids = invites.iter().map(|invite| invite.id).collect::<Vec<_>>();
    let invite_uses = state
        .server_store
        .list_server_invite_uses(&invite_ids)
        .await
        .map_err(ServerError::Internal)?;
    let mut uses_by_invite = HashMap::<Uuid, u32>::new();
    for invite_use in invite_uses {
        *uses_by_invite.entry(invite_use.invite_id).or_default() += 1;
    }
    let now = Utc::now();

    Ok(invites
        .into_iter()
        .map(|invite| {
            let uses = uses_by_invite.get(&invite.id);
            invite_link(&invite, uses, now)
        })
        .collect())
}

/// Преобразует приглашение в контракт ссылки с признаком текущей доступности.
fn invite_link(
    invite: &ServerInvite,
    uses: Option<&u32>,
    now: DateTime<Utc>,
) -> ServerOwnInviteLink {
    let uses = uses.copied().unwrap_or_default();
    let is_expired = invite
        .expires_at
        .is_some_and(|expires_at| expires_at <= now);
    let is_exhausted = invite.max_uses.is_some_and(|max_uses| uses >= max_uses);

    ServerOwnInviteLink {
        code: invite.id.to_string(),
        created_at: invite.created_at.to_rfc3339(),
        expires_at: invite.expires_at.map(|expires_at| expires_at.to_rfc3339()),
        max_uses: invite.max_uses,
        uses,
        is_active: invite.revoked_at.is_none() && !is_expired && !is_exhausted,
    }
}

/// Считает количество действующих ссылок, которые занимают лимит.
fn active_link_count(links: &[ServerOwnInviteLink]) -> u32 {
    links.iter().filter(|link| link.is_active).count() as u32
}

#[cfg(test)]
mod tests;
