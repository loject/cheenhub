//! Плашка режима «только чтение» для комнаты с ограниченным доступом.

use dioxus::prelude::*;

/// Рендерит уведомление о том, что писать в комнату нельзя.
pub(crate) fn read_only_notice() -> Element {
    rsx! {
        div { class: "px-3 py-2.5", role: "status", "aria-live": "polite",
            div { class: "flex items-start gap-2.5 rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 py-2.5",
                svg { class: "mt-0.5 size-4 shrink-0 text-zinc-500", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v4m0 4h.01M10.3 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.7 3.86a2 2 0 0 0-3.4 0Z" }
                }
                p { class: "text-[12px] leading-5 text-zinc-400",
                    "В этой комнате писать могут только участники с выбранными ролями. Ты можешь читать сообщения."
                }
            }
        }
    }
}
