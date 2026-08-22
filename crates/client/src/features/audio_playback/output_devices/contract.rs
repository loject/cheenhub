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
pub(crate) struct AudioOutputDevicesResult {
    /// `None`, если API перечисления устройств недоступен.
    ///
    /// Пустой список означает, что API доступен, но устройства не найдены.
    pub(crate) devices: Option<Vec<AudioOutputDevice>>,
    /// Выбор конкретного устройства выполняет системная audio policy.
    pub(crate) system_managed: bool,
    /// Подписи устройств скрыты до выдачи разрешения.
    pub(crate) permission_required: bool,
}
