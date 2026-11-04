//! Локальный список точных hostname, отдельный для каждого аккаунта.

use dioxus_sdk_storage::{LocalStorage, StorageBacking};

fn key(user_id: &str) -> String { format!("cheenhub.external_links.trusted_domains.{user_id}") }

pub(super) fn contains(user_id: &str, hostname: &str) -> bool {
    LocalStorage::get::<Vec<String>>(&key(user_id)).unwrap_or_default().iter().any(|domain| domain == hostname)
}

pub(super) fn trust(user_id: &str, hostname: &str) {
    let key = key(user_id);
    let mut domains = LocalStorage::get::<Vec<String>>(&key).unwrap_or_default();
    if !domains.iter().any(|domain| domain == hostname) {
        domains.push(hostname.to_owned());
        LocalStorage::set(key, &domains);
    }
}
