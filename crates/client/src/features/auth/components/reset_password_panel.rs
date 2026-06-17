//! Компонент панели формы подтверждения сброса пароля.

use cheenhub_contracts::rest::PasswordResetConfirmRequest;
use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::api;
use crate::features::auth::components::text_input::TextInput;

#[component]
pub(crate) fn ResetPasswordPanel(token: Option<String>) -> Element {
    let mut password = use_signal(String::new);
    let mut repeated_password = use_signal(String::new);
    let mut status = use_signal(PasswordResetConfirmStatus::default);
    let token = token.unwrap_or_default();
    let has_token = !token.trim().is_empty();
    let is_busy = matches!(status(), PasswordResetConfirmStatus::Loading);
    let is_success = matches!(status(), PasswordResetConfirmStatus::Succeeded);

    rsx! {
        div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-5 shadow-[0_24px_80px_rgba(0,0,0,0.35)] sm:p-6",
            div { class: "mb-6",
                div { class: "mb-2 text-[10px] uppercase tracking-[0.24em] text-zinc-600", "Новый пароль" }
                h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "Задать новый пароль" }
                p { class: "mt-1.5 text-[13px] leading-5 text-zinc-500", "Придумай новый пароль для аккаунта CheenHub." }
            }

            if has_token {
                form { class: "space-y-4",
                    TextInput {
                        input_type: "password",
                        label: "Новый пароль",
                        name: "password",
                        placeholder: "••••••••",
                        autocomplete: "new-password",
                        value: password(),
                        oninput: move |value| password.set(value)
                    }
                    TextInput {
                        input_type: "password",
                        label: "Повтори пароль",
                        name: "repeated-password",
                        placeholder: "••••••••",
                        autocomplete: "new-password",
                        value: repeated_password(),
                        oninput: move |value| repeated_password.set(value)
                    }
                    match status() {
                        PasswordResetConfirmStatus::Idle | PasswordResetConfirmStatus::Loading => rsx! {},
                        PasswordResetConfirmStatus::Succeeded => rsx! {
                            p { class: "rounded-xl border border-emerald-500/20 bg-emerald-500/10 px-3 py-2 text-[12px] leading-5 text-emerald-100",
                                "Пароль обновлен. Теперь можно войти с новым паролем."
                            }
                        },
                        PasswordResetConfirmStatus::Failed(error) => rsx! {
                            p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                                "{error}"
                            }
                        },
                    }
                    if is_success {
                        Link {
                            to: Route::Login {},
                            class: "flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white",
                            "Войти"
                        }
                    } else {
                        button {
                            r#type: "button",
                            disabled: is_busy || password().is_empty() || repeated_password().is_empty(),
                            class: "btn-p flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] disabled:cursor-not-allowed disabled:opacity-60",
                            onclick: move |_| {
                                if is_busy {
                                    return;
                                }
                                let new_password = password();
                                if new_password != repeated_password() {
                                    status.set(PasswordResetConfirmStatus::Failed("Пароли не совпадают.".to_owned()));
                                    return;
                                }
                                let request = PasswordResetConfirmRequest {
                                    token: token.clone(),
                                    new_password,
                                };
                                status.set(PasswordResetConfirmStatus::Loading);
                                info!("confirming password reset token");
                                spawn(async move {
                                    match api::confirm_password_reset(request).await {
                                        Ok(()) => {
                                            info!("password reset token confirmed");
                                            status.set(PasswordResetConfirmStatus::Succeeded);
                                            password.set(String::new());
                                            repeated_password.set(String::new());
                                        }
                                        Err(error) => {
                                            warn!(%error, "password reset confirmation failed");
                                            status.set(PasswordResetConfirmStatus::Failed(error));
                                        }
                                    }
                                });
                            },
                            if is_busy { "Сохраняем..." } else { "Сохранить пароль" }
                        }
                    }
                }
            } else {
                div { class: "rounded-2xl border border-amber-500/20 bg-amber-500/10 px-4 py-3 text-[12px] leading-5 text-amber-100",
                    "Ссылка для сброса пароля неполная. Запроси новое письмо и открой ссылку из него."
                }
                Link {
                    to: Route::ForgotPassword {},
                    class: "mt-4 flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white",
                    "Запросить новую ссылку"
                }
            }

            div { class: "mt-5 rounded-2xl border border-zinc-800 bg-zinc-950/80 px-4 py-3 text-[12px] leading-5 text-zinc-500",
                "После смены пароля используй его для обычного входа по email."
            }
        }
    }
}

#[derive(Clone, Default, PartialEq)]
enum PasswordResetConfirmStatus {
    #[default]
    Idle,
    Loading,
    Succeeded,
    Failed(String),
}
