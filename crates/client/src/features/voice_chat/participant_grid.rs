//! Voice participant grid component.

use std::collections::HashMap;

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::*;

use crate::features::app::components::user_context_menu::UserContextMenu;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::AudioPlaybackHandle;
use crate::features::camera::{CameraHandle, CameraStatus};

use super::participant_focus_strip::{FocusStripSelection, ParticipantFocusStrip};
use super::participant_grid_data::{
    focus_strip_tiles, participant_grid_layout, participant_tiles, preferred_focus_tile_key,
};
use super::participant_tile::VoiceParticipantTile;
use super::state::VoiceConnectionHandle;
use super::video_streams::{ParticipantVideoHandle, ParticipantVideoSource};

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
    user_id: String,
    x: f64,
    y: f64,
}

/// Renders voice room participants.
#[component]
pub(crate) fn VoiceParticipantGrid(
    server_id: String,
    room_id: String,
    participants: Vec<VoiceRoomParticipant>,
    speaking_user_ids: Vec<String>,
    status: VoiceParticipantGridStatus,
    can_kick_voice: bool,
    on_retry: EventHandler<()>,
) -> Element {
    let mut open_user_menu = use_signal(|| None::<UserMenuState>);
    let mut focused_tile_key = use_signal(|| None::<String>);
    let mut user_volumes = use_signal(HashMap::<String, u32>::new);
    let playback = use_context::<AudioPlaybackHandle>();
    let voice = use_context::<VoiceConnectionHandle>();
    let camera = use_context::<CameraHandle>();
    let participant_video = use_context::<ParticipantVideoHandle>();
    let current_user_id = use_context::<CurrentUserContext>().require_user().id;
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
    let kick_user_id = open_user_menu().map(|m| m.user_id.clone());
    let kick_server_id = server_id.clone();
    let kick_room_id = room_id.clone();
    let camera_live = matches!(camera.status(), CameraStatus::Live);
    let camera_user_ids = participant_video.live_user_ids(ParticipantVideoSource::Camera);
    let screen_user_ids = participant_video.live_user_ids(ParticipantVideoSource::ScreenShare);
    let participant_tiles = participant_tiles(
        &participants,
        &speaking_user_ids,
        &camera_user_ids,
        &screen_user_ids,
        &current_user_id,
        camera_live,
    );
    let grid_layout = participant_grid_layout(participant_tiles.len());
    let focus_strip_items = focus_strip_tiles(&participants, &participant_tiles, &current_user_id);
    let active_focus_tile_key = focused_tile_key().and_then(|key| {
        participant_tiles
            .iter()
            .any(|tile| tile.key == key)
            .then_some(key)
    });
    let focused = active_focus_tile_key.is_some();
    let preferred_focus_tile_key = preferred_focus_tile_key(&participant_tiles);
    let display_mode_label = if focused {
        "Вернуться к сетке участников"
    } else {
        "Сфокусировать главный тайл"
    };

    rsx! {
        div {
            class: "voice-stage relative flex min-h-0 flex-1 items-center justify-center overflow-y-auto p-5 pt-6 pb-[108px] max-[900px]:pt-4 max-[900px]:pb-[100px] lg:p-6 lg:pt-6 lg:pb-[108px]",
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
                div {
                    class: "participants-grid m-auto flex h-full min-h-0 w-full flex-wrap items-stretch justify-center gap-4 max-[900px]:gap-3",
                    "data-count": "{grid_layout.count}",
                    "data-columns": "{grid_layout.columns}",
                    "data-rows": "{grid_layout.rows}",
                    "data-focused": if focused { "true" } else { "false" },
                    style: "{grid_layout.style}",
                    for tile in participant_tiles {
                        VoiceParticipantTile {
                            key: "{tile.key}",
                            focused: active_focus_tile_key.as_deref() == Some(tile.key.as_str()),
                            speaking: tile.speaking,
                            media: tile.media,
                            participant: tile.participant.clone(),
                            on_toggle_focus: {
                                let focus_tile_key = tile.key.clone();
                                let focus_user_id = tile.participant.user_id.clone();
                                let focus_media = tile.media;
                                let tile_focused = active_focus_tile_key.as_deref() == Some(focus_tile_key.as_str());
                                move |_| {
                                    if tile_focused {
                                        info!(
                                            user_id = %focus_user_id,
                                            tile_key = %focus_tile_key,
                                            media = focus_media.log_kind(),
                                            "closed focused voice participant tile"
                                        );
                                        focused_tile_key.set(None);
                                    } else {
                                        info!(
                                            user_id = %focus_user_id,
                                            tile_key = %focus_tile_key,
                                            media = focus_media.log_kind(),
                                            "opened focused voice participant tile"
                                        );
                                        focused_tile_key.set(Some(focus_tile_key.clone()));
                                    }
                                }
                            },
                            on_open_user_menu: move |(name, user_id, x, y)| {
                                open_voice_user_menu(&mut open_user_menu, name, user_id, x, y);
                            },
                            on_toggle_user_menu: move |(name, user_id, x, y)| {
                                toggle_voice_user_menu(&mut open_user_menu, name, user_id, x, y);
                            },
                        }
                    }
                }
                if focused {
                    ParticipantFocusStrip {
                        tiles: focus_strip_items,
                        active_tile_key: active_focus_tile_key.clone(),
                        on_select: move |selection: FocusStripSelection| {
                            let target_tile_key = selection.tile_key;
                            let target_user_id = selection.user_id;
                            let target_media = selection.media;
                            if focused_tile_key().as_deref() != Some(target_tile_key.as_str()) {
                                info!(
                                    user_id = %target_user_id,
                                    tile_key = %target_tile_key,
                                    media = target_media.log_kind(),
                                    "opened focused voice participant tile from focus strip"
                                );
                                focused_tile_key.set(Some(target_tile_key));
                            }
                        },
                    }
                }
                button {
                    r#type: "button",
                    disabled: !focused && preferred_focus_tile_key.is_none(),
                    class: "voice-display-mode-button group z-40 flex h-11 w-11 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/85 text-zinc-300 shadow-[0_18px_50px_rgba(0,0,0,0.38)] backdrop-blur-xl transition-[background-color,border-color,color,transform,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-accent/35 hover:bg-accent/10 hover:text-zinc-100 disabled:cursor-default disabled:opacity-50",
                    "aria-label": display_mode_label,
                    onclick: {
                        let next_focus_tile_key = preferred_focus_tile_key.clone();
                        move |event| {
                            event.stop_propagation();
                            if focused {
                                info!("closed focused voice participant tile from display mode button");
                                focused_tile_key.set(None);
                            } else if let Some(tile_key) = next_focus_tile_key.clone() {
                                info!(
                                    tile_key = %tile_key,
                                    "opened focused voice participant tile from display mode button"
                                );
                                focused_tile_key.set(Some(tile_key));
                            }
                        }
                    },
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] right-0 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100",
                        "{display_mode_label}"
                    }
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                        if focused {
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 9H5V5m10 4h4V5M9 15H5v4m10-4h4v4" }
                        } else {
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 3H5a2 2 0 0 0-2 2v3m13-5h3a2 2 0 0 1 2 2v3M8 21H5a2 2 0 0 1-2-2v-3m13 5h3a2 2 0 0 0 2-2v-3" }
                        }
                    }
                }
            }
            if let Some(menu) = open_user_menu() {
                UserContextMenu {
                    name: menu.name,
                    is_self: menu.user_id == current_user_id,
                    can_kick_voice,
                    volume: user_volumes().get(&menu.user_id).copied().unwrap_or(100),
                    x: menu.x,
                    y: menu.y,
                    on_volume_change: move |vol: u32| {
                        user_volumes.write().insert(menu.user_id.clone(), vol);
                        playback.set_user_volume(&menu.user_id, vol);
                    },
                    on_kick_voice: move |_| {
                        if let Some(ref uid) = kick_user_id {
                            open_user_menu.set(None);
                            voice.kick_member(
                                kick_server_id.clone(),
                                kick_room_id.clone(),
                                uid.clone(),
                            );
                        }
                    },
                }
            }
        }
    }
}

fn open_voice_user_menu(
    open_user_menu: &mut Signal<Option<UserMenuState>>,
    name: String,
    user_id: String,
    x: f64,
    y: f64,
) {
    info!(user_id = %user_id, "opened voice participant user menu");
    open_user_menu.set(Some(UserMenuState {
        name,
        user_id,
        x,
        y,
    }));
}

fn toggle_voice_user_menu(
    open_user_menu: &mut Signal<Option<UserMenuState>>,
    name: String,
    user_id: String,
    x: f64,
    y: f64,
) {
    let already_open_for_user = open_user_menu
        .read()
        .as_ref()
        .is_some_and(|menu| menu.user_id == user_id);

    if already_open_for_user {
        info!(user_id = %user_id, "closed voice participant user menu from tile menu button");
        open_user_menu.set(None);
    } else {
        info!(user_id = %user_id, "opened voice participant user menu from tile menu button");
        open_user_menu.set(Some(UserMenuState {
            name,
            user_id,
            x,
            y,
        }));
    }
}
