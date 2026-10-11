//! Разделяет восстановление зарегистрированного действия и явное открытие системного UI.

/// Выбранная операция до BindShortcuts с учётом сохранённого назначения.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Plan {
    /// Нет подтверждённого назначения, автоматический bind запрещён.
    BindingRequired,
    /// Повторный bind подключает сохранённое системное действие к новой сессии.
    Restore,
    /// Пользователь явно разрешил первый BindShortcuts с выбором.
    FirstAssignment,
    /// Пользователь явно открыл ConfigureShortcuts v2.
    Reassign,
    /// v1 не предоставляет UI повторного назначения.
    SettingsRequired,
}

impl Plan {
    /// Только регистрация или подключение действия требуют BindShortcuts.
    pub(super) fn requires_bind(&self) -> bool {
        matches!(self, Self::Restore | Self::FirstAssignment | Self::Reassign)
    }
}

/// Учитывает реальный ListShortcuts и версию API, не доверяя одному локальному LABEL.
pub(super) fn plan(assigning: bool, listed: bool, saved: bool, version: u32) -> Plan {
    if !assigning {
        return if listed || saved {
            Plan::Restore
        } else {
            Plan::BindingRequired
        };
    }
    if listed || saved {
        if version >= 2 {
            Plan::Reassign
        } else {
            Plan::SettingsRequired
        }
    } else {
        Plan::FirstAssignment
    }
}

#[cfg(test)]
mod tests;
