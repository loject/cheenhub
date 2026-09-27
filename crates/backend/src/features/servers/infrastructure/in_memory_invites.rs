//! Атомарные операции приглашений в in-memory-хранилище.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::AcceptInviteOutcome;
use super::in_memory::InMemoryState;
use crate::features::servers::domain::{ServerInvite, ServerInviteUse, ServerMember};

pub(super) fn insert_server_invite(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    creator_user_id: &Uuid,
    max_uses: Option<u32>,
    expires_at: Option<DateTime<Utc>>,
) -> anyhow::Result<ServerInvite> {
    let mut state = shared_state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    let invite = ServerInvite {
        id: Uuid::new_v4(),
        server_id: *server_id,
        creator_user_id: *creator_user_id,
        max_uses,
        expires_at,
        created_at: Utc::now(),
        revoked_at: None,
        deleted_at: None,
    };
    state.invites.push(invite.clone());

    Ok(invite)
}

pub(super) fn list_server_invites(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
) -> anyhow::Result<Vec<ServerInvite>> {
    let state = shared_state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    let mut invites = state
        .invites
        .iter()
        .filter(|invite| invite.server_id == *server_id)
        .cloned()
        .collect::<Vec<_>>();
    invites.sort_by_key(|invite| std::cmp::Reverse(invite.created_at));

    Ok(invites)
}

pub(super) fn list_server_invites_by_creator(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    creator_user_id: &Uuid,
) -> anyhow::Result<Vec<ServerInvite>> {
    let state = shared_state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    let mut invites = state
        .invites
        .iter()
        .filter(|invite| {
            invite.server_id == *server_id
                && invite.creator_user_id == *creator_user_id
                && invite.deleted_at.is_none()
        })
        .cloned()
        .collect::<Vec<_>>();
    invites.sort_by_key(|invite| std::cmp::Reverse(invite.created_at));

    Ok(invites)
}

pub(super) fn soft_delete_server_invite_created_by(
    shared_state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    invite_id: &Uuid,
    creator_user_id: &Uuid,
    deleted_at: DateTime<Utc>,
) -> anyhow::Result<Option<ServerInvite>> {
    let mut state = shared_state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    let Some(invite) = state.invites.iter_mut().find(|invite| {
        invite.server_id == *server_id
            && invite.id == *invite_id
            && invite.creator_user_id == *creator_user_id
            && invite.deleted_at.is_none()
    }) else {
        return Ok(None);
    };

    invite.deleted_at = Some(deleted_at);

    Ok(Some(invite.clone()))
}

pub(super) fn accept_server_invite(
    shared_state: &Mutex<InMemoryState>,
    invite_id: &Uuid,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<AcceptInviteOutcome> {
    let mut state = shared_state
        .lock()
        .map_err(|_| anyhow::anyhow!("in-memory server store lock poisoned"))?;
    let Some(invite) = state
        .invites
        .iter()
        .find(|invite| invite.id == *invite_id && invite.deleted_at.is_none())
        .cloned()
    else {
        return Ok(AcceptInviteOutcome::NotFound);
    };
    if invite
        .expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        return Ok(AcceptInviteOutcome::Expired);
    }
    if invite.revoked_at.is_some() {
        return Ok(AcceptInviteOutcome::Revoked);
    }
    if state.members.iter().any(|member| {
        member.server_id == invite.server_id
            && member.user_id == *user_id
            && member.left_at.is_none()
    }) {
        return Ok(AcceptInviteOutcome::AlreadyMember);
    }
    let uses = state
        .invite_uses
        .iter()
        .filter(|invite_use| invite_use.invite_id == *invite_id)
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    if invite.max_uses.is_some_and(|max_uses| uses >= max_uses) {
        return Ok(AcceptInviteOutcome::Exhausted);
    }

    state.members.push(ServerMember {
        id: Uuid::new_v4(),
        server_id: invite.server_id,
        user_id: *user_id,
        joined_at: now,
        left_at: None,
    });
    state.invite_uses.push(ServerInviteUse {
        id: Uuid::new_v4(),
        invite_id: *invite_id,
        user_id: *user_id,
        used_at: now,
    });

    Ok(AcceptInviteOutcome::Accepted)
}
