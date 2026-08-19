//! Voice participant tile component.

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::*;

use crate::features::app::components::avatar::{UserAvatar, use_avatar_seed};

use super::video_streams::{ParticipantVideoCanvas, ParticipantVideoSource};

/// Видеороль или fallback-содержимое плитки участника.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VoiceParticipantTileMedia {
    /// Плитка без видеопотока, показывающая аватар участника.
    Avatar,
    /// Плитка камеры участника.
    Camera,
    /// Плитка демонстрации экрана участника.
    ScreenShare,
}

impl VoiceParticipantTileMedia {
    /// Возвращает стабильное имя медиароли для клиентских логов.
    pub(crate) fn log_kind(self) -> &'static str {
        match self {
            Self::Avatar => "avatar",
            Self::Camera => "camera",
            Self::ScreenShare => "screen_share",
        }
    }
}

/// Renders one voice room participant.
#[component]
pub(crate) fn VoiceParticipantTile(
    participant: VoiceRoomParticipant,
    focused: bool,
    speaking: bool,
    media: VoiceParticipantTileMedia,
    on_toggle_focus: EventHandler<()>,
    on_open_user_menu: EventHandler<(String, String, f64, f64)>,
    on_toggle_user_menu: EventHandler<(String, String, f64, f64)>,
) -> Element {
    use_avatar_seed(participant.user_id.clone());
    let screen_sharing = media == VoiceParticipantTileMedia::ScreenShare;
    let camera_on = matches!(media, VoiceParticipantTileMedia::Camera);
    let video_on = screen_sharing || camera_on;
    let focused_video = focused && video_on;
    let tile_class = if focused_video && speaking {
        "user-tile relative overflow-hidden rounded-lg border border-emerald-400/55 bg-zinc-950 p-0 shadow-none transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-emerald-300/65"
    } else if focused_video && screen_sharing {
        "user-tile relative overflow-hidden rounded-lg border border-sky-400/25 bg-zinc-950 p-0 shadow-none transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-sky-300/35"
    } else if focused_video {
        "user-tile relative overflow-hidden rounded-lg border border-cyan-400/25 bg-zinc-950 p-0 shadow-none transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-cyan-300/35"
    } else if screen_sharing && speaking {
        "user-tile relative overflow-hidden rounded-[20px] bg-zinc-950 p-4 shadow-[0_0_0_2px_rgba(52,211,153,.70)] transition-[background-color,transform,box-shadow] duration-200 ease-in-out"
    } else if screen_sharing {
        "user-tile relative overflow-hidden rounded-[20px] border border-sky-400/35 bg-zinc-950 p-4 shadow-[0_18px_70px_rgba(2,132,199,.16)] transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-sky-300/50"
    } else if camera_on && speaking {
        "user-tile relative overflow-hidden rounded-[20px] bg-zinc-950 p-4 shadow-[0_0_0_2px_rgba(52,211,153,.70)] transition-[background-color,transform,box-shadow] duration-200 ease-in-out"
    } else if camera_on {
        "user-tile relative overflow-hidden rounded-[20px] border border-cyan-400/35 bg-zinc-950 p-4 shadow-[0_18px_70px_rgba(6,182,212,.14)] transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-cyan-300/50"
    } else if speaking {
        "user-tile relative overflow-hidden rounded-[20px] bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 shadow-[0_0_0_2px_rgba(52,211,153,.70)] transition-[background-color,transform,box-shadow] duration-200 ease-in-out"
    } else {
        "user-tile relative overflow-hidden rounded-[20px] border border-accent/25 bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 transition-[border-color,background,transform,box-shadow] duration-200 ease-in-out hover:border-white/15"
    };

    rsx! {
        article {
            "data-avatar": "",
            "data-focused": if focused { "true" } else { "false" },
            "data-speaking": if speaking { "true" } else { "false" },
            "data-camera": if camera_on { "true" } else { "false" },
            "data-video": if video_on { "true" } else { "false" },
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
                ParticipantVideoCanvas {
                    user_id: participant.user_id.clone(),
                    source: ParticipantVideoSource::ScreenShare,
                }
                if !focused {
                    div { class: "pointer-events-none absolute inset-0 z-[1] bg-gradient-to-t from-zinc-950/65 via-transparent to-zinc-950/20" }
                }
            } else if camera_on {
                ParticipantVideoCanvas {
                    user_id: participant.user_id.clone(),
                    source: ParticipantVideoSource::Camera,
                }
                if !focused {
                    div { class: "pointer-events-none absolute inset-0 z-[1] bg-gradient-to-t from-zinc-950/65 via-transparent to-zinc-950/20" }
                }
            }
            button {
                r#type: "button",
                class: if focused_video { "absolute inset-0 z-10 rounded-lg focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-accent" } else { "absolute inset-0 z-10 rounded-[20px] focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-accent" },
                "aria-label": if focused { "Вернуться к сетке участников" } else { "Сфокусировать участника" },
                onclick: move |event| {
                    event.stop_propagation();
                    on_toggle_focus.call(());
                },
            }
            div { class: "absolute right-3 top-3 z-30",
                button {
                    r#type: "button",
                    class: "flex h-11 w-11 items-center justify-center rounded-xl bg-zinc-950/88 text-zinc-400 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,color,box-shadow] duration-150 hover:bg-zinc-900 hover:text-zinc-100 active:scale-[0.96]",
                    "aria-label": "Меню пользователя",
                    onclick: {
                        let nickname = participant.nickname.clone();
                        let user_id = participant.user_id.clone();
                        move |event| {
                            event.stop_propagation();
                            let point = event.client_coordinates();
                            on_toggle_user_menu.call((nickname.clone(), user_id.clone(), point.x, point.y));
                        }
                    },
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                    }
                }
            }
            if !screen_sharing && !camera_on {
                div { class: "relative z-10 flex min-h-full flex-col items-center justify-center text-center",
                    UserAvatar {
                        nickname: participant.nickname.clone(),
                        avatar_url: participant.avatar_url.clone(),
                        class: "flex h-20 w-20 items-center justify-center rounded-full bg-zinc-900/80 text-[26px] font-bold text-zinc-100 shadow-[0_16px_36px_rgba(0,0,0,.24)] outline outline-1 -outline-offset-1 outline-white/10".to_owned(),
                        avatar_seed: Some(participant.user_id.clone()),
                    }
                }
            }
            div { class: "absolute inset-x-4 bottom-4 z-20 flex justify-center",
                div { class: "max-w-full rounded-xl border border-zinc-800 bg-zinc-950/80 px-3 py-1.5 text-[13px] font-semibold text-zinc-100 shadow-[0_12px_30px_rgba(0,0,0,.32)] backdrop-blur-xl",
                    div { class: "flex min-w-0 items-center gap-1.5",
                        if speaking {
                            span { class: "h-2 w-2 shrink-0 rounded-full bg-emerald-400 shadow-[0_0_0_3px_rgba(52,211,153,.12)]" }
                            span { class: "sr-only", "Говорит" }
                        }
                        if screen_sharing {
                            svg { class: "h-3.5 w-3.5 shrink-0 opacity-80", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 20h8m-4-4v-9m0 0-3 3m3-3 3 3" }
                            }
                        } else if camera_on {
                            svg { class: "h-3.5 w-3.5 shrink-0 opacity-80", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m15 10 4.55-2.28A1 1 0 0 1 21 8.62v6.76a1 1 0 0 1-1.45.9L15 14m0-4v4m0-4a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2" }
                            }
                        }
                        div { class: "truncate", "{participant.nickname}" }
                    }
                }
            }
        }
    }
}
