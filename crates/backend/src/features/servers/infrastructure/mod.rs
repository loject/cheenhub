//! Server infrastructure layer.

mod entities;
mod in_memory;

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set,
};
use uuid::Uuid;

use crate::features::servers::domain::Server;
use crate::features::servers::infrastructure::entities::servers;

pub(crate) use in_memory::InMemoryServerStore;

/// Server storage boundary.
#[async_trait]
pub(crate) trait ServerStore: Send + Sync {
    /// Inserts a new server for a user.
    async fn insert_server(&self, owner_user_id: &Uuid, name: String) -> anyhow::Result<Server>;

    /// Lists servers owned by a user.
    async fn list_servers(&self, owner_user_id: &Uuid) -> anyhow::Result<Vec<Server>>;
}

/// Postgres-backed server storage.
pub(crate) struct PostgresServerStore {
    database: DatabaseConnection,
}

impl PostgresServerStore {
    /// Builds a Postgres-backed server storage.
    pub(crate) fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ServerStore for PostgresServerStore {
    async fn insert_server(&self, owner_user_id: &Uuid, name: String) -> anyhow::Result<Server> {
        insert_server(&self.database, owner_user_id, name).await
    }

    async fn list_servers(&self, owner_user_id: &Uuid) -> anyhow::Result<Vec<Server>> {
        list_servers(&self.database, owner_user_id).await
    }
}

/// Inserts a new server for a user.
async fn insert_server(
    database: &impl ConnectionTrait,
    owner_user_id: &Uuid,
    name: String,
) -> anyhow::Result<Server> {
    let now = Utc::now();
    let model = servers::ActiveModel {
        id: Set(Uuid::new_v4()),
        owner_user_id: Set(*owner_user_id),
        name: Set(name),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(database)
    .await?;

    Ok(model.into())
}

/// Lists servers owned by a user.
async fn list_servers(
    database: &impl ConnectionTrait,
    owner_user_id: &Uuid,
) -> anyhow::Result<Vec<Server>> {
    Ok(servers::Entity::find()
        .filter(servers::Column::OwnerUserId.eq(*owner_user_id))
        .all(database)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

impl From<servers::Model> for Server {
    fn from(row: servers::Model) -> Self {
        Self {
            id: row.id,
            owner_user_id: row.owner_user_id,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
