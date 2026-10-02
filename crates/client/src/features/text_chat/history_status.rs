//! Статус загрузки истории без размонтирования уже открытых сообщений.

use super::ChatHistoryLoadingState;
use dioxus::prelude::*;

/// Показывает начальную загрузку или ненавязчивое обновление открытого чата.
#[component]
pub(super) fn ChatHistoryStatus(
    loading: bool,
    error: Option<String>,
    has_messages: bool,
    on_retry: EventHandler<()>,
) -> Element {
    let status_class = if has_messages {
        "absolute right-3 top-3 z-20 max-w-[calc(100%-1.5rem)] rounded-2xl bg-zinc-900/95 px-4 py-2 text-[12px] shadow-[0_4px_20px_rgba(0,0,0,0.3)]"
    } else {
        "mx-auto max-w-md rounded-[18px] bg-red-500/[0.08] px-6 py-5 text-center text-[12px] leading-5 text-red-200 shadow-[0_0_0_1px_rgba(248,113,113,0.16)]"
    };
    rsx! {
        if loading && !has_messages {
            ChatHistoryLoadingState {}
        } else if loading {
            div { class: status_class, role: "status", "aria-live": "polite",
                div { class: "flex items-center gap-2 text-zinc-400",
                    span { class: "h-3.5 w-3.5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-400", "aria-hidden": "true" }
                    "Обновляем сообщения…"
                }
            }
        } else if let Some(error) = error {
            div { class: status_class, role: "alert",
                p { class: "font-semibold text-red-100", "Не удалось обновить сообщения" }
                p { class: "text-red-200", "{error}" }
                button {
                    r#type: "button",
                    class: "mt-2 min-h-10 rounded-xl bg-red-400/10 px-4 text-[12px] font-medium text-red-100 transition-[background-color,color,transform] duration-150 hover:bg-red-400/15 hover:text-white active:scale-[0.96]",
                    onclick: move |_| on_retry.call(()),
                    "Повторить"
                }
            }
        }
    }
}
