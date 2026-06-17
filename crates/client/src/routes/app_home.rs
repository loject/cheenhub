//! Компонент маршрута аутентифицированного приложения.

use dioxus::prelude::*;

use crate::features::app::AppPage;

#[component]
pub(crate) fn AppHome() -> Element {
    rsx! {
        AppPage {}
    }
}
