//! Voice participant tile component.

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::*;

use crate::features::app::components::avatar::{UserAvatar, use_avatar_seed};

use super::screen_video::ScreenVideoCanvas;

/// Renders one voice room participant.
#[component]
pub(crate) fn VoiceParticipantTile(
    participant: VoiceRoomParticipant,
    speaking: bool,
    screen_sharing: bool,
    on_open_user_menu: EventHandler<(String, String, f64, f64)>,
) -> Element {
    use_avatar_seed(participant.user_id.clone());
    let tile_class = if screen_sharing && speaking {
        "user-tile relative overflow-hidden rounded-[20px] border border-emerald-400/75 bg-zinc-950 p-4 shadow-[0_0_0_1px_rgba(52,211,153,.24),0_18px_70px_rgba(16,185,129,.18)] transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-emerald-300/80"
    } else if screen_sharing {
        "user-tile relative overflow-hidden rounded-[20px] border border-sky-400/35 bg-zinc-950 p-4 shadow-[0_18px_70px_rgba(2,132,199,.16)] transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-sky-300/50"
    } else if speaking {
        "user-tile relative overflow-hidden rounded-[20px] border border-emerald-400/75 bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 shadow-[0_0_0_1px_rgba(52,211,153,.24),0_18px_70px_rgba(16,185,129,.18)] transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-emerald-300/80"
    } else {
        "user-tile relative overflow-hidden rounded-[20px] border border-accent/25 bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-white/15"
    };

    rsx! {
        article {
            key: "{participant.user_id}",
            "data-avatar": "",
            "data-speaking": if speaking { "true" } else { "false" },
            style: "--avatar-bg: rgba(24,24,27,.80);",
            class: tile_class,
            oncontextmenu: {
                let nickname = participant.nickname.clone();
                let user_id = participant.user_id.clone();
                move |event| {
                    event.prevent_default();
                    event.stop_propagation();
                    let point = event.client_coordinates();
                    on_open_user_menu.call((nickname.clone(), user_id.clone(), point.x, point.y));
                }
            },
            if screen_sharing {
                ScreenVideoCanvas {
                    user_id: participant.user_id.clone(),
                }
                div { class: "pointer-events-none absolute inset-0 z-[1] bg-gradient-to-t from-zinc-950/65 via-transparent to-zinc-950/20" }
            }
            if speaking {
                div { class: "pointer-events-none absolute inset-0 rounded-[20px] bg-emerald-400/[0.035]" }
                div { class: "pointer-events-none absolute inset-x-4 bottom-12 z-0 flex h-10 justify-center items-end gap-1 text-emerald-300/55",
                    span { class: "inline-block h-4 w-1 origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current" }
                    span { class: "inline-block h-7 w-1 origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current [animation-delay:.10s]" }
                    span { class: "inline-block h-5 w-1 origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current [animation-delay:.20s]" }
                    span { class: "inline-block h-8 w-1 origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current [animation-delay:.30s]" }
                }
            }
            div { class: "absolute right-3 top-3 z-20",
                button {
                    r#type: "button",
                    class: "rounded-xl border border-zinc-800 bg-zinc-950 p-2 text-zinc-500 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:text-zinc-200",
                    "aria-label": "Меню пользователя",
                    onclick: {
                        let nickname = participant.nickname.clone();
                        let user_id = participant.user_id.clone();
                        move |event| {
                            event.stop_propagation();
                            let point = event.client_coordinates();
                            on_open_user_menu.call((nickname.clone(), user_id.clone(), point.x, point.y));
                        }
                    },
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                    }
                }
            }
            if !screen_sharing {
                div { class: "relative z-10 flex min-h-full flex-col items-center justify-center text-center",
                    UserAvatar {
                        nickname: participant.nickname.clone(),
                        avatar_url: participant.avatar_url.clone(),
                        class: "flex h-20 w-20 items-center justify-center rounded-full border border-white/10 bg-zinc-900/80 text-[26px] font-bold text-zinc-100 shadow-[0_16px_36px_rgba(0,0,0,.24)]".to_owned(),
                        avatar_seed: Some(participant.user_id.clone()),
                    }
                }
            }
            div { class: "absolute inset-x-4 bottom-4 z-20 flex justify-center",
                div { class: "max-w-full rounded-xl border border-zinc-800 bg-zinc-950/80 px-3 py-1.5 text-[13px] font-semibold text-zinc-100 shadow-[0_12px_30px_rgba(0,0,0,.32)] backdrop-blur-xl",
                    div { class: "truncate", "{participant.nickname}" }
                }
            }
        }
    }
}
