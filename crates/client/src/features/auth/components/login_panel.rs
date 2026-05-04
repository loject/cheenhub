//! Login form panel component.

use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::behavior::show_todo_alert;
use crate::features::auth::components::provider_button::ProviderButton;
use crate::features::auth::components::text_input::TextInput;
use crate::features::auth::domain::AuthProvider;

#[component]
pub(crate) fn LoginPanel() -> Element {
    rsx! {
        div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-5 shadow-[0_24px_80px_rgba(0,0,0,0.35)] sm:p-6",
            div { class: "mb-6",
                div { class: "mb-2 text-[10px] uppercase tracking-[0.24em] text-zinc-600", "Авторизация" }
                h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "Войти в CheenHub" }
                p { class: "mt-1.5 text-[13px] leading-5 text-zinc-500", "Используй email и пароль или внешний аккаунт." }
            }

            form { class: "space-y-4",
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
                    autocomplete: "current-password"
                }
                button {
                    r#type: "button",
                    class: "btn-p flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)]",
                    onclick: move |_| show_todo_alert(),
                    "Войти"
                }
            }

            div { class: "my-5 flex items-center gap-3",
                div { class: "h-px flex-1 bg-zinc-800" }
                span { class: "text-[11px] uppercase tracking-[0.18em] text-zinc-600", "или" }
                div { class: "h-px flex-1 bg-zinc-800" }
            }

            div { class: "grid gap-2",
                ProviderButton { provider: AuthProvider::Google }
                ProviderButton { provider: AuthProvider::Discord }
            }

            div { class: "mt-5 rounded-2xl border border-zinc-800 bg-zinc-950/80 px-4 py-3 text-[12px] leading-5 text-zinc-500",
                "Мы бережно относимся к входу в аккаунт: выбирай привычный способ и продолжай общение без лишних шагов."
            }

            div { class: "mt-4 text-center text-[13px] text-zinc-500",
                "Нет аккаунта? "
                Link {
                    to: Route::Register {},
                    class: "font-medium text-zinc-200 transition hover:text-white",
                    "Создать аккаунт"
                }
            }
        }
    }
}
