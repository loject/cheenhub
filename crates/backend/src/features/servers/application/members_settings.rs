//! Server member settings application flows.

use std::collections::{HashMap, HashSet};

use cheenhub_contracts::realtime::{
    KickServerMember, ListServerMembers, ServerMemberEntry, ServerMemberKicked, ServerMemberList,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::features::servers::domain::{ServerInvite, ServerInviteUse, ServerMember};
use crate::features::servers::error::ServerError;
use crate::state::AppState;

use super::support::{owned_server, parse_server_id};

const MIN_EXCLUSION_SECONDS: u64 = 60;
const MAX_EXCLUSION_SECONDS: u64 = 60 * 60 * 24 * 365 * 100;

/// Lists active server members for a server owned by the current user.
pub(crate) async fn list_server_members(
    state: &AppState,
    owner_user_id: &Uuid,
    request: ListServerMembers,
) -> Result<ServerMemberList, ServerError> {
    let server_id = parse_server_id(request.server_id)?;
    let server = owned_server(state, &server_id, owner_user_id).await?;
    let members = state
        .server_store
        .list_active_server_members(&server.id)
        .await
        .map_err(ServerError::Internal)?;
    let invite_codes = invite_codes_by_member(state, &server.id).await?;
    let user_ids = members.iter().map(|member| member.user_id).collect();
    let users = users_by_id(state, user_ids).await?;
    let entries = member_entries(&server.owner_user_id, members, invite_codes, users);

    tracing::debug!(
        server_id = %server.id,
        member_count = entries.len(),
        "listed server members for settings"
    );

    Ok(ServerMemberList {
        server_id: server.id.to_string(),
        members: entries,
    })
}

/// Kicks an active member from a server owned by the current user.
pub(crate) async fn kick_server_member(
    state: &AppState,
    owner_user_id: &Uuid,
    request: KickServerMember,
) -> Result<ServerMemberKicked, ServerError> {
    let server_id = parse_server_id(request.server_id)?;
    let target_user_id = Uuid::parse_str(&request.user_id)
        .map_err(|_| ServerError::BadRequest("Пользователь не найден.".to_owned()))?;
    let server = owned_server(state, &server_id, owner_user_id).await?;
    if server.owner_user_id == target_user_id {
        return Err(ServerError::BadRequest(
            "Владельца сервера нельзя исключить.".to_owned(),
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

    let excluded_until = exclusion_deadline(request.exclusion_duration_seconds)?;
    if let Some(expires_at) = excluded_until {
        state
            .server_store
            .insert_server_member_exclusion(&server.id, &target_user_id, owner_user_id, expires_at)
            .await
            .map_err(ServerError::Internal)?;
    }
    state
        .server_store
        .leave_server(&server.id, &target_user_id)
        .await
        .map_err(ServerError::Internal)?;

    tracing::info!(
        server_id = %server.id,
        user_id = %target_user_id,
        excluded_until = excluded_until.as_ref().map(|value| value.to_rfc3339()),
        "kicked server member"
    );

    Ok(ServerMemberKicked {
        server_id: server.id.to_string(),
        user_id: target_user_id.to_string(),
        excluded_until: excluded_until.map(|value| value.to_rfc3339()),
    })
}

fn exclusion_deadline(seconds: Option<u64>) -> Result<Option<chrono::DateTime<Utc>>, ServerError> {
    let Some(seconds) = seconds else {
        return Ok(None);
    };
    if !(MIN_EXCLUSION_SECONDS..=MAX_EXCLUSION_SECONDS).contains(&seconds) {
        return Err(ServerError::BadRequest(
            "Срок исключения должен быть от 1 минуты до 1 года.".to_owned(),
        ));
    }
    let seconds = i64::try_from(seconds)
        .map_err(|_| ServerError::BadRequest("Срок исключения слишком большой.".to_owned()))?;

    Ok(Some(Utc::now() + Duration::seconds(seconds)))
}

async fn invite_codes_by_member(
    state: &AppState,
    server_id: &Uuid,
) -> Result<HashMap<Uuid, (String, String)>, ServerError> {
    let invites = state
        .server_store
        .list_server_invites(server_id)
        .await
        .map_err(ServerError::Internal)?;
    let invite_ids = invites.iter().map(|invite| invite.id).collect::<Vec<_>>();
    let uses = state
        .server_store
        .list_server_invite_uses(&invite_ids)
        .await
        .map_err(ServerError::Internal)?;
    let invite_codes = invites
        .into_iter()
        .map(|invite| (invite.id, invite))
        .collect::<HashMap<_, _>>();

    Ok(latest_invite_use_by_member(invite_codes, uses))
}

fn latest_invite_use_by_member(
    invites: HashMap<Uuid, ServerInvite>,
    uses: Vec<ServerInviteUse>,
) -> HashMap<Uuid, (String, String)> {
    let mut result = HashMap::new();
    for invite_use in uses {
        if result.contains_key(&invite_use.user_id) {
            continue;
        }
        let Some(invite) = invites.get(&invite_use.invite_id) else {
            continue;
        };
        result.insert(
            invite_use.user_id,
            (invite.id.to_string(), invite_use.used_at.to_rfc3339()),
        );
    }

    result
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

fn member_entries(
    owner_user_id: &Uuid,
    members: Vec<ServerMember>,
    mut invite_codes: HashMap<Uuid, (String, String)>,
    users: HashMap<Uuid, String>,
) -> Vec<ServerMemberEntry> {
    members
        .into_iter()
        .map(|member| {
            let invite = invite_codes.remove(&member.user_id);
            ServerMemberEntry {
                user_id: member.user_id.to_string(),
                nickname: users
                    .get(&member.user_id)
                    .cloned()
                    .unwrap_or_else(|| "Удаленный пользователь".to_owned()),
                is_owner: member.user_id == *owner_user_id,
                joined_at: member.joined_at.to_rfc3339(),
                invite_code: invite.as_ref().map(|(code, _)| code.clone()),
                invite_used_at: invite.map(|(_, used_at)| used_at),
            }
        })
        .collect()
}
