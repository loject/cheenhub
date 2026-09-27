//! Валидация входных данных сервера.

use cheenhub_contracts::rest::{ServerRoomWriteAccess, ServerRoomWriteAccessMode};
use uuid::Uuid;

use crate::features::servers::domain::ServerRoomWriteAccess as ValidatedWriteAccess;

/// Нормализованный ввод для создания сервера.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidCreateServer {
    /// Человекочитаемое имя сервера.
    pub(crate) name: String,
}

/// Проверяет и нормализует ввод для создания сервера.
pub(crate) fn create_server(name: String) -> Result<ValidCreateServer, &'static str> {
    let name = name.trim().to_owned();
    let len = name.chars().count();

    if !(2..=48).contains(&len) {
        return Err("Название сервера должно быть длиной от 2 до 48 символов.");
    }

    Ok(ValidCreateServer { name })
}

/// Нормализованный ввод для комнаты сервера.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidServerRoom {
    /// Человекочитаемое имя комнаты.
    pub(crate) name: String,
    /// Настройка доступа к записи в комнату.
    pub(crate) write_access: ValidatedWriteAccess,
}

/// Проверяет и нормализует ввод для комнаты.
pub(crate) fn server_room(
    name: String,
    write_access: ServerRoomWriteAccess,
) -> Result<ValidServerRoom, &'static str> {
    let name = name.trim().to_owned();
    let len = name.chars().count();

    if !(1..=48).contains(&len) {
        return Err("Название комнаты должно быть длиной от 1 до 48 символов.");
    }

    Ok(ValidServerRoom {
        name,
        write_access: normalized_write_access(write_access)?,
    })
}

/// Нормализует настройку доступа к записи: вне режима выбора ролей список очищается, дубликаты убираются.
pub(crate) fn normalized_write_access(
    write_access: ServerRoomWriteAccess,
) -> Result<ValidatedWriteAccess, &'static str> {
    if write_access.mode != ServerRoomWriteAccessMode::SelectedRoles {
        return Ok(ValidatedWriteAccess::all_members());
    }

    let mut role_ids: Vec<Uuid> = Vec::new();
    for role_id in write_access.role_ids {
        let role_id = Uuid::parse_str(&role_id)
            .map_err(|_| "Выбрана роль, которая не принадлежит этому серверу.")?;
        if !role_ids.contains(&role_id) {
            role_ids.push(role_id);
        }
    }

    Ok(ValidatedWriteAccess {
        mode: ServerRoomWriteAccessMode::SelectedRoles,
        role_ids,
    })
}

/// Нормализованный ввод для создания приглашения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValidCreateServerInvite {
    /// Необязательный максимальный лимит использований приглашения.
    pub(crate) max_uses: Option<u32>,
    /// Необязательный срок жизни приглашения в днях.
    pub(crate) expires_in_days: Option<u32>,
}

/// Проверяет ввод для создания приглашения сервера.
pub(crate) fn create_server_invite(
    max_uses: Option<u32>,
    expires_in_days: Option<u32>,
) -> Result<ValidCreateServerInvite, &'static str> {
    if matches!(max_uses, Some(0 | 1000..)) {
        return Err("Лимит использований должен быть от 1 до 999.");
    }

    if matches!(expires_in_days, Some(0 | 366..)) {
        return Err("Срок действия должен быть от 1 до 365 дней.");
    }

    Ok(ValidCreateServerInvite {
        max_uses,
        expires_in_days,
    })
}

#[cfg(test)]
mod tests {
    use cheenhub_contracts::rest::{ServerRoomWriteAccess, ServerRoomWriteAccessMode};
    use uuid::Uuid;

    use super::{ValidatedWriteAccess, create_server};

    #[test]
    fn trims_valid_server_name() {
        let valid = create_server("  CheenHub Dev  ".to_owned()).expect("name should be valid");

        assert_eq!(valid.name, "CheenHub Dev");
    }

    #[test]
    fn rejects_empty_server_name() {
        assert!(create_server("   ".to_owned()).is_err());
    }

    #[test]
    fn rejects_short_server_name() {
        assert!(create_server("a".to_owned()).is_err());
    }

    #[test]
    fn rejects_long_server_name() {
        assert!(create_server("a".repeat(49)).is_err());
    }

    #[test]
    fn trims_valid_room_name() {
        let valid = super::server_room("  x  ".to_owned(), ServerRoomWriteAccess::default())
            .expect("room name should be valid");

        assert_eq!(valid.name, "x");
    }

    #[test]
    fn rejects_empty_room_name() {
        assert!(super::server_room("   ".to_owned(), ServerRoomWriteAccess::default()).is_err());
    }

    #[test]
    fn rejects_long_room_name() {
        assert!(super::server_room("a".repeat(49), ServerRoomWriteAccess::default()).is_err());
    }

    #[test]
    fn room_write_access_defaults_to_all_members() {
        let valid = super::server_room("общий".to_owned(), ServerRoomWriteAccess::default())
            .expect("room should be valid");

        assert_eq!(valid.write_access, ValidatedWriteAccess::all_members());
    }

    #[test]
    fn clears_room_write_roles_in_all_members_mode() {
        let valid = super::server_room(
            "общий".to_owned(),
            ServerRoomWriteAccess {
                mode: ServerRoomWriteAccessMode::AllMembers,
                role_ids: vec![Uuid::new_v4().to_string()],
            },
        )
        .expect("room should be valid");

        assert_eq!(valid.write_access, ValidatedWriteAccess::all_members());
    }

    #[test]
    fn rejects_malformed_room_write_role() {
        let result = super::server_room(
            "курилка".to_owned(),
            ServerRoomWriteAccess {
                mode: ServerRoomWriteAccessMode::SelectedRoles,
                role_ids: vec!["не-uuid".to_owned()],
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn deduplicates_room_write_roles() {
        let role_id = Uuid::new_v4();
        let valid = super::server_room(
            "курилка".to_owned(),
            ServerRoomWriteAccess {
                mode: ServerRoomWriteAccessMode::SelectedRoles,
                role_ids: vec![role_id.to_string(), role_id.to_string()],
            },
        )
        .expect("room should be valid");

        assert_eq!(
            valid.write_access.mode,
            ServerRoomWriteAccessMode::SelectedRoles
        );
        assert_eq!(valid.write_access.role_ids, vec![role_id]);
    }

    #[test]
    fn accepts_valid_invite_settings() {
        let valid = super::create_server_invite(Some(30), Some(7))
            .expect("invite settings should be valid");

        assert_eq!(valid.max_uses, Some(30));
        assert_eq!(valid.expires_in_days, Some(7));
    }

    #[test]
    fn rejects_invalid_invite_usage_limit() {
        assert!(super::create_server_invite(Some(0), None).is_err());
        assert!(super::create_server_invite(Some(1000), None).is_err());
    }

    #[test]
    fn rejects_invalid_invite_expiration() {
        assert!(super::create_server_invite(None, Some(0)).is_err());
        assert!(super::create_server_invite(None, Some(366)).is_err());
    }
}
