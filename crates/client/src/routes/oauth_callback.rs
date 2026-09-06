//! Компонент маршрута OAuth callback браузерного клиента.

use crate::Route;
use crate::features::auth::OAuthRegistrationForm;
use crate::features::auth::api::{self, OAuthCompletion};
use dioxus::prelude::*;

/// Получает результат браузерного входа и показывает локальную форму регистрации.
#[component]
pub(crate) fn OAuthCallback(
    code: Option<String>,
    handoff_code: Option<String>,
    error: Option<String>,
) -> Element {
    let navigator = use_navigator();
    let handoff = handoff_code.or(code).unwrap_or_default();
    let completion = use_resource(move || {
        let handoff = handoff.clone();
        let error = error.clone();
        async move {
            if error.is_some() {
                warn!("Google OAuth callback rejected");
                return Err("Google не завершил вход. Попробуй ещё раз.".to_owned());
            }
            if handoff.trim().is_empty() {
                return Err("Google не вернул подтверждение входа. Попробуй ещё раз.".to_owned());
            }
            let result = if api::has_tokens() {
                api::complete_google_account_link(handoff)
                    .await
                    .map(|_| OAuthCompletion::Linked)
            } else {
                api::complete_google_oauth(handoff).await
            };
            if matches!(
                result,
                Ok(OAuthCompletion::Authenticated(_) | OAuthCompletion::Linked)
            ) {
                let _ = navigator.replace(Route::AppHome {});
            }
            result
        }
    });
    let result = completion.read().clone();
    rsx! {
        main { class: "grid min-h-screen place-items-center bg-zinc-950 px-4 py-8 text-zinc-100",
            section { class: "w-full max-w-md rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-5 shadow-[0_24px_80px_rgba(0,0,0,0.35)] sm:p-6",
                match result {
                    Some(Ok(OAuthCompletion::RegistrationRequired(registration))) => rsx! {
                        OAuthRegistrationForm {
                            registration,
                            on_complete: move |_| { let _ = navigator.replace(Route::AppHome {}); },
                            on_restart: move |_| { let _ = navigator.replace(Route::Login { password_reset: None }); },
                        }
                    },
                    Some(Err(error)) => rsx! {
                        h1 { class: "text-balance text-xl font-semibold text-zinc-50", "Не удалось войти через Google" }
                        p { role: "alert", class: "mt-2 text-pretty text-sm text-red-200", "{error}" }
                        button {
                            r#type: "button", class: "mt-5 flex min-h-11 w-full items-center justify-center rounded-xl bg-accent px-3 text-sm font-semibold text-white",
                            onclick: move |_| {
                                let route = if api::has_tokens() { Route::AppHome {} } else { Route::Login { password_reset: None } };
                                let _ = navigator.replace(route);
                            },
                            "Вернуться в CheenHub"
                        }
                    },
                    _ => rsx! {
                        div { role: "status", class: "flex items-center gap-3",
                            span { aria_hidden: "true", class: "h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-300" }
                            p { class: "text-sm text-zinc-300", "Завершаем вход через Google..." }
                        }
                    },
                }
            }
        }
    }
}
