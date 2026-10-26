//! Проверки иерархии при управлении серверными ролями.

use std::collections::HashSet;

use cheenhub_contracts::realtime::{ServerRoleKind, ServerRolePermission};
use uuid::Uuid;

use crate::features::servers::domain::{Server, ServerRole};
use crate::features::servers::error::ServerError;
use crate::state::AppState;

use super::support::user_role_ids;

/// Предел прав участника, которому разрешено управлять ролями.
pub(super) struct RoleManagementScope {
    pub highest_role_position: u32,
    pub allowed_permissions: Vec<ServerRolePermission>,
}

/// Возвращает `None` для владельца; остальных проверяет по назначенным ролям.
pub(super) async fn role_management_scope(
    state: &AppState,
    server: &Server,
    actor_user_id: &Uuid,
    roles: &[ServerRole],
) -> Result<Option<RoleManagementScope>, ServerError> {
    if server.owner_user_id == *actor_user_id {
        return Ok(None);
    }

    if state
        .server_store
        .find_active_server_member(&server.id, actor_user_id)
        .await
        .map_err(ServerError::Internal)?
        .is_none()
    {
        return Err(access_denied());
    }

    let assigned_ids = user_role_ids(state, &server.id, actor_user_id).await?;
    let granted_roles = roles
        .iter()
        .filter(|role| role.kind == ServerRoleKind::Member || assigned_ids.contains(&role.id))
        .collect::<Vec<_>>();
    if !granted_roles.iter().any(|role| {
        role.permissions
            .contains(&ServerRolePermission::ManageRoles)
    }) {
        return Err(access_denied());
    }

    let highest_role_position = granted_roles
        .iter()
        .map(|role| role.position)
        .min()
        .ok_or_else(access_denied)?;
    let mut allowed_permissions = Vec::new();
    for role in granted_roles {
        for permission in &role.permissions {
            if !allowed_permissions.contains(permission) {
                allowed_permissions.push(*permission);
            }
        }
    }

    Ok(Some(RoleManagementScope {
        highest_role_position,
        allowed_permissions,
    }))
}

impl RoleManagementScope {
    /// Управлять можно только обычной ролью строго ниже своей наивысшей.
    pub fn can_manage_role(&self, role: &ServerRole) -> bool {
        role.kind == ServerRoleKind::Custom
            && role.position > self.highest_role_position
            && !role
                .permissions
                .contains(&ServerRolePermission::ManageRoles)
    }

    /// Проверяет весь результирующий список, включая удаление и перестановку.
    pub fn validate_role_changes(
        &self,
        previous: &[ServerRole],
        proposed: &[ServerRole],
    ) -> Result<(), ServerError> {
        let mut seen_ids = HashSet::new();
        for role in proposed {
            if !seen_ids.insert(role.id) {
                return Err(change_denied());
            }
        }

        for old_role in previous {
            if !self.can_manage_role(old_role) {
                let new_role = proposed
                    .iter()
                    .find(|role| role.id == old_role.id)
                    .ok_or_else(change_denied)?;
                if !unchanged(old_role, new_role) {
                    return Err(change_denied());
                }
            }
        }

        for new_role in proposed {
            let old_role = previous.iter().find(|role| role.id == new_role.id);
            if old_role.is_some_and(|old| !self.can_manage_role(old)) {
                continue;
            }
            if !self.can_manage_role(new_role) {
                return Err(change_denied());
            }
            let permissions_changed = old_role
                .is_none_or(|old| !same_permissions(&old.permissions, &new_role.permissions));
            if permissions_changed
                && new_role
                    .permissions
                    .iter()
                    .any(|permission| !self.allowed_permissions.contains(permission))
            {
                return Err(change_denied());
            }
        }

        Ok(())
    }
}

fn same_permissions(a: &[ServerRolePermission], b: &[ServerRolePermission]) -> bool {
    a.len() == b.len() && a.iter().all(|permission| b.contains(permission))
}

fn unchanged(a: &ServerRole, b: &ServerRole) -> bool {
    a.id == b.id
        && a.kind == b.kind
        && (a.kind == ServerRoleKind::Member || a.position == b.position)
        && a.name == b.name
        && a.color == b.color
        && same_permissions(&a.permissions, &b.permissions)
}

fn access_denied() -> ServerError {
    ServerError::NotFound("Сервер не найден или недоступен.".to_owned())
}

fn change_denied() -> ServerError {
    ServerError::BadRequest(
        "Можно управлять только обычными ролями ниже своей наивысшей роли и в пределах собственных разрешений."
            .to_owned(),
    )
}
