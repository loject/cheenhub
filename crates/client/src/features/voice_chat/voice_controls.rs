//! Floating voice room controls component.

use std::rc::Rc;

use dioxus::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::realtime::RealtimeHandle;

use super::realtime;
use super::state::{VoiceConnectionHandle, VoiceConnectionState};

/// Renders floating controls for the active voice room.
#[component]
pub(crate) fn VoiceControls(server_id: String, room_id: String) -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let realtime_handle = use_context::<RealtimeHandle>();
    let state = voice.state();
    let microphone_status = microphone.status();
    let microphone_level = microphone.level();
    let is_active_room = state.is_active_room(&server_id, &room_id);
    let is_leaving = matches!(state, VoiceConnectionState::Disconnecting { .. });
    let microphone_live = matches!(microphone_status, MicrophoneStatus::Live);
    let microphone_starting = matches!(microphone_status, MicrophoneStatus::Starting);
    let microphone_speaking = microphone_live && microphone_level.active;
    let microphone_level_height =
        (microphone_level.rms / microphone_level.threshold.max(0.001)).clamp(0.08, 1.0) * 100.0;
    let microphone_label = match microphone_status {
        MicrophoneStatus::Idle => "Включить микрофон",
        MicrophoneStatus::Starting => "Запрашиваем микрофон",
        MicrophoneStatus::Live => "Выключить микрофон",
        MicrophoneStatus::PermissionDenied => "Доступ к микрофону запрещен",
        MicrophoneStatus::Error(_) => "Микрофон недоступен",
    };
    let toggle_microphone = microphone.clone();
    let leave_microphone = microphone.clone();

    if !is_active_room {
        return rsx! {};
    }

    rsx! {
        div { class: "voice-floating-controls pointer-events-none absolute inset-x-0 bottom-0 z-50 flex justify-center p-5",
            div { class: "pointer-events-auto flex items-center gap-3 rounded-[24px] border border-zinc-800 bg-zinc-950/85 p-2 shadow-[0_18px_50px_rgba(0,0,0,0.38)] drop-shadow-[0_12px_24px_rgba(0,0,0,.28)] backdrop-blur-xl",
                button {
                    r#type: "button",
                    disabled: microphone_starting,
                    class: if microphone_speaking {
                        "group relative flex h-14 w-14 items-center justify-center overflow-hidden rounded-xl border border-emerald-300/80 bg-emerald-500/20 text-emerald-50 shadow-[0_0_0_1px_rgba(52,211,153,.25),0_14px_36px_rgba(16,185,129,.22)] transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-emerald-300 hover:bg-emerald-500/25 disabled:cursor-wait disabled:opacity-60"
                    } else if microphone_live {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-emerald-500/35 bg-emerald-500/15 text-emerald-100 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-emerald-400/45 hover:bg-emerald-500/20 disabled:cursor-wait disabled:opacity-60"
                    } else {
                        "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-200 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-zinc-700 hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-60"
                    },
                    "aria-label": microphone_label,
                    onclick: move |_| {
                        let send_realtime = realtime_handle.clone();
                        let send_server_id = server_id.clone();
                        let send_room_id = room_id.clone();
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
                    class: "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-200 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-zinc-700 hover:bg-zinc-900",
                    "aria-label": "Экран",
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Экран" }
                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                        path { stroke_linecap: "round", d: "M8 20h8m-4-4v4" }
                    }
                }
                button {
                    r#type: "button",
                    disabled: is_leaving,
                    class: "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-300 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-red-500/30 hover:bg-red-500/10 hover:text-red-200 disabled:cursor-wait disabled:opacity-60",
                    "aria-label": "Выйти из голосовой комнаты",
                    onclick: move |_| {
                        leave_microphone.stop();
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
