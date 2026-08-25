//! Компонент маршрута входа.

use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::LoginPage;
use crate::features::auth::api;
use crate::features::auth::guest_guard::{
    GuestAuthGuardDecision, GuestAuthPage, decide_guest_auth_guard,
};

#[component]
pub(crate) fn Login(password_reset: Option<String>) -> Element {
    let navigator = use_navigator();
    let decision = decide_guest_auth_guard(GuestAuthPage::Login, api::has_tokens());

    use_effect(move || {
        if let GuestAuthGuardDecision::RedirectToAppHome { source } = decision {
            info!(
                route = source.path(),
                target = "/app",
                "redirecting authenticated user away from guest auth page"
            );
            let _ = navigator.replace(Route::AppHome {});
        }
    });

    if matches!(decision, GuestAuthGuardDecision::RedirectToAppHome { .. }) {
        return rsx! {
            div { class: "grid min-h-screen place-items-center bg-zinc-950 px-5 text-zinc-300",
                "Открываем CheenHub..."
            }
        };
    }

    rsx! {
        LoginPage {
            password_reset_succeeded: password_reset.as_deref() == Some("success")
        }
    }
}
