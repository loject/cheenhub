//! Postgres-backed server storage.

use async_trait::async_trait;
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::features::servers::domain::{
    Server, ServerAccess, ServerInvite, ServerInviteUse, ServerMember, ServerMemberExclusion,
    ServerRole, ServerRoom,
};
use crate::features::servers::infrastructure::ServerStore;
use crate::features::servers::infrastructure::entities::{
    server_invite_uses, server_invites, server_member_exclusions, server_members, server_rooms,
    servers,
};
use crate::features::servers::infrastructure::postgres_conversions::{
    room_kind_as_str, server_room_from_model,
};
use crate::features::servers::infrastructure::postgres_roles;

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
        let now = Utc::now();
        let model = servers::ActiveModel {
            id: Set(Uuid::new_v4()),
            owner_user_id: Set(*owner_user_id),
            name: Set(name),
            avatar_image_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.database)
        .await?;

        Ok(model.into())
    }

    async fn list_servers(&self, user_id: &Uuid) -> anyhow::Result<Vec<ServerAccess>> {
        let mut result: Vec<ServerAccess> = servers::Entity::find()
            .filter(servers::Column::OwnerUserId.eq(*user_id))
            .all(&self.database)
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
            .all(&self.database)
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
                .all(&self.database)
                .await?
                .into_iter()
                .map(|row| ServerAccess {
                    server: row.into(),
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
        Ok(servers::Entity::find()
            .filter(servers::Column::Id.eq(*server_id))
            .filter(servers::Column::OwnerUserId.eq(*owner_user_id))
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn update_server_name(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
        name: String,
    ) -> anyhow::Result<Option<Server>> {
        let Some(server) = servers::Entity::find()
            .filter(servers::Column::Id.eq(*server_id))
            .filter(servers::Column::OwnerUserId.eq(*owner_user_id))
            .one(&self.database)
            .await?
        else {
            return Ok(None);
        };
        let mut server = server.into_active_model();
        server.name = Set(name);
        server.updated_at = Set(Utc::now());

        Ok(Some(server.update(&self.database).await?.into()))
    }

    async fn update_server_avatar_image_id(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
        avatar_image_id: Uuid,
    ) -> anyhow::Result<Option<Server>> {
        let Some(server) = servers::Entity::find()
            .filter(servers::Column::Id.eq(*server_id))
            .filter(servers::Column::OwnerUserId.eq(*owner_user_id))
            .one(&self.database)
            .await?
        else {
            return Ok(None);
        };
        let mut server = server.into_active_model();
        server.avatar_image_id = Set(Some(avatar_image_id));
        server.updated_at = Set(Utc::now());

        Ok(Some(server.update(&self.database).await?.into()))
    }

    async fn insert_server_invite(
        &self,
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
            revoked_at: Set(None),
        }
        .insert(&self.database)
        .await?;

        Ok(model.into())
    }

    async fn find_server_invite(&self, code: &Uuid) -> anyhow::Result<Option<ServerInvite>> {
        Ok(server_invites::Entity::find_by_id(*code)
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn list_server_invites(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerInvite>> {
        Ok(server_invites::Entity::find()
            .filter(server_invites::Column::ServerId.eq(*server_id))
            .order_by_desc(server_invites::Column::CreatedAt)
            .all(&self.database)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn list_server_invite_uses(
        &self,
        invite_ids: &[Uuid],
    ) -> anyhow::Result<Vec<ServerInviteUse>> {
        if invite_ids.is_empty() {
            return Ok(Vec::new());
        }

        Ok(server_invite_uses::Entity::find()
            .filter(server_invite_uses::Column::InviteId.is_in(invite_ids.iter().copied()))
            .order_by_desc(server_invite_uses::Column::UsedAt)
            .all(&self.database)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn revoke_server_invite(
        &self,
        server_id: &Uuid,
        invite_id: &Uuid,
    ) -> anyhow::Result<Option<ServerInvite>> {
        let Some(invite) = server_invites::Entity::find()
            .filter(server_invites::Column::ServerId.eq(*server_id))
            .filter(server_invites::Column::Id.eq(*invite_id))
            .one(&self.database)
            .await?
        else {
            return Ok(None);
        };
        if invite.revoked_at.is_some() {
            return Ok(Some(invite.into()));
        }

        let mut invite = invite.into_active_model();
        invite.revoked_at = Set(Some(Utc::now()));
        Ok(Some(invite.update(&self.database).await?.into()))
    }

    async fn find_server(&self, server_id: &Uuid) -> anyhow::Result<Option<Server>> {
        Ok(servers::Entity::find_by_id(*server_id)
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn insert_server_member(
        &self,
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
        .insert(&self.database)
        .await?;

        Ok(model.into())
    }

    async fn find_active_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<ServerMember>> {
        Ok(server_members::Entity::find()
            .filter(server_members::Column::ServerId.eq(*server_id))
            .filter(server_members::Column::UserId.eq(*user_id))
            .filter(server_members::Column::LeftAt.is_null())
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn list_active_server_members(
        &self,
        server_id: &Uuid,
    ) -> anyhow::Result<Vec<ServerMember>> {
        Ok(server_members::Entity::find()
            .filter(server_members::Column::ServerId.eq(*server_id))
            .filter(server_members::Column::LeftAt.is_null())
            .order_by_asc(server_members::Column::JoinedAt)
            .all(&self.database)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn leave_server(&self, server_id: &Uuid, user_id: &Uuid) -> anyhow::Result<()> {
        let Some(member) = server_members::Entity::find()
            .filter(server_members::Column::ServerId.eq(*server_id))
            .filter(server_members::Column::UserId.eq(*user_id))
            .filter(server_members::Column::LeftAt.is_null())
            .one(&self.database)
            .await?
        else {
            return Ok(());
        };
        let mut member = member.into_active_model();
        member.left_at = Set(Some(Utc::now()));
        member.update(&self.database).await?;

        Ok(())
    }

    async fn insert_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        initiator_user_id: &Uuid,
        expires_at: chrono::DateTime<Utc>,
    ) -> anyhow::Result<ServerMemberExclusion> {
        let model = server_member_exclusions::ActiveModel {
            id: Set(Uuid::new_v4()),
            server_id: Set(*server_id),
            user_id: Set(*user_id),
            initiator_user_id: Set(*initiator_user_id),
            expires_at: Set(expires_at),
            created_at: Set(Utc::now()),
        }
        .insert(&self.database)
        .await?;

        Ok(model.into())
    }

    async fn find_active_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        now: chrono::DateTime<Utc>,
    ) -> anyhow::Result<Option<ServerMemberExclusion>> {
        Ok(server_member_exclusions::Entity::find()
            .filter(server_member_exclusions::Column::ServerId.eq(*server_id))
            .filter(server_member_exclusions::Column::UserId.eq(*user_id))
            .filter(server_member_exclusions::Column::ExpiresAt.gt(now))
            .order_by_desc(server_member_exclusions::Column::ExpiresAt)
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn insert_server_invite_use(
        &self,
        invite_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerInviteUse> {
        let model = server_invite_uses::ActiveModel {
            id: Set(Uuid::new_v4()),
            invite_id: Set(*invite_id),
            user_id: Set(*user_id),
            used_at: Set(Utc::now()),
        }
        .insert(&self.database)
        .await?;

        Ok(model.into())
    }

    async fn count_server_invite_uses(&self, invite_id: &Uuid) -> anyhow::Result<u32> {
        let count = server_invite_uses::Entity::find()
            .filter(server_invite_uses::Column::InviteId.eq(*invite_id))
            .count(&self.database)
            .await?;

        Ok(count.try_into().unwrap_or(u32::MAX))
    }

    async fn insert_server_room(
        &self,
        server_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
    ) -> anyhow::Result<ServerRoom> {
        let position = server_rooms::Entity::find()
            .filter(server_rooms::Column::ServerId.eq(*server_id))
            .order_by_desc(server_rooms::Column::Position)
            .one(&self.database)
            .await?
            .map(|room| room.position.saturating_add(1))
            .unwrap_or(0);
        let now = Utc::now();
        let model = server_rooms::ActiveModel {
            id: Set(Uuid::new_v4()),
            server_id: Set(*server_id),
            name: Set(name),
            kind: Set(room_kind_as_str(kind).to_owned()),
            position: Set(position),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.database)
        .await?;

        server_room_from_model(model)
    }

    async fn list_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerRoom>> {
        let rows = server_rooms::Entity::find()
            .filter(server_rooms::Column::ServerId.eq(*server_id))
            .order_by_asc(server_rooms::Column::Position)
            .all(&self.database)
            .await?;

        rows.into_iter().map(server_room_from_model).collect()
    }

    async fn find_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> anyhow::Result<Option<ServerRoom>> {
        server_rooms::Entity::find()
            .filter(server_rooms::Column::ServerId.eq(*server_id))
            .filter(server_rooms::Column::Id.eq(*room_id))
            .one(&self.database)
            .await?
            .map(server_room_from_model)
            .transpose()
    }

    async fn update_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
    ) -> anyhow::Result<Option<ServerRoom>> {
        let Some(room) = server_rooms::Entity::find()
            .filter(server_rooms::Column::ServerId.eq(*server_id))
            .filter(server_rooms::Column::Id.eq(*room_id))
            .one(&self.database)
            .await?
        else {
            return Ok(None);
        };
        let mut room = room.into_active_model();
        room.name = Set(name);
        room.kind = Set(room_kind_as_str(kind).to_owned());
        room.updated_at = Set(Utc::now());
        let room = room.update(&self.database).await?;

        server_room_from_model(room).map(Some)
    }

    async fn delete_server_room(&self, server_id: &Uuid, room_id: &Uuid) -> anyhow::Result<()> {
        if let Some(room) = server_rooms::Entity::find()
            .filter(server_rooms::Column::ServerId.eq(*server_id))
            .filter(server_rooms::Column::Id.eq(*room_id))
            .one(&self.database)
            .await?
        {
            server_rooms::Entity::delete_by_id(room.id)
                .exec(&self.database)
                .await?;
        }

        Ok(())
    }

    async fn count_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<u32> {
        let count = server_rooms::Entity::find()
            .filter(server_rooms::Column::ServerId.eq(*server_id))
            .count(&self.database)
            .await?;

        Ok(count.try_into().unwrap_or(u32::MAX))
    }

    async fn list_server_roles(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerRole>> {
        postgres_roles::list_server_roles(&self.database, server_id).await
    }

    async fn replace_server_roles(
        &self,
        server_id: &Uuid,
        roles: Vec<ServerRole>,
    ) -> anyhow::Result<Vec<ServerRole>> {
        postgres_roles::replace_server_roles(&self.database, server_id, roles).await
    }
}
