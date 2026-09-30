//! Панель выбора эмодзи с локальным поиском и категориями.

use super::emoji_catalog::{CATEGORIES, filtered_emojis};
use dioxus::prelude::*;

/// Показывает категории, поиск и доступные эмодзи.
#[component]
pub(super) fn EmojiPicker(
    id: String,
    position_style: String,
    disabled: bool,
    on_select: EventHandler<String>,
    on_close: EventHandler<()>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut category = use_signal(|| None::<usize>);
    let emojis = filtered_emojis(&query(), category());
    rsx! {
        div {
            id: id.clone(), popover: "auto",
            style: format!("{position_style} margin: 0; right: auto; bottom: auto; width: min(320px, calc(100dvw - 24px)); max-height: calc(100dvh - 24px);"),
            class: "fixed z-50 flex flex-col gap-3 overflow-y-auto rounded-2xl border border-zinc-700 bg-zinc-900 p-3 text-zinc-100 shadow-[0_16px_50px_rgba(0,0,0,0.5)] [&:not(:popover-open)]:hidden",
            role: "dialog", "aria-label": "Выбор эмодзи",
            onkeydown: move |event| { if event.key() == Key::Escape { event.stop_propagation(); on_close.call(()); } },
            div { class: "flex items-center justify-between gap-2",
                p { class: "text-sm font-medium", "Эмодзи" }
                button { r#type: "button", class: "rounded-lg px-2 py-1 text-zinc-400 hover:bg-white/5 hover:text-white", "aria-label": "Закрыть выбор эмодзи", popovertarget: id.clone(), popovertargetaction: "hide", onclick: move |_| on_close.call(()),
                    svg { class: "size-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true", path { d: "m6 6 12 12M18 6 6 18" } }
                }
            }
            input { r#type: "search", autofocus: true, value: "{query()}", placeholder: "Найти эмодзи", "aria-label": "Найти эмодзи",
                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-2 text-sm outline-none focus:border-blue-400",
                oninput: move |event| { query.set(event.value()); category.set(None); },
            }
            div { class: "flex gap-1 overflow-x-auto pb-1", "aria-label": "Категории эмодзи",
                button { r#type: "button", class: "shrink-0 rounded-lg px-2 py-1 text-xs text-blue-200 hover:bg-white/10", "aria-pressed": category().is_none(), onclick: move |_| category.set(None), "Все" }
                for (index, name) in CATEGORIES.iter().enumerate() {
                    button { key: "{index}", r#type: "button", class: "shrink-0 rounded-lg px-2 py-1 text-xs text-zinc-300 hover:bg-white/10 aria-pressed:bg-blue-500/20 aria-pressed:text-blue-200", "aria-pressed": category() == Some(index), onclick: move |_| category.set(Some(index)), "{name}" }
                }
            }
            if emojis.is_empty() {
                div { class: "py-6 text-center text-sm text-zinc-400", role: "status",
                    p { "Эмодзи не найдены" }
                    button { r#type: "button", class: "mt-2 text-blue-300 hover:text-blue-200", onclick: move |_| { query.set(String::new()); category.set(None); }, "Показать все" }
                }
            } else {
                div { class: "grid max-h-56 grid-cols-8 gap-1 overflow-y-auto",
                    for emoji in emojis {
                        button { key: "{emoji.symbol}", r#type: "button", disabled, popovertarget: id.clone(), popovertargetaction: "hide", class: "flex size-8 items-center justify-center rounded-lg text-xl transition-colors hover:bg-white/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300", title: emoji.name, "aria-label": emoji.name,
                            onclick: move |_| on_select.call(emoji.symbol.to_owned()), "{emoji.symbol}" }
                    }
                }
            }
        }
    }
}
