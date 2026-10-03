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
mod tests;
