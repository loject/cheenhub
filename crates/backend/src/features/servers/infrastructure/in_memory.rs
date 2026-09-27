//! Простое in-memory-хранилище серверов.
use anyhow::anyhow;
use async_trait::async_trait;
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use uuid::Uuid;

use crate::features::servers::domain::{
    Server, ServerAccess, ServerInvite, ServerInviteUse, ServerMember, ServerMemberExclusion,
    ServerRole, ServerRoom, ServerRoomWriteAccess,
};
use crate::features::servers::infrastructure::{AcceptInviteOutcome, ServerStore};
/// In-memory-хранилище серверов для локального запуска и тестов.
#[derive(Default)]
pub(crate) struct InMemoryServerStore {
    pub(super) state: Mutex<InMemoryState>,
}

#[derive(Default)]
pub(super) struct InMemoryState {
    servers: Vec<Server>,
    pub(super) invites: Vec<ServerInvite>,
    pub(super) members: Vec<ServerMember>,
    pub(super) exclusions: Vec<ServerMemberExclusion>,
    pub(super) invite_uses: Vec<ServerInviteUse>,
    pub(super) rooms: Vec<ServerRoom>,
    pub(super) roles: Vec<ServerRole>,
    /// (server_id, user_id, role_id, granted_by_user_id)
    pub(super) member_roles: Vec<(Uuid, Uuid, Uuid, Uuid)>,
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
            avatar_image_id: None,
            audio_bitrate_bps: cheenhub_contracts::media::VOICE_AUDIO_BITRATE_BPS,
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

    async fn update_server_name(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
        name: String,
    ) -> anyhow::Result<Option<Server>> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let Some(server) = state
            .servers
            .iter_mut()
            .find(|server| server.id == *server_id && server.owner_user_id == *owner_user_id)
        else {
            return Ok(None);
        };

        server.name = name;
        server.updated_at = Utc::now();
        Ok(Some(server.clone()))
    }

    async fn update_server_avatar_image_id(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
        avatar_image_id: Uuid,
    ) -> anyhow::Result<Option<Server>> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let Some(server) = state
            .servers
            .iter_mut()
            .find(|server| server.id == *server_id && server.owner_user_id == *owner_user_id)
        else {
            return Ok(None);
        };

        server.avatar_image_id = Some(avatar_image_id);
        server.updated_at = Utc::now();
        Ok(Some(server.clone()))
    }

    async fn update_server_audio_bitrate(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
        audio_bitrate_bps: u32,
    ) -> anyhow::Result<Option<Server>> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let Some(server) = state
            .servers
            .iter_mut()
            .find(|server| server.id == *server_id && server.owner_user_id == *owner_user_id)
        else {
            return Ok(None);
        };

        server.audio_bitrate_bps = audio_bitrate_bps;
        server.updated_at = Utc::now();
        Ok(Some(server.clone()))
    }

    async fn insert_server_invite(
        &self,
        server_id: &Uuid,
        creator_user_id: &Uuid,
        max_uses: Option<u32>,
        expires_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<ServerInvite> {
        super::in_memory_invites::insert_server_invite(
            &self.state,
            server_id,
            creator_user_id,
            max_uses,
            expires_at,
        )
    }

    async fn find_server_invite(&self, code: &Uuid) -> anyhow::Result<Option<ServerInvite>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state
            .invites
            .iter()
            .find(|invite| invite.id == *code && invite.deleted_at.is_none())
            .cloned())
    }

    async fn list_server_invites(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerInvite>> {
        super::in_memory_invites::list_server_invites(&self.state, server_id)
    }

    async fn list_server_invites_by_creator(
        &self,
        server_id: &Uuid,
        creator_user_id: &Uuid,
    ) -> anyhow::Result<Vec<ServerInvite>> {
        super::in_memory_invites::list_server_invites_by_creator(
            &self.state,
            server_id,
            creator_user_id,
        )
    }

    async fn soft_delete_server_invite_created_by(
        &self,
        server_id: &Uuid,
        invite_id: &Uuid,
        creator_user_id: &Uuid,
        deleted_at: DateTime<Utc>,
    ) -> anyhow::Result<Option<ServerInvite>> {
        super::in_memory_invites::soft_delete_server_invite_created_by(
            &self.state,
            server_id,
            invite_id,
            creator_user_id,
            deleted_at,
        )
    }

    async fn list_server_invite_uses(
        &self,
        invite_ids: &[Uuid],
    ) -> anyhow::Result<Vec<ServerInviteUse>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        let mut uses = state
            .invite_uses
            .iter()
            .filter(|invite_use| invite_ids.contains(&invite_use.invite_id))
            .cloned()
            .collect::<Vec<_>>();
        uses.sort_by_key(|invite_use| std::cmp::Reverse(invite_use.used_at));

        Ok(uses)
    }

    async fn revoke_server_invite(
        &self,
        server_id: &Uuid,
        invite_id: &Uuid,
    ) -> anyhow::Result<Option<ServerInvite>> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let Some(invite) = state
            .invites
            .iter_mut()
            .find(|invite| invite.server_id == *server_id && invite.id == *invite_id)
        else {
            return Ok(None);
        };
        if invite.revoked_at.is_none() {
            invite.revoked_at = Some(Utc::now());
        }

        Ok(Some(invite.clone()))
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
        super::in_memory_members::insert_server_member(&self.state, server_id, user_id)
    }

    async fn find_active_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<ServerMember>> {
        super::in_memory_members::find_active_server_member(&self.state, server_id, user_id)
    }

    async fn list_active_server_members(
        &self,
        server_id: &Uuid,
    ) -> anyhow::Result<Vec<ServerMember>> {
        super::in_memory_members::list_active_server_members(&self.state, server_id)
    }

    async fn leave_server(&self, server_id: &Uuid, user_id: &Uuid) -> anyhow::Result<()> {
        super::in_memory_members::leave_server(&self.state, server_id, user_id)
    }

    async fn insert_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        initiator_user_id: &Uuid,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<ServerMemberExclusion> {
        super::in_memory_members::insert_server_member_exclusion(
            &self.state,
            server_id,
            user_id,
            initiator_user_id,
            expires_at,
        )
    }

    async fn find_active_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<ServerMemberExclusion>> {
        super::in_memory_members::find_active_server_member_exclusion(
            &self.state,
            server_id,
            user_id,
            now,
        )
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

    async fn accept_server_invite(
        &self,
        invite_id: &Uuid,
        user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<AcceptInviteOutcome> {
        super::in_memory_invites::accept_server_invite(&self.state, invite_id, user_id, now)
    }

    async fn insert_server_room(
        &self,
        server_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
        write_access: ServerRoomWriteAccess,
    ) -> anyhow::Result<ServerRoom> {
        super::in_memory_rooms::insert_server_room(&self.state, server_id, name, kind, write_access)
    }

    async fn list_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerRoom>> {
        super::in_memory_rooms::list_server_rooms(&self.state, server_id)
    }

    async fn find_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> anyhow::Result<Option<ServerRoom>> {
        super::in_memory_rooms::find_server_room(&self.state, server_id, room_id)
    }

    async fn update_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
        write_access: ServerRoomWriteAccess,
    ) -> anyhow::Result<Option<ServerRoom>> {
        super::in_memory_rooms::update_server_room(
            &self.state,
            server_id,
            room_id,
            name,
            kind,
            write_access,
        )
    }

    async fn delete_server_room(&self, server_id: &Uuid, room_id: &Uuid) -> anyhow::Result<()> {
        super::in_memory_rooms::delete_server_room(&self.state, server_id, room_id)
    }

    async fn count_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<u32> {
        super::in_memory_rooms::count_server_rooms(&self.state, server_id)
    }

    async fn list_server_roles(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerRole>> {
        super::in_memory_roles::list_server_roles(&self.state, server_id)
    }

    async fn replace_server_roles(
        &self,
        server_id: &Uuid,
        roles: Vec<ServerRole>,
    ) -> anyhow::Result<Vec<ServerRole>> {
        super::in_memory_roles::replace_server_roles(&self.state, server_id, roles)
    }

    async fn list_server_member_roles(
        &self,
        server_id: &Uuid,
    ) -> anyhow::Result<Vec<(Uuid, Uuid)>> {
        super::in_memory_roles::list_server_member_roles(&self.state, server_id)
    }

    async fn assign_server_member_role(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        role_id: &Uuid,
        granted_by_user_id: &Uuid,
    ) -> anyhow::Result<()> {
        super::in_memory_roles::assign_server_member_role(
            &self.state,
            server_id,
            user_id,
            role_id,
            granted_by_user_id,
        )
    }

    async fn revoke_server_member_role(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        role_id: &Uuid,
    ) -> anyhow::Result<()> {
        super::in_memory_roles::revoke_server_member_role(&self.state, server_id, user_id, role_id)
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
