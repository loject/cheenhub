//! User password settings section.

use cheenhub_contracts::rest::ChangeCurrentUserPasswordRequest;
use dioxus::prelude::*;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::auth::api;

use super::styles::{input_class, primary_button_class};

/// Renders password change controls.
#[component]
pub(crate) fn PasswordSettingsSection() -> Element {
    let current_user_context = use_context::<CurrentUserContext>();
    let current_user = current_user_context.require_user();
    let requires_current_password = current_user.has_password;
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut new_password_confirmation = use_signal(String::new);
    let mut status = use_signal(PasswordChangeStatus::default);
    let current_value = current_password();
    let new_value = new_password();
    let confirmation_value = new_password_confirmation();
    let is_busy = matches!(status(), PasswordChangeStatus::Loading);
    let has_current_password = !requires_current_password || !current_value.is_empty();
    let password_is_new = !requires_current_password || current_value != new_value;
    let is_valid = has_current_password
        && (8..=128).contains(&new_value.chars().count())
        && new_value == confirmation_value
        && password_is_new;

    rsx! {
        div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
            h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50",
                if requires_current_password { "Пароль" } else { "Локальный пароль" }
            }
            p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                if requires_current_password {
                    "После смены пароля мы отправим уведомление на email аккаунта."
                } else {
                    "У аккаунта еще нет локального пароля. Задай новый пароль для входа по email."
                }
            }

            div { class: if requires_current_password { "mt-4 grid gap-3 lg:grid-cols-3" } else { "mt-4 grid gap-3 lg:grid-cols-2" },
                if requires_current_password {
                    label { class: "block",
                        span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Текущий пароль" }
                        input {
                            r#type: "password",
                            value: current_value.clone(),
                            autocomplete: "current-password",
                            disabled: is_busy,
                            class: input_class(),
                            oninput: move |event| {
                                current_password.set(event.value());
                                reset_status(&mut status);
                            },
                        }
                    }
                }
                label { class: "block",
                    span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Новый пароль" }
                    input {
                        r#type: "password",
                        value: new_value.clone(),
                        autocomplete: "new-password",
                        disabled: is_busy,
                        class: input_class(),
                        oninput: move |event| {
                            new_password.set(event.value());
                            reset_status(&mut status);
                        },
                    }
                    p { class: "mt-1.5 text-[11px] leading-4 text-zinc-500", "8-128 символов." }
                }
                label { class: "block",
                    span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Повтор пароля" }
                    input {
                        r#type: "password",
                        value: confirmation_value.clone(),
                        autocomplete: "new-password",
                        disabled: is_busy,
                        class: input_class(),
                        oninput: move |event| {
                            new_password_confirmation.set(event.value());
                            reset_status(&mut status);
                        },
                    }
                    if !confirmation_value.is_empty() && new_value != confirmation_value {
                        p { class: "mt-1.5 text-[11px] leading-4 text-red-300", "Пароли не совпадают." }
                    }
                }
            }

            match status() {
                PasswordChangeStatus::Idle | PasswordChangeStatus::Loading => rsx! {},
                PasswordChangeStatus::Succeeded => rsx! {
                    div { class: "mt-4 rounded-xl border border-emerald-500/25 bg-emerald-500/10 px-3 py-2 text-[12px] text-emerald-200", "Пароль успешно изменен." }
                },
                PasswordChangeStatus::Failed(error) => rsx! {
                    div { class: "mt-4 rounded-xl border border-red-500/25 bg-red-500/10 px-3 py-2 text-[12px] text-red-200", "{error}" }
                },
            }

            div { class: "mt-4 flex justify-end",
                button {
                    r#type: "button",
                    disabled: is_busy || !is_valid,
                    class: primary_button_class(),
                    onclick: move |_| {
                        if is_busy || !is_valid {
                            return;
                        }
                        let request = ChangeCurrentUserPasswordRequest {
                            current_password: current_password(),
                            new_password: new_password(),
                            new_password_confirmation: new_password_confirmation(),
                        };
                        let updated_user = if requires_current_password {
                            None
                        } else {
                            Some(cheenhub_contracts::rest::AuthUser {
                                has_password: true,
                                ..current_user.clone()
                            })
                        };
                        status.set(PasswordChangeStatus::Loading);
                        info!("changing current user password");
                        spawn(async move {
                            match api::change_current_user_password(request).await {
                                Ok(()) => {
                                    info!("current user password changed");
                                    current_password.set(String::new());
                                    new_password.set(String::new());
                                    new_password_confirmation.set(String::new());
                                    status.set(PasswordChangeStatus::Succeeded);
                                    if let Some(updated_user) = updated_user {
                                        current_user_context.set_user(updated_user);
                                    }
                                }
                                Err(error) => {
                                    warn!(%error, "current user password change failed");
                                    status.set(PasswordChangeStatus::Failed(error));
                                }
                            }
                        });
                    },
                    if is_busy {
                        "Сохраняем..."
                    } else if requires_current_password {
                        "Изменить пароль"
                    } else {
                        "Задать пароль"
                    }
                }
            }
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
enum PasswordChangeStatus {
    #[default]
    Idle,
    Loading,
    Succeeded,
    Failed(String),
}

fn reset_status(status: &mut Signal<PasswordChangeStatus>) {
    if !matches!(
        status(),
        PasswordChangeStatus::Idle | PasswordChangeStatus::Loading
    ) {
        status.set(PasswordChangeStatus::Idle);
    }
}
