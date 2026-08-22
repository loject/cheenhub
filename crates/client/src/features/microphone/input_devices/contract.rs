//! Общие типы перечисления устройств ввода аудио.

/// Одно устройство ввода аудио.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AudioInputDevice {
    /// Идентификатор устройства ввода.
    pub(crate) device_id: String,
    /// Отображаемое имя устройства ввода.
    pub(crate) label: String,
}

/// Результат перечисления устройств ввода аудио.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AudioInputDevicesResult {
    /// `None`, если API перечисления устройств недоступен.
    ///
    /// Пустой список означает, что API доступен, но устройства не найдены.
    pub(crate) devices: Option<Vec<AudioInputDevice>>,
    /// Выбор конкретного устройства выполняет системная audio policy.
    pub(crate) system_managed: bool,
    /// Подписи устройств скрыты до выдачи разрешения.
    pub(crate) permission_required: bool,
    /// Пользователь запретил доступ к микрофону.
    pub(crate) permission_denied: bool,
}
