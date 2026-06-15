//! Persistent audio playback preference storage.

use dioxus::prelude::{info, warn};
use dioxus_sdk_storage::{LocalStorage, StorageBacking};

const OUTPUT_DEVICE_ID_KEY: &str = "cheenhub.audio_playback.output_device_id";
const OUTPUT_DEVICE_LABEL_KEY: &str = "cheenhub.audio_playback.output_device_label";
const OUTPUT_VOLUME_PERCENT_KEY: &str = "cheenhub.audio_playback.output_volume_percent";
const JITTER_BUFFER_MS_KEY: &str = "cheenhub.audio_playback.jitter_buffer_ms";
const DEFAULT_OUTPUT_VOLUME_PERCENT: u32 = 100;
/// Default inbound voice jitter buffer delay in milliseconds.
pub(crate) const DEFAULT_JITTER_BUFFER_MS: u32 = 120;
/// Minimum inbound voice jitter buffer delay in milliseconds.
pub(crate) const MIN_JITTER_BUFFER_MS: u32 = 40;
/// Maximum inbound voice jitter buffer delay in milliseconds.
pub(crate) const MAX_JITTER_BUFFER_MS: u32 = 400;

/// Stored audio output device preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredOutputDevice {
    /// Browser sink/device ID to pass into `AudioContext.setSinkId`.
    pub(crate) device_id: String,
    /// Browser-provided device label used to recover when device IDs rotate.
    pub(crate) label: Option<String>,
}

/// Loads the preferred audio output device from local storage.
pub(crate) fn load_output_device() -> Option<StoredOutputDevice> {
    let device_id = get::<LocalStorage>(OUTPUT_DEVICE_ID_KEY).filter(|id| !id.is_empty())?;
    let label = get::<LocalStorage>(OUTPUT_DEVICE_LABEL_KEY).filter(|label| !label.is_empty());
    info!(
        has_label = label.is_some(),
        "loaded audio output device preference"
    );

    Some(StoredOutputDevice { device_id, label })
}

/// Saves the preferred audio output device into local storage.
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

/// Clears the preferred audio output device from local storage.
pub(crate) fn clear_output_device() {
    remove::<LocalStorage>(OUTPUT_DEVICE_ID_KEY);
    remove::<LocalStorage>(OUTPUT_DEVICE_LABEL_KEY);
}

/// Loads the preferred audio output volume percentage.
pub(crate) fn load_output_volume_percent() -> u32 {
    let volume = get::<LocalStorage>(OUTPUT_VOLUME_PERCENT_KEY)
        .and_then(|value| value.parse::<u32>().ok())
        .map(clamp_volume_percent)
        .unwrap_or(DEFAULT_OUTPUT_VOLUME_PERCENT);
    info!(volume, "loaded audio output volume preference");
    volume
}

/// Saves the preferred audio output volume percentage.
pub(crate) fn save_output_volume_percent(volume_percent: u32) {
    let volume_percent = clamp_volume_percent(volume_percent);
    set::<LocalStorage>(OUTPUT_VOLUME_PERCENT_KEY, &volume_percent.to_string());
    info!(
        volume = volume_percent,
        "saved audio output volume preference"
    );
}

/// Loads the preferred inbound voice jitter buffer delay.
pub(crate) fn load_jitter_buffer_ms() -> u32 {
    let buffer_ms = get::<LocalStorage>(JITTER_BUFFER_MS_KEY)
        .and_then(|value| value.parse::<u32>().ok())
        .map(clamp_jitter_buffer_ms)
        .unwrap_or(DEFAULT_JITTER_BUFFER_MS);
    info!(buffer_ms, "loaded inbound voice jitter buffer preference");
    buffer_ms
}

/// Saves the preferred inbound voice jitter buffer delay.
pub(crate) fn save_jitter_buffer_ms(buffer_ms: u32) {
    let buffer_ms = clamp_jitter_buffer_ms(buffer_ms);
    set::<LocalStorage>(JITTER_BUFFER_MS_KEY, &buffer_ms.to_string());
    info!(buffer_ms, "saved inbound voice jitter buffer preference");
}

/// Clamps an inbound voice jitter buffer delay to the supported range.
pub(crate) fn clamp_jitter_buffer_ms(buffer_ms: u32) -> u32 {
    buffer_ms.clamp(MIN_JITTER_BUFFER_MS, MAX_JITTER_BUFFER_MS)
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
