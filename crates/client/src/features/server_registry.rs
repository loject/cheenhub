//! Реактивный реестр метаданных серверов, ролей и прав текущего пользователя.

use cheenhub_contracts::realtime::{ServerRoleEntry, ServerRoleKind, ServerRoleSummary};
use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

mod permissions;
pub(crate) use permissions::ServerPermissions;

/// Доступ к единому реестру в рамках текущей аутентифицированной оболочки.
#[derive(Clone, Copy)]
pub(crate) struct ServerRegistry {
    data: Signal<RegistryData>,
}

/// Создаёт локальный реактивный реестр для компонентов приложения.
pub(crate) fn use_server_registry() -> ServerRegistry {
    ServerRegistry {
        data: use_signal(RegistryData::default),
    }
}

impl ServerRegistry {
    /// Возвращает актуальные роли для выбора доступа к комнате.
    pub(crate) fn roles(self, server_id: &str) -> Vec<ServerRoleSummary> {
        self.data
            .read()
            .servers
            .iter()
            .find(|server| server.id == server_id)
            .map(|server| server.roles.clone())
            .unwrap_or_default()
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
    /// Читает актуальный список серверов с реактивной подпиской.
    pub(crate) fn list(self) -> Vec<ServerSummary> {
        self.data.read().servers.clone()
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
        self.data.write().servers = servers;
    }

    /// Добавляет сервер или заменяет его сводку после подтверждённого изменения.
    pub(crate) fn upsert(mut self, server: ServerSummary) {
        debug!(server_id = %server.id, "updated client server registry entry");
        let mut data = self.data.write();
        if let Some(saved) = data.servers.iter_mut().find(|saved| saved.id == server.id) {
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
    }

    /// Обновляет подтверждённые роли, сохраняя остальные метаданные сервера.
    pub(crate) fn set_roles(mut self, server_id: &str, roles: &[ServerRoleEntry]) {
        let roles = roles
            .iter()
            .filter(|role| role.kind != ServerRoleKind::Owner)
            .map(|role| ServerRoleSummary {
                role_id: role.role_id.clone(),
                name: role.name.clone(),
                color: role.color.clone(),
                kind: role.kind,
                permissions: role.permissions.clone(),
            })
            .collect();
        if self.data.write().update_roles(server_id, roles) {
            debug!(%server_id, "updated roles in client server registry");
        } else {
            warn!(%server_id, "cannot update roles for unavailable client server registry entry");
        }
    }
}

#[derive(Default)]
struct RegistryData {
    servers: Vec<ServerSummary>,
}

impl RegistryData {
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
}
