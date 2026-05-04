//! Registration form panel component.

use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::behavior::show_todo_alert;
use crate::features::auth::components::text_input::TextInput;

#[component]
pub(crate) fn RegisterPanel() -> Element {
    let mut accepts_policies = use_signal(|| false);
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
                    autocomplete: "nickname"
                }
                TextInput {
                    input_type: "email",
                    label: "Email",
                    name: "email",
                    placeholder: "you@example.com",
                    autocomplete: "email"
                }
                TextInput {
                    input_type: "password",
                    label: "Password",
                    name: "password",
                    placeholder: "••••••••",
                    autocomplete: "new-password"
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
                button {
                    r#type: "button",
                    class: "btn-p flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)]",
                    onclick: move |_| show_todo_alert(),
                    "Создать аккаунт"
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
