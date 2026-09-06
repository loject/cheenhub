//! Форма завершения регистрации после подтверждения аккаунта Google.

use crate::features::auth::api::{self, OAuthCompletion, OAuthRegistrationRequired};
use crate::features::auth::{LegalAcceptanceAction, LegalAcceptanceFields};
use cheenhub_contracts::rest::OAuthRegistrationRequest;
use dioxus::prelude::*;

/// Принимает никнейм и независимые согласия, возвращая завершённую локальную сессию.
#[component]
pub(crate) fn OAuthRegistrationForm(
    registration: OAuthRegistrationRequired,
    on_complete: EventHandler<OAuthCompletion>,
    on_restart: EventHandler<()>,
) -> Element {
    let mut nickname = use_signal(|| registration.suggested_nickname.clone().unwrap_or_default());
    let mut accepts_terms = use_signal(|| false);
    let mut accepts_personal_data = use_signal(|| false);
    let mut form_error = use_signal(String::new);
    let mut is_submitting = use_signal(|| false);
    rsx! {
        div { class: "space-y-4",
            div {
                h1 { class: "text-balance text-[20px] font-semibold tracking-[-0.04em] text-zinc-50", "Выбери никнейм" }
                p { class: "mt-2 text-pretty text-[13px] leading-5 text-zinc-400", "Google подтвердил аккаунт. Осталось выбрать имя в CheenHub." }
            }
            label { class: "block",
                span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Никнейм" }
                input {
                    r#type: "text", value: nickname(), maxlength: "32", autocomplete: "nickname", disabled: is_submitting(),
                    oninput: move |event| nickname.set(event.value()),
                    class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition-[border-color,box-shadow] focus:border-accent focus:ring-2 focus:ring-accent/20",
                }
            }
            LegalAcceptanceFields {
                accepts_terms: accepts_terms(), accepts_personal_data: accepts_personal_data(),
                on_change: move |action| match action {
                    LegalAcceptanceAction::TermsChanged(value) => accepts_terms.set(value),
                    LegalAcceptanceAction::PersonalDataChanged(value) => accepts_personal_data.set(value),
                }
            }
            if !form_error().is_empty() {
                p { role: "alert", class: "rounded-xl bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200", "{form_error()}" }
            }
            button {
                r#type: "button",
                class: "flex h-11 w-full items-center justify-center gap-2 rounded-xl bg-accent px-4 text-[13px] font-semibold text-white transition-transform active:not-disabled:scale-[0.96] disabled:cursor-not-allowed disabled:opacity-60",
                disabled: is_submitting() || nickname().trim().is_empty() || !accepts_terms() || !accepts_personal_data(),
                onclick: move |_| {
                    if is_submitting() || !accepts_terms() || !accepts_personal_data() { return; }
                    let request = OAuthRegistrationRequest {
                        registration_token: registration.registration_token.clone(), nickname: nickname().trim().to_owned(),
                        accepts_terms: accepts_terms(), accepts_personal_data: accepts_personal_data(),
                    };
                    form_error.set(String::new());
                    is_submitting.set(true);
                    info!("submitting Google account registration");
                    spawn(async move {
                        match complete_registration(request).await {
                            Ok(completion) => { info!("Google account registration completed"); on_complete.call(completion); }
                            Err(error) => {
                                warn!("Google account registration failed"); form_error.set(error); is_submitting.set(false);
                            }
                        }
                    });
                },
                if is_submitting() {
                    span { aria_hidden: "true", class: "h-4 w-4 animate-spin rounded-full border-2 border-white/30 border-t-white" }
                    "Создаём аккаунт..."
                } else { "Продолжить" }
            }
            button {
                r#type: "button",
                class: "flex h-11 w-full items-center justify-center rounded-xl border border-zinc-700 px-4 text-[13px] font-medium text-zinc-300 transition-colors hover:not-disabled:bg-zinc-800 disabled:cursor-not-allowed disabled:opacity-60",
                disabled: is_submitting(),
                onclick: move |_| {
                    if is_submitting() { return; }
                    info!("Google account registration restart requested");
                    on_restart.call(());
                },
                "Начать заново"
            }
        }
    }
}

async fn complete_registration(
    request: OAuthRegistrationRequest,
) -> Result<OAuthCompletion, String> {
    use futures_util::future::{Either, select};
    let submission = api::register_with_google_oauth(request);
    let deadline = crate::features::runtime::sleep_duration(std::time::Duration::from_secs(20));
    futures_util::pin_mut!(submission, deadline);
    match select(submission, deadline).await {
        Either::Left((result, _)) => result,
        Either::Right(_) => {
            Err("Сервер не ответил вовремя. Попробуй продолжить ещё раз.".to_owned())
        }
    }
}
