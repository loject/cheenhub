//! Server invite settings application flows.

use std::collections::{HashMap, HashSet};

use cheenhub_contracts::realtime::{
    KickServerInviteMember, ListServerInvites, RevokeServerInvite, ServerInviteJoinedMember,
    ServerInviteLink, ServerInviteList, ServerInviteMemberKicked, ServerInviteRevoked,
};
use uuid::Uuid;

use crate::features::servers::domain::{ServerInvite, ServerInviteUse};
use crate::features::servers::error::ServerError;
use crate::state::AppState;

use super::support::{owned_server, parse_server_id};

/// Lists invite links for a server owned by the current user.
pub(crate) async fn list_server_invites(
    state: &AppState,
    owner_user_id: &Uuid,
    request: ListServerInvites,
) -> Result<ServerInviteList, ServerError> {
    let server_id = parse_server_id(request.server_id)?;
    let server = owned_server(state, &server_id, owner_user_id).await?;
    let invites = state
        .server_store
        .list_server_invites(&server.id)
        .await
        .map_err(ServerError::Internal)?;
    let invite_ids = invites.iter().map(|invite| invite.id).collect::<Vec<_>>();
    let invite_uses = state
        .server_store
        .list_server_invite_uses(&invite_ids)
        .await
        .map_err(ServerError::Internal)?;
    let invite_links = invite_links(state, &server.id, invites, invite_uses).await?;

    tracing::debug!(
        server_id = %server.id,
        invite_count = invite_links.len(),
        "listed server invites for settings"
    );

    Ok(ServerInviteList {
        server_id: server.id.to_string(),
        invites: invite_links,
    })
}

/// Revokes one invite link for a server owned by the current user.
pub(crate) async fn revoke_server_invite(
    state: &AppState,
    owner_user_id: &Uuid,
    request: RevokeServerInvite,
) -> Result<ServerInviteRevoked, ServerError> {
    let server_id = parse_server_id(request.server_id)?;
    let invite_id = Uuid::parse_str(&request.code)
        .map_err(|_| ServerError::BadRequest("Приглашение не найдено.".to_owned()))?;
    let server = owned_server(state, &server_id, owner_user_id).await?;
    let Some(invite) = state
        .server_store
        .revoke_server_invite(&server.id, &invite_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Приглашение не найдено.".to_owned()));
    };
    let Some(revoked_at) = invite.revoked_at else {
        return Err(ServerError::Internal(anyhow::anyhow!(
            "server invite revoke did not set revoked_at"
        )));
    };

    tracing::info!(
        server_id = %server.id,
        invite_code = %invite.id,
        "revoked server invite"
    );

    Ok(ServerInviteRevoked {
        server_id: server.id.to_string(),
        code: invite.id.to_string(),
        revoked_at: revoked_at.to_rfc3339(),
    })
}

/// Kicks a member that joined through a server invite.
pub(crate) async fn kick_server_invite_member(
    state: &AppState,
    owner_user_id: &Uuid,
    request: KickServerInviteMember,
) -> Result<ServerInviteMemberKicked, ServerError> {
    let server_id = parse_server_id(request.server_id)?;
    let invite_id = Uuid::parse_str(&request.invite_code)
        .map_err(|_| ServerError::BadRequest("Приглашение не найдено.".to_owned()))?;
    let target_user_id = Uuid::parse_str(&request.user_id)
        .map_err(|_| ServerError::BadRequest("Пользователь не найден.".to_owned()))?;
    let server = owned_server(state, &server_id, owner_user_id).await?;
    if server.owner_user_id == target_user_id {
        return Err(ServerError::BadRequest(
            "Владельца сервера нельзя исключить.".to_owned(),
        ));
    }
    let Some(invite) = state
        .server_store
        .find_server_invite(&invite_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Приглашение не найдено.".to_owned()));
    };
    if invite.server_id != server.id {
        return Err(ServerError::NotFound("Приглашение не найдено.".to_owned()));
    }
    let used_invite = state
        .server_store
        .list_server_invite_uses(&[invite.id])
        .await
        .map_err(ServerError::Internal)?
        .into_iter()
        .any(|invite_use| invite_use.user_id == target_user_id);
    if !used_invite {
        return Err(ServerError::NotFound(
            "Пользователь не присоединялся по этой ссылке.".to_owned(),
        ));
    }
    if state
        .server_store
        .find_active_server_member(&server.id, &target_user_id)
        .await
        .map_err(ServerError::Internal)?
        .is_none()
    {
        return Err(ServerError::BadRequest(
            "Пользователь уже не состоит на сервере.".to_owned(),
        ));
    }

    state
        .server_store
        .leave_server(&server.id, &target_user_id)
        .await
        .map_err(ServerError::Internal)?;

    tracing::info!(
        server_id = %server.id,
        invite_code = %invite.id,
        user_id = %target_user_id,
        "kicked server invite member"
    );

    Ok(ServerInviteMemberKicked {
        server_id: server.id.to_string(),
        invite_code: invite.id.to_string(),
        user_id: target_user_id.to_string(),
    })
}

async fn invite_links(
    state: &AppState,
    server_id: &Uuid,
    invites: Vec<ServerInvite>,
    invite_uses: Vec<ServerInviteUse>,
) -> Result<Vec<ServerInviteLink>, ServerError> {
    let mut user_ids = invites
        .iter()
        .map(|invite| invite.creator_user_id)
        .collect::<HashSet<_>>();
    user_ids.extend(invite_uses.iter().map(|invite_use| invite_use.user_id));
    let users = users_by_id(state, user_ids).await?;
    let mut uses_by_invite = HashMap::<Uuid, Vec<ServerInviteUse>>::new();
    for invite_use in invite_uses {
        uses_by_invite
            .entry(invite_use.invite_id)
            .or_default()
            .push(invite_use);
    }

    let mut result = Vec::with_capacity(invites.len());
    for invite in invites {
        let uses = uses_by_invite.remove(&invite.id).unwrap_or_default();
        let mut joined_members = Vec::with_capacity(uses.len());
        for invite_use in &uses {
            let is_active_member = state
                .server_store
                .find_active_server_member(server_id, &invite_use.user_id)
                .await
                .map_err(ServerError::Internal)?
                .is_some();
            joined_members.push(ServerInviteJoinedMember {
                user_id: invite_use.user_id.to_string(),
                nickname: nickname_for(&users, &invite_use.user_id),
                joined_at: invite_use.used_at.to_rfc3339(),
                is_active_member,
            });
        }

        result.push(ServerInviteLink {
            code: invite.id.to_string(),
            author_user_id: invite.creator_user_id.to_string(),
            author_nickname: nickname_for(&users, &invite.creator_user_id),
            created_at: invite.created_at.to_rfc3339(),
            expires_at: invite.expires_at.map(|expires_at| expires_at.to_rfc3339()),
            max_uses: invite.max_uses,
            uses: uses.len().try_into().unwrap_or(u32::MAX),
            revoked_at: invite.revoked_at.map(|revoked_at| revoked_at.to_rfc3339()),
            joined_members,
        });
    }

    Ok(result)
}

async fn users_by_id(
    state: &AppState,
    user_ids: HashSet<Uuid>,
) -> Result<HashMap<Uuid, String>, ServerError> {
    let mut users = HashMap::with_capacity(user_ids.len());
    for user_id in user_ids {
        let Some(user) = state
            .auth_store
            .find_user_by_id(&user_id)
            .await
            .map_err(ServerError::Internal)?
        else {
            continue;
        };
        users.insert(user.id, user.nickname);
    }

    Ok(users)
}

fn nickname_for(users: &HashMap<Uuid, String>, user_id: &Uuid) -> String {
    users
        .get(user_id)
        .cloned()
        .unwrap_or_else(|| "Удаленный пользователь".to_owned())
}
