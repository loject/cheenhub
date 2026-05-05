//! Server context menu component.

use dioxus::prelude::*;

/// Renders server-level actions.
#[component]
pub(crate) fn ServerContextMenu(is_owner: bool, on_create_invite: EventHandler<()>) -> Element {
    rsx! {
        div {
            class: "absolute left-4 right-4 top-[86px] z-40 overflow-hidden rounded-[20px] border border-zinc-800 bg-zinc-950/95 p-1.5 shadow-[0_20px_60px_rgba(0,0,0,.55)] backdrop-blur-xl",
            onclick: move |event| event.stop_propagation(),
            button { r#type: "button", class: "flex w-full items-center justify-between rounded-xl px-3 py-2.5 text-left text-[13px] text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:bg-zinc-900 hover:text-zinc-100",
                span { class: "flex items-center gap-2",
                    svg { class: "h-4 w-4 text-zinc-500", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.5 6h9.75M10.5 6a1.5 1.5 0 1 1-3 0m3 0a1.5 1.5 0 1 0-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-9.75 0h9.75" }
                    }
                    "Параметры сервера"
                }
            }
            button {
                r#type: "button",
                class: "flex w-full items-center gap-2 rounded-xl px-3 py-2.5 text-left text-[13px] text-blue-200 transition-[background,border-color,color,transform,opacity] duration-150 hover:bg-accent/10 hover:text-blue-100",
                onclick: move |_| on_create_invite.call(()),
                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13.19 8.688a4.5 4.5 0 0 1 1.242 7.244l-4.5 4.5a4.5 4.5 0 0 1-6.364-6.364l1.757-1.757m13.35-.622 1.757-1.757a4.5 4.5 0 0 0-6.364-6.364l-4.5 4.5a4.5 4.5 0 0 0 1.242 7.244" }
                }
                "Создать ссылку приглашения"
            }
            div { class: "my-1 border-t border-zinc-800" }
            if is_owner {
                div { class: "group relative",
                    button {
                        r#type: "button",
                        disabled: true,
                        class: "flex w-full cursor-not-allowed items-center gap-2 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300/35 opacity-70",
                        "aria-describedby": "server-owner-leave-tooltip",
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                        }
                        "Выйти с сервера"
                    }
                    span {
                        id: "server-owner-leave-tooltip",
                        role: "tooltip",
                        class: "pointer-events-none absolute bottom-[calc(100%+8px)] left-2 z-50 w-[230px] translate-y-1 rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-2 text-[11px] leading-4 text-zinc-300 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-within:translate-y-0 group-focus-within:opacity-100",
                        "Владелец сервера не может покинуть сервер"
                    }
                }
            } else {
                button { r#type: "button", class: "flex w-full items-center gap-2 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:bg-red-500/10 hover:text-red-200",
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                    }
                    "Выйти с сервера"
                }
            }
        }
    }
}
