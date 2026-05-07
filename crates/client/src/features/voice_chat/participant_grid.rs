//! Voice participant grid component.

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::*;

use crate::features::app::components::user_context_menu::UserContextMenu;

#[derive(Clone, PartialEq)]
struct UserMenuState {
    name: String,
    x: f64,
    y: f64,
}

/// Renders voice room participants.
#[component]
pub(crate) fn VoiceParticipantGrid(participants: Vec<VoiceRoomParticipant>) -> Element {
    let mut open_user_menu = use_signal(|| None::<UserMenuState>);
    let count = participants.len().clamp(1, 12);

    rsx! {
        div {
            class: "voice-stage flex min-h-0 flex-1 items-center justify-center overflow-y-auto p-5 pt-6 pb-[108px] max-[900px]:pt-4 max-[900px]:pb-[100px] lg:p-6 lg:pt-6 lg:pb-[108px]",
            onclick: move |_| open_user_menu.set(None),
            if participants.is_empty() {
                div { class: "max-w-sm text-center",
                    div { class: "mx-auto grid h-16 w-16 place-items-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-zinc-500",
                        svg { class: "h-7 w-7", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                        }
                    }
                    h2 { class: "mt-4 text-[16px] font-semibold text-zinc-100", "В голосовой комнате пока никого нет" }
                    p { class: "mt-2 text-[13px] leading-6 text-zinc-500", "Войди в комнату, чтобы появиться в списке участников." }
                }
            } else {
                div { class: "participants-grid m-auto flex min-h-full w-[min(100%,1320px)] flex-wrap content-center items-stretch justify-center gap-4 max-[900px]:gap-3", "data-count": "{count}",
                    for participant in participants {
                        article {
                            key: "{participant.user_id}",
                            "data-avatar": participant_initial(&participant.nickname),
                            style: "--avatar-bg: rgba(24,24,27,.80);",
                            class: "user-tile relative overflow-hidden rounded-[20px] border border-accent/25 bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 transition-[border-color,background,transform] duration-200 ease-in-out hover:border-white/15",
                            oncontextmenu: {
                                let nickname = participant.nickname.clone();
                                move |event| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    let point = event.client_coordinates();
                                    open_user_menu.set(Some(UserMenuState { name: nickname.clone(), x: point.x, y: point.y }));
                                }
                            },
                            div { class: "absolute right-3 top-3 z-20",
                                button {
                                    r#type: "button",
                                    class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px rounded-xl border border-zinc-800 bg-zinc-950 p-2 text-zinc-500 hover:border-zinc-700 hover:text-zinc-200",
                                    "aria-label": "Меню пользователя",
                                    onclick: {
                                        let nickname = participant.nickname.clone();
                                        move |event| {
                                            event.stop_propagation();
                                            let point = event.client_coordinates();
                                            open_user_menu.set(Some(UserMenuState { name: nickname.clone(), x: point.x, y: point.y }));
                                        }
                                    },
                                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                                    }
                                }
                            }
                            div { class: "relative z-10 flex min-h-full flex-col justify-end text-left",
                                div { class: "flex items-center gap-2",
                                    div { class: "text-[14px] font-semibold text-zinc-50", "{participant.nickname}" }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(menu) = open_user_menu() {
                UserContextMenu {
                    name: menu.name,
                    volume: "100",
                    x: menu.x,
                    y: menu.y,
                }
            }
        }
    }
}

fn participant_initial(nickname: &str) -> String {
    nickname
        .chars()
        .next()
        .map(|value| value.to_uppercase().collect())
        .unwrap_or_else(|| "?".to_owned())
}
