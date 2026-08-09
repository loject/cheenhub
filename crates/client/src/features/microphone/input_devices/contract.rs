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
pub(crate) enum AudioInputDevicesResult {
    /// API перечисления устройств недоступен.
    NotSupported,
    /// Выбор конкретного устройства выполняет системная audio policy.
    SystemManaged,
    /// Устройства есть, но подписи скрыты до выдачи разрешения.
    PermissionRequired,
    /// Пользователь запретил доступ к микрофону.
    PermissionDenied,
    /// Устройства ввода аудио не найдены.
    NoDevices,
    /// Доступен список устройств ввода аудио.
    Available(Vec<AudioInputDevice>),
}
