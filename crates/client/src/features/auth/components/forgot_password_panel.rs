//! Password reset request form panel component.

use cheenhub_contracts::rest::PasswordResetRequest;
use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::api;
use crate::features::auth::components::text_input::TextInput;

#[component]
pub(crate) fn ForgotPasswordPanel() -> Element {
    let mut email = use_signal(String::new);
    let mut status = use_signal(PasswordResetRequestStatus::default);
    let is_busy = matches!(status(), PasswordResetRequestStatus::Loading);
    let is_success = matches!(status(), PasswordResetRequestStatus::Succeeded);

    rsx! {
        div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-5 shadow-[0_24px_80px_rgba(0,0,0,0.35)] sm:p-6",
            div { class: "mb-6",
                div { class: "mb-2 text-[10px] uppercase tracking-[0.24em] text-zinc-600", "Восстановление" }
                h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "Сбросить пароль" }
                p { class: "mt-1.5 text-[13px] leading-5 text-zinc-500", "Укажи email аккаунта, и мы отправим ссылку для смены пароля." }
            }

            form { class: "space-y-4",
                TextInput {
                    input_type: "email",
                    label: "Email",
                    name: "email",
                    placeholder: "you@example.com",
                    autocomplete: "email",
                    value: email(),
                    oninput: move |value| {
                        email.set(value);
                        if is_success {
                            status.set(PasswordResetRequestStatus::Idle);
                        }
                    }
                }
                match status() {
                    PasswordResetRequestStatus::Idle | PasswordResetRequestStatus::Loading => rsx! {},
                    PasswordResetRequestStatus::Succeeded => rsx! {
                        p { class: "rounded-xl border border-emerald-500/20 bg-emerald-500/10 px-3 py-2 text-[12px] leading-5 text-emerald-100",
                            "Если такой email зарегистрирован, письмо со ссылкой уже отправлено."
                        }
                    },
                    PasswordResetRequestStatus::Failed(error) => rsx! {
                        p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                            "{error}"
                        }
                    },
                }
                button {
                    r#type: "button",
                    disabled: is_busy || email().trim().is_empty(),
                    class: "btn-p flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] disabled:cursor-not-allowed disabled:opacity-60",
                    onclick: move |_| {
                        if is_busy {
                            return;
                        }
                        let request = PasswordResetRequest {
                            email: email().trim().to_owned(),
                        };
                        status.set(PasswordResetRequestStatus::Loading);
                        info!("requesting password reset email");
                        spawn(async move {
                            match api::request_password_reset(request).await {
                                Ok(()) => {
                                    info!("password reset email request accepted");
                                    status.set(PasswordResetRequestStatus::Succeeded);
                                }
                                Err(error) => {
                                    warn!(%error, "password reset email request failed");
                                    status.set(PasswordResetRequestStatus::Failed(error));
                                }
                            }
                        });
                    },
                    if is_busy { "Отправляем..." } else { "Отправить ссылку" }
                }
            }

            div { class: "mt-5 rounded-2xl border border-zinc-800 bg-zinc-950/80 px-4 py-3 text-[12px] leading-5 text-zinc-500",
                "Ссылка работает ограниченное время. Если письмо не пришло, проверь папку со спамом или отправь запрос еще раз."
            }

            div { class: "mt-4 text-center text-[13px] text-zinc-500",
                "Вспомнил пароль? "
                Link {
                    to: Route::Login {},
                    class: "font-medium text-zinc-200 transition hover:text-white",
                    "Вернуться ко входу"
                }
            }
        }
    }
}

#[derive(Clone, Default, PartialEq)]
enum PasswordResetRequestStatus {
    #[default]
    Idle,
    Loading,
    Succeeded,
    Failed(String),
}
