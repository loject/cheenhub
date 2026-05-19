//! Individual room list item for the server rooms sidebar.

use cheenhub_contracts::rest::ServerRoomSummary;
use dioxus::prelude::*;

use super::server_rooms_state::{room_icon, room_icon_class};

#[component]
pub(super) fn RoomListItem(
    room: ServerRoomSummary,
    is_active: bool,
    is_owner: bool,
    on_select: EventHandler<()>,
    on_edit: EventHandler<()>,
    on_delete: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            "data-active": if is_active { "true" } else { "false" },
            class: "group relative flex w-full items-center justify-between rounded-lg border border-transparent px-2.5 py-2 text-left text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 data-[active=true]:border-accent/25 data-[active=true]:bg-accent/10 data-[active=true]:text-zinc-100",
            button {
                r#type: "button",
                class: "flex min-w-0 flex-1 items-center gap-2 text-left",
                "aria-label": "Открыть комнату {room.name}",
                onclick: move |_| on_select(()),
                span { class: room_icon_class(room.kind), "{room_icon(room.kind)}" }
                span { class: "truncate text-[12px] font-medium transition-[opacity] duration-150 max-[1440px]:opacity-0 max-[1440px]:group-hover/rooms:opacity-100 max-[1440px]:group-focus-within/rooms:opacity-100", "{room.name}" }
            }
            if is_owner {
                span { class: "ml-2 flex shrink-0 items-center gap-1 opacity-0 transition group-hover:opacity-100 group-focus-within:opacity-100 max-[1440px]:hidden max-[1440px]:group-hover/rooms:flex max-[1440px]:group-focus-within/rooms:flex",
                    button {
                        r#type: "button",
                        class: "rounded-md p-1 text-zinc-600 hover:bg-zinc-800 hover:text-zinc-200",
                        "aria-label": "Изменить комнату {room.name}",
                        onclick: move |_| on_edit(()),
                        svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "rounded-md p-1 text-zinc-600 hover:bg-red-500/10 hover:text-red-200",
                        "aria-label": "Удалить комнату {room.name}",
                        onclick: move |_| on_delete(()),
                        svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673A2.25 2.25 0 0 1 15.916 21H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" }
                        }
                    }
                }
            }
        }
    }
}
