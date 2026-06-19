//! Floating voice room controls component.

use std::rc::Rc;

use dioxus::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::AudioPlaybackHandle;
use crate::features::camera::{CameraHandle, CameraStatus};
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::realtime::RealtimeHandle;
use crate::features::screen_share::{ScreenShareHandle, ScreenShareStatus};

use super::realtime;
use super::state::{VoiceConnectionHandle, VoiceConnectionState};
use super::video_streams::{ParticipantVideoFrame, ParticipantVideoHandle, ParticipantVideoSource};

/// Renders floating controls for the active voice room.
#[component]
pub(crate) fn VoiceControls(server_id: String, room_id: String) -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let camera = use_context::<CameraHandle>();
    let screen_share = use_context::<ScreenShareHandle>();
    let playback = use_context::<AudioPlaybackHandle>();
    let realtime_handle = use_context::<RealtimeHandle>();
    let current_user_id = use_context::<CurrentUserContext>().require_user().id;
    let participant_video = use_context::<ParticipantVideoHandle>();
    let state = voice.state();
    let microphone_status = microphone.status();
    let camera_status = camera.status();
    let screen_share_status = screen_share.status();
    let microphone_level = microphone.level();
    let is_active_room = state.is_active_room(&server_id, &room_id);
    let is_leaving = matches!(state, VoiceConnectionState::Disconnecting { .. });
    let media_controls_enabled = matches!(state, VoiceConnectionState::Connected { .. });
    let output_muted = playback.is_muted();
    let microphone_live = matches!(microphone_status, MicrophoneStatus::Live);
    let microphone_starting = matches!(microphone_status, MicrophoneStatus::Starting);
    let microphone_speaking = microphone_live && microphone_level.active;
    let camera_live = matches!(camera_status, CameraStatus::Live);
    let camera_starting = matches!(camera_status, CameraStatus::Starting);
    let screen_share_live = matches!(screen_share_status, ScreenShareStatus::Live);
    let screen_share_starting = matches!(screen_share_status, ScreenShareStatus::Starting);
    let microphone_level_height =
        (microphone_level.rms / microphone_level.threshold.max(0.001)).clamp(0.08, 1.0) * 100.0;
    let microphone_label = match &microphone_status {
        MicrophoneStatus::Idle => "Включить микрофон".to_owned(),
        MicrophoneStatus::Starting => "Запрашиваем микрофон".to_owned(),
        MicrophoneStatus::Live => "Выключить микрофон".to_owned(),
        MicrophoneStatus::PermissionDenied => "Доступ к микрофону запрещен".to_owned(),
        MicrophoneStatus::Error(message) => message.clone(),
    };
    let microphone_label = if output_muted {
        "Включить микрофон (включит звук)".to_owned()
    } else {
        microphone_label
    };
    let camera_label = match camera_status {
        CameraStatus::Idle => "Включить камеру",
        CameraStatus::Starting => "Запрашиваем камеру",
        CameraStatus::Live => "Выключить камеру",
        CameraStatus::PermissionDenied => "Доступ к камере запрещен",
        CameraStatus::Error(_) => "Камера недоступна",
    };
    let screen_share_label = match screen_share_status {
        ScreenShareStatus::Idle => "Начать демонстрацию экрана",
        ScreenShareStatus::Starting => "Запрашиваем демонстрацию экрана",
        ScreenShareStatus::Live => "Остановить демонстрацию экрана",
        ScreenShareStatus::PermissionDenied => "Доступ к экрану запрещен",
        ScreenShareStatus::Error(_) => "Демонстрация экрана недоступна",
    };
    let toggle_microphone = microphone.clone();
    let toggle_camera = camera.clone();
    let toggle_screen_share = screen_share.clone();
    let leave_microphone = microphone.clone();
    let leave_camera = camera.clone();
    let leave_screen_share = screen_share.clone();
    let unmute_playback = playback.clone();
    let microphone_realtime_handle = realtime_handle.clone();
    let microphone_server_id = server_id.clone();
    let microphone_room_id = room_id.clone();
    let camera_realtime_handle = realtime_handle.clone();
    let camera_server_id = server_id.clone();
    let camera_room_id = room_id.clone();
    let camera_current_user_id = current_user_id.clone();
    let camera_participant_video = participant_video.clone();
    let screen_realtime_handle = realtime_handle.clone();
    let screen_server_id = server_id.clone();
    let screen_room_id = room_id.clone();

    if !is_active_room {
        return rsx! {};
    }

    rsx! {
        div { class: "voice-floating-controls pointer-events-none absolute inset-x-0 bottom-0 z-50 flex justify-center p-5",
            div { class: "pointer-events-auto flex items-center gap-3 rounded-[24px] border border-zinc-800 bg-zinc-950/85 p-2 shadow-[0_18px_50px_rgba(0,0,0,0.38)] drop-shadow-[0_12px_24px_rgba(0,0,0,.28)] backdrop-blur-xl",
                button {
                    r#type: "button",
                    disabled: !media_controls_enabled || microphone_starting,
                    class: if microphone_speaking {
                        "group relative flex h-14 w-14 items-center justify-center overflow-hidden rounded-xl border border-emerald-300/80 bg-emerald-500/20 text-emerald-50 shadow-[0_0_0_1px_rgba(52,211,153,.25),0_14px_36px_rgba(16,185,129,.22)] transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-emerald-300 hover:bg-emerald-500/25 disabled:cursor-wait disabled:opacity-60"
                    } else if microphone_live {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-emerald-500/35 bg-emerald-500/15 text-emerald-100 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-emerald-400/45 hover:bg-emerald-500/20 disabled:cursor-wait disabled:opacity-60"
                    } else {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-200 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-zinc-700 hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-60"
                    },
                    "aria-label": microphone_label,
                    onclick: move |_| {
                        if !media_controls_enabled {
                            return;
                        }
                        if output_muted {
                            unmute_playback.set_muted(false);
                        }
                        let send_realtime = microphone_realtime_handle.clone();
                        let send_server_id = microphone_server_id.clone();
                        let send_room_id = microphone_room_id.clone();
                        toggle_microphone.toggle(Rc::new(move |frame| {
                            let frame_realtime = send_realtime.clone();
                            let frame_server_id = send_server_id.clone();
                            let frame_room_id = send_room_id.clone();
                            spawn_local(async move {
                                if let Err(error) = realtime::send_voice_frame(
                                    &frame_realtime,
                                    &frame_server_id,
                                    &frame_room_id,
                                    frame,
                                )
                                .await
                                {
                                    warn!(
                                        %error,
                                        server_id = %frame_server_id,
                                        room_id = %frame_room_id,
                                        "failed to send encoded voice frame"
                                    );
                                }
                            });
                        }));
                    },
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "{microphone_label}" }
                    if microphone_live {
                        span {
                            class: "pointer-events-none absolute inset-x-2 bottom-2 h-1 overflow-hidden rounded-full bg-zinc-950/60",
                            span {
                                class: if microphone_speaking {
                                    "block h-full rounded-full bg-emerald-300 transition-[width] duration-100"
                                } else {
                                    "block h-full rounded-full bg-emerald-700/45 transition-[width] duration-100"
                                },
                                style: "width: {microphone_level_height}%;",
                            }
                        }
                    }
                    if microphone_live {
                        svg { class: "relative z-10 h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                        }
                    } else {
                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 9.34V7a3 3 0 0 0-5.68-1.34M9 9v2a3 3 0 0 0 5.12 2.12M19 11a7 7 0 0 1-7 7m0 0v3m0-3a7 7 0 0 1-7-7m3 10h8M3 3l18 18" }
                        }
                    }
                }
                button {
                    r#type: "button",
                    disabled: !media_controls_enabled || camera_starting,
                    class: if camera_live {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-cyan-400/45 bg-cyan-500/15 text-cyan-100 shadow-[0_0_0_1px_rgba(34,211,238,.16),0_14px_36px_rgba(6,182,212,.14)] transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-cyan-300/55 hover:bg-cyan-500/20 disabled:cursor-wait disabled:opacity-60"
                    } else if matches!(camera_status, CameraStatus::PermissionDenied | CameraStatus::Error(_)) {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-amber-500/35 bg-amber-500/10 text-amber-100 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-amber-400/45 hover:bg-amber-500/15 disabled:cursor-wait disabled:opacity-60"
                    } else {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-200 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-zinc-700 hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-60"
                    },
                    "aria-label": camera_label,
                    onclick: move |_| {
                        if !media_controls_enabled {
                            return;
                        }
                        let send_realtime = camera_realtime_handle.clone();
                        let send_server_id = camera_server_id.clone();
                        let send_room_id = camera_room_id.clone();
                        let local_user_id = camera_current_user_id.clone();
                        let local_video = camera_participant_video.clone();
                        toggle_camera.toggle(Rc::new(move |frame| {
                            let frame_realtime = send_realtime.clone();
                            let frame_server_id = send_server_id.clone();
                            let frame_room_id = send_room_id.clone();
                            local_video.publish_frame(
                                ParticipantVideoSource::Camera,
                                ParticipantVideoFrame::from_local_camera(
                                    frame_room_id.clone(),
                                    local_user_id.clone(),
                                    frame.clone(),
                                ),
                            );
                            spawn_local(async move {
                                if let Err(error) = realtime::send_camera_frame(
                                    &frame_realtime,
                                    &frame_server_id,
                                    &frame_room_id,
                                    frame,
                                )
                                .await
                                {
                                    warn!(
                                        %error,
                                        server_id = %frame_server_id,
                                        room_id = %frame_room_id,
                                        "failed to send encoded camera frame"
                                    );
                                }
                            });
                        }));
                    },
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "{camera_label}" }
                    if camera_live {
                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m15 10 4.55-2.28A1 1 0 0 1 21 8.62v6.76a1 1 0 0 1-1.45.9L15 14m0-4v4m0-4a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2" }
                        }
                    } else {
                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 3l18 18M15 10l4.55-2.28A1 1 0 0 1 21 8.62v6.76a1 1 0 0 1-.4.8M15 10v1.5M15 14a2 2 0 0 1-2 2H6.5M5 8h8a2 2 0 0 1 1.5.68M3 10v4a2 2 0 0 0 2 2h1" }
                        }
                    }
                }
                button {
                    r#type: "button",
                    disabled: !media_controls_enabled || screen_share_starting,
                    class: if screen_share_live {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-sky-400/45 bg-sky-500/15 text-sky-100 shadow-[0_0_0_1px_rgba(56,189,248,.16),0_14px_36px_rgba(14,165,233,.14)] transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-sky-300/55 hover:bg-sky-500/20 disabled:cursor-wait disabled:opacity-60"
                    } else if matches!(screen_share_status, ScreenShareStatus::PermissionDenied | ScreenShareStatus::Error(_)) {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-amber-500/35 bg-amber-500/10 text-amber-100 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-amber-400/45 hover:bg-amber-500/15 disabled:cursor-wait disabled:opacity-60"
                    } else {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-200 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-zinc-700 hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-60"
                    },
                    "aria-label": screen_share_label,
                    onclick: move |_| {
                        if !media_controls_enabled {
                            return;
                        }
                        let send_realtime = screen_realtime_handle.clone();
                        let send_server_id = screen_server_id.clone();
                        let send_room_id = screen_room_id.clone();
                        toggle_screen_share.toggle(Rc::new(move |frame| {
                            let frame_realtime = send_realtime.clone();
                            let frame_server_id = send_server_id.clone();
                            let frame_room_id = send_room_id.clone();
                            spawn_local(async move {
                                if let Err(error) = realtime::send_screen_frame(
                                    &frame_realtime,
                                    &frame_server_id,
                                    &frame_room_id,
                                    frame,
                                )
                                .await
                                {
                                    warn!(
                                        %error,
                                        server_id = %frame_server_id,
                                        room_id = %frame_room_id,
                                        "failed to send encoded screen frame"
                                    );
                                }
                            });
                        }));
                    },
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "{screen_share_label}" }
                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 20h8m-4-4v-9m0 0-3 3m3-3 3 3" }
                    }
                }
                button {
                    r#type: "button",
                    disabled: is_leaving,
                    class: "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-300 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-red-500/30 hover:bg-red-500/10 hover:text-red-200 disabled:cursor-wait disabled:opacity-60",
                    "aria-label": "Выйти из голосовой комнаты",
                    onclick: move |_| {
                        leave_microphone.stop();
                        leave_camera.stop();
                        leave_screen_share.stop();
                        voice.leave();
                    },
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Выйти" }
                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                    }
                }
            }
        }
    }
}
