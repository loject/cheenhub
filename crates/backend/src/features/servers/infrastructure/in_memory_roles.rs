//! In-memory role storage helpers.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use uuid::Uuid;

use crate::features::servers::domain::ServerRole;
use crate::features::servers::infrastructure::in_memory::InMemoryState;

pub(super) fn list_server_roles(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
) -> anyhow::Result<Vec<ServerRole>> {
    let state = state.lock().map_err(|_| poisoned())?;
    let mut roles = state
        .roles
        .iter()
        .filter(|role| role.server_id == *server_id)
        .cloned()
        .collect::<Vec<_>>();
    roles.sort_by_key(|role| role.position);

    Ok(roles)
}

pub(super) fn replace_server_roles(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    roles: Vec<ServerRole>,
) -> anyhow::Result<Vec<ServerRole>> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    state.roles.retain(|role| role.server_id != *server_id);
    state.roles.extend(roles.clone());

    Ok(roles)
}

pub(super) fn list_server_member_roles(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
) -> anyhow::Result<Vec<(Uuid, Uuid)>> {
    let state = state.lock().map_err(|_| poisoned())?;
    Ok(state
        .member_roles
        .iter()
        .filter(|(sid, _, _, _)| *sid == *server_id)
        .map(|(_, uid, rid, _)| (*uid, *rid))
        .collect())
}

pub(super) fn active_member_role_ids_for_users(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    user_ids: &[Uuid],
) -> anyhow::Result<HashMap<Uuid, Vec<Uuid>>> {
    let requested = user_ids.iter().copied().collect::<HashSet<_>>();
    if requested.is_empty() {
        return Ok(HashMap::new());
    }
    let state = state.lock().map_err(|_| poisoned())?;
    let mut result = state
        .members
        .iter()
        .filter(|member| {
            member.server_id == *server_id
                && member.left_at.is_none()
                && requested.contains(&member.user_id)
        })
        .map(|member| (member.user_id, Vec::new()))
        .collect::<HashMap<_, _>>();
    for (assigned_server, user_id, role_id, _) in &state.member_roles {
        if assigned_server == server_id
            && let Some(roles) = result.get_mut(user_id)
        {
            roles.push(*role_id);
        }
    }
    Ok(result)
}

pub(super) fn assign_server_member_role(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    user_id: &Uuid,
    role_id: &Uuid,
    granted_by_user_id: &Uuid,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let already_exists = state
        .member_roles
        .iter()
        .any(|(sid, uid, rid, _)| *sid == *server_id && *uid == *user_id && *rid == *role_id);
    if !already_exists {
        state
            .member_roles
            .push((*server_id, *user_id, *role_id, *granted_by_user_id));
    }

    Ok(())
}

pub(super) fn revoke_server_member_role(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    user_id: &Uuid,
    role_id: &Uuid,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    state
        .member_roles
        .retain(|(sid, uid, rid, _)| !(*sid == *server_id && *uid == *user_id && *rid == *role_id));

    Ok(())
}

fn poisoned() -> anyhow::Error {
    anyhow::anyhow!("in-memory server store lock poisoned")
}
