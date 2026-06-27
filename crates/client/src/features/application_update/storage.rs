//! Постоянное хранение UI-решений по обновлениям приложения.

use dioxus::prelude::warn;
use dioxus_sdk_storage::{LocalStorage, StorageBacking};

const DEFERRED_VERSION_KEY: &str = "cheenhub.application_update.deferred_version";
const DEFERRED_UNTIL_KEY: &str = "cheenhub.application_update.deferred_until_epoch_seconds";

/// Сохраненная отсрочка напоминания об обновлении.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredUpdateDeferral {
    /// Версия обновления, для которой пользователь выбрал отсрочку.
    pub(crate) version: String,
    /// Время следующего напоминания в секундах UNIX epoch.
    pub(crate) until_epoch_seconds: u64,
}

/// Загружает сохраненную отсрочку напоминания об обновлении.
pub(crate) fn load_deferral() -> Option<StoredUpdateDeferral> {
    let version = get::<LocalStorage>(DEFERRED_VERSION_KEY).filter(|value| !value.is_empty())?;
    let until_epoch_seconds = get::<LocalStorage>(DEFERRED_UNTIL_KEY)?
        .parse::<u64>()
        .ok()?;

    Some(StoredUpdateDeferral {
        version,
        until_epoch_seconds,
    })
}

/// Сохраняет отсрочку напоминания об обновлении.
pub(crate) fn save_deferral(version: &str, until_epoch_seconds: u64) {
    set::<LocalStorage>(DEFERRED_VERSION_KEY, version);
    set::<LocalStorage>(DEFERRED_UNTIL_KEY, &until_epoch_seconds.to_string());
}

/// Очищает отсрочку напоминания об обновлении.
pub(crate) fn clear_deferral() {
    remove::<LocalStorage>(DEFERRED_VERSION_KEY);
    remove::<LocalStorage>(DEFERRED_UNTIL_KEY);
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
        warn!("failed to verify application update storage write");
    }
}

fn remove<S>(key: &str)
where
    S: StorageBacking<Key = String>,
{
    S::set(key.to_owned(), &Option::<String>::None);
}
