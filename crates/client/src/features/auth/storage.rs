//! Browser token storage for authentication.

use dioxus_sdk_storage::{LocalStorage, SessionStorage, StorageBacking};

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
    if let Some(tokens) = load_from::<LocalStorage>() {
        return Some(tokens);
    }

    let tokens = load_from::<SessionStorage>()?;
    save(&tokens.access_token, &tokens.refresh_token);
    Some(tokens)
}

fn load_from<S>() -> Option<StoredTokens>
where
    S: StorageBacking<Key = String>,
{
    let access_token = get::<S>(ACCESS_TOKEN_KEY)?;
    let refresh_token = get::<S>(REFRESH_TOKEN_KEY)?;

    Some(StoredTokens {
        access_token,
        refresh_token,
    })
}

/// Saves tokens into browser storage.
pub(crate) fn save(access_token: &str, refresh_token: &str) {
    set::<LocalStorage>(ACCESS_TOKEN_KEY, access_token);
    set::<LocalStorage>(REFRESH_TOKEN_KEY, refresh_token);
    set::<SessionStorage>(ACCESS_TOKEN_KEY, access_token);
    set::<SessionStorage>(REFRESH_TOKEN_KEY, refresh_token);
}

/// Clears all stored authentication tokens.
pub(crate) fn clear() {
    remove::<LocalStorage>(ACCESS_TOKEN_KEY);
    remove::<LocalStorage>(REFRESH_TOKEN_KEY);
    remove::<SessionStorage>(ACCESS_TOKEN_KEY);
    remove::<SessionStorage>(REFRESH_TOKEN_KEY);
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
