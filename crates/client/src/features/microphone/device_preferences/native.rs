//! Выбор правил хранения предпочтений аудиоустройства.

#[cfg(all(target_os = "linux", feature = "linux"))]
#[path = "linux.rs"]
mod linux;
#[cfg(all(target_os = "linux", feature = "linux"))]
pub(crate) use linux::{DEVICE_ID_KEY, DEVICE_LABEL_KEY, RECOVER_BY_LABEL};

#[cfg(not(all(target_os = "linux", feature = "linux")))]
/// Ключ идентификатора устройства прежнего платформенного backend.
pub(crate) const DEVICE_ID_KEY: &str = "cheenhub.microphone.input_device_id";
#[cfg(not(all(target_os = "linux", feature = "linux")))]
/// Ключ отображаемого имени устройства.
pub(crate) const DEVICE_LABEL_KEY: &str = "cheenhub.microphone.input_device_label";
#[cfg(not(all(target_os = "linux", feature = "linux")))]
/// Разрешает прежнее восстановление по имени на остальных платформах.
pub(crate) const RECOVER_BY_LABEL: bool = true;
