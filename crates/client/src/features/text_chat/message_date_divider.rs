//! Визуальный разделитель календарных дней в истории сообщений.

use dioxus::prelude::*;

/// Рендерит подпись календарного дня между группами сообщений.
#[component]
pub(crate) fn ChatMessageDateDivider(label: String) -> Element {
    rsx! {
        div { class: "sticky top-0 z-10 -my-1 flex items-center gap-4 bg-[#08090b]/92 py-2 backdrop-blur-md",
            span { class: "h-px min-w-6 flex-1 bg-zinc-800/80" }
            time { class: "shrink-0 text-[10px] font-medium uppercase tracking-[0.12em] text-zinc-600",
                "{label}"
            }
            span { class: "h-px min-w-6 flex-1 bg-zinc-800/80" }
        }
    }
}
