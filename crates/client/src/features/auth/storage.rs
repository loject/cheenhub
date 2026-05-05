//! Browser token storage for authentication.

use dioxus_sdk_storage::{SessionStorage, StorageBacking};

const ACCESS_TOKEN_KEY: &str = "cheenhub.access_token";
const REFRESH_TOKEN_KEY: &str = "cheenhub.refresh_token";

/// Stored authentication token pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredTokens {
    /// Short-lived access JWT.
    pub(crate) access_token: String,
    /// Long-lived opaque refresh token.
    pub(crate) refresh_token: String,
}

/// Loads tokens from Dioxus-managed browser storage boundary.
pub(crate) fn load() -> Option<StoredTokens> {
    let access_token = get(ACCESS_TOKEN_KEY)?;
    let refresh_token = get(REFRESH_TOKEN_KEY)?;

    Some(StoredTokens {
        access_token,
        refresh_token,
    })
}

/// Saves tokens into browser storage.
pub(crate) fn save(access_token: &str, refresh_token: &str) {
    set(ACCESS_TOKEN_KEY, access_token);
    set(REFRESH_TOKEN_KEY, refresh_token);
}

/// Clears all stored authentication tokens.
pub(crate) fn clear() {
    remove(ACCESS_TOKEN_KEY);
    remove(REFRESH_TOKEN_KEY);
}

fn get(key: &str) -> Option<String> {
    SessionStorage::get::<Option<String>>(&key.to_owned()).flatten()
}

fn set(key: &str, value: &str) {
    SessionStorage::set(key.to_owned(), &Some(value.to_owned()));
}

fn remove(key: &str) {
    SessionStorage::set(key.to_owned(), &Option::<String>::None);
}
