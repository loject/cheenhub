//! Визуальный разделитель календарных дней в истории сообщений.

use dioxus::prelude::*;

/// Рендерит подпись календарного дня между группами сообщений.
#[component]
pub(crate) fn ChatMessageDateDivider(label: String, overflowing: bool) -> Element {
    let label_class = if overflowing {
        "rounded-full bg-zinc-900/95 px-3 py-1 text-[10px] font-medium uppercase tracking-[0.12em] text-zinc-400"
    } else {
        "px-3 py-1 text-[10px] font-medium uppercase tracking-[0.12em] text-zinc-600"
    };
    rsx! {
        div { class: "sticky top-0 z-30 -my-1 flex items-center justify-center gap-4 py-1",
            if !overflowing {
                span { class: "h-px min-w-6 flex-1 bg-zinc-800/80" }
            }
            time { class: label_class,
                "{label}"
            }
            if !overflowing {
                span { class: "h-px min-w-6 flex-1 bg-zinc-800/80" }
            }
        }
    }
}
