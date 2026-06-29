//! Persistent microphone preference storage.

use dioxus::prelude::{info, warn};
use dioxus_sdk_storage::{LocalStorage, StorageBacking};

use super::backend::MicrophoneActivationMode;

const INPUT_DEVICE_ID_KEY: &str = "cheenhub.microphone.input_device_id";
const INPUT_DEVICE_LABEL_KEY: &str = "cheenhub.microphone.input_device_label";
const INPUT_VOLUME_PERCENT_KEY: &str = "cheenhub.microphone.input_volume_percent";
const ACTIVATION_MODE_KEY: &str = "cheenhub.microphone.activation_mode";
const VAD_THRESHOLD_PERCENT_KEY: &str = "cheenhub.microphone.vad_threshold_percent";
const DEFAULT_INPUT_VOLUME_PERCENT: u32 = 100;
const DEFAULT_VAD_THRESHOLD_PERCENT: u32 = 20;

/// Stored microphone input device preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredInputDevice {
    /// Идентификатор устройства ввода для browser/native backend.
    pub(crate) device_id: String,
    /// Метка устройства для восстановления preference, когда platform backend меняет идентификаторы.
    pub(crate) label: Option<String>,
}

/// Loads the preferred microphone input device from local storage.
pub(crate) fn load_input_device() -> Option<StoredInputDevice> {
    let device_id = load_input_device_id()?;
    let label = get::<LocalStorage>(INPUT_DEVICE_LABEL_KEY).filter(|label| !label.is_empty());
    info!(
        has_label = label.is_some(),
        "loaded microphone input device preference"
    );

    Some(StoredInputDevice { device_id, label })
}

/// Loads the preferred microphone input device ID from local storage.
fn load_input_device_id() -> Option<String> {
    get::<LocalStorage>(INPUT_DEVICE_ID_KEY).filter(|device_id| !device_id.is_empty())
}

/// Saves the preferred microphone input device into local storage.
pub(crate) fn save_input_device(device_id: &str, label: Option<&str>) {
    if device_id.is_empty() {
        clear_input_device_id();
        return;
    }

    set::<LocalStorage>(INPUT_DEVICE_ID_KEY, device_id);
    match label.filter(|label| !label.is_empty()) {
        Some(label) => set::<LocalStorage>(INPUT_DEVICE_LABEL_KEY, label),
        None => remove::<LocalStorage>(INPUT_DEVICE_LABEL_KEY),
    }
    info!(
        has_label = label.is_some_and(|label| !label.is_empty()),
        "saved microphone input device preference"
    );
}

/// Clears the preferred microphone input device from local storage.
pub(crate) fn clear_input_device_id() {
    remove::<LocalStorage>(INPUT_DEVICE_ID_KEY);
    remove::<LocalStorage>(INPUT_DEVICE_LABEL_KEY);
}

/// Loads the preferred microphone input volume percentage.
pub(crate) fn load_input_volume_percent() -> u32 {
    let volume = get::<LocalStorage>(INPUT_VOLUME_PERCENT_KEY)
        .and_then(|value| value.parse::<u32>().ok())
        .map(clamp_volume_percent)
        .unwrap_or(DEFAULT_INPUT_VOLUME_PERCENT);
    info!(volume, "loaded microphone input volume preference");
    volume
}

/// Saves the preferred microphone input volume percentage.
pub(crate) fn save_input_volume_percent(volume_percent: u32) {
    let volume_percent = clamp_volume_percent(volume_percent);
    set::<LocalStorage>(INPUT_VOLUME_PERCENT_KEY, &volume_percent.to_string());
    info!(
        volume = volume_percent,
        "saved microphone input volume preference"
    );
}

/// Loads the preferred microphone activation mode.
pub(crate) fn load_activation_mode() -> MicrophoneActivationMode {
    let mode = match get::<LocalStorage>(ACTIVATION_MODE_KEY).as_deref() {
        Some("always_on") => MicrophoneActivationMode::AlwaysActive,
        _ => MicrophoneActivationMode::VoiceActivated,
    };
    info!(?mode, "loaded microphone activation mode preference");
    mode
}

/// Saves the preferred microphone activation mode.
pub(crate) fn save_activation_mode(mode: MicrophoneActivationMode) {
    let value = match mode {
        MicrophoneActivationMode::AlwaysActive => "always_on",
        MicrophoneActivationMode::VoiceActivated => "voice_activation",
    };
    set::<LocalStorage>(ACTIVATION_MODE_KEY, value);
    info!(?mode, "saved microphone activation mode preference");
}

/// Loads the preferred voice activation threshold percentage.
pub(crate) fn load_vad_threshold_percent() -> u32 {
    let threshold = get::<LocalStorage>(VAD_THRESHOLD_PERCENT_KEY)
        .and_then(|value| value.parse::<u32>().ok())
        .map(clamp_percent)
        .unwrap_or(DEFAULT_VAD_THRESHOLD_PERCENT);
    info!(threshold, "loaded microphone vad threshold preference");
    threshold
}

/// Saves the preferred voice activation threshold percentage.
pub(crate) fn save_vad_threshold_percent(threshold_percent: u32) {
    let threshold_percent = clamp_percent(threshold_percent);
    set::<LocalStorage>(VAD_THRESHOLD_PERCENT_KEY, &threshold_percent.to_string());
    info!(
        threshold = threshold_percent,
        "saved microphone vad threshold preference"
    );
}

fn clamp_volume_percent(volume_percent: u32) -> u32 {
    volume_percent.min(200)
}

fn clamp_percent(percent: u32) -> u32 {
    percent.min(100)
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
        warn!("failed to verify microphone preference storage write");
    }
}

fn remove<S>(key: &str)
where
    S: StorageBacking<Key = String>,
{
    S::set(key.to_owned(), &Option::<String>::None);
}
