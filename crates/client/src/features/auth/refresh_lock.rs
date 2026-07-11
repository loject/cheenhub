//! Координация обновления access token между вкладками браузера.

use dioxus::logger::tracing::{debug, warn};
use dioxus_sdk_storage::{LocalStorage, StorageBacking};
use uuid::Uuid;
use web_time::{SystemTime, UNIX_EPOCH};

use crate::features::auth::storage::{self, StoredTokens};
use crate::features::runtime::sleep_ms;

const REFRESH_LOCK_KEY: &str = "cheenhub.auth.refresh_lock";
const LOCK_TTL_MS: u64 = 30_000;
const LOCK_WAIT_TIMEOUT_MS: u64 = 35_000;
const LOCK_POLL_MS: u32 = 120;
const LOCK_SETTLE_MS: u32 = 40;

/// Результат попытки занять общий lock обновления refresh token.
pub(crate) enum RefreshLockOutcome {
    /// Текущая вкладка может отправлять `/auth/refresh`.
    Acquired(RefreshLockGuard),
    /// Другая вкладка уже обновила токены, можно использовать новый access token.
    TokensChanged(String),
    /// Дождаться освобождения lock не удалось.
    TimedOut,
}

/// Guard общего lock обновления refresh token.
pub(crate) struct RefreshLockGuard {
    owner: String,
}

impl Drop for RefreshLockGuard {
    fn drop(&mut self) {
        let Some(lock) = read_lock() else {
            return;
        };
        if lock.owner == self.owner {
            remove_lock();
        }
    }
}

/// Пытается занять refresh-lock или дождаться, пока соседняя вкладка обновит токены.
pub(crate) async fn acquire(tokens: &StoredTokens) -> RefreshLockOutcome {
    let owner = Uuid::new_v4().to_string();
    let mut waited_ms = 0_u64;

    loop {
        if let Some(access_token) = storage::access_token_if_changed(tokens) {
            if waited_ms > 0 {
                debug!(
                    waited_ms,
                    "auth refresh tokens changed while waiting for cross-tab lock"
                );
            }
            return RefreshLockOutcome::TokensChanged(access_token);
        }

        if let Some(guard) = try_acquire(&owner).await {
            if waited_ms > 0 {
                debug!(waited_ms, "acquired auth refresh cross-tab lock");
            }
            return RefreshLockOutcome::Acquired(guard);
        }

        if waited_ms == 0 {
            debug!("waiting for another tab to finish auth refresh");
        }

        if waited_ms >= LOCK_WAIT_TIMEOUT_MS {
            warn!(
                waited_ms,
                "timed out waiting for auth refresh cross-tab lock"
            );
            return RefreshLockOutcome::TimedOut;
        }

        sleep_ms(LOCK_POLL_MS).await;
        waited_ms = waited_ms.saturating_add(u64::from(LOCK_POLL_MS));
    }
}

async fn try_acquire(owner: &str) -> Option<RefreshLockGuard> {
    let now_ms = now_millis();
    if let Some(lock) = read_lock() {
        if lock.expires_at_ms > now_ms {
            return None;
        }
        debug!("replacing expired auth refresh cross-tab lock");
    }

    write_lock(owner, now_ms.saturating_add(LOCK_TTL_MS));
    sleep_ms(LOCK_SETTLE_MS).await;
    let lock = read_lock()?;
    if lock.owner != owner {
        return None;
    }

    Some(RefreshLockGuard {
        owner: owner.to_owned(),
    })
}

#[derive(Debug)]
struct StoredLock {
    owner: String,
    expires_at_ms: u64,
}

fn read_lock() -> Option<StoredLock> {
    let raw = LocalStorage::get::<Option<String>>(&REFRESH_LOCK_KEY.to_owned()).flatten()?;
    let (owner, expires_at_ms) = raw.split_once('|')?;
    let expires_at_ms = expires_at_ms.parse::<u64>().ok()?;

    Some(StoredLock {
        owner: owner.to_owned(),
        expires_at_ms,
    })
}

fn write_lock(owner: &str, expires_at_ms: u64) {
    LocalStorage::set(
        REFRESH_LOCK_KEY.to_owned(),
        &Some(format!("{owner}|{expires_at_ms}")),
    );
}

fn remove_lock() {
    LocalStorage::set(REFRESH_LOCK_KEY.to_owned(), &Option::<String>::None);
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
