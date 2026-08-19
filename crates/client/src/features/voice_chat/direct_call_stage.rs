//! Сцена активного личного звонка с приоритетом собеседника.

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;
use crate::features::app::components::user_context_menu::UserContextMenu;
use crate::features::audio_playback::AudioPlaybackHandle;

use super::video_streams::{
    ParticipantVideoCanvas, ParticipantVideoHandle, ParticipantVideoSource,
};

/// Состояние call-specific сцены личного звонка.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum DirectCallStageStatus {
    /// Собеседник присутствует, медиасоединение готово.
    Connected,
    /// Клиент присоединяется к медиасессии.
    Connecting,
    /// Realtime и медиасессия восстанавливаются после разрыва.
    Recovering,
    /// Локальный пользователь подключён и ждёт собеседника.
    WaitingForPeer,
    /// Подключение не удалось и требует повторной попытки.
    Error {
        /// Понятное пользователю описание ошибки.
        message: String,
    },
}

/// Показывает собеседника как главный объект личного звонка и локальное видео как preview.
#[component]
pub(crate) fn DirectCallStage(
    peer_nickname: String,
    peer_avatar_url: Option<String>,
    peer_user_id: String,
    current_user_id: String,
    participants: Vec<VoiceRoomParticipant>,
    speaking_user_ids: Vec<String>,
    status: DirectCallStageStatus,
    on_retry: EventHandler<()>,
) -> Element {
    let video = use_context::<ParticipantVideoHandle>();
    let playback = use_context::<AudioPlaybackHandle>();
    let mut menu_position = use_signal(|| None::<(f64, f64)>);
    let mut peer_volume = use_signal(|| 100_u32);
    let peer = participants
        .iter()
        .find(|participant| participant.user_id == peer_user_id);
    let peer_nickname = peer
        .map(|participant| participant.nickname.clone())
        .unwrap_or(peer_nickname);
    let peer_avatar_url = peer
        .and_then(|participant| participant.avatar_url.clone())
        .or(peer_avatar_url);
    let camera_user_ids = video.live_user_ids(ParticipantVideoSource::Camera);
    let screen_user_ids = video.live_user_ids(ParticipantVideoSource::ScreenShare);
    let peer_source = if screen_user_ids
        .iter()
        .any(|user_id| user_id == &peer_user_id)
    {
        Some(ParticipantVideoSource::ScreenShare)
    } else if camera_user_ids
        .iter()
        .any(|user_id| user_id == &peer_user_id)
    {
        Some(ParticipantVideoSource::Camera)
    } else {
        None
    };
    let local_camera_live = camera_user_ids
        .iter()
        .any(|user_id| user_id == &current_user_id);
    let peer_speaking = speaking_user_ids
        .iter()
        .any(|user_id| user_id == &peer_user_id);
    let stage_state = stage_state_content(&status, &peer_nickname);

    rsx! {
        div {
            class: "voice-stage direct-call-participant-stage relative flex min-h-0 flex-1 items-center justify-center overflow-hidden px-4 pb-[116px] pt-4 sm:px-6 sm:pt-6",
            onclick: move |_| menu_position.set(None),
            if let Some((title, body, retry)) = stage_state {
                div { class: "voice-empty-state max-w-sm text-center",
                    div { class: "mx-auto grid h-16 w-16 place-items-center rounded-[20px] bg-zinc-900/80 text-zinc-400 shadow-[0_0_0_1px_rgba(255,255,255,.08),0_14px_40px_rgba(0,0,0,.24)]",
                        if retry {
                            svg { class: "h-7 w-7", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M20 11a8.1 8.1 0 1 0-2.4 5.8M20 4v7h-7" }
                            }
                        } else {
                            div { class: "h-7 w-7 animate-spin rounded-full border-2 border-zinc-700 border-t-accent", "aria-hidden": "true" }
                        }
                    }
                    h2 { class: "mt-4 text-balance text-[17px] font-semibold text-zinc-100", "{title}" }
                    p { class: "mt-2 text-pretty text-[13px] leading-6 text-zinc-400", "{body}" }
                    if retry {
                        button {
                            r#type: "button",
                            class: "mt-4 inline-flex min-h-11 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white transition-[scale,background-color] duration-150 ease-out hover:bg-blue-400 active:scale-[0.96]",
                            onclick: move |event| {
                                event.stop_propagation();
                                on_retry.call(());
                            },
                            "Повторить подключение"
                        }
                    }
                }
            } else {
                article {
                    class: if peer_speaking {
                        "direct-call-peer-card relative h-full max-h-[760px] min-h-[260px] w-full max-w-5xl overflow-hidden rounded-[24px] bg-zinc-900/72 shadow-[0_0_0_1px_rgba(96,165,250,.48),0_18px_54px_rgba(0,0,0,.28)] transition-[box-shadow,background-color] duration-200"
                    } else {
                        "direct-call-peer-card relative h-full max-h-[760px] min-h-[260px] w-full max-w-5xl overflow-hidden rounded-[24px] bg-zinc-900/72 shadow-[0_0_0_1px_rgba(255,255,255,.09),0_18px_54px_rgba(0,0,0,.28)] transition-[box-shadow,background-color] duration-200"
                    },
                    if let Some(source) = peer_source {
                        ParticipantVideoCanvas {
                            user_id: peer_user_id.clone(),
                            source,
                        }
                        div { class: "pointer-events-none absolute inset-0 z-[1] bg-gradient-to-t from-zinc-950/70 via-transparent to-zinc-950/20" }
                    } else {
                        div { class: "relative z-10 flex h-full min-h-[260px] flex-col items-center justify-center bg-[radial-gradient(circle_at_center,rgba(255,255,255,.055),transparent_38%)] px-6 text-center",
                            div { class: "grid h-32 w-32 place-items-center rounded-full bg-zinc-950/42 p-1.5 shadow-[0_0_0_1px_rgba(255,255,255,.12),0_18px_50px_rgba(0,0,0,.30)]",
                                UserAvatar {
                                    nickname: peer_nickname.clone(),
                                    avatar_url: peer_avatar_url,
                                    class: "flex h-full w-full items-center justify-center rounded-full bg-zinc-800 text-[38px] font-bold text-zinc-50 shadow-[inset_0_0_0_1px_rgba(255,255,255,.08)]".to_owned(),
                                    avatar_seed: Some(peer_user_id.clone()),
                                }
                            }
                            h2 { class: "mt-5 max-w-full truncate text-[18px] font-semibold text-zinc-100", "{peer_nickname}" }
                            div {
                                class: if peer_speaking {
                                    "mt-2 flex items-center gap-1.5 text-[12px] font-medium text-blue-200"
                                } else {
                                    "invisible mt-2 flex items-center gap-1.5 text-[12px] font-medium text-blue-200"
                                },
                                svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                                }
                                span { "Говорит" }
                            }
                        }
                    }
                    if peer_source.is_some() {
                        div { class: "absolute inset-x-4 bottom-4 z-20 flex justify-center",
                            div { class: "flex max-w-full items-center gap-2 rounded-[14px] bg-zinc-950/82 px-3.5 py-2 text-[13px] font-semibold text-zinc-100 shadow-[0_0_0_1px_rgba(255,255,255,.08),0_12px_30px_rgba(0,0,0,.30)] backdrop-blur-xl",
                                if peer_speaking {
                                    span { class: "h-2 w-2 shrink-0 rounded-full bg-blue-400 shadow-[0_0_0_3px_rgba(96,165,250,.12)]" }
                                    span { class: "sr-only", "Говорит" }
                                }
                                span { class: "truncate", "{peer_nickname}" }
                                if peer_source == Some(ParticipantVideoSource::ScreenShare) {
                                    span { class: "shrink-0 text-[11px] font-medium text-blue-200", "Демонстрация экрана" }
                                }
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "absolute right-3 top-3 z-30 flex h-11 w-11 items-center justify-center rounded-xl bg-zinc-950/82 text-zinc-300 shadow-[0_0_0_1px_rgba(255,255,255,.08),0_10px_28px_rgba(0,0,0,.28)] backdrop-blur-xl transition-[scale,background-color,color,box-shadow] duration-150 hover:bg-zinc-900 hover:text-zinc-100 active:scale-[0.96]",
                        "aria-label": "Настройки звука собеседника",
                        onclick: move |event| {
                            event.stop_propagation();
                            let point = event.client_coordinates();
                            menu_position.set(Some((point.x, point.y)));
                        },
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                        }
                    }
                }
                if local_camera_live {
                    div { class: "direct-call-self-preview absolute right-6 top-[92px] z-20 h-[112px] w-[84px] overflow-hidden rounded-[18px] bg-zinc-950 shadow-[0_0_0_1px_rgba(255,255,255,.12),0_14px_36px_rgba(0,0,0,.38)] sm:h-[156px] sm:w-[116px]",
                        ParticipantVideoCanvas {
                            user_id: current_user_id,
                            source: ParticipantVideoSource::Camera,
                        }
                        span { class: "absolute bottom-2 left-2 z-10 rounded-lg bg-zinc-950/78 px-2 py-1 text-[10px] font-semibold text-zinc-200 backdrop-blur-lg", "Вы" }
                    }
                }
            }
            if let Some((x, y)) = menu_position() {
                UserContextMenu {
                    name: peer_nickname,
                    is_self: false,
                    can_kick_voice: false,
                    volume: peer_volume(),
                    x,
                    y,
                    on_volume_change: move |volume| {
                        peer_volume.set(volume);
                        playback.set_user_volume(&peer_user_id, volume);
                    },
                    on_kick_voice: move |_| {},
                }
            }
        }
    }
}

fn stage_state_content(
    status: &DirectCallStageStatus,
    peer_nickname: &str,
) -> Option<(String, String, bool)> {
    match status {
        DirectCallStageStatus::Connected => None,
        DirectCallStageStatus::Connecting => Some((
            "Соединяем звонок".to_owned(),
            format!("Готовим защищённое медиасоединение с {peer_nickname}."),
            false,
        )),
        DirectCallStageStatus::Recovering => Some((
            "Восстанавливаем связь".to_owned(),
            "Звонок останется открытым, пока CheenHub повторно подключается.".to_owned(),
            false,
        )),
        DirectCallStageStatus::WaitingForPeer => Some((
            "Ждём собеседника".to_owned(),
            format!("{peer_nickname} ещё подключается к медиасессии."),
            false,
        )),
        DirectCallStageStatus::Error { message } => Some((
            "Не удалось восстановить звонок".to_owned(),
            message.clone(),
            true,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{DirectCallStageStatus, stage_state_content};

    #[test]
    fn keeps_connected_stage_free_for_peer_media() {
        assert!(stage_state_content(&DirectCallStageStatus::Connected, "Лиса").is_none());
    }

    #[test]
    fn exposes_retry_only_for_terminal_media_error() {
        let connecting = stage_state_content(&DirectCallStageStatus::Connecting, "Лиса")
            .expect("connecting state should have explanatory content");
        let error = stage_state_content(
            &DirectCallStageStatus::Error {
                message: "Нет связи".to_owned(),
            },
            "Лиса",
        )
        .expect("error state should have explanatory content");

        assert!(!connecting.2);
        assert!(error.2);
        assert_eq!(error.1, "Нет связи");
    }
}
