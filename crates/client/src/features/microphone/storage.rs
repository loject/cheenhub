//! Persistent microphone preference storage.

use dioxus::prelude::{info, warn};
use dioxus_sdk_storage::{LocalStorage, StorageBacking};

const INPUT_DEVICE_ID_KEY: &str = "cheenhub.microphone.input_device_id";
const INPUT_DEVICE_LABEL_KEY: &str = "cheenhub.microphone.input_device_label";

/// Stored microphone input device preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredInputDevice {
    /// Browser device ID to pass into `getUserMedia`.
    pub(crate) device_id: String,
    /// Browser-provided device label used to recover when device IDs rotate.
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
