//! Кнопка входа в desktop через системный браузер, не затрагивающая Android и web.

use dioxus::prelude::*;

#[cfg(all(feature = "desktop", any(target_os = "linux", target_os = "windows")))]
use crate::Route;
#[cfg(all(feature = "desktop", any(target_os = "linux", target_os = "windows")))]
use crate::features::auth::browser_login::{self, Progress};
#[cfg(all(feature = "desktop", any(target_os = "linux", target_os = "windows")))]
use dioxus::core::Task;

/// Показывает кнопку и ход попытки только на поддерживаемых desktop-платформах.
#[cfg(all(feature = "desktop", any(target_os = "linux", target_os = "windows")))]
#[component]
pub(super) fn BrowserLoginButton() -> Element {
    let navigator = use_navigator();
    let mut task = use_signal(|| None::<Task>);
    let mut progress = use_signal(|| Progress::Opening);
    let mut status = use_signal(String::new);
    let busy = task().is_some();

    rsx! {
        div { class: "mb-5 space-y-3 rounded-2xl border border-blue-500/15 bg-blue-500/5 p-3",
            button {
                r#type: "button", disabled: busy,
                class: "flex min-h-11 w-full items-center justify-center gap-2 rounded-xl bg-accent px-4 text-[13px] font-semibold text-white transition hover:brightness-110 disabled:cursor-wait disabled:opacity-70",
                onclick: move |_| {
                    if task().is_some() { return; }
                    progress.set(Progress::Opening);
                    status.set(String::new());
                    let pending = spawn(async move {
                        let result = browser_login::authenticate(move |stage| progress.set(stage)).await;
                        task.set(None);
                        progress.set(Progress::Opening);
                        match result {
                            Ok(_) => { let _ = navigator.replace(Route::AppHome {}); }
                            Err(message) => { status.set(message); }
                        }
                    });
                    task.set(Some(pending));
                },
                if busy {
                    span { aria_hidden: "true", class: "h-4 w-4 animate-spin rounded-full border-2 border-white/30 border-t-white" }
                    if progress() == Progress::Completing { "Завершаем вход..." }
                    else if matches!(progress(), Progress::Waiting(_)) { "Ждём подтверждения..." }
                    else { "Открываем браузер..." }
                } else { "Войти через браузер" }
            }
            if busy {
                div { role: "status", class: "space-y-2 px-1 text-center",
                    if let Progress::Waiting(code) = progress() {
                        p { class: "text-xs leading-5 text-zinc-300", "Подтверди вход на открывшейся странице CheenHub. Сверь код:" }
                        p { class: "font-mono text-xl font-semibold tracking-[0.14em] text-blue-200", "{code}" }
                    } else {
                        p { class: "text-xs leading-5 text-zinc-400", "Ожидаем ответа сервера..." }
                    }
                    button {
                        r#type: "button", disabled: progress() == Progress::Completing,
                        class: "min-h-10 rounded-xl px-3 text-xs font-medium text-zinc-300 transition hover:bg-zinc-800 disabled:opacity-50",
                        onclick: move |_| {
                            if progress() == Progress::Completing { return; }
                            if let Some(pending) = task.take() { pending.cancel(); }
                            progress.set(Progress::Opening);
                            status.set("Вход отменён. Можно попробовать снова.".to_owned());
                        },
                        "Отменить"
                    }
                }
            }
            if !status().is_empty() {
                p { role: "alert", class: "text-pretty px-1 text-xs leading-5 text-red-300", "{status()}" }
            }
        }
    }
}

/// На остальных платформах новый вход не отображается.
#[cfg(not(all(feature = "desktop", any(target_os = "linux", target_os = "windows"))))]
#[component]
pub(super) fn BrowserLoginButton() -> Element {
    rsx! {}
}
