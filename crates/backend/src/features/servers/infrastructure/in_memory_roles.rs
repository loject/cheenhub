//! In-memory role storage helpers.

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

fn poisoned() -> anyhow::Error {
    anyhow::anyhow!("in-memory server store lock poisoned")
}
