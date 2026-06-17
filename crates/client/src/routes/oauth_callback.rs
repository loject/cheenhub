//! Компонент маршрута OAuth callback.

use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::api::{self, OAuthCompletion, OAuthRegistrationRequired};

/// Обрабатывает callback-ответы OAuth-провайдера.
#[component]
pub(crate) fn OAuthCallback(
    code: Option<String>,
    handoff_code: Option<String>,
    error: Option<String>,
) -> Element {
    let navigator = use_navigator();
    let mut state = use_signal(|| OAuthCallbackState::Loading);
    let mut started = use_signal(|| false);
    let mut nickname = use_signal(String::new);
    let mut accepts_policies = use_signal(|| false);
    let mut form_error = use_signal(String::new);
    let mut is_submitting = use_signal(|| false);
    let handoff = handoff_code.or(code).unwrap_or_default();
    let has_session = api::has_tokens();
    let checkbox_class = if accepts_policies() {
        "flex h-5 w-5 shrink-0 items-center justify-center rounded-lg bg-accent text-white transition"
    } else {
        "flex h-5 w-5 shrink-0 items-center justify-center rounded-lg bg-zinc-800 text-transparent transition hover:bg-zinc-700"
    };

    let effect_handoff = handoff.clone();
    use_effect(move || {
        if started() {
            return;
        }
        started.set(true);

        if let Some(error) = error.clone() {
            let message = oauth_callback_error_message(&error);
            warn!(%message, "google oauth callback returned error");
            state.set(OAuthCallbackState::Failed(message));
            return;
        }

        if effect_handoff.trim().is_empty() {
            state.set(OAuthCallbackState::Failed(
                "Google не вернул код для завершения входа.".to_owned(),
            ));
            return;
        }

        let handoff_code = effect_handoff.clone();
        spawn(async move {
            let result = if api::has_tokens() {
                api::complete_google_account_link(handoff_code)
                    .await
                    .map(|_| OAuthCompletion::Linked)
            } else {
                api::complete_google_oauth(handoff_code, None).await
            };

            match result {
                Ok(OAuthCompletion::Authenticated(_)) | Ok(OAuthCompletion::Linked) => {
                    let _ = navigator.replace(Route::AppHome {});
                }
                Ok(OAuthCompletion::RegistrationRequired(registration)) => {
                    if let Some(suggested) = registration.suggested_nickname.clone() {
                        nickname.set(suggested);
                    }
                    state.set(OAuthCallbackState::RegistrationRequired(registration));
                }
                Err(error) => state.set(OAuthCallbackState::Failed(error)),
            }
        });
    });

    rsx! {
        main { class: "grid min-h-screen place-items-center bg-zinc-950 px-4 py-8 text-zinc-100",
            section { class: "w-full max-w-md rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-5 shadow-[0_24px_80px_rgba(0,0,0,0.35)] sm:p-6",
                match state() {
                    OAuthCallbackState::Loading => rsx! {
                        div { class: "flex items-center gap-3",
                            div { class: "h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-300" }
                            div {
                                h1 { class: "text-[18px] font-semibold tracking-[-0.03em] text-zinc-50", "Завершаем вход через Google" }
                                p { class: "mt-1 text-[13px] leading-5 text-zinc-500", "Проверяем ответ Google и открываем CheenHub." }
                            }
                        }
                    },
                    OAuthCallbackState::RegistrationRequired(registration) => rsx! {
                        h1 { class: "text-[20px] font-semibold tracking-[-0.04em] text-zinc-50", "Выбери никнейм" }
                        p { class: "mt-2 text-[13px] leading-5 text-zinc-500",
                            if let Some(email) = registration.email {
                                "Google подтвердил {email}. Осталось выбрать имя в CheenHub."
                            } else {
                                "Google подтвердил аккаунт. Осталось выбрать имя в CheenHub."
                            }
                        }
                        form { class: "mt-5 space-y-4",
                            label { class: "block",
                                span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Никнейм" }
                                input {
                                    r#type: "text",
                                    value: nickname(),
                                    maxlength: "32",
                                    autocomplete: "nickname",
                                    oninput: move |event| nickname.set(event.value()),
                                    class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent focus:ring-2 focus:ring-accent/20",
                                }
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
                            if !form_error().is_empty() {
                                p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200", "{form_error()}" }
                            }
                            button {
                                r#type: "button",
                                class: "flex h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white disabled:cursor-not-allowed disabled:opacity-60",
                                disabled: is_submitting() || nickname().trim().is_empty() || !accepts_policies(),
                                onclick: move |_| {
                                    if !accepts_policies() {
                                        form_error.set("Нужно принять правила сервиса.".to_owned());
                                        return;
                                    }
                                    if is_submitting() {
                                        return;
                                    }
                                    let handoff_code = handoff.clone();
                                    let chosen_nickname = nickname().trim().to_owned();
                                    form_error.set(String::new());
                                    is_submitting.set(true);
                                    spawn(async move {
                                        match api::complete_google_oauth(handoff_code, Some(chosen_nickname)).await {
                                            Ok(OAuthCompletion::Authenticated(_)) => {
                                                let _ = navigator.replace(Route::AppHome {});
                                            }
                                            Ok(OAuthCompletion::RegistrationRequired(registration)) => {
                                                is_submitting.set(false);
                                                state.set(OAuthCallbackState::RegistrationRequired(registration));
                                            }
                                            Ok(OAuthCompletion::Linked) => {
                                                let _ = navigator.replace(Route::AppHome {});
                                            }
                                            Err(error) => {
                                                is_submitting.set(false);
                                                form_error.set(error);
                                            }
                                        }
                                    });
                                },
                                if is_submitting() { "Создаем..." } else { "Продолжить" }
                            }
                        }
                    },
                    OAuthCallbackState::Failed(error) => rsx! {
                        h1 { class: "text-[20px] font-semibold tracking-[-0.04em] text-zinc-50", "Не удалось войти через Google" }
                        p { class: "mt-2 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200", "{error}" }
                        div { class: "mt-5 flex flex-col gap-2 sm:flex-row",
                            if has_session {
                                button {
                                    r#type: "button",
                                    class: "flex h-10 flex-1 items-center justify-center rounded-xl bg-accent px-3 text-[12px] font-semibold text-white",
                                    onclick: move |_| {
                                        let _ = navigator.replace(Route::AppHome {});
                                    },
                                    "Открыть CheenHub"
                                }
                            } else {
                                button {
                                    r#type: "button",
                                    class: "flex h-10 flex-1 items-center justify-center rounded-xl bg-accent px-3 text-[12px] font-semibold text-white",
                                    onclick: move |_| {
                                        let _ = navigator.replace(Route::Login {});
                                    },
                                    "Вернуться ко входу"
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum OAuthCallbackState {
    Loading,
    RegistrationRequired(OAuthRegistrationRequired),
    Failed(String),
}

fn oauth_callback_error_message(error: &str) -> String {
    let error = error.replace('+', " ");
    let error = error.trim().trim_end_matches('.');
    if error.is_empty() {
        return "Google не завершил вход. Попробуй еще раз.".to_owned();
    }

    format!("Google не завершил вход: {error}.")
}
