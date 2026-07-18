//! Выбор платформенной реализации refresh-lock.

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
#[path = "web.rs"]
mod implementation;

/// Guard выбранной платформенной реализации refresh-lock.
pub(crate) struct RefreshLockGuard(#[allow(dead_code)] implementation::RefreshLockGuard);

pub(super) async fn try_acquire() -> Result<Option<RefreshLockGuard>, String> {
    implementation::try_acquire()
        .await
        .map(|guard| guard.map(RefreshLockGuard))
}
