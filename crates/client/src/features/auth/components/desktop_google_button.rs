//! Запуск, ожидание и отмена Google OAuth внутри desktop-клиента.

use crate::features::auth::api::OAuthCompletion;
use crate::features::auth::desktop_oauth;
use cheenhub_contracts::rest::OAuthFlow;
use dioxus::core::Task;
use dioxus::prelude::*;

/// Показывает управление попыткой; родитель обрабатывает её завершённый результат.
#[component]
pub(crate) fn DesktopGoogleButton(
    flow: OAuthFlow,
    on_complete: EventHandler<OAuthCompletion>,
) -> Element {
    let mut task = use_signal(|| None::<Task>);
    let mut progress = use_signal(|| desktop_oauth::Progress::Opening);
    let mut status = use_signal(String::new);
    let busy = task().is_some();
    rsx! {
        div { class: "space-y-2",
            button {
                r#type: "button", disabled: busy,
                class: "flex min-h-11 w-full items-center justify-center gap-3 rounded-xl bg-zinc-950 px-3 text-[13px] font-medium text-zinc-200 shadow-[0_0_0_1px_rgba(255,255,255,0.08)] transition-[background-color,scale] hover:bg-zinc-900 active:not-disabled:scale-[0.96] disabled:cursor-wait",
                onclick: move |_| {
                    if task().is_some() { return; }
                    progress.set(desktop_oauth::Progress::Opening);
                    status.set(String::new());
                    let pending = spawn(async move {
                        let result = desktop_oauth::authenticate(flow, move |stage| progress.set(stage)).await;
                        task.set(None);
                        progress.set(desktop_oauth::Progress::Opening);
                        match result {
                            Ok(completion) => on_complete.call(completion),
                            Err(error) => { warn!(?flow, "desktop Google OAuth did not complete"); status.set(error); }
                        }
                    });
                    task.set(Some(pending));
                },
                if busy {
                    span { aria_hidden: "true", class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300" }
                    if progress() == desktop_oauth::Progress::Completing { "Завершаем вход..." }
                    else if progress() == desktop_oauth::Progress::Waiting { "Ожидаем Google..." } else { "Открываем Google..." }
                } else if flow == OAuthFlow::Link { "Подключить Google" }
                else { "Продолжить с Google" }
            }
            if busy {
                div { role: "status", class: "space-y-2 rounded-2xl bg-zinc-900/60 p-3",
                    p { class: "text-pretty text-xs leading-5 text-zinc-400", "Заверши вход в открывшемся браузере. CheenHub продолжит здесь автоматически." }
                    button {
                        r#type: "button", disabled: progress() == desktop_oauth::Progress::Completing, class: "disabled:cursor-wait disabled:opacity-50 min-h-10 rounded-xl px-3 text-xs font-medium text-zinc-200 transition-[background-color,scale] hover:bg-zinc-800 active:scale-[0.96]",
                        onclick: move |_| {
                            if progress() == desktop_oauth::Progress::Completing { return; }
                            if let Some(pending) = task.take() { pending.cancel(); }
                            progress.set(desktop_oauth::Progress::Opening); status.set("Вход отменён. Можно попробовать снова.".to_owned());
                            info!(?flow, "desktop Google OAuth cancelled by user");
                        },
                        "Отменить"
                    }
                }
            }
            if !status().is_empty() {
                p { role: "status", class: "text-pretty text-xs leading-5 text-zinc-300", "{status()}" }
            }
        }
    }
}
