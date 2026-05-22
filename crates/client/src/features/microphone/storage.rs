//! Persistent microphone preference storage.

use dioxus_sdk_storage::{LocalStorage, StorageBacking};

const INPUT_DEVICE_ID_KEY: &str = "cheenhub.microphone.input_device_id";

/// Loads the preferred microphone input device ID from local storage.
pub(crate) fn load_input_device_id() -> Option<String> {
    get::<LocalStorage>(INPUT_DEVICE_ID_KEY).filter(|device_id| !device_id.is_empty())
}

/// Saves the preferred microphone input device ID into local storage.
pub(crate) fn save_input_device_id(device_id: &str) {
    if device_id.is_empty() {
        clear_input_device_id();
        return;
    }

    set::<LocalStorage>(INPUT_DEVICE_ID_KEY, device_id);
}

/// Clears the preferred microphone input device ID from local storage.
pub(crate) fn clear_input_device_id() {
    remove::<LocalStorage>(INPUT_DEVICE_ID_KEY);
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
}

fn remove<S>(key: &str)
where
    S: StorageBacking<Key = String>,
{
    S::set(key.to_owned(), &Option::<String>::None);
}
