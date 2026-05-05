//! Simple in-memory server storage.

use std::sync::Mutex;

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::features::servers::domain::Server;
use crate::features::servers::infrastructure::ServerStore;

/// In-memory server storage for local runs and tests.
#[derive(Default)]
pub(crate) struct InMemoryServerStore {
    state: Mutex<InMemoryState>,
}

#[derive(Default)]
struct InMemoryState {
    servers: Vec<Server>,
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
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory server store lock poisoned")
}
