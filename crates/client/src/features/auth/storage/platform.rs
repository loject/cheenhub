//! Выбор платформенной реализации атомарного auth-хранилища.

#[cfg(feature = "web")]
#[path = "web.rs"]
mod implementation;

#[cfg(any(feature = "windows", feature = "linux", feature = "macos"))]
#[path = "native.rs"]
mod implementation;

#[cfg(not(any(
    feature = "web",
    feature = "windows",
    feature = "linux",
    feature = "macos"
)))]
#[path = "fallback.rs"]
mod implementation;

use super::StoredTokens;

pub(super) fn load() -> Result<Option<StoredTokens>, String> {
    implementation::load()
}

pub(super) fn save(tokens: &StoredTokens) -> Result<(), String> {
    implementation::save(tokens)
}

pub(super) fn clear() -> Result<(), String> {
    implementation::clear()
}
