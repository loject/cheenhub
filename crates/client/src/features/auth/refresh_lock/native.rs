//! Native межпроцессный refresh-lock на атомарно создаваемом файле.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const LOCK_TTL_MS: u64 = 30_000;

pub(super) struct RefreshLockGuard {
    owner: String,
    path: PathBuf,
}

impl Drop for RefreshLockGuard {
    fn drop(&mut self) {
        if fs::read_to_string(&self.path)
            .ok()
            .and_then(|value| value.split_once('|').map(|(owner, _)| owner.to_owned()))
            .as_deref()
            == Some(self.owner.as_str())
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(super) async fn try_acquire() -> Result<Option<RefreshLockGuard>, String> {
    let path = lock_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "auth lock path has no parent".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let owner = uuid::Uuid::new_v4().to_string();
    let expires_at = now_millis().saturating_add(LOCK_TTL_MS);

    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            write!(file, "{owner}|{expires_at}").map_err(|error| error.to_string())?;
            file.sync_all().map_err(|error| error.to_string())?;
            Ok(Some(RefreshLockGuard { owner, path }))
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if lock_expired(&path) {
                let _ = fs::remove_file(&path);
            }
            Ok(None)
        }
        Err(error) => Err(error.to_string()),
    }
}

fn lock_expired(path: &std::path::Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|value| {
            value
                .split_once('|')
                .and_then(|(_, expires)| expires.parse::<u64>().ok())
        })
        .is_none_or(|expires_at| expires_at <= now_millis())
}

fn lock_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(path)
            .join("CheenHub")
            .join("auth-refresh.lock"));
    }
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path)
            .join("cheenhub")
            .join("auth-refresh.lock"));
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| path.join(".local/share/cheenhub/auth-refresh.lock"))
        .ok_or_else(|| "native auth lock directory is unavailable".to_owned())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
