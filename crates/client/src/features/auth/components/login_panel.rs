//! Компонент панели формы входа.

use cheenhub_contracts::rest::LoginRequest;
use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::api;
use crate::features::auth::components::provider_button::ProviderButton;
use crate::features::auth::components::text_input::TextInput;
use crate::features::auth::domain::AuthProvider;

#[component]
pub(crate) fn LoginPanel(password_reset_succeeded: bool) -> Element {
    let navigator = use_navigator();
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut status = use_signal(String::new);
    let mut is_busy = use_signal(|| false);
    let mut show_password = use_signal(|| false);

    rsx! {
        div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-5 shadow-[0_24px_80px_rgba(0,0,0,0.35)] sm:p-6",
            div { class: "mb-6",
                div { class: "mb-2 text-[10px] uppercase tracking-[0.24em] text-zinc-600", "Авторизация" }
                h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "Войти в CheenHub" }
                p { class: "mt-1.5 text-[13px] leading-5 text-zinc-500", "Используй email и пароль или внешний аккаунт." }
            }

            form {
                class: "space-y-4",
                onsubmit: move |event| {
                    event.prevent_default();
                    if is_busy() {
                        return;
                    }

                    is_busy.set(true);
                    status.set(String::new());
                    let request = LoginRequest {
                        email: email(),
                        password: password(),
                    };
                    info!("starting password login");
                    spawn(async move {
                        match api::login(request).await {
                            Ok(_) => {
                                info!("password login succeeded");
                                let _ = navigator.replace(Route::AppHome {});
                            }
                            Err(error) => {
                                warn!(%error, "password login failed");
                                status.set(error);
                                is_busy.set(false);
                            }
                        };
                    });
                },
                if password_reset_succeeded {
                    div {
                        class: "rounded-xl border border-emerald-500/20 bg-emerald-500/10 px-3 py-2.5 text-[12px] leading-5 text-emerald-100",
                        role: "status",
                        aria_live: "polite",
                        p { class: "font-medium", "Пароль обновлён" }
                        p { class: "text-emerald-200/75", "Всё готово — войди с новым паролем." }
                    }
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
                label { class: "block",
                    span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Password" }
                    div { class: "relative",
                        input {
                            r#type: if show_password() { "text" } else { "password" },
                            name: "password",
                            placeholder: "••••••••",
                            autocomplete: "current-password",
                            value: password(),
                            oninput: move |event| password.set(event.value()),
                            class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 pr-10 text-[14px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
                        }
                        button {
                            r#type: "button",
                            class: "absolute right-0 top-0 flex h-11 w-10 items-center justify-center text-zinc-500 transition hover:text-white",
                            "aria-label": if show_password() { "Скрыть пароль" } else { "Показать пароль" },
                            title: if show_password() { "Скрыть пароль" } else { "Показать пароль" },
                            onclick: move |_| show_password.set(!show_password()),
                            svg {
                                class: "h-5 w-5",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.8",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    d: "M2.25 12s3.75-6.75 9.75-6.75S21.75 12 21.75 12 18 18.75 12 18.75 2.25 12 2.25 12Z"
                                }
                                circle { cx: "12", cy: "12", r: "2.75" }
                                if show_password() {
                                    path {
                                        stroke_linecap: "round",
                                        d: "M4 4l16 16"
                                    }
                                }
                            }
                        }
                    }
                }
                if !status().is_empty() {
                    p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                        "{status()}"
                    }
                }
                button {
                    r#type: "submit",
                    disabled: is_busy(),
                    class: "btn-p flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)]",
                    if is_busy() { "Входим..." } else { "Войти" }
                }
            }

            div { class: "mt-3 text-right text-[12px]",
                Link {
                    to: Route::ForgotPassword {},
                    class: "font-medium text-zinc-400 transition hover:text-white",
                    "Забыли пароль?"
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
