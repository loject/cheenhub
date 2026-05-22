//! Persistent audio playback preference storage.

use dioxus::prelude::{info, warn};
use dioxus_sdk_storage::{LocalStorage, StorageBacking};

const OUTPUT_DEVICE_ID_KEY: &str = "cheenhub.audio_playback.output_device_id";
const OUTPUT_DEVICE_LABEL_KEY: &str = "cheenhub.audio_playback.output_device_label";

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
