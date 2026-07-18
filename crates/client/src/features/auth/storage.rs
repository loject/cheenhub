//! Платформенная граница хранения пары auth-токенов.

use dioxus::logger::tracing::{info, warn};
use dioxus_sdk_storage::{LocalStorage, SessionStorage, StorageBacking};
use serde::{Deserialize, Serialize};

#[path = "storage/platform.rs"]
mod platform;

const ACCESS_TOKEN_KEY: &str = "cheenhub.access_token";
const REFRESH_TOKEN_KEY: &str = "cheenhub.refresh_token";

/// Сохраненная пара токенов аутентификации.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredTokens {
    /// Short-lived access JWT.
    pub(crate) access_token: String,
    /// Long-lived opaque refresh token.
    pub(crate) refresh_token: String,
}

/// Загружает атомарно сохранённую пару токенов или мигрирует legacy-ключи.
pub(crate) fn load() -> Option<StoredTokens> {
    match platform::load() {
        Ok(Some(tokens)) => return Some(tokens),
        Ok(None) => {}
        Err(error) => warn!(%error, "failed to load atomic auth token pair"),
    }

    let tokens = load_legacy()?;
    if let Err(error) = platform::save(&tokens) {
        warn!(%error, "failed to migrate legacy auth token pair");
    } else {
        clear_legacy();
        info!("migrated legacy auth token pair to atomic storage");
    }
    Some(tokens)
}

/// Возвращает текущий access token, если сохраненная пара токенов уже изменилась.
pub(crate) fn access_token_if_changed(tokens: &StoredTokens) -> Option<String> {
    let stored = load()?;
    if stored == *tokens {
        return None;
    }
    Some(stored.access_token)
}

/// Атомарно сохраняет access и refresh token как одну пару.
pub(crate) fn save(access_token: &str, refresh_token: &str) {
    let tokens = StoredTokens {
        access_token: access_token.to_owned(),
        refresh_token: refresh_token.to_owned(),
    };
    if let Err(error) = platform::save(&tokens) {
        warn!(%error, "failed to save atomic auth token pair");
    }
}

/// Очищает атомарное и legacy-хранилища токенов.
pub(crate) fn clear() {
    if let Err(error) = platform::clear() {
        warn!(%error, "failed to clear atomic auth token pair");
    }
    clear_legacy();
}

fn load_legacy() -> Option<StoredTokens> {
    load_legacy_from::<LocalStorage>().or_else(load_legacy_from::<SessionStorage>)
}

fn load_legacy_from<S>() -> Option<StoredTokens>
where
    S: StorageBacking<Key = String>,
{
    Some(StoredTokens {
        access_token: S::get::<Option<String>>(&ACCESS_TOKEN_KEY.to_owned()).flatten()?,
        refresh_token: S::get::<Option<String>>(&REFRESH_TOKEN_KEY.to_owned()).flatten()?,
    })
}

fn clear_legacy() {
    for key in [ACCESS_TOKEN_KEY, REFRESH_TOKEN_KEY] {
        LocalStorage::set(key.to_owned(), &Option::<String>::None);
        SessionStorage::set(key.to_owned(), &Option::<String>::None);
    }
}
