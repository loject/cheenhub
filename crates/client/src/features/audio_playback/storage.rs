//! Хранилище постоянных предпочтений воспроизведения аудио.

use dioxus::prelude::{info, warn};
use dioxus_sdk_storage::{LocalStorage, StorageBacking};

const OUTPUT_DEVICE_ID_KEY: &str = "cheenhub.audio_playback.output_device_id";
const OUTPUT_DEVICE_LABEL_KEY: &str = "cheenhub.audio_playback.output_device_label";
const OUTPUT_VOLUME_PERCENT_KEY: &str = "cheenhub.audio_playback.output_volume_percent";
const JITTER_BUFFER_MS_KEY: &str = "cheenhub.audio_playback.jitter_buffer_ms";
const DEFAULT_OUTPUT_VOLUME_PERCENT: u32 = 100;
/// Задержка джиттер-буфера для входящего голоса по умолчанию, в микросекундах.
pub(crate) const DEFAULT_JITTER_BUFFER_US: u32 = 10_000;
/// Минимальная задержка джиттер-буфера для входящего голоса, в микросекундах.
pub(crate) const MIN_JITTER_BUFFER_US: u32 = 500;
/// Максимальная задержка джиттер-буфера для входящего голоса, в микросекундах.
pub(crate) const MAX_JITTER_BUFFER_US: u32 = 400_000;

/// Сохраненное предпочтение устройства аудиовывода.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredOutputDevice {
    /// Идентификатор устройства вывода для browser/native backend.
    pub(crate) device_id: String,
    /// Метка устройства для восстановления preference, когда platform backend меняет идентификаторы.
    pub(crate) label: Option<String>,
}

/// Загружает предпочтительное устройство аудиовывода из локального хранилища.
pub(crate) fn load_output_device() -> Option<StoredOutputDevice> {
    let device_id = get::<LocalStorage>(OUTPUT_DEVICE_ID_KEY).filter(|id| !id.is_empty())?;
    let label = get::<LocalStorage>(OUTPUT_DEVICE_LABEL_KEY).filter(|label| !label.is_empty());
    info!(
        has_label = label.is_some(),
        "loaded audio output device preference"
    );

    Some(StoredOutputDevice { device_id, label })
}

/// Сохраняет предпочтительное устройство аудиовывода в локальное хранилище.
pub(crate) fn save_output_device(device_id: &str, label: Option<&str>) {
    if device_id.is_empty() {
        clear_output_device();
        return;
    }

    set::<LocalStorage>(OUTPUT_DEVICE_ID_KEY, device_id);
    match label.filter(|label| !label.is_empty()) {
        Some(label) => set::<LocalStorage>(OUTPUT_DEVICE_LABEL_KEY, label),
        None => remove::<LocalStorage>(OUTPUT_DEVICE_LABEL_KEY),
    }
    info!(
        has_label = label.is_some_and(|label| !label.is_empty()),
        "saved audio output device preference"
    );
}

/// Очищает предпочтительное устройство аудиовывода из локального хранилища.
pub(crate) fn clear_output_device() {
    remove::<LocalStorage>(OUTPUT_DEVICE_ID_KEY);
    remove::<LocalStorage>(OUTPUT_DEVICE_LABEL_KEY);
}

/// Загружает предпочтительный процент громкости аудиовывода.
pub(crate) fn load_output_volume_percent() -> u32 {
    let volume = get::<LocalStorage>(OUTPUT_VOLUME_PERCENT_KEY)
        .and_then(|value| value.parse::<u32>().ok())
        .map(clamp_volume_percent)
        .unwrap_or(DEFAULT_OUTPUT_VOLUME_PERCENT);
    info!(volume, "loaded audio output volume preference");
    volume
}

/// Сохраняет предпочтительный процент громкости аудиовывода.
pub(crate) fn save_output_volume_percent(volume_percent: u32) {
    let volume_percent = clamp_volume_percent(volume_percent);
    set::<LocalStorage>(OUTPUT_VOLUME_PERCENT_KEY, &volume_percent.to_string());
    info!(
        volume = volume_percent,
        "saved audio output volume preference"
    );
}

/// Загружает предпочтительную задержку jitter buffer входящего голоса в микросекундах.
pub(crate) fn load_jitter_buffer_us() -> u32 {
    let buffer_us = get::<LocalStorage>(JITTER_BUFFER_MS_KEY)
        .and_then(|value| parse_jitter_buffer_us(&value))
        .map(clamp_jitter_buffer_us)
        .unwrap_or(DEFAULT_JITTER_BUFFER_US);
    info!(buffer_us, "loaded inbound voice jitter buffer preference");
    buffer_us
}

/// Сохраняет предпочтительную задержку jitter buffer входящего голоса в микросекундах.
pub(crate) fn save_jitter_buffer_us(buffer_us: u32) {
    let buffer_us = clamp_jitter_buffer_us(buffer_us);
    let buffer_ms = f64::from(buffer_us) / 1_000.0;
    set::<LocalStorage>(JITTER_BUFFER_MS_KEY, &buffer_ms.to_string());
    info!(buffer_us, "saved inbound voice jitter buffer preference");
}

/// Ограничивает задержку jitter buffer входящего голоса поддерживаемым диапазоном.
pub(crate) fn clamp_jitter_buffer_us(buffer_us: u32) -> u32 {
    buffer_us.clamp(MIN_JITTER_BUFFER_US, MAX_JITTER_BUFFER_US)
}

fn parse_jitter_buffer_us(value: &str) -> Option<u32> {
    value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .map(|buffer_ms| (buffer_ms * 1_000.0).round() as u32)
}

fn clamp_volume_percent(volume_percent: u32) -> u32 {
    volume_percent.min(200)
}

fn get<S>(key: &str) -> Option<String>
where
    S: StorageBacking<Key = String>,
{
    S::get::<Option<String>>(&key.to_owned()).flatten()
}

fn set<S>(key: &str, value: &str)
where
    S: StorageBacking<Key = String>,
{
    S::set(key.to_owned(), &Some(value.to_owned()));
    if get::<S>(key).as_deref() != Some(value) {
        warn!("failed to verify audio output preference storage write");
    }
}

fn remove<S>(key: &str)
where
    S: StorageBacking<Key = String>,
{
    S::set(key.to_owned(), &Option::<String>::None);
}

#[cfg(test)]
mod tests {
    use super::parse_jitter_buffer_us;

    #[test]
    fn reads_legacy_integer_milliseconds_as_microseconds() {
        assert_eq!(parse_jitter_buffer_us("120"), Some(120_000));
    }

    #[test]
    fn reads_fractional_milliseconds_as_microseconds() {
        assert_eq!(parse_jitter_buffer_us("0.5"), Some(500));
    }
}
