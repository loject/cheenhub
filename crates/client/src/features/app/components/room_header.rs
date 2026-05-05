//! Current room header component.

use dioxus::prelude::*;

use super::app_shell::ActiveRoom;

/// Renders the active room title and join voice affordance.
#[component]
pub(crate) fn RoomHeader(room: ActiveRoom) -> Element {
    let (badge, dot_class, subtitle) = match room.kind {
        "text" => (
            "текст",
            "h-1.5 w-1.5 rounded-full bg-zinc-600",
            "текстовая комната",
        ),
        "mixed-chat" => (
            "текст + голос",
            "h-1.5 w-1.5 rounded-full bg-accent",
            "текстовая + голосовая комната · не в голосе",
        ),
        _ => (
            "голос и текст",
            "h-1.5 w-1.5 rounded-full bg-accent",
            "5 участников · текстовая + голосовая комната",
        ),
    };

    rsx! {
        div { class: "flex h-[72px] shrink-0 items-center justify-between gap-4 border-b border-zinc-800/80 bg-zinc-950/85 px-6 backdrop-blur-xl",
            div { class: "min-w-0",
                div { class: "flex items-center gap-3",
                    h1 { class: "truncate text-[15px] font-semibold tracking-[-0.04em] text-zinc-50", "{room.name}" }
                    span { class: "inline-flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900/80 px-2.5 py-1 text-[11px] text-zinc-400",
                        span { class: dot_class }
                        "{badge}"
                    }
                }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "{subtitle}" }
            }
            button { id: "join-voice-button", r#type: "button", class: "join-voice-button group relative hidden h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-accent/30 bg-accent/10 text-blue-100 transition hover:border-accent/45 hover:bg-accent/15 hover:text-white", "aria-label": "Войти в голосовую комнату",
                span { class: "pointer-events-none absolute right-0 top-[calc(100%+10px)] -translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Войти в голосовую комнату" }
                svg { class: "h-4 w-4 -scale-x-100", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M2.25 6.75c0 8.284 6.716 15 15 15h2.25a2.25 2.25 0 0 0 2.25-2.25v-1.372c0-.516-.351-.966-.852-1.091l-4.423-1.106a1.125 1.125 0 0 0-1.173.417l-.97 1.293a1.125 1.125 0 0 1-1.21.38 12.035 12.035 0 0 1-7.143-7.143 1.125 1.125 0 0 1 .38-1.21l1.293-.97c.37-.277.527-.756.417-1.173L6.963 3.102A1.125 1.125 0 0 0 5.872 2.25H4.5A2.25 2.25 0 0 0 2.25 4.5v2.25Z" }
                }
            }
        }
    }
}
