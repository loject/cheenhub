//! Хранение системной идентичности сервера в in-memory окружении.

use std::sync::Mutex;
use uuid::Uuid;

use crate::features::servers::domain::{Server, ServerAccess, ServerSystemAccount};
use crate::features::servers::infrastructure::in_memory::InMemoryState;

pub(super) fn list_servers(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
) -> anyhow::Result<Vec<ServerAccess>> {
    let state = state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    let mut result = state
        .servers
        .iter()
        .filter(|server| server.owner_user_id == *user_id)
        .cloned()
        .map(|server| ServerAccess {
            server,
            is_member: true,
        })
        .collect::<Vec<_>>();
    let joined_ids = state
        .members
        .iter()
        .filter(|member| member.user_id == *user_id && member.left_at.is_none())
        .map(|member| member.server_id)
        .filter(|id| !result.iter().any(|access| access.server.id == *id))
        .collect::<Vec<_>>();
    result.extend(
        state
            .servers
            .iter()
            .filter(|server| joined_ids.contains(&server.id))
            .cloned()
            .map(|server| ServerAccess {
                server,
                is_member: true,
            }),
    );
    Ok(result)
}

pub(super) fn find_owned(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    owner_id: &Uuid,
) -> anyhow::Result<Option<ServerSystemAccount>> {
    let state = state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    if !state
        .servers
        .iter()
        .any(|server| server.id == *server_id && server.owner_user_id == *owner_id)
    {
        return Ok(None);
    }
    Ok(state
        .system_accounts
        .iter()
        .find(|account| account.id == *server_id)
        .cloned())
}

pub(super) fn find_for_server(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
) -> anyhow::Result<Option<ServerSystemAccount>> {
    let state = state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    Ok(state
        .system_accounts
        .iter()
        .find(|account| account.id == *server_id)
        .cloned())
}

pub(super) fn update_nickname(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    owner_id: &Uuid,
    nickname: String,
) -> anyhow::Result<Option<ServerSystemAccount>> {
    let mut state = state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    if !state
        .servers
        .iter()
        .any(|server| server.id == *server_id && server.owner_user_id == *owner_id)
    {
        return Ok(None);
    }
    let Some(account) = state
        .system_accounts
        .iter_mut()
        .find(|account| account.id == *server_id)
    else {
        anyhow::bail!("server {server_id} has no system account");
    };
    account.nickname = nickname;
    Ok(Some(account.clone()))
}

pub(super) fn update_avatar(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    owner_id: &Uuid,
    avatar_id: Uuid,
) -> anyhow::Result<Option<Server>> {
    let mut state = state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    let Some(server) = state
        .servers
        .iter_mut()
        .find(|server| server.id == *server_id && server.owner_user_id == *owner_id)
    else {
        return Ok(None);
    };
    server.avatar_image_id = Some(avatar_id);
    server.updated_at = chrono::Utc::now();
    let result = server.clone();
    let Some(account) = state
        .system_accounts
        .iter_mut()
        .find(|account| account.id == *server_id)
    else {
        anyhow::bail!("server {server_id} has no system account");
    };
    account.avatar_image_id = Some(avatar_id);
    Ok(Some(result))
}
