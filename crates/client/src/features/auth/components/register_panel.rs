//! Registration form panel component.

use cheenhub_contracts::rest::RegisterRequest;
use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::api;
use crate::features::auth::components::text_input::TextInput;

#[component]
pub(crate) fn RegisterPanel() -> Element {
    let navigator = use_navigator();
    let mut nickname = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut accepts_policies = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut is_busy = use_signal(|| false);
    let checkbox_class = if accepts_policies() {
        "flex h-5 w-5 shrink-0 items-center justify-center rounded-lg bg-accent text-white transition"
    } else {
        "flex h-5 w-5 shrink-0 items-center justify-center rounded-lg bg-zinc-800 text-transparent transition hover:bg-zinc-700"
    };

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
                div { class: "flex items-center gap-3 text-[12px] leading-5 text-zinc-500",
                    input {
                        r#type: "hidden",
                        name: "terms",
                        value: "{accepts_policies()}"
                    }
                    button {
                        r#type: "button",
                        aria_pressed: "{accepts_policies()}",
                        class: "{checkbox_class}",
                        onclick: move |_| accepts_policies.set(!accepts_policies()),
                        svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "3", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" }
                        }
                    }
                    span {
                        "Я принимаю правила сервиса, политику конфиденциальности и согласен с обработкой данных аккаунта."
                    }
                }
                if !status().is_empty() {
                    p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                        "{status()}"
                    }
                }
                button {
                    r#type: "button",
                    disabled: is_busy(),
                    class: "btn-p flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)]",
                    onclick: move |_| {
                        is_busy.set(true);
                        status.set(String::new());
                        let request = RegisterRequest {
                            nickname: nickname(),
                            email: email(),
                            password: password(),
                            accepts_policies: accepts_policies(),
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
