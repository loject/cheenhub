//! Страница явного разрешения входа настольного CheenHub через текущую веб-сессию.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::Route;
use crate::features::app::components::avatar::UserAvatar;
use crate::features::auth::{api, browser_handoff};

/// Проверяет код и показывает единственный текущий аккаунт перед передачей входа.
#[component]
pub(crate) fn BrowserDesktopConfirm(attempt: Option<String>) -> Element {
    let navigator = use_navigator();
    let attempt_id = attempt
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok())
        .or_else(browser_handoff::pending);
    let mut is_busy = use_signal(|| false);
    let mut is_approved = use_signal(|| false);
    let mut error = use_signal(String::new);

    use_effect(move || {
        if let Some(attempt_id) = attempt_id {
            browser_handoff::remember(attempt_id);
            if !api::has_tokens() {
                let _ = navigator.replace(Route::Login {
                    password_reset: None,
                });
            }
        }
    });

    let confirmation = use_resource(move || async move {
        let attempt_id =
            attempt_id.ok_or_else(|| "Ссылка подтверждения некорректна.".to_owned())?;
        let details = browser_handoff::details(attempt_id).await?;
        let user = api::current_user().await?;
        Ok::<_, String>((details, user))
    });
    let result = confirmation.read().clone();

    rsx! {
        main { class: "grid min-h-screen place-items-center bg-zinc-950 px-4 py-10 text-zinc-100",
            section { class: "w-full max-w-md space-y-5 rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-6 shadow-[0_24px_80px_rgba(0,0,0,0.35)]",
                div { class: "space-y-2",
                    p { class: "text-[11px] font-semibold uppercase tracking-[0.2em] text-blue-300", "Безопасный вход" }
                    h1 { class: "text-2xl font-semibold tracking-tight", "Войти в приложение CheenHub?" }
                    p { class: "text-sm leading-6 text-zinc-400", "Подтверди вход только если ты сам открыл CheenHub на компьютере. Сверь проверочный код в браузере и приложении." }
                }
                if is_approved() {
                    div { role: "status", class: "space-y-3 rounded-2xl border border-emerald-700/40 bg-emerald-950/30 p-4",
                        p { class: "font-semibold text-emerald-200", "Вход подтверждён" }
                        p { class: "text-sm leading-6 text-emerald-100/80", "Возвращайся в приложение на компьютере. Оно автоматически завершит вход. Вкладку можно закрыть." }
                    }
                } else {
                    match result {
                        Some(Ok((details, user))) => rsx! {
                            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                                p { class: "mb-3 text-xs text-zinc-500", "Ты входишь как" }
                                div { class: "flex items-center gap-3",
                                    UserAvatar { nickname: user.nickname.clone(), avatar_url: user.avatar_url.clone(), class: "h-12 w-12 rounded-2xl".to_owned(), avatar_seed: Some(user.id.clone()) }
                                    div { class: "min-w-0",
                                        p { class: "truncate text-sm font-semibold text-zinc-100", "{user.nickname}" }
                                        p { class: "truncate text-xs text-zinc-400", "{browser_handoff::mask_email(&user.email)}" }
                                    }
                                }
                            }
                            div { class: "rounded-2xl border border-blue-500/25 bg-blue-500/10 p-4 text-center",
                                p { class: "text-xs text-blue-200/75", "Код на обоих экранах должен совпасть" }
                                p { class: "mt-2 font-mono text-2xl font-semibold tracking-[0.18em] text-blue-100", "{details.verification_code}" }
                            }
                            if !error().is_empty() {
                                p { role: "alert", class: "text-sm text-red-300", "{error()}" }
                            }
                            button {
                                r#type: "button", disabled: is_busy(),
                                class: "flex min-h-11 w-full items-center justify-center rounded-xl bg-accent px-4 text-sm font-semibold text-white transition disabled:opacity-60",
                                onclick: move |_| {
                                    let Some(attempt_id) = attempt_id else { return; };
                                    is_busy.set(true);
                                    error.set(String::new());
                                    spawn(async move {
                                        match browser_handoff::approve(attempt_id).await {
                                            Ok(()) => { browser_handoff::clear(); is_approved.set(true); }
                                            Err(message) => { error.set(message); is_busy.set(false); }
                                        }
                                    });
                                },
                                if is_busy() { "Подтверждаем..." } else { "Подтвердить вход" }
                            }
                            button {
                                r#type: "button", disabled: is_busy(),
                                class: "min-h-11 w-full rounded-xl border border-zinc-700 px-4 text-sm font-medium text-zinc-300 transition hover:bg-zinc-800 disabled:opacity-50",
                                onclick: move |_| {
                                    browser_handoff::clear();
                                    let _ = navigator.replace(Route::AppHome {});
                                },
                                "Это не я"
                            }
                        },
                        Some(Err(message)) => rsx! {
                            p { role: "alert", class: "rounded-xl bg-red-500/10 p-4 text-sm text-red-200", "{message}" }
                            button { r#type: "button", class: "min-h-11 w-full rounded-xl border border-zinc-700 px-4 text-sm text-zinc-200",
                                onclick: move |_| {
                                    browser_handoff::clear();
                                    let _ = navigator.replace(if api::has_tokens() { Route::AppHome {} } else { Route::Login { password_reset: None } });
                                },
                                "Вернуться в CheenHub"
                            }
                        },
                        None => rsx! {
                            div { role: "status", class: "flex items-center gap-3 text-sm text-zinc-400",
                                span { aria_hidden: "true", class: "h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-300" }
                                "Проверяем запрос на вход..."
                            }
                        },
                    }
                }
            }
        }
    }
}
