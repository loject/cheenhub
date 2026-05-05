//! Server infrastructure layer.

mod entities;
mod in_memory;

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};
use uuid::Uuid;

use crate::features::servers::domain::{
    Server, ServerAccess, ServerInvite, ServerInviteUse, ServerMember,
};
use crate::features::servers::infrastructure::entities::{
    server_invite_uses, server_invites, server_members, servers,
};

pub(crate) use in_memory::InMemoryServerStore;

/// Server storage boundary.
#[async_trait]
pub(crate) trait ServerStore: Send + Sync {
    /// Inserts a new server for a user.
    async fn insert_server(&self, owner_user_id: &Uuid, name: String) -> anyhow::Result<Server>;

    /// Lists servers available to a user.
    async fn list_servers(&self, user_id: &Uuid) -> anyhow::Result<Vec<ServerAccess>>;

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

    /// Finds a server invite by code.
    async fn find_server_invite(&self, code: &Uuid) -> anyhow::Result<Option<ServerInvite>>;

    /// Finds a server by id.
    async fn find_server(&self, server_id: &Uuid) -> anyhow::Result<Option<Server>>;

    /// Inserts a new active server member row.
    async fn insert_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerMember>;

    /// Finds an active server member row.
    async fn find_active_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<ServerMember>>;

    /// Marks an active server membership as left.
    async fn leave_server(&self, server_id: &Uuid, user_id: &Uuid) -> anyhow::Result<()>;

    /// Inserts a successful invite use row.
    async fn insert_server_invite_use(
        &self,
        invite_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerInviteUse>;

    /// Counts successful uses for an invite.
    async fn count_server_invite_uses(&self, invite_id: &Uuid) -> anyhow::Result<u32>;
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

    async fn list_servers(&self, user_id: &Uuid) -> anyhow::Result<Vec<ServerAccess>> {
        list_servers(&self.database, user_id).await
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

    async fn find_server_invite(&self, code: &Uuid) -> anyhow::Result<Option<ServerInvite>> {
        find_server_invite(&self.database, code).await
    }

    async fn find_server(&self, server_id: &Uuid) -> anyhow::Result<Option<Server>> {
        find_server(&self.database, server_id).await
    }

    async fn insert_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerMember> {
        insert_server_member(&self.database, server_id, user_id).await
    }

    async fn find_active_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<ServerMember>> {
        find_active_server_member(&self.database, server_id, user_id).await
    }

    async fn leave_server(&self, server_id: &Uuid, user_id: &Uuid) -> anyhow::Result<()> {
        leave_server(&self.database, server_id, user_id).await
    }

    async fn insert_server_invite_use(
        &self,
        invite_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerInviteUse> {
        insert_server_invite_use(&self.database, invite_id, user_id).await
    }

    async fn count_server_invite_uses(&self, invite_id: &Uuid) -> anyhow::Result<u32> {
        count_server_invite_uses(&self.database, invite_id).await
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

/// Lists servers available to a user.
async fn list_servers(
    database: &impl ConnectionTrait,
    user_id: &Uuid,
) -> anyhow::Result<Vec<ServerAccess>> {
    let mut result: Vec<ServerAccess> = servers::Entity::find()
        .filter(servers::Column::OwnerUserId.eq(*user_id))
        .all(database)
        .await?
        .into_iter()
        .map(|row| ServerAccess {
            server: row.into(),
            is_member: true,
        })
        .collect();

    let member_rows = server_members::Entity::find()
        .filter(server_members::Column::UserId.eq(*user_id))
        .filter(server_members::Column::LeftAt.is_null())
        .all(database)
        .await?;
    let joined_server_ids = member_rows
        .into_iter()
        .map(|member| member.server_id)
        .filter(|server_id| !result.iter().any(|access| access.server.id == *server_id))
        .collect::<Vec<_>>();

    if joined_server_ids.is_empty() {
        return Ok(result);
    }

    result.extend(
        servers::Entity::find()
            .filter(servers::Column::Id.is_in(joined_server_ids))
            .all(database)
            .await?
            .into_iter()
            .map(|row| ServerAccess {
                server: row.into(),
                is_member: true,
            }),
    );

    Ok(result)
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

/// Finds a server invite by code.
async fn find_server_invite(
    database: &impl ConnectionTrait,
    code: &Uuid,
) -> anyhow::Result<Option<ServerInvite>> {
    Ok(server_invites::Entity::find_by_id(*code)
        .one(database)
        .await?
        .map(Into::into))
}

/// Finds a server by id.
async fn find_server(
    database: &impl ConnectionTrait,
    server_id: &Uuid,
) -> anyhow::Result<Option<Server>> {
    Ok(servers::Entity::find_by_id(*server_id)
        .one(database)
        .await?
        .map(Into::into))
}

/// Inserts a new active server member row.
async fn insert_server_member(
    database: &impl ConnectionTrait,
    server_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<ServerMember> {
    let model = server_members::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(*server_id),
        user_id: Set(*user_id),
        joined_at: Set(Utc::now()),
        left_at: Set(None),
    }
    .insert(database)
    .await?;

    Ok(model.into())
}

/// Finds an active server member row.
async fn find_active_server_member(
    database: &impl ConnectionTrait,
    server_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<Option<ServerMember>> {
    Ok(server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(*server_id))
        .filter(server_members::Column::UserId.eq(*user_id))
        .filter(server_members::Column::LeftAt.is_null())
        .one(database)
        .await?
        .map(Into::into))
}

/// Marks an active server membership as left.
async fn leave_server(
    database: &impl ConnectionTrait,
    server_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<()> {
    let Some(member) = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(*server_id))
        .filter(server_members::Column::UserId.eq(*user_id))
        .filter(server_members::Column::LeftAt.is_null())
        .one(database)
        .await?
    else {
        return Ok(());
    };
    let mut member = member.into_active_model();
    member.left_at = Set(Some(Utc::now()));
    member.update(database).await?;

    Ok(())
}

/// Inserts a successful invite use row.
async fn insert_server_invite_use(
    database: &impl ConnectionTrait,
    invite_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<ServerInviteUse> {
    let model = server_invite_uses::ActiveModel {
        id: Set(Uuid::new_v4()),
        invite_id: Set(*invite_id),
        user_id: Set(*user_id),
        used_at: Set(Utc::now()),
    }
    .insert(database)
    .await?;

    Ok(model.into())
}

/// Counts successful uses for an invite.
async fn count_server_invite_uses(
    database: &impl ConnectionTrait,
    invite_id: &Uuid,
) -> anyhow::Result<u32> {
    let count = server_invite_uses::Entity::find()
        .filter(server_invite_uses::Column::InviteId.eq(*invite_id))
        .count(database)
        .await?;

    Ok(count.try_into().unwrap_or(u32::MAX))
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

impl From<server_members::Model> for ServerMember {
    fn from(row: server_members::Model) -> Self {
        Self {
            id: row.id,
            server_id: row.server_id,
            user_id: row.user_id,
            joined_at: row.joined_at,
            left_at: row.left_at,
        }
    }
}

impl From<server_invite_uses::Model> for ServerInviteUse {
    fn from(row: server_invite_uses::Model) -> Self {
        Self {
            id: row.id,
            invite_id: row.invite_id,
            user_id: row.user_id,
            used_at: row.used_at,
        }
    }
}
