//! Simple in-memory server storage.
use anyhow::anyhow;
use async_trait::async_trait;
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use uuid::Uuid;

use crate::features::servers::domain::{
    Server, ServerAccess, ServerInvite, ServerInviteUse, ServerMember, ServerMemberExclusion,
    ServerRole, ServerRoom,
};
use crate::features::servers::infrastructure::ServerStore;
/// In-memory server storage for local runs and tests.
#[derive(Default)]
pub(crate) struct InMemoryServerStore {
    pub(super) state: Mutex<InMemoryState>,
}

#[derive(Default)]
pub(super) struct InMemoryState {
    servers: Vec<Server>,
    invites: Vec<ServerInvite>,
    members: Vec<ServerMember>,
    exclusions: Vec<ServerMemberExclusion>,
    invite_uses: Vec<ServerInviteUse>,
    rooms: Vec<ServerRoom>,
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
            revoked_at: None,
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

    async fn list_server_invites(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerInvite>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        let mut invites = state
            .invites
            .iter()
            .filter(|invite| invite.server_id == *server_id)
            .cloned()
            .collect::<Vec<_>>();
        invites.sort_by_key(|invite| std::cmp::Reverse(invite.created_at));
        Ok(invites)
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

    async fn list_active_server_members(
        &self,
        server_id: &Uuid,
    ) -> anyhow::Result<Vec<ServerMember>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        let mut members = state
            .members
            .iter()
            .filter(|member| member.server_id == *server_id && member.left_at.is_none())
            .cloned()
            .collect::<Vec<_>>();
        members.sort_by_key(|member| member.joined_at);

        Ok(members)
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

    async fn insert_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        initiator_user_id: &Uuid,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<ServerMemberExclusion> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let exclusion = ServerMemberExclusion {
            id: Uuid::new_v4(),
            server_id: *server_id,
            user_id: *user_id,
            initiator_user_id: *initiator_user_id,
            expires_at,
            created_at: Utc::now(),
        };

        state.exclusions.push(exclusion.clone());

        Ok(exclusion)
    }

    async fn find_active_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<ServerMemberExclusion>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state
            .exclusions
            .iter()
            .filter(|exclusion| {
                exclusion.server_id == *server_id
                    && exclusion.user_id == *user_id
                    && exclusion.expires_at > now
            })
            .max_by_key(|exclusion| exclusion.expires_at)
            .cloned())
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

    async fn insert_server_room(
        &self,
        server_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
    ) -> anyhow::Result<ServerRoom> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let position = state
            .rooms
            .iter()
            .filter(|room| room.server_id == *server_id)
            .map(|room| room.position)
            .max()
            .map(|position| position.saturating_add(1))
            .unwrap_or(0);
        let now = Utc::now();
        let room = ServerRoom {
            id: Uuid::new_v4(),
            server_id: *server_id,
            name,
            kind,
            position,
            created_at: now,
            updated_at: now,
        };

        state.rooms.push(room.clone());

        Ok(room)
    }

    async fn list_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerRoom>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        let mut rooms = state
            .rooms
            .iter()
            .filter(|room| room.server_id == *server_id)
            .cloned()
            .collect::<Vec<_>>();
        rooms.sort_by_key(|room| room.position);

        Ok(rooms)
    }

    async fn find_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> anyhow::Result<Option<ServerRoom>> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state
            .rooms
            .iter()
            .find(|room| room.server_id == *server_id && room.id == *room_id)
            .cloned())
    }

    async fn update_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
    ) -> anyhow::Result<Option<ServerRoom>> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let Some(room) = state
            .rooms
            .iter_mut()
            .find(|room| room.server_id == *server_id && room.id == *room_id)
        else {
            return Ok(None);
        };

        room.name = name;
        room.kind = kind;
        room.updated_at = Utc::now();

        Ok(Some(room.clone()))
    }

    async fn delete_server_room(&self, server_id: &Uuid, room_id: &Uuid) -> anyhow::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        state
            .rooms
            .retain(|room| room.server_id != *server_id || room.id != *room_id);

        Ok(())
    }

    async fn count_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<u32> {
        let state = self.state.lock().map_err(|_| poisoned())?;

        Ok(state
            .rooms
            .iter()
            .filter(|room| room.server_id == *server_id)
            .count()
            .try_into()
            .unwrap_or(u32::MAX))
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
