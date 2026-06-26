//! Native-реализация маршрута лендинга.
#![cfg_attr(target_family = "wasm", allow(dead_code, unused_imports))]

use dioxus::prelude::*;

use crate::Route;

/// Рендерит native-маршрут лендинга с перенаправлением на вход.
#[component]
pub(crate) fn LandingRoute() -> Element {
    let navigator = use_navigator();
    use_effect(move || {
        info!(
            target = "/login",
            "redirecting native landing route to login"
        );
        let _ = navigator.replace(Route::Login {});
    });

    rsx! {
        div { class: "grid min-h-screen place-items-center bg-zinc-950 px-5 text-zinc-300",
            "Открываем вход в CheenHub..."
        }
    }
}

/// Возвращает домашний маршрут native-клиента.
pub(crate) fn public_home_route() -> Route {
    Route::Login {}
}

/// Возвращает подпись домашнего действия native-клиента.
pub(crate) fn public_home_label() -> &'static str {
    "Войти"
}

/// Возвращает доступность публичного лендинга на native-клиенте.
pub(crate) fn public_landing_available() -> bool {
    false
}
