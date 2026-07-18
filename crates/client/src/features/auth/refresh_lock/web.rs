//! Browser refresh-lock через localStorage с проверкой владельца.

use dioxus_sdk_storage::{LocalStorage, StorageBacking};
use web_time::{SystemTime, UNIX_EPOCH};

use crate::features::runtime::sleep_ms;

const REFRESH_LOCK_KEY: &str = "cheenhub.auth.refresh_lock";
const LOCK_TTL_MS: u64 = 30_000;
const LOCK_SETTLE_MS: u32 = 40;

pub(super) struct RefreshLockGuard {
    owner: String,
}

impl Drop for RefreshLockGuard {
    fn drop(&mut self) {
        if read_lock().is_some_and(|lock| lock.owner == self.owner) {
            LocalStorage::set(REFRESH_LOCK_KEY.to_owned(), &Option::<String>::None);
        }
    }
}

pub(super) async fn try_acquire() -> Result<Option<RefreshLockGuard>, String> {
    let now = now_millis();
    if read_lock().is_some_and(|lock| lock.expires_at_ms > now) {
        return Ok(None);
    }
    let owner = uuid::Uuid::new_v4().to_string();
    LocalStorage::set(
        REFRESH_LOCK_KEY.to_owned(),
        &Some(format!("{owner}|{}", now.saturating_add(LOCK_TTL_MS))),
    );
    sleep_ms(LOCK_SETTLE_MS).await;
    Ok(read_lock()
        .filter(|lock| lock.owner == owner)
        .map(|_| RefreshLockGuard { owner }))
}

struct StoredLock {
    owner: String,
    expires_at_ms: u64,
}

fn read_lock() -> Option<StoredLock> {
    let raw = LocalStorage::get::<Option<String>>(&REFRESH_LOCK_KEY.to_owned()).flatten()?;
    let (owner, expires_at_ms) = raw.split_once('|')?;
    Some(StoredLock {
        owner: owner.to_owned(),
        expires_at_ms: expires_at_ms.parse().ok()?,
    })
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
