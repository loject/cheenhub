//! Browser-реализация хранения пары auth-токенов одним localStorage-значением.

use dioxus_sdk_storage::{LocalStorage, StorageBacking};

use super::super::StoredTokens;

const TOKEN_PAIR_KEY: &str = "cheenhub.auth.token_pair";

pub(super) fn load() -> Result<Option<StoredTokens>, String> {
    Ok(LocalStorage::get::<Option<StoredTokens>>(&TOKEN_PAIR_KEY.to_owned()).flatten())
}

pub(super) fn save(tokens: &StoredTokens) -> Result<(), String> {
    LocalStorage::set(TOKEN_PAIR_KEY.to_owned(), &Some(tokens.clone()));
    Ok(())
}

pub(super) fn clear() -> Result<(), String> {
    LocalStorage::set(TOKEN_PAIR_KEY.to_owned(), &Option::<StoredTokens>::None);
    Ok(())
}
