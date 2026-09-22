//! Кнопка раскрытия меню выбранного сервера.

use dioxus::prelude::*;

/// Показывает имя сервера и сообщает о нажатии для раскрытия меню.
#[component]
pub(crate) fn ServerRoomsMenuTrigger(
    server_name: String,
    is_owner: bool,
    is_open: bool,
    text_class: &'static str,
    icon_class: &'static str,
    on_toggle: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "flex w-full items-center justify-between rounded-2xl border border-zinc-800 bg-zinc-900/80 px-4 py-3 text-left transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-700 hover:bg-zinc-800",
            "aria-haspopup": "menu",
            "aria-expanded": if is_open { "true" } else { "false" },
            onclick: move |event| {
                event.stop_propagation();
                on_toggle.call(());
            },
            span { class: text_class,
                span { class: "block text-[13px] font-semibold tracking-[-0.02em] text-zinc-100", "{server_name}" }
                span { class: "mt-0.5 block text-[11px] text-zinc-500",
                    if is_owner { "Владелец сервера" } else { "Участник сервера" }
                }
            }
            svg { class: icon_class, fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 9 6 6 6-6" }
            }
        }
    }
}
