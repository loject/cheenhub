//! Атомарные операции приглашений в in-memory-хранилище.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::AcceptInviteOutcome;
use super::in_memory::InMemoryState;
use crate::features::servers::domain::{ServerInviteUse, ServerMember};

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
        .find(|invite| invite.id == *invite_id)
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
