//! Server infrastructure layer.

mod entities;
mod in_memory;
mod in_memory_roles;
mod postgres;
mod postgres_conversions;
mod postgres_roles;

use async_trait::async_trait;
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use uuid::Uuid;

use crate::features::servers::domain::{
    Server, ServerAccess, ServerInvite, ServerInviteUse, ServerMember, ServerMemberExclusion,
    ServerRole, ServerRoom,
};

pub(crate) use in_memory::InMemoryServerStore;
pub(crate) use postgres::PostgresServerStore;

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

    /// Updates a server name owned by a user.
    async fn update_server_name(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
        name: String,
    ) -> anyhow::Result<Option<Server>>;

    /// Updates a server avatar image owned by a user.
    async fn update_server_avatar_image_id(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
        avatar_image_id: Uuid,
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

    /// Lists server invites in newest-first order.
    async fn list_server_invites(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerInvite>>;

    /// Lists successful invite uses for invite ids in newest-first order.
    async fn list_server_invite_uses(
        &self,
        invite_ids: &[Uuid],
    ) -> anyhow::Result<Vec<ServerInviteUse>>;

    /// Marks a server invite as revoked.
    async fn revoke_server_invite(
        &self,
        server_id: &Uuid,
        invite_id: &Uuid,
    ) -> anyhow::Result<Option<ServerInvite>>;

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

    /// Lists active server members in oldest-first join order.
    async fn list_active_server_members(
        &self,
        server_id: &Uuid,
    ) -> anyhow::Result<Vec<ServerMember>>;

    /// Marks an active server membership as left.
    async fn leave_server(&self, server_id: &Uuid, user_id: &Uuid) -> anyhow::Result<()>;

    /// Inserts a temporary server-member exclusion.
    async fn insert_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        initiator_user_id: &Uuid,
        expires_at: chrono::DateTime<Utc>,
    ) -> anyhow::Result<ServerMemberExclusion>;

    /// Finds an active server-member exclusion.
    async fn find_active_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        now: chrono::DateTime<Utc>,
    ) -> anyhow::Result<Option<ServerMemberExclusion>>;

    /// Inserts a successful invite use row.
    async fn insert_server_invite_use(
        &self,
        invite_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerInviteUse>;

    /// Counts successful uses for an invite.
    async fn count_server_invite_uses(&self, invite_id: &Uuid) -> anyhow::Result<u32>;

    /// Inserts a new server room.
    async fn insert_server_room(
        &self,
        server_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
    ) -> anyhow::Result<ServerRoom>;

    /// Lists rooms for a server in display order.
    async fn list_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerRoom>>;

    /// Finds a room that belongs to a server.
    async fn find_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> anyhow::Result<Option<ServerRoom>>;

    /// Updates a room that belongs to a server.
    async fn update_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
    ) -> anyhow::Result<Option<ServerRoom>>;

    /// Deletes a room that belongs to a server.
    async fn delete_server_room(&self, server_id: &Uuid, room_id: &Uuid) -> anyhow::Result<()>;

    /// Counts rooms that belong to a server.
    async fn count_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<u32>;

    /// Lists roles for a server in display order.
    async fn list_server_roles(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerRole>>;

    /// Replaces all roles for a server.
    async fn replace_server_roles(
        &self,
        server_id: &Uuid,
        roles: Vec<ServerRole>,
    ) -> anyhow::Result<Vec<ServerRole>>;
}
