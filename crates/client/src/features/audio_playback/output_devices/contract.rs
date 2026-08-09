//! Общие типы перечисления устройств вывода аудио.

/// Одно устройство вывода аудио.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AudioOutputDevice {
    /// Идентификатор устройства вывода.
    pub(crate) device_id: String,
    /// Отображаемое имя устройства вывода.
    pub(crate) label: String,
}

/// Результат перечисления устройств вывода аудио.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AudioOutputDevicesResult {
    /// API перечисления устройств недоступен.
    NotSupported,
    /// Выбор конкретного устройства выполняет системная audio policy.
    SystemManaged,
    /// Устройства есть, но подписи скрыты до выдачи разрешения.
    PermissionRequired,
    /// Устройства вывода аудио не найдены.
    NoDevices,
    /// Доступен список устройств вывода аудио.
    Available(Vec<AudioOutputDevice>),
}
