//! Заглушка автоматического запуска для неподдерживаемых платформ.

/// Сообщает, доступно ли управление автоматическим запуском.
pub(crate) const fn is_supported() -> bool {
    false
}

/// Возвращает выключенное состояние на неподдерживаемой платформе.
pub(crate) fn is_enabled() -> Result<bool, String> {
    Ok(false)
}

/// Отклоняет изменение автоматического запуска на неподдерживаемой платформе.
pub(crate) fn set_enabled(_enabled: bool) -> Result<(), String> {
    Err("Автозапуск CheenHub поддерживается только в Windows.".to_owned())
}

/// Возвращает отсутствие скрытого системного запуска.
pub(crate) fn started_hidden() -> bool {
    false
}
