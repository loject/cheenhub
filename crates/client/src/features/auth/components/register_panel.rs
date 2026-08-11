//! Компонент панели формы регистрации.

use cheenhub_contracts::rest::RegisterRequest;
use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::api;
use crate::features::auth::components::text_input::TextInput;
use crate::features::auth::{LegalAcceptanceAction, LegalAcceptanceFields};

#[component]
pub(crate) fn RegisterPanel() -> Element {
    let navigator = use_navigator();
    let mut nickname = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut accepts_terms = use_signal(|| false);
    let mut accepts_personal_data = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut is_busy = use_signal(|| false);

    rsx! {
        div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-5 shadow-[0_24px_80px_rgba(0,0,0,0.35)] sm:p-6",
            div { class: "mb-6",
                div { class: "mb-2 text-[10px] uppercase tracking-[0.24em] text-zinc-600", "Регистрация" }
                h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "Создать аккаунт" }
                p { class: "mt-1.5 text-[13px] leading-5 text-zinc-500", "Укажи email и пароль, чтобы начать пользоваться CheenHub." }
            }

            form { class: "space-y-4",
                TextInput {
                    input_type: "text",
                    label: "Никнейм",
                    name: "nickname",
                    placeholder: "cheenhero",
                    autocomplete: "nickname",
                    value: nickname(),
                    oninput: move |value| nickname.set(value)
                }
                TextInput {
                    input_type: "email",
                    label: "Email",
                    name: "email",
                    placeholder: "you@example.com",
                    autocomplete: "email",
                    value: email(),
                    oninput: move |value| email.set(value)
                }
                TextInput {
                    input_type: "password",
                    label: "Password",
                    name: "password",
                    placeholder: "••••••••",
                    autocomplete: "new-password",
                    value: password(),
                    oninput: move |value| password.set(value)
                }
                LegalAcceptanceFields {
                    accepts_terms: accepts_terms(),
                    accepts_personal_data: accepts_personal_data(),
                    on_change: move |action| match action {
                        LegalAcceptanceAction::TermsChanged(value) => accepts_terms.set(value),
                        LegalAcceptanceAction::PersonalDataChanged(value) => accepts_personal_data.set(value),
                    }
                }
                if !status().is_empty() {
                    p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                        "{status()}"
                    }
                }
                button {
                    r#type: "button",
                    disabled: is_busy() || !accepts_terms() || !accepts_personal_data(),
                    class: "btn-p flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] disabled:cursor-not-allowed disabled:opacity-60",
                    onclick: move |_| {
                        is_busy.set(true);
                        status.set(String::new());
                        let request = RegisterRequest {
                            nickname: nickname(),
                            email: email(),
                            password: password(),
                            accepts_terms: accepts_terms(),
                            accepts_personal_data: accepts_personal_data(),
                        };
                        spawn(async move {
                            match api::register(request).await {
                                Ok(_) => {
                                    let _ = navigator.replace(Route::AppHome {});
                                }
                                Err(error) => {
                                    status.set(error);
                                    is_busy.set(false);
                                }
                            };
                        });
                    },
                    if is_busy() { "Создаем..." } else { "Создать аккаунт" }
                }
            }

            div { class: "mt-5 rounded-2xl border border-zinc-800 bg-zinc-950/80 px-4 py-3 text-[12px] leading-5 text-zinc-500",
                "Аккаунт поможет сохранить комнаты, друзей и персональные настройки между устройствами."
            }

            div { class: "mt-4 text-center text-[13px] text-zinc-500",
                "Уже есть аккаунт? "
                Link {
                    to: Route::Login {},
                    class: "font-medium text-zinc-200 transition hover:text-white",
                    "Войти"
                }
            }
        }
    }
}
