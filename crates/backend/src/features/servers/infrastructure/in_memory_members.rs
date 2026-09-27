//! In-memory membership and exclusion storage helpers.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::servers::domain::{ServerMember, ServerMemberExclusion};
use crate::features::servers::infrastructure::in_memory::InMemoryState;

pub(super) fn insert_server_member(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<ServerMember> {
    let mut state = shared_state.lock().map_err(|_| poisoned())?;
    let member = ServerMember {
        id: Uuid::new_v4(),
        server_id: *server_id,
        user_id: *user_id,
        joined_at: Utc::now(),
        left_at: None,
    };

    state.members.push(member.clone());

    Ok(member)
}

pub(super) fn find_active_server_member(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<Option<ServerMember>> {
    let state = shared_state.lock().map_err(|_| poisoned())?;

    Ok(state
        .members
        .iter()
        .find(|member| {
            member.server_id == *server_id && member.user_id == *user_id && member.left_at.is_none()
        })
        .cloned())
}

pub(super) fn list_active_server_members(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
) -> anyhow::Result<Vec<ServerMember>> {
    let state = shared_state.lock().map_err(|_| poisoned())?;
    let mut members = state
        .members
        .iter()
        .filter(|member| member.server_id == *server_id && member.left_at.is_none())
        .cloned()
        .collect::<Vec<_>>();
    members.sort_by_key(|member| member.joined_at);

    Ok(members)
}

pub(super) fn leave_server(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<()> {
    let mut state = shared_state.lock().map_err(|_| poisoned())?;

    if let Some(member) = state.members.iter_mut().find(|member| {
        member.server_id == *server_id && member.user_id == *user_id && member.left_at.is_none()
    }) {
        member.left_at = Some(Utc::now());
    }

    Ok(())
}

pub(super) fn insert_server_member_exclusion(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    user_id: &Uuid,
    initiator_user_id: &Uuid,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<ServerMemberExclusion> {
    let mut state = shared_state.lock().map_err(|_| poisoned())?;
    let exclusion = ServerMemberExclusion {
        id: Uuid::new_v4(),
        server_id: *server_id,
        user_id: *user_id,
        initiator_user_id: *initiator_user_id,
        expires_at,
        created_at: Utc::now(),
    };

    state.exclusions.push(exclusion.clone());

    Ok(exclusion)
}

pub(super) fn find_active_server_member_exclusion(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<ServerMemberExclusion>> {
    let state = shared_state.lock().map_err(|_| poisoned())?;

    Ok(state
        .exclusions
        .iter()
        .filter(|exclusion| {
            exclusion.server_id == *server_id
                && exclusion.user_id == *user_id
                && exclusion.expires_at > now
        })
        .max_by_key(|exclusion| exclusion.expires_at)
        .cloned())
}

fn poisoned() -> anyhow::Error {
    anyhow::anyhow!("in-memory server store lock poisoned")
}
