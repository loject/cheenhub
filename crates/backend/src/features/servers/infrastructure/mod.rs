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

use crate::features::servers::domain::{Server, ServerInvite};
use crate::features::servers::infrastructure::entities::{server_invites, servers};

pub(crate) use in_memory::InMemoryServerStore;

/// Server storage boundary.
#[async_trait]
pub(crate) trait ServerStore: Send + Sync {
    /// Inserts a new server for a user.
    async fn insert_server(&self, owner_user_id: &Uuid, name: String) -> anyhow::Result<Server>;

    /// Lists servers owned by a user.
    async fn list_servers(&self, owner_user_id: &Uuid) -> anyhow::Result<Vec<Server>>;

    /// Finds a server owned by a user.
    async fn find_owned_server(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
    ) -> anyhow::Result<Option<Server>>;

    /// Inserts a new server invite.
    async fn insert_server_invite(
        &self,
        server_id: &Uuid,
        creator_user_id: &Uuid,
        max_uses: Option<u32>,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> anyhow::Result<ServerInvite>;
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

    async fn find_owned_server(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
    ) -> anyhow::Result<Option<Server>> {
        find_owned_server(&self.database, server_id, owner_user_id).await
    }

    async fn insert_server_invite(
        &self,
        server_id: &Uuid,
        creator_user_id: &Uuid,
        max_uses: Option<u32>,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> anyhow::Result<ServerInvite> {
        insert_server_invite(
            &self.database,
            server_id,
            creator_user_id,
            max_uses,
            expires_at,
        )
        .await
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

/// Finds a server owned by a user.
async fn find_owned_server(
    database: &impl ConnectionTrait,
    server_id: &Uuid,
    owner_user_id: &Uuid,
) -> anyhow::Result<Option<Server>> {
    Ok(servers::Entity::find()
        .filter(servers::Column::Id.eq(*server_id))
        .filter(servers::Column::OwnerUserId.eq(*owner_user_id))
        .one(database)
        .await?
        .map(Into::into))
}

/// Inserts a new server invite.
async fn insert_server_invite(
    database: &impl ConnectionTrait,
    server_id: &Uuid,
    creator_user_id: &Uuid,
    max_uses: Option<u32>,
    expires_at: Option<chrono::DateTime<Utc>>,
) -> anyhow::Result<ServerInvite> {
    let created_at = Utc::now();
    let model = server_invites::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(*server_id),
        creator_user_id: Set(*creator_user_id),
        max_uses: Set(max_uses.map(|value| value as i32)),
        expires_at: Set(expires_at),
        created_at: Set(created_at),
    }
    .insert(database)
    .await?;

    Ok(model.into())
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

impl From<server_invites::Model> for ServerInvite {
    fn from(row: server_invites::Model) -> Self {
        Self {
            id: row.id,
            server_id: row.server_id,
            creator_user_id: row.creator_user_id,
            max_uses: row.max_uses.map(|value| value as u32),
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}
