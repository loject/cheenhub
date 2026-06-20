//! Server role settings application flows.

use std::collections::HashSet;

use cheenhub_contracts::realtime::{
    ListServerRoles, SaveServerRoles, ServerRoleDraft, ServerRoleEntry, ServerRoleKind,
    ServerRoleList, ServerRolePermission, ServerRolesSaved,
};
use chrono::Utc;
use uuid::Uuid;

use crate::features::servers::domain::ServerRole;
use crate::features::servers::error::ServerError;
use crate::state::AppState;

use super::support::{owned_server, parse_server_id};

/// Lists roles for a server owned by the current user.
pub(crate) async fn list_server_roles(
    state: &AppState,
    owner_user_id: &Uuid,
    request: ListServerRoles,
) -> Result<ServerRoleList, ServerError> {
    let server_id = parse_server_id(request.server_id)?;
    let server = owned_server(state, &server_id, owner_user_id).await?;
    let roles = ensure_server_roles(state, &server.id).await?;
    let member_count = active_member_count(state, &server.id).await?;

    tracing::debug!(
        server_id = %server.id,
        role_count = roles.len(),
        "listed server roles for settings"
    );

    Ok(ServerRoleList {
        server_id: server.id.to_string(),
        roles: role_entries(roles, member_count),
    })
}

/// Saves roles for a server owned by the current user.
pub(crate) async fn save_server_roles(
    state: &AppState,
    owner_user_id: &Uuid,
    request: SaveServerRoles,
) -> Result<ServerRolesSaved, ServerError> {
    let server_id = parse_server_id(request.server_id)?;
    let server = owned_server(state, &server_id, owner_user_id).await?;
    let existing_roles = ensure_server_roles(state, &server.id).await?;
    let roles = normalize_role_drafts(&server.id, existing_roles, request.roles)?;
    let saved_roles = state
        .server_store
        .replace_server_roles(&server.id, roles)
        .await
        .map_err(ServerError::Internal)?;
    let member_count = active_member_count(state, &server.id).await?;

    tracing::info!(
        server_id = %server.id,
        role_count = saved_roles.len(),
        "saved server roles"
    );

    Ok(ServerRolesSaved {
        server_id: server.id.to_string(),
        roles: role_entries(saved_roles, member_count),
    })
}

async fn ensure_server_roles(
    state: &AppState,
    server_id: &Uuid,
) -> Result<Vec<ServerRole>, ServerError> {
    let roles = state
        .server_store
        .list_server_roles(server_id)
        .await
        .map_err(ServerError::Internal)?;
    if has_required_roles(&roles) {
        return Ok(roles);
    }

    let normalized = with_required_roles(server_id, roles);
    let saved = state
        .server_store
        .replace_server_roles(server_id, normalized)
        .await
        .map_err(ServerError::Internal)?;

    tracing::info!(
        server_id = %server_id,
        role_count = saved.len(),
        "initialized required server roles"
    );

    Ok(saved)
}

fn has_required_roles(roles: &[ServerRole]) -> bool {
    roles.iter().any(|role| role.kind == ServerRoleKind::Owner)
        && roles.iter().any(|role| role.kind == ServerRoleKind::Member)
}

fn with_required_roles(server_id: &Uuid, mut roles: Vec<ServerRole>) -> Vec<ServerRole> {
    let now = Utc::now();
    if !roles.iter().any(|role| role.kind == ServerRoleKind::Owner) {
        roles.insert(
            0,
            ServerRole {
                id: Uuid::new_v4(),
                server_id: *server_id,
                name: "Владелец".to_owned(),
                color: "#f59e0b".to_owned(),
                kind: ServerRoleKind::Owner,
                position: 0,
                permissions: all_permissions(),
                created_at: now,
                updated_at: now,
            },
        );
    }
    if !roles.iter().any(|role| role.kind == ServerRoleKind::Member) {
        roles.push(ServerRole {
            id: Uuid::new_v4(),
            server_id: *server_id,
            name: "Участник".to_owned(),
            color: "#94a3b8".to_owned(),
            kind: ServerRoleKind::Member,
            position: roles.len().try_into().unwrap_or(u32::MAX),
            permissions: Vec::new(),
            created_at: now,
            updated_at: now,
        });
    }

    roles.sort_by_key(|role| match role.kind {
        ServerRoleKind::Owner => 0,
        ServerRoleKind::Custom => 1,
        ServerRoleKind::Member => 2,
    });
    for (index, role) in roles.iter_mut().enumerate() {
        role.server_id = *server_id;
        role.position = index.try_into().unwrap_or(u32::MAX);
        if role.kind == ServerRoleKind::Owner {
            role.permissions = all_permissions();
        }
    }

    roles
}

fn normalize_role_drafts(
    server_id: &Uuid,
    existing_roles: Vec<ServerRole>,
    drafts: Vec<ServerRoleDraft>,
) -> Result<Vec<ServerRole>, ServerError> {
    let existing_owner = existing_roles
        .iter()
        .find(|role| role.kind == ServerRoleKind::Owner)
        .cloned()
        .unwrap_or_else(|| required_role(server_id, ServerRoleKind::Owner));
    let existing_member = existing_roles
        .iter()
        .find(|role| role.kind == ServerRoleKind::Member)
        .cloned()
        .unwrap_or_else(|| required_role(server_id, ServerRoleKind::Member));
    let existing_ids = existing_roles
        .iter()
        .map(|role| role.id)
        .collect::<HashSet<_>>();

    let owner_draft = drafts
        .iter()
        .find(|draft| draft.kind == ServerRoleKind::Owner);
    let member_draft = drafts
        .iter()
        .find(|draft| draft.kind == ServerRoleKind::Member);
    let mut roles = Vec::with_capacity(drafts.len().max(2));
    roles.push(role_from_draft(server_id, existing_owner, owner_draft, 0)?);

    for draft in drafts
        .iter()
        .filter(|draft| draft.kind == ServerRoleKind::Custom)
    {
        let role_id = draft
            .role_id
            .as_deref()
            .and_then(|role_id| Uuid::parse_str(role_id).ok())
            .filter(|role_id| existing_ids.contains(role_id))
            .unwrap_or_else(Uuid::new_v4);
        let now = Utc::now();
        roles.push(ServerRole {
            id: role_id,
            server_id: *server_id,
            name: validate_role_name(&draft.name)?,
            color: validate_role_color(&draft.color)?,
            kind: ServerRoleKind::Custom,
            position: roles.len().try_into().unwrap_or(u32::MAX),
            permissions: unique_permissions(draft.permissions.clone()),
            created_at: now,
            updated_at: now,
        });
    }

    let member_position = roles.len().try_into().unwrap_or(u32::MAX);
    roles.push(role_from_draft(
        server_id,
        existing_member,
        member_draft,
        member_position,
    )?);

    Ok(roles)
}

fn role_from_draft(
    server_id: &Uuid,
    existing: ServerRole,
    draft: Option<&ServerRoleDraft>,
    position: u32,
) -> Result<ServerRole, ServerError> {
    let mut role = existing;
    role.server_id = *server_id;
    role.position = position;
    role.updated_at = Utc::now();
    if let Some(draft) = draft {
        role.name = validate_role_name(&draft.name)?;
        role.color = validate_role_color(&draft.color)?;
        if role.kind == ServerRoleKind::Owner {
            role.permissions = all_permissions();
        } else {
            role.permissions = unique_permissions(draft.permissions.clone());
        }
    }

    Ok(role)
}

fn required_role(server_id: &Uuid, kind: ServerRoleKind) -> ServerRole {
    let now = Utc::now();
    match kind {
        ServerRoleKind::Owner => ServerRole {
            id: Uuid::new_v4(),
            server_id: *server_id,
            name: "Владелец".to_owned(),
            color: "#f59e0b".to_owned(),
            kind,
            position: 0,
            permissions: all_permissions(),
            created_at: now,
            updated_at: now,
        },
        ServerRoleKind::Member => ServerRole {
            id: Uuid::new_v4(),
            server_id: *server_id,
            name: "Участник".to_owned(),
            color: "#94a3b8".to_owned(),
            kind,
            position: 1,
            permissions: Vec::new(),
            created_at: now,
            updated_at: now,
        },
        ServerRoleKind::Custom => ServerRole {
            id: Uuid::new_v4(),
            server_id: *server_id,
            name: "Новая роль".to_owned(),
            color: "#a855f7".to_owned(),
            kind,
            position: 1,
            permissions: Vec::new(),
            created_at: now,
            updated_at: now,
        },
    }
}

async fn active_member_count(state: &AppState, server_id: &Uuid) -> Result<u32, ServerError> {
    let count = state
        .server_store
        .list_active_server_members(server_id)
        .await
        .map_err(ServerError::Internal)?
        .len();

    Ok(count.try_into().unwrap_or(u32::MAX))
}

fn role_entries(roles: Vec<ServerRole>, member_count: u32) -> Vec<ServerRoleEntry> {
    roles
        .into_iter()
        .map(|role| {
            let permissions = if role.kind == ServerRoleKind::Owner {
                all_permissions()
            } else {
                role.permissions
            };
            ServerRoleEntry {
                role_id: role.id.to_string(),
                name: role.name,
                color: role.color,
                members: role_member_count(role.kind, member_count),
                is_required: role.kind != ServerRoleKind::Custom,
                kind: role.kind,
                permissions,
            }
        })
        .collect()
}

fn role_member_count(kind: ServerRoleKind, member_count: u32) -> u32 {
    match kind {
        ServerRoleKind::Owner => 1,
        ServerRoleKind::Member => member_count,
        ServerRoleKind::Custom => 0,
    }
}

fn validate_role_name(name: &str) -> Result<String, ServerError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ServerError::BadRequest(
            "Название роли не может быть пустым.".to_owned(),
        ));
    }
    if trimmed.chars().count() > 32 {
        return Err(ServerError::BadRequest(
            "Название роли не должно быть длиннее 32 символов.".to_owned(),
        ));
    }

    Ok(trimmed.to_owned())
}

fn validate_role_color(color: &str) -> Result<String, ServerError> {
    let valid = color.strip_prefix('#').is_some_and(|hex| {
        hex.len() == 6 && hex.chars().all(|character| character.is_ascii_hexdigit())
    });
    if !valid {
        return Err(ServerError::BadRequest(
            "Цвет роли должен быть в формате #RRGGBB.".to_owned(),
        ));
    }

    Ok(color.to_lowercase())
}

fn unique_permissions(permissions: Vec<ServerRolePermission>) -> Vec<ServerRolePermission> {
    let mut result = Vec::new();
    for permission in permissions {
        if !result.contains(&permission) {
            result.push(permission);
        }
    }

    result
}

fn all_permissions() -> Vec<ServerRolePermission> {
    vec![
        ServerRolePermission::CreateInviteLinks,
        ServerRolePermission::KickServerMembers,
        ServerRolePermission::ManageRoles,
        ServerRolePermission::KickVoiceMembers,
        ServerRolePermission::DeleteMessages,
    ]
}
