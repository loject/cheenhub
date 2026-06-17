//! Страница маршрута сброса пароля.

use dioxus::prelude::*;

use crate::features::auth::components::auth_header::AuthHeader;
use crate::features::auth::components::auth_hero::AuthHero;
use crate::features::auth::components::reset_password_panel::ResetPasswordPanel;

/// Рендерит страницу подтверждения сброса пароля CheenHub.
#[component]
pub(crate) fn ResetPasswordPage(token: Option<String>) -> Element {
    rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-100 selection:bg-zinc-700/40",
            div { class: "grid-bg flex min-h-screen flex-col",
                AuthHeader {}
                main { class: "flex flex-1 items-center px-5 py-10 lg:px-8",
                    section { class: "mx-auto grid w-full max-w-6xl gap-8 lg:grid-cols-[minmax(0,1fr)_420px] lg:items-center",
                        AuthHero {}
                        ResetPasswordPanel { token }
                    }
                }
            }
        }
    }
}
