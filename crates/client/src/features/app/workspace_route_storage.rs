//! Хранилище последней внутренней ссылки авторизованного приложения.

use std::str::FromStr;

use dioxus::prelude::warn;
use dioxus_sdk_storage::{LocalStorage, StorageBacking};

use crate::Route;

use super::workspace_route::AppWorkspaceRoute;

const LAST_WORKSPACE_ROUTE_KEY_PREFIX: &str = "cheenhub.workspace.last.";

/// Загружает последнюю внутреннюю ссылку приложения для пользователя.
pub(crate) fn load(user_id: &str) -> Option<Route> {
    let saved_route = LocalStorage::get::<Option<String>>(&storage_key(user_id)).flatten()?;
    let route = match Route::from_str(&saved_route) {
        Ok(route) => route,
        Err(error) => {
            warn!(%saved_route, %error, "failed to parse saved app workspace route");
            return None;
        }
    };

    AppWorkspaceRoute::from_route(&route).map(|_| route)
}

/// Сохраняет последнюю внутреннюю ссылку приложения для пользователя.
pub(crate) fn save(user_id: &str, route: &Route) {
    if AppWorkspaceRoute::from_route(route).is_none() {
        return;
    }

    LocalStorage::set(storage_key(user_id), &Some(route.to_string()));
}

fn storage_key(user_id: &str) -> String {
    format!("{LAST_WORKSPACE_ROUTE_KEY_PREFIX}{user_id}")
}
