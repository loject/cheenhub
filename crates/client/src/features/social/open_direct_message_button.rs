//! Открытие личного диалога из внешнего меню пользователя через social realtime.

use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;

use super::realtime;

/// Открывает или создаёт диалог и передаёт его ID владельцу навигации.
///
/// Во время запроса блокирует повторный клик; при отказе оставляет меню открытым
/// и показывает ошибку с возможностью повторить действие.
#[component]
pub(crate) fn OpenDirectMessageButton(user_id: String, on_open: EventHandler<String>) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let mut pending = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        button {
            r#type: "button",
            disabled: pending(),
            class: "mt-1 flex min-h-10 w-full items-center gap-2 rounded-[10px] px-2.5 py-2 text-left text-[13px] text-zinc-200 transition-[background-color,color,opacity] duration-150 hover:bg-zinc-900 hover:text-white disabled:cursor-wait disabled:opacity-60",
            onclick: move |_| {
                if *pending.peek() { return; }
                pending.set(true);
                error.set(None);
                let realtime = realtime.clone();
                let user_id = user_id.clone();
                spawn(async move {
                    info!(peer_user_id = %user_id, "opening direct dialog from user menu");
                    let result = realtime::open_dm_conversation(&realtime, user_id.clone()).await;
                    pending.set(false);
                    match result {
                        Ok(conversation) => on_open.call(conversation.id),
                        Err(message) => {
                            warn!(peer_user_id = %user_id, %message, "failed to open direct dialog from user menu");
                            error.set(Some(message));
                        }
                    }
                });
            },
            if pending() {
                span { class: "h-3.5 w-3.5 shrink-0 animate-spin rounded-full border-2 border-current border-t-transparent", "aria-hidden": "true" }
                "Открываем диалог…"
            } else {
                "Открыть личный диалог"
            }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "px-2.5 py-2 text-[12px] leading-5 text-red-200", "{message}" }
        }
    }
}
