//! Страница явного подтверждения восстановления аккаунта.

use crate::Route;
use crate::features::auth::account_deletion_api;
use crate::features::auth::components::auth_header::AuthHeader;
use dioxus::prelude::*;

/// Предлагает отменить удаление аккаунта до окончания срока восстановления.
#[component]
pub(crate) fn RestoreAccountPage(token: Option<String>) -> Element {
    let mut loading = use_signal(|| false);
    let mut restored = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let token = token.unwrap_or_default();
    let has_token = !token.trim().is_empty();
    rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-100",
            AuthHeader {}
            main { class: "mx-auto flex min-h-[80vh] max-w-lg items-center px-5 py-10",
                section { class: "w-full rounded-3xl border border-zinc-800 bg-zinc-900/90 p-6",
                    h1 { class: "text-2xl font-semibold", "Восстановление аккаунта" }
                    if restored() {
                        p { role: "status", class: "mt-4 text-emerald-200", "Аккаунт восстановлен. Теперь можно снова войти и продолжить общение." }
                    } else if has_token {
                        p { class: "mt-4 text-sm leading-6 text-zinc-400", "Подтверди восстановление, чтобы отменить удаление аккаунта. Это доступно в течение 30 дней после запроса на удаление." }
                        if let Some(message) = error() {
                            p { role: "alert", class: "mt-4 text-sm text-red-200", "{message}" }
                        }
                        button {
                            r#type: "button",
                            disabled: loading(),
                            class: "mt-6 flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-sm font-semibold text-white disabled:opacity-60",
                            onclick: move |_| {
                                if loading() { return; }
                                loading.set(true);
                                error.set(None);
                                let token = token.clone();
                                info!("requesting account restoration");
                                spawn(async move {
                                    match account_deletion_api::restore_account(token).await {
                                        Ok(()) => {
                                            info!("account restoration confirmed");
                                            restored.set(true);
                                        }
                                        Err(message) => {
                                            warn!("account restoration request failed");
                                            error.set(Some(message));
                                        }
                                    }
                                    loading.set(false);
                                });
                            },
                            if loading() { "Восстанавливаем…" } else { "Восстановить аккаунт" }
                        }
                    } else {
                        p { role: "alert", class: "mt-4 text-sm leading-6 text-amber-100", "Ссылка неполная. Открой полную ссылку из письма об удалении аккаунта." }
                    }
                    Link {
                        to: Route::Login { password_reset: None },
                        class: "mt-5 block text-center text-sm text-blue-300 hover:text-blue-200",
                        "Перейти ко входу"
                    }
                }
            }
        }
    }
}
