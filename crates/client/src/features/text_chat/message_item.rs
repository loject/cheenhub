//! Text chat message item component.

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

/// Renders one text chat message row.
#[component]
pub(super) fn ChatMessageItem(message: TextChatMessage, animate: bool) -> Element {
    let class = if animate {
        "chat-message flex gap-3"
    } else {
        "flex gap-3"
    };

    rsx! {
        div { class,
            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100",
                "{initial(&message.author_nickname)}"
            }
            div { class: "min-w-0 flex-1",
                div { class: "mb-1 flex items-center gap-2",
                    span { class: "truncate text-[12px] font-semibold text-zinc-100", "{message.author_nickname}" }
                    span { class: "shrink-0 text-[10px] text-zinc-600", "{message_time(&message.created_at)}" }
                }
                div { class: "message-bubble whitespace-pre-wrap break-words rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]",
                    "{message.body}"
                }
            }
        }
    }
}

fn initial(nickname: &str) -> String {
    nickname
        .chars()
        .next()
        .map(|letter| letter.to_uppercase().collect())
        .unwrap_or_else(|| "?".to_owned())
}

fn message_time(created_at: &str) -> String {
    created_at
        .split('T')
        .nth(1)
        .and_then(|time| time.get(0..5))
        .unwrap_or("")
        .to_owned()
}
