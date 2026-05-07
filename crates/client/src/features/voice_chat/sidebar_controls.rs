//! Sidebar voice connection controls.

use std::rc::Rc;

use dioxus::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::realtime::RealtimeHandle;

use super::realtime;
use super::state::{VoiceConnectionHandle, VoiceConnectionState};

/// Renders animated sidebar voice controls for the active voice connection.
#[component]
pub(crate) fn SidebarVoiceControls() -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let realtime_handle = use_context::<RealtimeHandle>();
    let mut output_muted = use_signal(|| false);
    let state = voice.state();
    let microphone_status = microphone.status();
    let microphone_level = microphone.level();
    let visible = state.shows_sidebar_controls();
    let target = state.active_target();
    let room_name = target
        .as_ref()
        .map(|target| target.room_name.clone())
        .unwrap_or_else(|| "Нет активной комнаты".to_owned());
    let status = match &state {
        VoiceConnectionState::Disconnected => "не подключен",
        VoiceConnectionState::Connecting { .. } => "подключаемся",
        VoiceConnectionState::Connected { participants, .. } => {
            if participants.len() == 1 {
                "1 участник"
            } else {
                "в голосовой комнате"
            }
        }
        VoiceConnectionState::Disconnecting { .. } => "отключаемся",
        VoiceConnectionState::Error { .. } => "нужна повторная попытка",
    };
    let error = match &state {
        VoiceConnectionState::Error { message, .. } => Some(message.clone()),
        _ => None,
    };
    let microphone_live = matches!(microphone_status, MicrophoneStatus::Live);
    let microphone_starting = matches!(microphone_status, MicrophoneStatus::Starting);
    let microphone_speaking = microphone_live && microphone_level.active;
    let microphone_level_width =
        (microphone_level.rms / microphone_level.threshold.max(0.001)).clamp(0.08, 1.0) * 100.0;
    let microphone_label = match microphone_status {
        MicrophoneStatus::Idle => "Включить микрофон",
        MicrophoneStatus::Starting => "Запрашиваем микрофон",
        MicrophoneStatus::Live => "Выключить микрофон",
        MicrophoneStatus::PermissionDenied => "Доступ к микрофону запрещен",
        MicrophoneStatus::Error(_) => "Микрофон недоступен",
    };
    let output_label = if output_muted() {
        "Включить звук"
    } else {
        "Отключить звук"
    };
    let microphone_button_class = if microphone_speaking {
        "relative flex h-9 items-center justify-center overflow-hidden rounded-xl border border-emerald-300/80 bg-emerald-500/20 text-emerald-50 shadow-[0_0_0_1px_rgba(52,211,153,.25),0_10px_28px_rgba(16,185,129,.20)] transition-[background,border-color,color,transform,box-shadow,opacity] duration-150 hover:-translate-y-px hover:border-emerald-300 hover:bg-emerald-500/25 disabled:cursor-wait disabled:opacity-60"
    } else if microphone_live {
        "flex h-9 items-center justify-center rounded-xl border border-emerald-500/35 bg-emerald-500/15 text-emerald-100 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-emerald-400/45 hover:bg-emerald-500/20 disabled:cursor-wait disabled:opacity-60"
    } else if matches!(
        microphone_status,
        MicrophoneStatus::PermissionDenied | MicrophoneStatus::Error(_)
    ) {
        "flex h-9 items-center justify-center rounded-xl border border-red-500/25 bg-red-500/10 text-red-200 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-red-500/35 hover:bg-red-500/15 disabled:cursor-wait disabled:opacity-60"
    } else {
        "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-60"
    };
    let output_button_class = if output_muted() {
        "flex h-9 items-center justify-center rounded-xl border border-blue-500/30 bg-blue-500/10 text-blue-100 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-blue-400/40 hover:bg-blue-500/15"
    } else {
        "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900"
    };
    let toggle_microphone = microphone.clone();
    let leave_microphone = microphone.clone();
    let target_for_microphone = target.clone();

    rsx! {
        div { class: "sidebar-voice-presence", "data-sidebar-voice-open": if visible { "true" } else { "false" },
            div { class: "sidebar-voice-presence-inner relative mb-2 rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-2.5",
                div { class: "mb-2 flex items-start gap-2",
                    div { class: "min-w-0 flex-1",
                        div { class: "truncate text-[11px] font-medium text-zinc-100", "{room_name}" }
                        div { class: "mt-0.5 flex min-w-0 items-center gap-1.5 text-[11px] text-zinc-500",
                            svg { class: "h-3.5 w-3.5 shrink-0 text-zinc-600", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                            }
                            span { class: "truncate", "{status}" }
                        }
                    }
                }
                div { class: "grid grid-cols-4 gap-2",
                    button {
                        r#type: "button",
                        disabled: microphone_starting || target_for_microphone.is_none(),
                        class: microphone_button_class,
                        "aria-label": microphone_label,
                        onclick: move |_| {
                            let Some(target) = target_for_microphone.clone() else {
                                return;
                            };
                            let send_realtime = realtime_handle.clone();
                            toggle_microphone.toggle(Rc::new(move |frame| {
                                let frame_realtime = send_realtime.clone();
                                let frame_server_id = target.server_id.clone();
                                let frame_room_id = target.room_id.clone();
                                spawn_local(async move {
                                    if let Err(error) = realtime::send_voice_frame(
                                        &frame_realtime,
                                        &frame_server_id,
                                        &frame_room_id,
                                        frame,
                                    )
                                    .await
                                    {
                                        web_sys::console::warn_1(&JsValue::from_str(&format!(
                                            "failed to send encoded voice frame: {error}"
                                        )));
                                    }
                                });
                            }));
                        },
                        if microphone_live {
                            span {
                                class: "pointer-events-none absolute inset-x-1.5 bottom-1 h-1 overflow-hidden rounded-full bg-zinc-950/60",
                                span {
                                    class: if microphone_speaking {
                                        "block h-full rounded-full bg-emerald-300 transition-[width] duration-100"
                                    } else {
                                        "block h-full rounded-full bg-emerald-700/45 transition-[width] duration-100"
                                    },
                                    style: "width: {microphone_level_width}%;",
                                }
                            }
                            svg { class: "relative z-10 h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                            }
                        } else {
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 9.34V7a3 3 0 0 0-5.68-1.34M9 9v2a3 3 0 0 0 5.12 2.12M19 11a7 7 0 0 1-7 7m0 0v3m0-3a7 7 0 0 1-7-7m3 10h8M3 3l18 18" }
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: output_button_class,
                        "aria-label": output_label,
                        onclick: move |_| output_muted.set(!output_muted()),
                        if output_muted() {
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m3 3 18 18M9.75 9.75 10.5 9v6l-2.25-2.25H5.25A1.5 1.5 0 0 1 3.75 11.25v-1.5m12.713-1.462a5.25 5.25 0 0 1 0 7.424M19.114 5.636a9 9 0 0 1 0 12.728M10.5 4.5 7.5 7.5" }
                            }
                        } else {
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.114 5.636a9 9 0 0 1 0 12.728M16.463 8.288a5.25 5.25 0 0 1 0 7.424M6.75 8.25 10.5 4.5v15l-3.75-3.75H3.75A1.5 1.5 0 0 1 2.25 14.25v-4.5A1.5 1.5 0 0 1 3.75 8.25h3Z" }
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900",
                        "aria-label": "Параметры голосового чата",
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.5 6h9.75M10.5 6a1.5 1.5 0 1 1-3 0m3 0a1.5 1.5 0 1 0-3 0M3.75 6H7.5m9 6h3.75m-3.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-9.75 0h9.75m-3 6h9.75m-9.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-3.75 0H7.5" }
                        }
                    }
                    button {
                        r#type: "button",
                        disabled: matches!(state, VoiceConnectionState::Disconnecting { .. }),
                        class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-red-500/35 hover:bg-red-500/10 hover:text-red-200 disabled:cursor-wait disabled:opacity-60",
                        "aria-label": "Выйти из голосового чата",
                        onclick: move |_| {
                            leave_microphone.stop();
                            voice.leave();
                        },
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                        }
                    }
                }
                if let Some(error) = error {
                    p { class: "mt-2 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[11px] leading-4 text-red-200", "{error}" }
                }
            }
        }
    }
}
