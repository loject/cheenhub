//! Маршрут восстановления удалённого аккаунта.

use crate::features::auth::RestoreAccountPage;
use dioxus::prelude::*;

/// Открывает подтверждение восстановления по ссылке из письма.
#[component]
pub(crate) fn RestoreAccount(token: Option<String>) -> Element {
    rsx! { RestoreAccountPage { key: "{token:?}", token } }
}
