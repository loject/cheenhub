//! Политика стартового маршрута Android.

use dioxus::prelude::info;

use crate::Route;

/// Не восстанавливает личную переписку при обычном запуске Android-приложения.
pub(crate) fn resolve(saved_route: Option<Route>) -> Route {
    match saved_route {
        Some(Route::AppDirectMessage { conversation_id }) => {
            info!(
                %conversation_id,
                "skipping saved direct conversation on Android startup"
            );
            Route::AppFriends {}
        }
        Some(route) => route,
        None => Route::AppFriends {},
    }
}
