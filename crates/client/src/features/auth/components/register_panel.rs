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
    let mut password_confirmation = use_signal(String::new);
    let mut accepts_terms = use_signal(|| false);
    let mut accepts_personal_data = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut is_busy = use_signal(|| false);
    let nickname_error = nickname_validation_error(&nickname());
    let email_error = email_validation_error(&email());
    let password_error = password_validation_error(&password());
    let password_confirmation_error =
        password_confirmation_validation_error(&password(), &password_confirmation());
    let is_ready = nickname_error.is_none()
        && email_error.is_none()
        && password_error.is_none()
        && password_confirmation_error.is_none()
        && !password_confirmation().is_empty()
        && accepts_terms()
        && accepts_personal_data();

    rsx! {
        div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-5 shadow-[0_24px_80px_rgba(0,0,0,0.35)] sm:p-6",
            div { class: "mb-6",
                div { class: "mb-2 text-[10px] uppercase tracking-[0.24em] text-zinc-600", "Регистрация" }
                h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "Создать аккаунт" }
                p { class: "mt-1.5 text-[13px] leading-5 text-zinc-500", "Укажи адрес электронной почты и пароль, чтобы начать пользоваться CheenHub." }
            }

            form { class: "space-y-4",
                TextInput {
                    input_type: "text",
                    label: "Никнейм",
                    name: "nickname",
                    placeholder: "cheenhero",
                    autocomplete: "nickname",
                    value: nickname(),
                    oninput: move |value| {
                        nickname.set(value);
                        status.set(String::new());
                    }
                }
                if !nickname().is_empty() {
                    if let Some(error) = nickname_error {
                        p { class: "-mt-2 text-[12px] leading-5 text-red-300", "{error}" }
                    }
                }
                TextInput {
                    input_type: "email",
                    label: "Адрес электронной почты",
                    name: "email",
                    placeholder: "you@example.com",
                    autocomplete: "email",
                    value: email(),
                    oninput: move |value| {
                        email.set(value);
                        status.set(String::new());
                    }
                }
                if !email().is_empty() {
                    if let Some(error) = email_error {
                        p { class: "-mt-2 text-[12px] leading-5 text-red-300", "{error}" }
                    }
                }
                TextInput {
                    input_type: "password",
                    label: "Пароль",
                    name: "password",
                    placeholder: "••••••••",
                    autocomplete: "new-password",
                    value: password(),
                    oninput: move |value| {
                        password.set(value);
                        status.set(String::new());
                    }
                }
                if !password().is_empty() {
                    if let Some(error) = password_error {
                        p { class: "-mt-2 text-[12px] leading-5 text-red-300", "{error}" }
                    }
                }
                TextInput {
                    input_type: "password",
                    label: "Подтвердите пароль",
                    name: "password-confirmation",
                    placeholder: "••••••••",
                    autocomplete: "new-password",
                    value: password_confirmation(),
                    oninput: move |value| {
                        password_confirmation.set(value);
                        status.set(String::new());
                    }
                }
                if !password_confirmation().is_empty() {
                    if let Some(error) = password_confirmation_error {
                        p { class: "-mt-2 text-[12px] leading-5 text-red-300", "{error}" }
                    }
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
                    disabled: is_busy() || !is_ready,
                    class: "btn-p flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] disabled:cursor-not-allowed disabled:opacity-60",
                    onclick: move |_| {
                        let nickname_value = nickname();
                        let email_value = email();
                        let password_value = password();
                        let password_confirmation_value = password_confirmation();
                        let Some(error) = registration_input_error(
                            &nickname_value,
                            &email_value,
                            &password_value,
                            &password_confirmation_value,
                        ) else {
                            if !accepts_terms() || !accepts_personal_data() {
                                status.set("Подтвердите оба обязательных документа.".to_owned());
                                return;
                            }
                            is_busy.set(true);
                            status.set(String::new());
                            let request = RegisterRequest {
                                nickname: nickname_value.trim().to_owned(),
                                email: email_value.trim().to_owned(),
                                password: password_value,
                                accepts_terms: accepts_terms(),
                                accepts_personal_data: accepts_personal_data(),
                            };
                            info!("starting email registration");
                            spawn(async move {
                                match api::register(request).await {
                                    Ok(_) => {
                                        info!("email registration succeeded");
                                        let _ = navigator.replace(Route::AppHome {});
                                    }
                                    Err(error) => {
                                        warn!(%error, "email registration failed");
                                        status.set(error);
                                        is_busy.set(false);
                                    }
                                };
                            });
                            return;
                        };

                        warn!(reason = error, "registration blocked by client validation");
                        status.set(error.to_owned());
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
                    to: Route::Login { password_reset: None },
                    class: "font-medium text-zinc-200 transition hover:text-white",
                    "Войти"
                }
            }
        }
    }
}

/// Возвращает причину, по которой данные регистрации пока нельзя отправлять.
fn registration_input_error(
    nickname: &str,
    email: &str,
    password: &str,
    password_confirmation: &str,
) -> Option<&'static str> {
    nickname_validation_error(nickname)
        .or_else(|| email_validation_error(email))
        .or_else(|| password_validation_error(password))
        .or_else(|| password_confirmation_validation_error(password, password_confirmation))
}

/// Проверяет никнейм по правилам регистрации.
fn nickname_validation_error(nickname: &str) -> Option<&'static str> {
    let nickname = nickname.trim();
    let length = nickname.chars().count();
    if (3..=32).contains(&length)
        && nickname
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        None
    } else {
        Some("Никнейм должен содержать от 3 до 32 латинских букв, цифр или знаков _.")
    }
}

/// Проверяет формат адреса электронной почты по правилам регистрации.
fn email_validation_error(email: &str) -> Option<&'static str> {
    let email = email.trim();
    let Some((local, domain)) = email.split_once('@') else {
        return Some("Укажите корректный адрес электронной почты.");
    };

    if local.is_empty()
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || email.len() > 320
    {
        Some("Укажите корректный адрес электронной почты.")
    } else {
        None
    }
}

/// Проверяет длину пароля по правилам регистрации.
fn password_validation_error(password: &str) -> Option<&'static str> {
    if (8..=128).contains(&password.chars().count()) {
        None
    } else {
        Some("Пароль должен содержать от 8 до 128 символов.")
    }
}

/// Проверяет совпадение пароля с его подтверждением.
fn password_confirmation_validation_error(
    password: &str,
    password_confirmation: &str,
) -> Option<&'static str> {
    if password == password_confirmation {
        None
    } else {
        Some("Пароли не совпадают.")
    }
}

#[cfg(test)]
mod tests {
    use super::registration_input_error;

    #[test]
    fn accepts_registration_fields_matching_server_rules() {
        assert_eq!(
            registration_input_error("cheen_hero", "hero@example.com", "password1", "password1"),
            None
        );
    }

    #[test]
    fn rejects_unusable_registration_fields() {
        assert!(
            registration_input_error("ab", "hero@example.com", "password1", "password1").is_some()
        );
        assert!(registration_input_error("cheen_hero", "hero", "password1", "password1").is_some());
        assert!(
            registration_input_error("cheen_hero", "hero@example.com", "short", "short").is_some()
        );
        assert!(
            registration_input_error("cheen_hero", "hero@example.com", "password1", "password2")
                .is_some()
        );
    }
}
