//! Переключатели разрешённых способов регистрации.

use cheenhub_contracts::rest::{
    HostRegistrationSettingsResponse, UpdateHostRegistrationSettingsRequest,
};
use dioxus::prelude::*;

use super::super::api;

/// Состояние сохранения настроек регистрации.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SaveState {
    Idle,
    Saving,
    Saved,
    Failed,
}

/// Изменяет и сохраняет доступные способы регистрации на хосте.
#[component]
pub(super) fn HostRegistrationSettingsForm(settings: HostRegistrationSettingsResponse) -> Element {
    let mut registration_enabled = use_signal(|| settings.registration_enabled);
    let mut email_password_enabled = use_signal(|| settings.email_password_registration_enabled);
    let mut saved_settings = use_signal(|| settings);
    let mut save_state = use_signal(|| SaveState::Idle);
    let mut error = use_signal(|| None::<String>);
    let changed = registration_enabled() != saved_settings().registration_enabled
        || email_password_enabled() != saved_settings().email_password_registration_enabled;
    let saving = save_state() == SaveState::Saving;

    rsx! {
        div { class: "mt-4 rounded-[18px] border border-zinc-800 bg-zinc-950/70 p-5",
            h2 { class: "text-[15px] font-semibold text-zinc-100", "Регистрация пользователей" }
            p { class: "mt-1.5 max-w-3xl text-pretty text-[13px] leading-5 text-zinc-500",
                "Выбери, могут ли новые пользователи создавать аккаунты на этом хосте."
            }

            label { class: "mt-5 flex cursor-pointer items-start gap-3 rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3",
                input {
                    class: "mt-1 size-4 accent-blue-500",
                    r#type: "checkbox",
                    checked: registration_enabled(),
                    disabled: saving,
                    onchange: move |event| registration_enabled.set(event.checked()),
                }
                span { class: "min-w-0",
                    span { class: "block text-[13px] font-medium text-zinc-100", "Разрешить регистрацию новых пользователей" }
                    span { class: "mt-1 block text-[12px] leading-5 text-zinc-500", "При отключении новые аккаунты нельзя будет создать ни одним способом. Вход существующих пользователей продолжит работать." }
                }
            }

            label { class: "mt-2 flex cursor-pointer items-start gap-3 rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3",
                input {
                    class: "mt-1 size-4 accent-blue-500",
                    r#type: "checkbox",
                    checked: email_password_enabled(),
                    disabled: saving,
                    onchange: move |event| email_password_enabled.set(event.checked()),
                }
                span { class: "min-w-0",
                    span { class: "block text-[13px] font-medium text-zinc-100", "Разрешить регистрацию по email и паролю" }
                    span { class: "mt-1 block text-[12px] leading-5 text-zinc-500", "Если отключить, создание аккаунтов через Google OAuth останется доступно, пока включена регистрация новых пользователей." }
                }
            }

            div { class: "mt-5 flex flex-col gap-3 border-t border-zinc-800 pt-4 sm:flex-row sm:items-center sm:justify-between",
                div { class: "min-h-5 text-pretty text-[12px]",
                    match save_state() {
                        SaveState::Idle => rsx! { span { class: "text-zinc-500", "Настройки применяются к новым регистрациям." } },
                        SaveState::Saving => rsx! { span { class: "text-blue-200", "Сохраняем..." } },
                        SaveState::Saved => rsx! { span { class: "text-emerald-200", "Настройки регистрации обновлены." } },
                        SaveState::Failed => rsx! { span { class: "text-red-200", "{error().unwrap_or_default()}" } },
                    }
                }
                button {
                    r#type: "button",
                    disabled: !changed || saving,
                    class: apply_button_class(changed && !saving),
                    onclick: move |_| {
                        let request = UpdateHostRegistrationSettingsRequest {
                            registration_enabled: registration_enabled(),
                            email_password_registration_enabled: email_password_enabled(),
                        };
                        save_state.set(SaveState::Saving);
                        error.set(None);
                        spawn(async move {
                            match api::update_registration_settings(request).await {
                                Ok(saved) => {
                                    info!(
                                        registration_enabled = saved.registration_enabled,
                                        email_password_registration_enabled = saved.email_password_registration_enabled,
                                        "host registration settings saved"
                                    );
                                    registration_enabled.set(saved.registration_enabled);
                                    email_password_enabled.set(saved.email_password_registration_enabled);
                                    saved_settings.set(saved);
                                    save_state.set(SaveState::Saved);
                                }
                                Err(failure) => {
                                    warn!(message = %failure.message(), "failed to save host registration settings");
                                    error.set(Some(failure.message().to_owned()));
                                    save_state.set(SaveState::Failed);
                                }
                            }
                        });
                    },
                    if saving { "Сохраняем..." } else { "Сохранить настройки" }
                }
            }
        }
    }
}

/// Класс кнопки сохранения настроек регистрации.
fn apply_button_class(enabled: bool) -> &'static str {
    if enabled {
        "inline-flex min-h-11 items-center justify-center rounded-xl bg-accent px-5 text-[13px] font-semibold text-white shadow-[0_8px_28px_rgba(59,130,246,0.18)] transition-[background-color,transform] duration-150 hover:bg-blue-400 active:scale-[0.97]"
    } else {
        "inline-flex min-h-11 cursor-not-allowed items-center justify-center rounded-xl bg-zinc-800 px-5 text-[13px] font-semibold text-zinc-500"
    }
}
