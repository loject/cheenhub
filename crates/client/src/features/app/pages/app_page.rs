//! Authenticated application shell page.

use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::api;

/// Renders the signed-in application home.
#[component]
pub(crate) fn AppPage() -> Element {
    let navigator = use_navigator();
    let user = use_signal(|| None);
    let mut status = use_signal(String::new);
    let mut is_busy = use_signal(|| false);
    let mut loaded_profile = use_signal(|| false);

    use_effect(move || {
        if !api::has_tokens() {
            navigator.replace(Route::Login {});
            return;
        }
        if loaded_profile() {
            return;
        }
        loaded_profile.set(true);

        spawn(async move {
            match api::current_user().await {
                Ok(current_user) => {
                    let mut user = user;
                    user.set(Some(current_user));
                }
                Err(_) => {
                    let _ = navigator.replace(Route::Login {});
                }
            }
        });
    });

    let Some(current_user) = user() else {
        return rsx! {
            div { class: "grid min-h-screen place-items-center bg-zinc-950 px-5 text-zinc-300",
                "Открываем CheenHub..."
            }
        };
    };

    rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-100 selection:bg-zinc-700/40",
            main { class: "mx-auto flex min-h-screen w-full max-w-5xl flex-col px-5 py-6 lg:px-8",
                header { class: "flex items-center justify-between border-b border-zinc-800 pb-5",
                    div {
                        div { class: "text-[11px] uppercase tracking-[0.22em] text-zinc-600", "CheenHub" }
                        h1 { class: "mt-1 text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "Добро пожаловать, {current_user.nickname}" }
                    }
                    button {
                        r#type: "button",
                        disabled: is_busy(),
                        class: "rounded-xl border border-zinc-800 px-4 py-2 text-[13px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:text-white disabled:cursor-not-allowed disabled:opacity-60",
                        onclick: move |_| {
                            is_busy.set(true);
                            status.set(String::new());
                            spawn(async move {
                                match api::logout().await {
                                    Ok(()) => {
                                        let _ = navigator.replace(Route::Login {});
                                    }
                                    Err(error) => {
                                        status.set(error);
                                        is_busy.set(false);
                                    }
                                };
                            });
                        },
                        "Выйти"
                    }
                }

                section { class: "grid flex-1 items-center py-10",
                    div { class: "max-w-2xl",
                        p { class: "text-[13px] uppercase tracking-[0.22em] text-zinc-600", "Аккаунт активен" }
                        h2 { class: "mt-3 text-4xl font-semibold tracking-[-0.05em] text-zinc-50 sm:text-5xl", "Можно переходить к комнатам и общению." }
                        if current_user.registered_at.is_empty() {
                            p { class: "mt-4 max-w-xl text-[15px] leading-7 text-zinc-400",
                                "Профиль привязан к {current_user.email}."
                            }
                        } else {
                            p { class: "mt-4 max-w-xl text-[15px] leading-7 text-zinc-400",
                                "Профиль привязан к {current_user.email}. Дата регистрации: {current_user.registered_at}."
                            }
                        }
                        if !status().is_empty() {
                            p { class: "mt-5 rounded-xl border border-red-500/20 bg-red-500/10 px-4 py-3 text-[13px] text-red-200",
                                "{status()}"
                            }
                        }
                    }
                }
            }
        }
    }
}
