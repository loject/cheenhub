//! Визуальный разделитель календарных дней в истории сообщений.

use dioxus::prelude::*;

/// Рендерит подпись календарного дня между группами сообщений.
#[component]
pub(crate) fn ChatMessageDateDivider(label: String) -> Element {
    rsx! {
        div { class: "sticky top-0 z-10 -my-2 flex justify-center py-0.5",
            span { class: "rounded-full border border-zinc-800 bg-zinc-900/90 px-3 py-1 text-[11px] font-medium text-zinc-400 shadow-[0_4px_14px_rgba(0,0,0,0.2)]",
                "{label}"
            }
        }
    }
}
