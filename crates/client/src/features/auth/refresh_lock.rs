//! Платформенная координация обновления refresh token.

use dioxus::logger::tracing::{debug, warn};

use crate::features::auth::storage::{self, StoredTokens};
use crate::features::runtime::sleep_ms;

#[path = "refresh_lock/platform.rs"]
mod platform;

const LOCK_WAIT_TIMEOUT_MS: u64 = 35_000;
const LOCK_POLL_MS: u32 = 120;

/// Результат попытки занять общий lock обновления refresh token.
pub(crate) enum RefreshLockOutcome {
    /// Текущий клиент может отправлять `/auth/refresh`.
    Acquired(platform::RefreshLockGuard),
    /// Другой клиент уже обновил токены, можно использовать новый access token.
    TokensChanged(String),
    /// Дождаться освобождения lock не удалось.
    TimedOut,
}

/// Пытается занять межвкладочный или межпроцессный refresh-lock.
pub(crate) async fn acquire(tokens: &StoredTokens) -> RefreshLockOutcome {
    let mut waited_ms = 0_u64;
    loop {
        if let Some(access_token) = storage::access_token_if_changed(tokens) {
            debug!(
                waited_ms,
                "auth tokens changed while waiting for refresh lock"
            );
            return RefreshLockOutcome::TokensChanged(access_token);
        }

        match platform::try_acquire().await {
            Ok(Some(guard)) => {
                debug!(waited_ms, "acquired auth refresh lock");
                return RefreshLockOutcome::Acquired(guard);
            }
            Ok(None) => {}
            Err(error) => warn!(%error, "failed to acquire auth refresh lock"),
        }

        if waited_ms >= LOCK_WAIT_TIMEOUT_MS {
            warn!(waited_ms, "timed out waiting for auth refresh lock");
            return RefreshLockOutcome::TimedOut;
        }
        sleep_ms(LOCK_POLL_MS).await;
        waited_ms = waited_ms.saturating_add(u64::from(LOCK_POLL_MS));
    }
}
