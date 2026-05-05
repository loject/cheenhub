//! Simple in-memory server storage.

use std::sync::Mutex;

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::servers::domain::{Server, ServerInvite};
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

    async fn list_servers(&self, owner_user_id: &Uuid) -> anyhow::Result<Vec<Server>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state
            .servers
            .iter()
            .filter(|server| server.owner_user_id == *owner_user_id)
            .cloned()
            .collect())
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
}

#[cfg(test)]
impl InMemoryServerStore {
    pub(crate) fn invites_for_tests(&self) -> anyhow::Result<Vec<ServerInvite>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state.invites.clone())
    }
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory server store lock poisoned")
}
