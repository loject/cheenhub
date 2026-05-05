//! Simple in-memory server storage.

use std::sync::Mutex;

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::servers::domain::{
    Server, ServerAccess, ServerInvite, ServerInviteUse, ServerMember,
};
use crate::features::servers::infrastructure::ServerStore;

/// In-memory server storage for local runs and tests.
#[derive(Default)]
pub(crate) struct InMemoryServerStore {
    state: Mutex<InMemoryState>,
}

#[derive(Default)]
struct InMemoryState {
    servers: Vec<Server>,
    invites: Vec<ServerInvite>,
    members: Vec<ServerMember>,
    invite_uses: Vec<ServerInviteUse>,
}

#[async_trait]
impl ServerStore for InMemoryServerStore {
    async fn insert_server(&self, owner_user_id: &Uuid, name: String) -> anyhow::Result<Server> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let now = Utc::now();
        let server = Server {
            id: Uuid::new_v4(),
            owner_user_id: *owner_user_id,
            name,
            created_at: now,
            updated_at: now,
        };

        state.servers.push(server.clone());

        Ok(server)
    }

    async fn list_servers(&self, user_id: &Uuid) -> anyhow::Result<Vec<ServerAccess>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
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

        let joined_server_ids = state
            .members
            .iter()
            .filter(|member| member.user_id == *user_id && member.left_at.is_none())
            .map(|member| member.server_id)
            .filter(|server_id| !result.iter().any(|access| access.server.id == *server_id))
            .collect::<Vec<_>>();

        result.extend(
            state
                .servers
                .iter()
                .filter(|server| joined_server_ids.contains(&server.id))
                .cloned()
                .map(|server| ServerAccess {
                    server,
                    is_member: true,
                }),
        );

        Ok(result)
    }

    async fn find_owned_server(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
    ) -> anyhow::Result<Option<Server>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state
            .servers
            .iter()
            .find(|server| server.id == *server_id && server.owner_user_id == *owner_user_id)
            .cloned())
    }

    async fn insert_server_invite(
        &self,
        server_id: &Uuid,
        creator_user_id: &Uuid,
        max_uses: Option<u32>,
        expires_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<ServerInvite> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let invite = ServerInvite {
            id: Uuid::new_v4(),
            server_id: *server_id,
            creator_user_id: *creator_user_id,
            max_uses,
            expires_at,
            created_at: Utc::now(),
        };

        state.invites.push(invite.clone());

        Ok(invite)
    }

    async fn find_server_invite(&self, code: &Uuid) -> anyhow::Result<Option<ServerInvite>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state
            .invites
            .iter()
            .find(|invite| invite.id == *code)
            .cloned())
    }

    async fn find_server(&self, server_id: &Uuid) -> anyhow::Result<Option<Server>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state
            .servers
            .iter()
            .find(|server| server.id == *server_id)
            .cloned())
    }

    async fn insert_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerMember> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
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

    async fn find_active_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<ServerMember>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state
            .members
            .iter()
            .find(|member| {
                member.server_id == *server_id
                    && member.user_id == *user_id
                    && member.left_at.is_none()
            })
            .cloned())
    }

    async fn leave_server(&self, server_id: &Uuid, user_id: &Uuid) -> anyhow::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;

        if let Some(member) = state.members.iter_mut().find(|member| {
            member.server_id == *server_id && member.user_id == *user_id && member.left_at.is_none()
        }) {
            member.left_at = Some(Utc::now());
        }

        Ok(())
    }

    async fn insert_server_invite_use(
        &self,
        invite_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerInviteUse> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let invite_use = ServerInviteUse {
            id: Uuid::new_v4(),
            invite_id: *invite_id,
            user_id: *user_id,
            used_at: Utc::now(),
        };

        state.invite_uses.push(invite_use.clone());

        Ok(invite_use)
    }

    async fn count_server_invite_uses(&self, invite_id: &Uuid) -> anyhow::Result<u32> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state
            .invite_uses
            .iter()
            .filter(|invite_use| invite_use.invite_id == *invite_id)
            .count()
            .try_into()
            .unwrap_or(u32::MAX))
    }
}

#[cfg(test)]
impl InMemoryServerStore {
    pub(crate) fn invites_for_tests(&self) -> anyhow::Result<Vec<ServerInvite>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state.invites.clone())
    }

    pub(crate) fn members_for_tests(&self) -> anyhow::Result<Vec<ServerMember>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state.members.clone())
    }

    pub(crate) fn invite_uses_for_tests(&self) -> anyhow::Result<Vec<ServerInviteUse>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state.invite_uses.clone())
    }
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory server store lock poisoned")
}
