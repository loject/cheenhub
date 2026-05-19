//! Voice participant grid component.

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::*;

use crate::features::app::components::user_context_menu::UserContextMenu;

use super::participant_tile::VoiceParticipantTile;

/// Empty or transient display state for the voice participant grid.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum VoiceParticipantGridStatus {
    /// The room is waiting for the join request to complete.
    Connecting,
    /// The room is joined but has no visible participants.
    Empty,
    /// The latest room-scoped voice action failed.
    Error {
        /// User-facing error message.
        message: String,
    },
}

#[derive(Clone, PartialEq)]
struct UserMenuState {
    name: String,
    x: f64,
    y: f64,
}

/// Renders voice room participants.
#[component]
pub(crate) fn VoiceParticipantGrid(
    participants: Vec<VoiceRoomParticipant>,
    speaking_user_ids: Vec<String>,
    status: VoiceParticipantGridStatus,
    on_retry: EventHandler<()>,
) -> Element {
    let mut open_user_menu = use_signal(|| None::<UserMenuState>);
    let count = participants.len().clamp(1, 12);
    let (title, body) = match &status {
        VoiceParticipantGridStatus::Connecting => (
            "Подключаемся к голосовой комнате",
            "Ждём ответ сервера и готовим список участников.",
        ),
        VoiceParticipantGridStatus::Empty => (
            "В голосовой комнате пока никого нет",
            "Войди в комнату, чтобы появиться в списке участников.",
        ),
        VoiceParticipantGridStatus::Error { .. } => (
            "Не удалось подключиться к голосовой комнате",
            "Сервер не ответил или вернул ошибку. Можно попробовать подключиться ещё раз.",
        ),
    };
    let error_message = match &status {
        VoiceParticipantGridStatus::Error { message } => Some(message.as_str()),
        _ => None,
    };
    let is_connecting = matches!(status, VoiceParticipantGridStatus::Connecting);
    let can_retry = matches!(status, VoiceParticipantGridStatus::Error { .. });

    rsx! {
        div {
            class: "voice-stage flex min-h-0 flex-1 items-center justify-center overflow-y-auto p-5 pt-6 pb-[108px] max-[900px]:pt-4 max-[900px]:pb-[100px] lg:p-6 lg:pt-6 lg:pb-[108px]",
            onclick: move |_| open_user_menu.set(None),
            if participants.is_empty() {
                div { class: "max-w-sm text-center",
                    div { class: "mx-auto grid h-16 w-16 place-items-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-zinc-500",
                        if is_connecting {
                            div { class: "h-7 w-7 animate-spin rounded-full border-2 border-zinc-700 border-t-accent" }
                        } else {
                            svg { class: "h-7 w-7", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                            }
                        }
                    }
                    h2 { class: "mt-4 text-[16px] font-semibold text-zinc-100", "{title}" }
                    p { class: "mt-2 text-[13px] leading-6 text-zinc-500", "{body}" }
                    if let Some(message) = error_message {
                        p { class: "mt-2 text-[12px] leading-5 text-zinc-600", "{message}" }
                    }
                    if can_retry {
                        button {
                            r#type: "button",
                            class: "mt-4 inline-flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white transition hover:bg-blue-400",
                            onclick: move |_| on_retry.call(()),
                            "Повторить подключение"
                        }
                    }
                }
            } else {
                div { class: "participants-grid m-auto flex min-h-full w-[min(100%,1320px)] flex-wrap content-center items-stretch justify-center gap-4 max-[900px]:gap-3", "data-count": "{count}",
                    for participant in participants {
                        VoiceParticipantTile {
                            key: "{participant.user_id}",
                            speaking: speaking_user_ids.iter().any(|user_id| user_id == &participant.user_id),
                            participant,
                            on_open_user_menu: move |(name, x, y)| {
                                open_user_menu.set(Some(UserMenuState { name, x, y }));
                            },
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
