//! Плашка режима «только чтение» для комнаты с ограниченным доступом.

use dioxus::prelude::*;

/// Рендерит уведомление о том, что писать в комнату нельзя.
pub(crate) fn read_only_notice() -> Element {
    rsx! {
        div { class: "px-3 py-2.5", role: "status", "aria-live": "polite",
            div { class: "flex items-center justify-center gap-3 rounded-xl border border-zinc-800/60 bg-zinc-900/30 px-3 py-2.5 text-center",
                svg { class: "size-4 shrink-0 text-zinc-500", fill: "none", stroke: "currentColor", stroke_width: "1.6", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5Z" }
                }
                div { class: "min-w-0 text-[12px] leading-5",
                    p { class: "font-medium text-zinc-400", "У вас нет прав на отправку сообщений" }
                }
            }
        }
    }
}
