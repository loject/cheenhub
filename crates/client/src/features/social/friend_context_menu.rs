//! Контекстное меню друга на social-экране.

use dioxus::prelude::*;

/// Рендерит контекстное меню действий над другом.
#[component]
pub(super) fn FriendContextMenu(
    friend_user_id: String,
    friend_nickname: String,
    x: f64,
    y: f64,
    on_close: EventHandler<()>,
    on_delete: EventHandler<String>,
) -> Element {
    let top = y + 8.0;
    let pos_style = format!(
        "left: clamp(12px, {x}px, calc(100vw - 238px)); top: clamp(12px, {top}px, calc(100vh - 116px));"
    );

    rsx! {
        div {
            class: "fixed inset-0 z-[999]",
            onclick: move |_| on_close.call(()),
        }
        div {
            class: "fixed z-[1000] w-[220px] overflow-hidden rounded-[16px] border border-zinc-800 bg-zinc-950/95 p-1.5 shadow-[0_20px_60px_rgba(0,0,0,.55)] backdrop-blur-xl",
            style: pos_style,
            onclick: move |event| event.stop_propagation(),
            div { class: "rounded-xl px-3 py-2",
                div { class: "truncate text-[13px] font-semibold text-zinc-100", "{friend_nickname}" }
                div { class: "mt-0.5 text-[11px] text-zinc-500", "Друг" }
            }
            div { class: "mx-1 my-1 border-t border-zinc-800/70" }
            button {
                r#type: "button",
                class: "flex w-full items-center gap-2.5 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 transition-[background,color] duration-150 hover:bg-red-500/10 hover:text-red-200",
                onclick: move |_| {
                    on_close.call(());
                    on_delete.call(friend_user_id.clone());
                },
                svg { class: "h-4 w-4 shrink-0", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12H9m12 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" }
                }
                "Удалить из друзей"
            }
        }
    }
}
