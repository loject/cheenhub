//! Инфраструктурный слой серверов.

mod entities;
mod in_memory;
mod in_memory_roles;
mod in_memory_rooms;
mod postgres;
mod postgres_conversions;
mod postgres_roles;
mod postgres_rooms;

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

/// Граница хранилища серверов.
#[async_trait]
pub(crate) trait ServerStore: Send + Sync {
    /// Вставляет новый сервер для пользователя.
    async fn insert_server(&self, owner_user_id: &Uuid, name: String) -> anyhow::Result<Server>;

    /// Возвращает серверы, доступные пользователю.
    async fn list_servers(&self, user_id: &Uuid) -> anyhow::Result<Vec<ServerAccess>>;

    /// Находит сервер, принадлежащий пользователю.
    async fn find_owned_server(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
    ) -> anyhow::Result<Option<Server>>;

    /// Обновляет имя сервера, принадлежащего пользователю.
    async fn update_server_name(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
        name: String,
    ) -> anyhow::Result<Option<Server>>;

    /// Обновляет изображение аватара сервера, принадлежащего пользователю.
    async fn update_server_avatar_image_id(
        &self,
        server_id: &Uuid,
        owner_user_id: &Uuid,
        avatar_image_id: Uuid,
    ) -> anyhow::Result<Option<Server>>;

    /// Вставляет новое приглашение сервера.
    async fn insert_server_invite(
        &self,
        server_id: &Uuid,
        creator_user_id: &Uuid,
        max_uses: Option<u32>,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> anyhow::Result<ServerInvite>;

    /// Находит приглашение сервера по коду.
    async fn find_server_invite(&self, code: &Uuid) -> anyhow::Result<Option<ServerInvite>>;

    /// Возвращает приглашения сервера в порядке от новых к старым.
    async fn list_server_invites(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerInvite>>;

    /// Возвращает успешные использования приглашений для идентификаторов приглашений в порядке от новых к старым.
    async fn list_server_invite_uses(
        &self,
        invite_ids: &[Uuid],
    ) -> anyhow::Result<Vec<ServerInviteUse>>;

    /// Помечает приглашение сервера как отозванное.
    async fn revoke_server_invite(
        &self,
        server_id: &Uuid,
        invite_id: &Uuid,
    ) -> anyhow::Result<Option<ServerInvite>>;

    /// Находит сервер по идентификатору.
    async fn find_server(&self, server_id: &Uuid) -> anyhow::Result<Option<Server>>;

    /// Вставляет новую активную строку участника сервера.
    async fn insert_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerMember>;

    /// Находит активную строку участника сервера.
    async fn find_active_server_member(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<ServerMember>>;

    /// Возвращает активных участников сервера в порядке присоединения от старых к новым.
    async fn list_active_server_members(
        &self,
        server_id: &Uuid,
    ) -> anyhow::Result<Vec<ServerMember>>;

    /// Помечает активное участие в сервере как завершенное.
    async fn leave_server(&self, server_id: &Uuid, user_id: &Uuid) -> anyhow::Result<()>;

    /// Вставляет временное исключение участника сервера.
    async fn insert_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        initiator_user_id: &Uuid,
        expires_at: chrono::DateTime<Utc>,
    ) -> anyhow::Result<ServerMemberExclusion>;

    /// Находит активное исключение участника сервера.
    async fn find_active_server_member_exclusion(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        now: chrono::DateTime<Utc>,
    ) -> anyhow::Result<Option<ServerMemberExclusion>>;

    /// Вставляет строку об успешном использовании приглашения.
    async fn insert_server_invite_use(
        &self,
        invite_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<ServerInviteUse>;

    /// Считает успешные использования приглашения.
    async fn count_server_invite_uses(&self, invite_id: &Uuid) -> anyhow::Result<u32>;

    /// Вставляет новую комнату сервера.
    async fn insert_server_room(
        &self,
        server_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
    ) -> anyhow::Result<ServerRoom>;

    /// Возвращает комнаты сервера в порядке отображения.
    async fn list_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerRoom>>;

    /// Находит комнату, принадлежащую серверу.
    async fn find_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> anyhow::Result<Option<ServerRoom>>;

    /// Обновляет комнату, принадлежащую серверу.
    async fn update_server_room(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
        name: String,
        kind: ServerRoomKind,
    ) -> anyhow::Result<Option<ServerRoom>>;

    /// Удаляет комнату, принадлежащую серверу.
    async fn delete_server_room(&self, server_id: &Uuid, room_id: &Uuid) -> anyhow::Result<()>;

    /// Считает комнаты, принадлежащие серверу.
    async fn count_server_rooms(&self, server_id: &Uuid) -> anyhow::Result<u32>;

    /// Возвращает роли сервера в порядке отображения.
    async fn list_server_roles(&self, server_id: &Uuid) -> anyhow::Result<Vec<ServerRole>>;

    /// Заменяет все роли сервера.
    async fn replace_server_roles(
        &self,
        server_id: &Uuid,
        roles: Vec<ServerRole>,
    ) -> anyhow::Result<Vec<ServerRole>>;

    /// Возвращает все назначения ролей (user_id, role_id) для сервера.
    async fn list_server_member_roles(&self, server_id: &Uuid)
    -> anyhow::Result<Vec<(Uuid, Uuid)>>;

    /// Назначает пользовательскую роль участнику сервера. Идемпотентно.
    async fn assign_server_member_role(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        role_id: &Uuid,
        granted_by_user_id: &Uuid,
    ) -> anyhow::Result<()>;

    /// Отзывает пользовательскую роль у участника сервера.
    async fn revoke_server_member_role(
        &self,
        server_id: &Uuid,
        user_id: &Uuid,
        role_id: &Uuid,
    ) -> anyhow::Result<()>;
}
