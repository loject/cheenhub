//! Политика стартового маршрута для платформ, сохраняющих последнюю рабочую область.

use crate::Route;

/// Восстанавливает сохранённую рабочую область или открывает список друзей.
pub(crate) fn resolve(saved_route: Option<Route>) -> Route {
    saved_route.unwrap_or(Route::AppFriends {})
}
