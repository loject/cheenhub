//! Text chat message item component.

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

use crate::features::app::components::avatar::{UserAvatar, use_avatar_seed};
use crate::features::app::current_user::CurrentUserContext;

/// Renders one text chat message row.
#[component]
pub(super) fn ChatMessageItem(
    message: TextChatMessage,
    animate: bool,
    removing: bool,
    on_delete: EventHandler<String>,
) -> Element {
    use_avatar_seed(message.author_user_id.clone());
    let current_user = use_context::<CurrentUserContext>().require_user();
    let is_own = message.author_user_id == current_user.id;
    let mut menu_pos = use_signal(|| None::<(f64, f64)>);

    let outer_class = match (animate, removing) {
        (_, true) => "chat-message-removing flex gap-3",
        (true, false) => "chat-message flex gap-3",
        (false, false) => "flex gap-3",
    };

    rsx! {
        div {
            class: outer_class,
            oncontextmenu: move |event| {
                if !is_own {
                    return;
                }
                event.prevent_default();
                event.stop_propagation();
                let p = event.client_coordinates();
                menu_pos.set(Some((p.x, p.y)));
            },
            UserAvatar {
                nickname: message.author_nickname.clone(),
                avatar_url: message.author_avatar_url.clone(),
                class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100".to_owned(),
            }
            div { class: "min-w-0 flex-1",
                div { class: "mb-1 flex items-center gap-2",
                    span { class: "truncate text-[12px] font-semibold text-zinc-100",
                        "{message.author_nickname}"
                    }
                    span { class: "shrink-0 text-[10px] text-zinc-600",
                        "{message_time(&message.created_at)}"
                    }
                }
                div { class: "message-bubble whitespace-pre-wrap break-words rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]",
                    "{message.body}"
                }
            }
        }

        if let Some((x, y)) = menu_pos() {
            {
                let message_id = message.id.clone();
                rsx! {
                    div {
                        class: "fixed inset-0 z-[999]",
                        onclick: move |_| menu_pos.set(None),
                    }
                    div {
                        class: "fixed z-[1000] min-w-[180px] overflow-hidden rounded-[16px] border border-zinc-800 bg-zinc-950/95 p-1.5 shadow-[0_20px_60px_rgba(0,0,0,.55)] backdrop-blur-xl",
                        style: "left: clamp(12px, {x}px, calc(100vw - 200px)); top: clamp(12px, {y}px, calc(100vh - 80px));",
                        onclick: move |event| event.stop_propagation(),
                        button {
                            r#type: "button",
                            class: "flex w-full items-center gap-2.5 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 transition-[background,color] duration-150 hover:bg-red-500/10 hover:text-red-200",
                            onclick: move |_| {
                                menu_pos.set(None);
                                on_delete.call(message_id.clone());
                            },
                            svg {
                                class: "h-4 w-4 shrink-0",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.9",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    d: "m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0",
                                }
                            }
                            "Удалить сообщение"
                        }
                    }
                }
            }
        }
    }
}

fn message_time(created_at: &str) -> String {
    created_at
        .split('T')
        .nth(1)
        .and_then(|time| time.get(0..5))
        .unwrap_or("")
        .to_owned()
}
