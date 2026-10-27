//! Реактивный реестр метаданных серверов, ролей и прав текущего пользователя.

use cheenhub_contracts::realtime::{
    ServerMemberEntry, ServerRoleEntry, ServerRoleKind, ServerRoleSummary,
};
use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

mod author_roles;
mod permissions;
mod role_events;
use author_roles::AuthorRoleCache;
pub(crate) use permissions::ServerPermissions;

/// Доступ к единому реестру в рамках текущей аутентифицированной оболочки.
#[derive(Clone, Copy)]
pub(crate) struct ServerRegistry {
    data: Signal<RegistryData>,
    author_roles: Signal<AuthorRoleCache>,
}

/// Создаёт локальный реактивный реестр для компонентов приложения.
pub(crate) fn use_server_registry() -> ServerRegistry {
    let registry = ServerRegistry {
        data: use_signal(RegistryData::default),
        author_roles: use_signal(AuthorRoleCache::default),
    };
    role_events::use_author_role_events(registry);
    registry
}

impl ServerRegistry {
    /// Возвращает актуальные роли для выбора доступа к комнате.
    pub(crate) fn roles(self, server_id: &str) -> Vec<ServerRoleSummary> {
        self.data
            .read()
            .servers
            .iter()
            .find(|server| server.id == server_id)
            .map(|server| {
                server
                    .roles
                    .iter()
                    .filter(|role| role.kind != ServerRoleKind::Owner)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Цвет ника определяется палитрой сервера и отдельно загруженными ролями автора.
    pub(crate) fn role_color(self, server_id: &str, author_user_id: &str) -> Option<String> {
        self.data
            .read()
            .role_color(server_id, author_user_id, &self.author_roles.read())
    }

    /// Поколение защищает кэш от старого ответа после изменения ролей.
    pub(crate) fn author_role_generation(self, server_id: &str) -> u64 {
        self.author_roles.read().generation(server_id)
    }

    /// Применяет адресную карту ролей только для авторов загруженной страницы.
    pub(crate) fn absorb_author_roles(
        mut self,
        server_id: &str,
        generation: u64,
        author_ids: &[String],
        assignments: &std::collections::HashMap<String, Vec<String>>,
    ) -> bool {
        self.author_roles
            .write()
            .absorb_page(server_id, generation, author_ids, assignments)
    }

    /// Авторы, отсутствующие в положительном или отрицательном кэше.
    pub(crate) fn unresolved_authors(self, server_id: &str, authors: &[String]) -> Vec<String> {
        self.author_roles.peek().unresolved(server_id, authors)
    }

    /// Отбрасывает устаревшие сведения об одном авторе.
    pub(crate) fn invalidate_author_roles(mut self, server_id: &str, user_id: &str) {
        self.author_roles
            .write()
            .invalidate_user(server_id, user_id);
    }

    /// Сбрасывает адресные назначения после изменения палитры или состава сервера.
    pub(crate) fn invalidate_server_author_roles(mut self, server_id: &str) {
        self.author_roles.write().invalidate_server(server_id);
    }

    /// Обновляет подтверждённые назначения ролей текущего пользователя.
    pub(crate) fn set_member_roles(mut self, server_id: &str, role_ids: Vec<String>) {
        if let Some(server) = self
            .data
            .write()
            .servers
            .iter_mut()
            .find(|server| server.id == server_id)
        {
            server.member_role_ids = role_ids;
            debug!(%server_id, "updated current user roles in client server registry");
        }
    }
    /// Согласует только уже закэшированных авторов со свежим списком участников.
    pub(crate) fn set_members(mut self, server_id: &str, members: &[ServerMemberEntry]) {
        let data = self.data.read();
        let Some(server) = data.servers.iter().find(|server| server.id == server_id) else {
            return;
        };
        let mut cache = self.author_roles.write();
        for member in members {
            if cache.roles(server_id, &member.user_id).is_some() {
                cache.set_user(
                    server_id,
                    &member.user_id,
                    with_default_roles(server, &member.role_ids, member.is_owner),
                );
            }
        }
        debug!(%server_id, member_count = members.len(), "refreshed known author roles");
    }

    /// Обновляет назначения одного пользователя после действия модератора.
    pub(crate) fn set_user_roles(
        mut self,
        server_id: &str,
        user_id: &str,
        role_ids: &[String],
        is_owner: bool,
    ) {
        let data = self.data.read();
        if let Some(server) = data.servers.iter().find(|server| server.id == server_id) {
            let ids = with_default_roles(server, role_ids, is_owner);
            self.author_roles.write().set_user(server_id, user_id, ids);
            debug!(%server_id, %user_id, "updated author roles in client cache");
        }
    }

    /// Немедленно скрывает цвет ника исключённого или вышедшего участника.
    pub(crate) fn remove_user_roles(mut self, server_id: &str, user_id: &str) {
        self.author_roles
            .write()
            .invalidate_user(server_id, user_id);
        debug!(%server_id, %user_id, "invalidated departed member author roles");
    }

    /// Читает актуальный список серверов с реактивной подпиской.
    pub(crate) fn list(self) -> Vec<ServerSummary> {
        self.data.read().servers.clone()
    }

    /// Возвращает имя сервера без копирования всего реестра.
    pub(crate) fn name(self, server_id: &str) -> Option<String> {
        self.data.read().name(server_id)
    }

    /// Возвращает актуальные права пользователя на указанном сервере.
    pub(crate) fn permissions(self, server_id: &str) -> ServerPermissions {
        self.data
            .read()
            .servers
            .iter()
            .find(|server| server.id == server_id)
            .map(ServerPermissions::from_server)
            .unwrap_or_default()
    }

    /// Заменяет реестр после успешной загрузки серверов.
    pub(crate) fn replace(mut self, servers: Vec<ServerSummary>) {
        debug!(
            server_count = servers.len(),
            "replaced client server registry"
        );
        let mut data = self.data.write();
        for old in &data.servers {
            if servers
                .iter()
                .find(|new| new.id == old.id)
                .is_none_or(|new| new.roles != old.roles)
            {
                self.author_roles.write().invalidate_server(&old.id);
            }
        }
        data.servers = servers;
    }

    /// Добавляет сервер или заменяет его сводку после подтверждённого изменения.
    pub(crate) fn upsert(mut self, server: ServerSummary) {
        debug!(server_id = %server.id, "updated client server registry entry");
        let mut data = self.data.write();
        if let Some(saved) = data.servers.iter_mut().find(|saved| saved.id == server.id) {
            if saved.roles != server.roles {
                self.author_roles.write().invalidate_server(&server.id);
            }
            *saved = server;
        } else {
            data.servers.push(server);
        }
    }

    /// Удаляет сервер после выхода пользователя из него.
    pub(crate) fn remove(mut self, server_id: &str) {
        debug!(%server_id, "removed client server registry entry");
        self.data
            .write()
            .servers
            .retain(|server| server.id != server_id);
        self.author_roles.write().invalidate_server(server_id);
    }

    /// Обновляет подтверждённые роли, сохраняя остальные метаданные сервера.
    pub(crate) fn set_roles(mut self, server_id: &str, roles: &[ServerRoleEntry]) {
        let roles = roles
            .iter()
            .map(|role| ServerRoleSummary {
                role_id: role.role_id.clone(),
                name: role.name.clone(),
                color: role.color.clone(),
                kind: role.kind,
                permissions: role.permissions.clone(),
            })
            .collect();
        if self.data.write().update_roles(server_id, roles) {
            self.author_roles.write().invalidate_server(server_id);
            debug!(%server_id, "updated roles and invalidated cached author assignments");
        } else {
            warn!(%server_id, "cannot update roles for unavailable client server registry entry");
        }
    }
}

fn with_default_roles(server: &ServerSummary, assigned: &[String], is_owner: bool) -> Vec<String> {
    let mut ids = assigned.to_vec();
    for role in &server.roles {
        if (role.kind == ServerRoleKind::Member || (is_owner && role.kind == ServerRoleKind::Owner))
            && !ids.contains(&role.role_id)
        {
            ids.push(role.role_id.clone());
        }
    }
    ids
}

#[derive(Default)]
struct RegistryData {
    servers: Vec<ServerSummary>,
}

impl RegistryData {
    fn role_color(
        &self,
        server_id: &str,
        author_user_id: &str,
        author_roles: &AuthorRoleCache,
    ) -> Option<String> {
        let server = self.servers.iter().find(|server| server.id == server_id)?;
        if author_user_id == server_id {
            return None;
        }
        let assigned = author_roles.roles(server_id, author_user_id)?;
        server
            .roles
            .iter()
            .find(|role| assigned.contains(&role.role_id))
            .map(|role| role.color.clone())
    }

    fn name(&self, server_id: &str) -> Option<String> {
        self.servers
            .iter()
            .find(|server| server.id == server_id)
            .map(|server| server.name.clone())
    }

    fn update_roles(&mut self, server_id: &str, roles: Vec<ServerRoleSummary>) -> bool {
        let Some(server) = self
            .servers
            .iter_mut()
            .find(|server| server.id == server_id)
        else {
            return false;
        };
        server
            .member_role_ids
            .retain(|id| roles.iter().any(|role| &role.role_id == id));
        server.roles = roles;
        true
    }
}

#[cfg(test)]
mod tests {
    use cheenhub_contracts::realtime::{ServerRoleKind, ServerRolePermission};

    use super::*;

    fn role(id: &str) -> ServerRoleSummary {
        ServerRoleSummary {
            role_id: id.to_owned(),
            name: id.to_owned(),
            color: "#94a3b8".to_owned(),
            kind: ServerRoleKind::Custom,
            permissions: vec![ServerRolePermission::ManageRooms],
        }
    }

    fn server(id: &str) -> ServerSummary {
        ServerSummary {
            id: id.to_owned(),
            name: "Сервер".to_owned(),
            avatar_url: Some("/avatar.png".to_owned()),
            is_owner: false,
            is_member: true,
            roles: vec![role("old")],
            member_role_ids: vec!["old".to_owned()],
        }
    }

    #[test]
    fn server_name_lookup_returns_only_the_requested_name() {
        let mut first = server("one");
        first.name = "Первый".to_owned();
        let mut second = server("two");
        second.name = "Второй".to_owned();
        let registry = RegistryData {
            servers: vec![first, second],
        };

        assert_eq!(registry.name("two"), Some("Второй".to_owned()));
        assert_eq!(registry.name("missing"), None);
    }

    #[test]
    fn saved_roles_replace_stale_roles_without_changing_other_servers_or_metadata() {
        let original = server("one");
        let other = server("two");
        let mut registry = RegistryData {
            servers: vec![original.clone(), other.clone()],
        };
        registry.update_roles("one", vec![role("old"), role("new")]);
        assert_eq!(registry.servers[0].roles, vec![role("old"), role("new")]);
        assert_eq!(registry.servers[0].name, original.name);
        assert_eq!(registry.servers[0].avatar_url, original.avatar_url);
        assert_eq!(
            registry.servers[0].member_role_ids,
            original.member_role_ids
        );
        assert_eq!(registry.servers[1], other);
    }

    #[test]
    fn deleted_roles_remove_current_user_assignments() {
        let mut registry = RegistryData {
            servers: vec![server("one")],
        };
        registry.update_roles("one", vec![role("new")]);
        assert!(registry.servers[0].member_role_ids.is_empty());
        assert!(!ServerPermissions::from_server(&registry.servers[0]).can_manage_rooms);
    }

    #[test]
    fn saved_permission_changes_apply_to_an_existing_assignment() {
        let mut registry = RegistryData {
            servers: vec![server("one")],
        };
        assert!(ServerPermissions::from_server(&registry.servers[0]).can_manage_rooms);
        let mut edited = role("old");
        edited.permissions.clear();
        registry.update_roles("one", vec![edited]);
        assert_eq!(registry.servers[0].member_role_ids, vec!["old"]);
        assert!(!ServerPermissions::from_server(&registry.servers[0]).can_manage_rooms);
    }

    #[test]
    fn role_response_for_an_unavailable_server_does_not_create_an_entry() {
        let mut registry = RegistryData {
            servers: vec![server("one")],
        };
        assert!(!registry.update_roles("unavailable", vec![role("new")]));
        assert_eq!(registry.servers, vec![server("one")]);
    }

    #[test]
    fn nickname_colors_use_server_role_order_and_cached_author_assignments() {
        let mut server = server("server");
        let mut owner = role("owner");
        owner.kind = ServerRoleKind::Owner;
        owner.color = "#facc15".to_owned();
        let mut higher = role("higher");
        higher.color = "#ef4444".to_owned();
        let mut lower = role("lower");
        lower.color = "#22c55e".to_owned();
        let mut member = role("member");
        member.kind = ServerRoleKind::Member;
        member.color = "#94a3b8".to_owned();
        server.roles = vec![owner, higher, lower, member];
        let registry = RegistryData {
            servers: vec![server],
        };
        let mut cache = AuthorRoleCache::default();
        cache.set_user("server", "owner-user", vec!["owner".to_owned()]);
        cache.set_user(
            "server",
            "member-user",
            vec!["lower".to_owned(), "higher".to_owned()],
        );
        cache.set_user("server", "plain-user", vec!["member".to_owned()]);

        assert_eq!(
            registry
                .role_color("server", "owner-user", &cache)
                .as_deref(),
            Some("#facc15")
        );
        assert_eq!(
            registry
                .role_color("server", "member-user", &cache)
                .as_deref(),
            Some("#ef4444")
        );
        assert_eq!(
            registry
                .role_color("server", "plain-user", &cache)
                .as_deref(),
            Some("#94a3b8")
        );
        assert_eq!(registry.role_color("server", "server", &cache), None);
        assert_eq!(registry.role_color("server", "unknown-user", &cache), None);
        assert_eq!(registry.role_color("unknown", "plain-user", &cache), None);
    }

    #[test]
    fn default_roles_include_owner_even_without_membership_row() {
        let mut server = server("server");
        let mut owner = role("owner");
        owner.kind = ServerRoleKind::Owner;
        let mut member = role("member");
        member.kind = ServerRoleKind::Member;
        server.roles = vec![owner, member];
        assert_eq!(
            with_default_roles(&server, &[], true),
            vec!["owner", "member"]
        );
        assert_eq!(with_default_roles(&server, &[], false), vec!["member"]);
    }

    #[test]
    fn removed_member_immediately_loses_cached_role_color() {
        let registry = RegistryData {
            servers: vec![server("server")],
        };
        let mut cache = AuthorRoleCache::default();
        cache.set_user("server", "removed", vec!["old".to_owned()]);
        cache.set_user("server", "remaining", vec!["old".to_owned()]);
        cache.invalidate_user("server", "removed");
        assert_eq!(registry.role_color("server", "removed", &cache), None);
        assert_eq!(
            registry
                .role_color("server", "remaining", &cache)
                .as_deref(),
            Some("#94a3b8")
        );
    }
}
