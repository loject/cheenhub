//! Понятная ошибка загрузки серверов с повторной попыткой.

use dioxus::prelude::*;

/// Предлагает повторить загрузку списка и показывает состояние повторного запроса.
#[component]
pub(super) fn ServerListError(is_loading: bool, on_retry: EventHandler<()>) -> Element {
    rsx! {
        div { role: "status", class: "w-full max-w-sm rounded-xl border border-red-500/20 bg-zinc-950 p-4 text-left shadow-xl",
            p { class: "text-sm font-semibold text-zinc-100", "Не удалось загрузить серверы" }
            p { class: "mt-2 text-[13px] leading-5 text-zinc-400", "Проверь подключение и попробуй ещё раз." }
            button {
                r#type: "button", disabled: is_loading,
                class: "mt-3 flex h-10 w-full items-center justify-center gap-2 rounded-xl bg-accent px-4 text-sm font-semibold text-white hover:bg-blue-400 disabled:opacity-60 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-400/70",
                onclick: move |_| { if !is_loading { on_retry.call(()); } },
                if is_loading { span { class: "size-4 animate-spin rounded-full border-2 border-white/30 border-t-white", "aria-hidden": "true" } }
                if is_loading { "Повторяем…" } else { "Повторить" }
            }
        }
    }
}
