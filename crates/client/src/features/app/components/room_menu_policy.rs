//! Доступность команд меню с учётом прав и актуального состояния списка.

/// Команды управления комнатами, доступные в контекстном меню списка.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RoomMenuCommand {
    /// Создать комнату на свободном месте списка.
    Create,
    /// Изменить выбранную комнату.
    Edit,
    /// Запросить подтверждение удаления выбранной комнаты.
    Delete,
}

/// Возвращает команды только для доступного и актуального контекста.
///
/// `room_exists` относится к текущему списку, а не снимку на момент открытия меню.
/// Во время загрузки или удаления выбранной комнаты команды недоступны.
pub(super) fn available_commands(
    can_manage: bool,
    loading: bool,
    room_target: bool,
    room_exists: bool,
    deleting: bool,
) -> &'static [RoomMenuCommand] {
    if !can_manage || loading {
        return &[];
    }
    if !room_target {
        return &[RoomMenuCommand::Create];
    }
    if !room_exists || deleting {
        return &[];
    }
    &[RoomMenuCommand::Edit, RoomMenuCommand::Delete]
}

#[cfg(test)]
mod tests;
